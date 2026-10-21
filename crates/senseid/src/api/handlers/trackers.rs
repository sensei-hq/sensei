//! Tracker connection routes (spec R8). Every route sits behind
//! `origin_guard::local_caller_guard`; reads included, as they return client
//! issue summaries. Errors are fixed codes, never raw messages.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::api::state::AppState;

use crate::api::handlers::err;
use crate::db::pg_store::trackers::TrackerRow;
use crate::tracker::connect::{ConnectError, connect, disconnect};
use crate::tracker::runtime::sync_stored;
use crate::tracker::status::status;
use crate::tracker::sync::SyncError;
use crate::tracker::usage::{
    sprint_usages, ticket_usage_by_repo, ticket_usages, unlinked_in_sprint,
};
use crate::tracker::{TrackerError, TrackerKind, ValidateError, adapter_for_kind};

/// One-second waits a first sync spends on a running sync before leaving it to the tick.
const FIRST_SYNC_BUSY_RETRIES: u32 = 15 * 60;

/// The house error envelope (`handlers::err`): a status and `{"error": code}`.
/// Helpers return it as their `Err`; handlers turn it into a response at the edge.
type ApiErr = (StatusCode, Json<serde_json::Value>);

fn fail(status: StatusCode, code: &str) -> ApiErr {
    err(status, code)
}

fn internal() -> ApiErr {
    fail(StatusCode::INTERNAL_SERVER_ERROR, "internal")
}

/// The workspace's uuid, or the response to return (404 for an unknown workspace:
/// `resolve_workspace_uuid` accepts any well-formed uuid without checking it).
async fn known_workspace(state: &AppState, id: &str) -> Result<uuid::Uuid, ApiErr> {
    let uuid = crate::api::util::resolve_workspace_uuid(state, id)
        .await
        .map_err(|_| internal())?
        .ok_or_else(|| fail(StatusCode::NOT_FOUND, "workspace_not_found"))?;
    match state.pg.get_workspace(&uuid).await {
        Ok(Some(_)) => Ok(uuid),
        Ok(None) => Err(fail(StatusCode::NOT_FOUND, "workspace_not_found")),
        Err(_) => Err(internal()),
    }
}

/// The connection as the API shows it. Never the token, never the keyring name.
async fn view(state: &AppState, row: &TrackerRow) -> Result<serde_json::Value, ApiErr> {
    let (tasks, sprints) = state.pg.tracker_counts(row.id).await.map_err(|_| internal())?;
    // R9: the stale threshold is the schedule's interval; the row is seeded, so a
    // missing one is a defect, not a reason to guess.
    let interval = state
        .pg
        .load_schedule("tracker_sync")
        .await
        .map_err(|_| internal())?
        .ok_or_else(internal)?
        .interval_secs;
    let st =
        status(row.last_success_at, row.last_error.is_some(), interval as i64, chrono::Utc::now());
    Ok(serde_json::json!({
        "id": row.id,
        "kind": row.kind,
        "base_url": row.base_url,
        "account": row.account,
        "external_scope": row.external_scope,
        "board_ref": row.board_ref,
        "board_name": row.params.get("board_name"),
        "status": st.as_str(),
        "last_success_at": row.last_success_at,
        "last_error": row.last_error,
        "counts": {"tasks": tasks, "sprints": sprints},
    }))
}

/// `GET /api/workspaces/{id}/trackers`
pub(crate) async fn list_trackers(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let workspace = match known_workspace(&state, &id).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    match state.pg.tracker_for_workspace(workspace).await {
        Ok(None) => Json(serde_json::json!([])).into_response(),
        Ok(Some(row)) => match view(&state, &row).await {
            Ok(v) => Json(serde_json::json!([v])).into_response(),
            Err(r) => r.into_response(),
        },
        Err(_) => internal().into_response(),
    }
}

/// `POST /api/workspaces/{id}/trackers/{kind}/connect`
pub(crate) async fn connect_tracker(
    State(state): State<AppState>,
    Path((id, kind)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let workspace = match known_workspace(&state, &id).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let Some(adapter) = TrackerKind::parse(&kind).and_then(adapter_for_kind) else {
        return fail(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_tracker").into_response();
    };
    let params = match adapter.connect_params(&body) {
        Ok(p) => p,
        Err(e) => return fail(StatusCode::UNPROCESSABLE_ENTITY, e.code()).into_response(),
    };
    // Connect stays interactive: no 429 retries during validation.
    let transport = match state.trackers.transports.transport(
        &params.base_url,
        adapter.auth(&params.account, &params.token),
        0,
    ) {
        Ok(t) => t,
        Err(e) => return fail(StatusCode::BAD_GATEWAY, e.code()).into_response(),
    };
    let (base_url, board_ref) = (params.base_url.clone(), params.board_ref.clone());
    let rt = state.trackers.clone();
    let connected = match connect(
        &state.pg,
        rt.secrets.clone(),
        adapter,
        transport.as_ref(),
        workspace,
        params,
    )
    .await
    {
        Ok(c) => c,
        Err(e) => return connect_failure(e, &base_url, &board_ref).into_response(),
    };
    let row = match state.pg.tracker_by_id(connected.tracker_id).await {
        Ok(Some(row)) => row,
        _ => return internal().into_response(),
    };
    // The first sync runs in the background: connect never waits on a sync.
    let (pg, first) = (state.pg.clone(), row.clone());
    tokio::spawn(async move {
        // A sync already running (e.g. on the old credential of a re-connect) is
        // waited out, bounded, rather than dropping the first sync.
        for _ in 0..FIRST_SYNC_BUSY_RETRIES {
            match sync_stored(&pg, &rt, &first).await {
                Err(SyncError::Busy) => tokio::time::sleep(std::time::Duration::from_secs(1)).await,
                Err(e) => {
                    tracing::warn!(tracker_id = %first.id, "tracker first sync: {e:?}");
                    return;
                }
                Ok(_) => return,
            }
        }
        tracing::warn!(tracker_id = %first.id, "tracker first sync: still busy; the scheduled sync will run it");
    });
    match view(&state, &row).await {
        Ok(v) => (
            StatusCode::CREATED,
            Json(serde_json::json!({"tracker": v, "first_sync": {"pending": true}})),
        )
            .into_response(),
        Err(r) => r.into_response(),
    }
}

fn connect_failure(e: ConnectError, base_url: &str, board_ref: &str) -> ApiErr {
    use crate::tracker::TrackerError;
    match e {
        ConnectError::BoardAlreadyConnected { workspace_id, workspace_name } => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "board_already_connected",
                "workspace": {"id": workspace_id, "name": workspace_name},
            })),
        ),
        ConnectError::Validate(ValidateError::BoardNotFound) => (
            StatusCode::NOT_FOUND,
            Json(
                serde_json::json!({"error": "board_not_found", "base_url": base_url, "board_ref": board_ref}),
            ),
        ),
        ConnectError::Validate(ValidateError::BoardProjectMismatch) => {
            fail(StatusCode::UNPROCESSABLE_ENTITY, "board_project_mismatch")
        }
        ConnectError::Validate(ValidateError::Tracker(TrackerError::Unauthorized)) => {
            fail(StatusCode::UNAUTHORIZED, "unauthorized")
        }
        ConnectError::Validate(ValidateError::Tracker(TrackerError::Forbidden)) => {
            fail(StatusCode::FORBIDDEN, "forbidden")
        }
        ConnectError::Validate(ValidateError::Tracker(e)) => {
            fail(StatusCode::BAD_GATEWAY, e.code())
        }
        ConnectError::CredentialStore => {
            fail(StatusCode::INTERNAL_SERVER_ERROR, "credential_store")
        }
        ConnectError::Db(_) => internal(),
    }
}

/// `DELETE /api/workspaces/{id}/trackers`
pub(crate) async fn disconnect_tracker(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let workspace = match known_workspace(&state, &id).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    match disconnect(&state.pg, state.trackers.secrets.clone(), workspace).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => fail(StatusCode::NOT_FOUND, "not_connected").into_response(),
        Err(_) => internal().into_response(),
    }
}

/// `POST /api/workspaces/{id}/trackers/sync`
pub(crate) async fn sync_tracker(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let workspace = match known_workspace(&state, &id).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let row = match state.pg.tracker_for_workspace(workspace).await {
        Ok(Some(row)) => row,
        Ok(None) => return fail(StatusCode::NOT_FOUND, "not_connected").into_response(),
        Err(_) => return internal().into_response(),
    };
    match sync_stored(&state.pg, &state.trackers, &row).await {
        Ok(c) => Json(serde_json::json!({"counts": {
            "sprints": c.sprints, "tasks": c.tasks, "removed": c.removed,
            "links": c.links, "skipped_members": c.skipped_members,
        }}))
        .into_response(),
        Err(SyncError::Busy) => fail(StatusCode::CONFLICT, "busy").into_response(),
        Err(SyncError::Gone) => fail(StatusCode::NOT_FOUND, "not_connected").into_response(),
        Err(SyncError::Failed(e)) => fail(StatusCode::BAD_GATEWAY, e.code()).into_response(),
        Err(SyncError::Db(_)) => internal().into_response(),
    }
}

/// `GET /api/workspaces/{id}/tracker-tasks/{key}`
pub(crate) async fn get_tracker_task(
    State(state): State<AppState>,
    Path((id, key)): Path<(String, String)>,
) -> Response {
    let workspace = match known_workspace(&state, &id).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let task = match state.pg.tracker_task_detail(workspace, &key).await {
        Ok(Some(task)) => task,
        Ok(None) => match discover_on_miss(&state, workspace, &key).await {
            Ok(task) => task,
            Err(r) => return r.into_response(),
        },
        Err(_) => return internal().into_response(),
    };
    // The ticket exists, so the workspace has a tracker.
    let tracker = match tracker_of(&state, workspace).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    let usage = match ticket_usages(&state.pg, tracker.id, Some(&[task.id])).await {
        Ok(mut u) => u.remove(&task.id).unwrap_or_default(),
        Err(_) => return internal().into_response(),
    };
    let by_repo = match ticket_usage_by_repo(&state.pg, tracker.id, task.id).await {
        Ok(r) => r,
        Err(_) => return internal().into_response(),
    };
    let sessions: Vec<serde_json::Value> = task
        .sessions
        .iter()
        .map(|s| {
            let links: Vec<serde_json::Value> = s
                .links
                .iter()
                .map(|l| {
                    serde_json::json!({
                        "source": l.source, "matched_key": l.matched_key, "mentions": l.mentions,
                    })
                })
                .collect();
            serde_json::json!({"id": s.id, "started_at": s.started_at, "links": links})
        })
        .collect();
    Json(serde_json::json!({
        "task": {
            "key": task.key, "summary": task.summary, "issue_type": task.issue_type,
            "status": task.status, "removed": task.removed,
        },
        "usage": usage,
        "by_repo": by_repo,
        "sessions": sessions,
    }))
    .into_response()
}

/// Longest a lookup waits on discovery before answering.
const LOOKUP_DISCOVERY: std::time::Duration = std::time::Duration::from_secs(30);

/// R11 lookup: a key in the board's shape that sensei has not synced is fetched
/// once before answering. Anything else stays `task_not_found`, with no request.
async fn discover_on_miss(
    state: &AppState,
    workspace: uuid::Uuid,
    key: &str,
) -> Result<crate::db::pg_store::trackers::TaskDetail, ApiErr> {
    use crate::tracker::discover::{DiscoverError, Scope};
    let not_found = || fail(StatusCode::NOT_FOUND, "task_not_found");
    let tracker = match state.pg.tracker_for_workspace(workspace).await {
        Ok(Some(t)) => t,
        Ok(None) => return Err(not_found()),
        Err(_) => return Err(internal()),
    };
    let keys = [key.to_string()];
    let run = crate::tracker::runtime::discover_stored(
        &state.pg,
        &state.trackers,
        &tracker,
        Scope::Keys(&keys),
    );
    match tokio::time::timeout(LOOKUP_DISCOVERY, run).await {
        Err(_) => Err(fail(StatusCode::BAD_GATEWAY, TrackerError::Transport.code())),
        Ok(Ok(out)) if !out.inserted.is_empty() => {
            match state.pg.tracker_task_detail(workspace, key).await {
                Ok(Some(task)) => Ok(task),
                Ok(None) => Err(not_found()),
                Err(_) => Err(internal()),
            }
        }
        Ok(Ok(out)) => match out.not_on_board.first() {
            Some(k) => Err((
                StatusCode::NOT_FOUND,
                Json(
                    serde_json::json!({"error": "task_not_on_board", "key": k, "board_ref": tracker.board_ref}),
                ),
            )),
            None => Err(not_found()),
        },
        Ok(Err(DiscoverError::Busy)) => Err(fail(StatusCode::CONFLICT, "busy")),
        Ok(Err(DiscoverError::Failed(e))) => Err(fail(StatusCode::BAD_GATEWAY, e.code())),
        Ok(Err(DiscoverError::Db(_))) => Err(internal()),
        // Paused after a recent failure, or re-connected meanwhile: answer from what is stored.
        Ok(Err(DiscoverError::Paused | DiscoverError::Gone)) => Err(not_found()),
    }
}

/// The workspace's tracker, or the response to return (404 `not_connected`).
async fn tracker_of(state: &AppState, workspace: uuid::Uuid) -> Result<TrackerRow, ApiErr> {
    match state.pg.tracker_for_workspace(workspace).await {
        Ok(Some(row)) => Ok(row),
        Ok(None) => Err(fail(StatusCode::NOT_FOUND, "not_connected")),
        Err(_) => Err(internal()),
    }
}

/// The workspace's tracker for a read route: the workspace must exist and be connected.
async fn connected(state: &AppState, id: &str) -> Result<TrackerRow, ApiErr> {
    let workspace = known_workspace(state, id).await?;
    tracker_of(state, workspace).await
}

/// `?sprint=` as one of the tracker's sprints (404 `sprint_not_found` otherwise).
async fn known_sprint(
    state: &AppState,
    tracker: uuid::Uuid,
    raw: &str,
) -> Result<uuid::Uuid, ApiErr> {
    let not_found = || fail(StatusCode::NOT_FOUND, "sprint_not_found");
    let id: uuid::Uuid = raw.parse().map_err(|_| not_found())?;
    let sprints = state.pg.tracker_sprints(tracker).await.map_err(|_| internal())?;
    sprints.iter().any(|s| s.id == id).then_some(id).ok_or_else(not_found)
}

/// `?sprint=<uuid>&include_removed=<bool>`
#[derive(Debug, serde::Deserialize)]
pub(crate) struct SprintQuery {
    sprint: Option<String>,
    #[serde(default)]
    backlog: bool,
    #[serde(default)]
    include_removed: bool,
}

/// `GET /api/workspaces/{id}/backlog`
pub(crate) async fn get_backlog(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let tracker = match connected(&state, &id).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    match crate::tracker::usage::backlog_usage(&state.pg, tracker.id).await {
        Ok(b) => Json(b).into_response(),
        Err(_) => internal().into_response(),
    }
}

/// `GET /api/workspaces/{id}/sprints`
pub(crate) async fn list_sprints(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let tracker = match connected(&state, &id).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    let sprints = match sprint_usages(&state.pg, tracker.id, chrono::Utc::now()).await {
        Ok(s) => s,
        Err(_) => return internal().into_response(),
    };
    let rows: Vec<serde_json::Value> = sprints
        .into_iter()
        .map(|su| {
            let s = su.sprint;
            let mut row = serde_json::json!({
                "id": s.id, "name": s.name, "state": s.state, "start_at": s.start_at,
                "end_at": s.end_at, "complete_at": s.complete_at, "tasks": s.tasks,
            });
            let parts = match su.parts {
                Ok(p) => serde_json::json!({
                    "candidates_in_window": p.candidates_in_window,
                    "linked_sprint": p.linked_sprint, "linked_other": p.linked_other,
                    "unlinked": p.unlinked, "reason": null,
                }),
                Err(reason) => serde_json::json!({
                    "candidates_in_window": null, "linked_sprint": null,
                    "linked_other": null, "unlinked": null, "reason": reason,
                }),
            };
            if let (Some(row), Some(parts)) = (row.as_object_mut(), parts.as_object()) {
                row.extend(parts.clone());
            }
            row
        })
        .collect();
    Json(rows).into_response()
}

/// `GET /api/workspaces/{id}/tracker-tasks?sprint=&include_removed=`
pub(crate) async fn list_tracker_tasks(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::extract::Query(q): axum::extract::Query<SprintQuery>,
) -> Response {
    let tracker = match connected(&state, &id).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    let sprint = match q.sprint.as_deref() {
        Some(raw) => match known_sprint(&state, tracker.id, raw).await {
            Ok(s) => Some(s),
            Err(r) => return r.into_response(),
        },
        None => None,
    };
    use crate::db::pg_store::tracker_usage::TaskScope;
    let scope = match (sprint, q.backlog) {
        (Some(_), true) => {
            return fail(StatusCode::UNPROCESSABLE_ENTITY, "sprint_or_backlog").into_response();
        }
        (Some(id), false) => TaskScope::Sprint(id),
        (None, true) => TaskScope::Backlog,
        (None, false) => TaskScope::All,
    };
    let tasks = match state.pg.tracker_task_rows(tracker.id, scope, q.include_removed).await {
        Ok(t) => t,
        Err(_) => return internal().into_response(),
    };
    let ids: Vec<uuid::Uuid> = tasks.iter().map(|t| t.id).collect();
    let mut usage = match ticket_usages(&state.pg, tracker.id, Some(&ids)).await {
        Ok(u) => u,
        Err(_) => return internal().into_response(),
    };
    let named = |(id, name): &(uuid::Uuid, String)| serde_json::json!({"id": id, "name": name});
    let rows: Vec<serde_json::Value> = tasks
        .iter()
        .map(|t| {
            serde_json::json!({
                "key": t.key, "summary": t.summary, "issue_type": t.issue_type,
                "status": t.status, "removed": t.removed,
                "current_sprint": t.current_sprint.as_ref().map(named),
                "sprints": t.sprints.iter().map(named).collect::<Vec<_>>(),
                // No linked sessions is a genuinely empty set: zero sessions, null sums.
                "usage": usage.remove(&t.id).unwrap_or_default(),
            })
        })
        .collect();
    Json(rows).into_response()
}

/// `GET /api/workspaces/{id}/sessions/unlinked?sprint=`
pub(crate) async fn list_unlinked_sessions(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::extract::Query(q): axum::extract::Query<SprintQuery>,
) -> Response {
    let tracker = match connected(&state, &id).await {
        Ok(t) => t,
        Err(r) => return r.into_response(),
    };
    let Some(raw) = q.sprint.as_deref() else {
        return fail(StatusCode::UNPROCESSABLE_ENTITY, "sprint_required").into_response();
    };
    let sprint = match known_sprint(&state, tracker.id, raw).await {
        Ok(s) => s,
        Err(r) => return r.into_response(),
    };
    match unlinked_in_sprint(&state.pg, tracker.id, sprint, chrono::Utc::now()).await {
        Ok(Some(sessions)) => Json(
            sessions
                .iter()
                .map(
                    |s| serde_json::json!({"id": s.id, "started_at": s.started_at, "task": s.task}),
                )
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Ok(None) => fail(StatusCode::NOT_FOUND, "sprint_not_found").into_response(),
        Err(_) => internal().into_response(),
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::{Value, json};
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::tracker::TrackerError;
    use crate::tracker::runtime::test_support::{FakeFactory, fake_runtime};
    use crate::tracker::secret::test_support::MemoryStore;
    use crate::tracker::test_support::FakeBoard;

    const SENTINEL: &str = "SENTINEL-TOKEN-XYZ";

    struct Harness {
        app: axum::Router,
        state: crate::api::state::AppState,
        store: Arc<MemoryStore>,
        factory: Arc<FakeFactory>,
        rt: Arc<crate::tracker::runtime::TrackerRuntime>,
    }

    async fn harness() -> Harness {
        let (rt, store, factory) = fake_runtime();
        let ctx = crate::tasks::test_support::make_ctx_with_trackers(rt.clone()).await;
        let state = ctx.app_state.clone();
        Harness { app: crate::api::routes::create_router(state.clone()), state, store, factory, rt }
    }

    fn site() -> String {
        format!("t{}.atlassian.net", uuid::Uuid::new_v4().simple())
    }

    async fn workspace(h: &Harness) -> uuid::Uuid {
        h.state
            .pg
            .create_workspace(&format!("_test:tracker-api:{}", uuid::Uuid::new_v4()), None, None)
            .await
            .unwrap()
    }

    /// One request through the real router, from the local CLI by default.
    async fn call(
        h: &Harness,
        method: &str,
        uri: &str,
        body: Option<Value>,
        origin: Option<&str>,
    ) -> (StatusCode, String) {
        let mut b = Request::builder().method(method).uri(uri).header("host", "127.0.0.1:7744");
        if let Some(o) = origin {
            b = b.header("origin", o);
        }
        let req = match body {
            Some(v) => b
                .header("content-type", "application/json")
                .body(Body::from(v.to_string()))
                .unwrap(),
            None => b.body(Body::empty()).unwrap(),
        };
        let resp = h.app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8_lossy(&bytes).to_string())
    }

    fn json(s: &str) -> Value {
        serde_json::from_str(s).unwrap_or(Value::Null)
    }

    fn connect_body(site: &str) -> Value {
        json!({
            "board_url": format!("https://{site}/jira/software/projects/AUTH/boards/42"),
            "email": "dev@example.com",
            "api_token": SENTINEL,
        })
    }

    async fn connect(h: &Harness, p: uuid::Uuid, site: &str) -> (StatusCode, String) {
        call(
            h,
            "POST",
            &format!("/api/workspaces/{p}/trackers/jira/connect"),
            Some(connect_body(site)),
            None,
        )
        .await
    }

    /// Poll `GET …/trackers` until the status leaves `pending` (≤ 3 s).
    async fn settled(h: &Harness, p: uuid::Uuid) -> Value {
        for _ in 0..300 {
            let (_, body) =
                call(h, "GET", &format!("/api/workspaces/{p}/trackers"), None, None).await;
            let v = json(&body);
            if v[0]["status"] != json!("pending") {
                return v;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("first sync never finished");
    }

    #[tokio::test]
    async fn a_workspace_without_a_tracker_lists_empty_and_an_unknown_workspace_is_404() {
        let h = harness().await;
        let p = workspace(&h).await;
        assert_eq!(
            call(&h, "GET", &format!("/api/workspaces/{p}/trackers"), None, None).await,
            (StatusCode::OK, "[]".into())
        );
        let ghost = uuid::Uuid::new_v4();
        assert_eq!(
            call(&h, "GET", &format!("/api/workspaces/{ghost}/trackers"), None, None).await.0,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn connect_returns_201_pending_then_the_first_sync_lands() {
        let h = harness().await;
        let p = workspace(&h).await;
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        let (status, body) = connect(&h, p, &s).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let v = json(&body);
        assert_eq!(v["first_sync"], json!({"pending": true}));
        assert_eq!(v["tracker"]["base_url"], json!(format!("https://{s}")));
        assert_eq!(v["tracker"]["board_ref"], json!("42"));
        let listed = settled(&h, p).await;
        assert_eq!(listed[0]["status"], json!("connected"), "{listed}");
        assert_eq!(listed[0]["counts"], json!({"tasks": 3, "sprints": 2}));
        assert_eq!(listed[0]["board_name"], json!("AUTH Scrum"));
        assert_eq!(listed[0]["external_scope"], json!("AUTH"));
    }

    #[tokio::test]
    async fn connect_refusals_carry_fixed_codes() {
        let h = harness().await;
        let p = workspace(&h).await;
        let uri = format!("/api/workspaces/{p}/trackers/jira/connect");
        let (st, b) = call(
            &h,
            "POST",
            &format!("/api/workspaces/{p}/trackers/gitlab/connect"),
            Some(connect_body(&site())),
            None,
        )
        .await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::UNPROCESSABLE_ENTITY, json!("unsupported_tracker"))
        );
        let bad_url = json!({"board_url": "https://evil.example/boards/42", "email": "e@x.io", "api_token": "t"});
        let (st, b) = call(&h, "POST", &uri, Some(bad_url), None).await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::UNPROCESSABLE_ENTITY, json!("invalid_base_url"))
        );
        let no_email = json!({"board_url": format!("https://{}/jira/software/projects/AUTH/boards/42", site()), "api_token": "t"});
        let (st, b) = call(&h, "POST", &uri, Some(no_email), None).await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::UNPROCESSABLE_ENTITY, json!("email_required"))
        );
    }

    #[tokio::test]
    async fn connect_maps_validation_failures() {
        let h = harness().await;
        let p = workspace(&h).await;
        let s = site();
        h.factory.serve(
            &format!("https://{s}"),
            FakeBoard::standard().fail("/rest/api/3/myself", TrackerError::Unauthorized),
        );
        let (st, b) = connect(&h, p, &s).await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::UNAUTHORIZED, json!("unauthorized"))
        );
        assert_eq!(
            call(&h, "GET", &format!("/api/workspaces/{p}/trackers"), None, None).await.1,
            "[]"
        );

        let s2 = site();
        h.factory.serve(
            &format!("https://{s2}"),
            FakeBoard::standard().fail("/rest/agile/1.0/board/42", TrackerError::NotFound),
        );
        let (st, b) = connect(&h, p, &s2).await;
        assert_eq!(st, StatusCode::NOT_FOUND);
        assert_eq!(
            json(&b),
            json!({"error": "board_not_found", "base_url": format!("https://{s2}"), "board_ref": "42"})
        );
    }

    #[tokio::test]
    async fn a_board_owned_by_another_workspace_is_409_with_that_workspace() {
        let h = harness().await;
        let (p1, p2) = (workspace(&h).await, workspace(&h).await);
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        assert_eq!(connect(&h, p1, &s).await.0, StatusCode::CREATED);
        let (st, b) = connect(&h, p2, &s).await;
        assert_eq!(st, StatusCode::CONFLICT);
        let v = json(&b);
        assert_eq!(
            (v["error"].clone(), v["workspace"]["id"].clone()),
            (json!("board_already_connected"), json!(p1.to_string()))
        );
        assert!(
            v["workspace"]["name"].as_str().is_some_and(|n| n.starts_with("_test:tracker-api:"))
        );
    }

    #[tokio::test]
    async fn a_keyring_failure_is_500_credential_store() {
        let h = harness().await;
        let p = workspace(&h).await;
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        *h.store.fail_set.lock().unwrap() = true;
        let (st, b) = connect(&h, p, &s).await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::INTERNAL_SERVER_ERROR, json!("credential_store"))
        );
    }

    #[tokio::test]
    async fn sync_returns_counts_busy_and_404() {
        let h = harness().await;
        let p = workspace(&h).await;
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        let uri = format!("/api/workspaces/{p}/trackers/sync");
        assert_eq!(
            call(&h, "POST", &uri, None, None).await.0,
            StatusCode::NOT_FOUND,
            "no tracker yet"
        );
        connect(&h, p, &s).await;
        settled(&h, p).await;
        let (st, b) = call(&h, "POST", &uri, None, None).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(
            json(&b)["counts"],
            json!({"sprints": 2, "tasks": 3, "removed": 0, "links": 0, "skipped_members": 0})
        );
        let id = h.state.pg.tracker_for_workspace(p).await.unwrap().unwrap().id;
        let _held = h.rt.locks.try_lock(id).unwrap();
        let (st, b) = call(&h, "POST", &uri, None, None).await;
        assert_eq!((st, json(&b)["error"].clone()), (StatusCode::CONFLICT, json!("busy")));
    }

    #[tokio::test]
    async fn a_failed_sync_is_502_with_its_code() {
        let h = harness().await;
        let p = workspace(&h).await;
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        connect(&h, p, &s).await;
        settled(&h, p).await;
        h.factory.serve(
            &format!("https://{s}"),
            FakeBoard::standard().fail("/rest/agile/1.0/board/42/sprint", TrackerError::Forbidden),
        );
        let (st, b) =
            call(&h, "POST", &format!("/api/workspaces/{p}/trackers/sync"), None, None).await;
        assert_eq!((st, json(&b)["error"].clone()), (StatusCode::BAD_GATEWAY, json!("forbidden")));
        let listed =
            json(&call(&h, "GET", &format!("/api/workspaces/{p}/trackers"), None, None).await.1);
        assert_eq!(
            (listed[0]["status"].clone(), listed[0]["last_error"].clone()),
            (json!("error"), json!("forbidden"))
        );
    }

    #[tokio::test]
    async fn disconnect_is_204_then_404() {
        let h = harness().await;
        let p = workspace(&h).await;
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        connect(&h, p, &s).await;
        settled(&h, p).await;
        let uri = format!("/api/workspaces/{p}/trackers");
        assert_eq!(call(&h, "DELETE", &uri, None, None).await.0, StatusCode::NO_CONTENT);
        assert_eq!(call(&h, "GET", &uri, None, None).await.1, "[]");
        assert_eq!(call(&h, "DELETE", &uri, None, None).await.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn the_routes_are_guarded_against_foreign_origins_and_hosts() {
        let h = harness().await;
        let p = workspace(&h).await;
        let uri = format!("/api/workspaces/{p}/trackers");
        assert_eq!(
            call(&h, "GET", &uri, None, Some("https://evil.example")).await.0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            call(&h, "GET", &uri, None, Some("tauri://localhost")).await.0,
            StatusCode::OK,
            "the desktop app"
        );
        let rebound = Request::builder()
            .uri(&uri)
            .header("host", "evil.example:7744")
            .body(Body::empty())
            .unwrap();
        assert_eq!(h.app.clone().oneshot(rebound).await.unwrap().status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn the_token_never_appears_in_any_response() {
        let h = harness().await;
        let p = workspace(&h).await;
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        let mut bodies = vec![connect(&h, p, &s).await.1];
        settled(&h, p).await;
        bodies.push(call(&h, "GET", &format!("/api/workspaces/{p}/trackers"), None, None).await.1);
        bodies.push(
            call(&h, "POST", &format!("/api/workspaces/{p}/trackers/sync"), None, None).await.1,
        );
        bodies.push(connect(&h, workspace(&h).await, &s).await.1); // 409
        *h.store.fail_set.lock().unwrap() = true;
        bodies.push(connect(&h, p, &s).await.1); // 500
        for b in bodies {
            assert!(!b.contains(SENTINEL), "token leaked: {b}");
        }
        let row = h.state.pg.tracker_for_workspace(p).await.unwrap().unwrap();
        assert!(!format!("{row:?}").contains(SENTINEL));
    }

    #[tokio::test]
    async fn the_same_board_typed_differently_is_still_409() {
        let h = harness().await;
        let (p1, p2) = (workspace(&h).await, workspace(&h).await);
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        assert_eq!(connect(&h, p1, &s).await.0, StatusCode::CREATED);
        let shouted = json!({
            "board_url": format!("https://{}/jira/software/projects/AUTH/boards/042?selectedIssue=AUTH-1", s.to_uppercase()),
            "email": "dev@example.com",
            "api_token": SENTINEL,
        });
        let (st, b) = call(
            &h,
            "POST",
            &format!("/api/workspaces/{p2}/trackers/jira/connect"),
            Some(shouted),
            None,
        )
        .await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::CONFLICT, json!("board_already_connected"))
        );
    }

    #[tokio::test]
    async fn the_first_sync_waits_out_a_running_sync_instead_of_being_dropped() {
        let h = harness().await;
        let p = workspace(&h).await;
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        connect(&h, p, &s).await;
        settled(&h, p).await;
        // A sync is running when the user re-connects.
        let id = h.state.pg.tracker_for_workspace(p).await.unwrap().unwrap().id;
        let held = h.rt.locks.try_lock(id).unwrap();
        assert_eq!(connect(&h, p, &s).await.0, StatusCode::CREATED);
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        drop(held);
        let listed = settled(&h, p).await;
        assert_eq!(listed[0]["status"], json!("connected"), "{listed}");
    }

    #[tokio::test]
    async fn a_ticket_lists_its_linked_sessions_by_current_or_former_key() {
        use crate::tracker::test_support::{repo_in, session_with};
        let h = harness().await;
        let p = workspace(&h).await;
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        let repo = repo_in(&h.state.pg, p).await;
        let sess = session_with(&h.state.pg, Some(repo), &["fix/auth-3"], &["AUTH-3 please"]).await;
        connect(&h, p, &s).await;
        settled(&h, p).await;

        let (st, b) =
            call(&h, "GET", &format!("/api/workspaces/{p}/tracker-tasks/auth-3"), None, None).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        let v = json(&b);
        assert_eq!(v["task"]["key"], json!("AUTH-3"));
        assert_eq!(v["task"]["summary"], json!("summary AUTH-3"));
        assert_eq!(v["task"]["removed"], json!(false));
        assert_eq!(v["sessions"].as_array().map(Vec::len), Some(1), "{b}");
        assert_eq!(v["sessions"][0]["id"], json!(sess));
        assert!(v["sessions"][0]["started_at"].is_string());
        assert_eq!(
            v["sessions"][0]["links"],
            json!([
                {"source": "branch", "matched_key": "AUTH-3", "mentions": 1},
                {"source": "prompt", "matched_key": "AUTH-3", "mentions": 1},
            ])
        );

        // Renamed in Jira: the old key still finds the ticket.
        h.factory.serve(
            &format!("https://{s}"),
            FakeBoard::standard().with(
                "/rest/agile/1.0/board/42/issue",
                2,
                json!({"startAt": 2, "total": 3, "issues": [{"id": "10003", "key": "PLAT-9", "fields": {}}]}),
            ),
        );
        let (st, b) =
            call(&h, "POST", &format!("/api/workspaces/{p}/trackers/sync"), None, None).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        let (st, b) =
            call(&h, "GET", &format!("/api/workspaces/{p}/tracker-tasks/AUTH-3"), None, None).await;
        assert_eq!((st, json(&b)["task"]["key"].clone()), (StatusCode::OK, json!("PLAT-9")), "{b}");

        // A key in the board's shape is looked for on the board first (R11).
        let missing =
            call(&h, "GET", &format!("/api/workspaces/{p}/tracker-tasks/AUTH-404"), None, None)
                .await;
        assert_eq!(
            (missing.0, json(&missing.1)["error"].clone()),
            (StatusCode::NOT_FOUND, json!("task_not_on_board"))
        );
        let foreign = call(
            &h,
            "GET",
            &format!("/api/workspaces/{p}/tracker-tasks/AUTH-3"),
            None,
            Some("https://evil.com"),
        )
        .await;
        assert_eq!(foreign.0, StatusCode::FORBIDDEN, "reads are guarded too");
        let unknown = uuid::Uuid::new_v4();
        let gone =
            call(&h, "GET", &format!("/api/workspaces/{unknown}/tracker-tasks/AUTH-3"), None, None)
                .await;
        assert_eq!(json(&gone.1)["error"], json!("workspace_not_found"));
    }

    #[tokio::test]
    async fn usage_routes_return_their_shapes_and_fixed_codes() {
        use crate::tracker::test_support::{repo_in, session_with};
        let h = harness().await;
        let p = workspace(&h).await;
        let get = |uri: String| {
            let h = &h;
            async move { call(h, "GET", &uri, None, None).await }
        };
        assert_eq!(
            json(&get(format!("/api/workspaces/{p}/sprints")).await.1)["error"],
            json!("not_connected")
        );
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        let repo = repo_in(&h.state.pg, p).await;
        let sess = session_with(&h.state.pg, Some(repo), &["fix/AUTH-2"], &[]).await;
        connect(&h, p, &s).await;
        settled(&h, p).await;

        let (st, b) = get(format!("/api/workspaces/{p}/sprints")).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        let sprints = json(&b);
        assert_eq!(sprints.as_array().map(Vec::len), Some(2), "{b}");
        assert_eq!(sprints[0]["name"], json!("Sprint 2"), "newest first");
        assert_eq!(sprints[0]["state"], json!("active"));
        assert_eq!(sprints[0]["tasks"], json!(2), "ticket count");
        assert!(sprints[0]["candidates_in_window"].is_number(), "{b}");
        for part in ["linked_sprint", "linked_other", "unlinked"] {
            assert!(sprints[0][part]["sessions"].is_number(), "{part}: {b}");
        }
        let s101 = sprints[0]["id"].as_str().unwrap().to_string();

        let (st, b) = get(format!("/api/workspaces/{p}/tracker-tasks")).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        let tasks = json(&b);
        assert_eq!(tasks.as_array().map(Vec::len), Some(3));
        let auth2 = tasks.as_array().unwrap().iter().find(|t| t["key"] == "AUTH-2").unwrap();
        assert_eq!(auth2["usage"]["sessions"], json!(1));
        assert_eq!(auth2["current_sprint"]["name"], json!("Sprint 2"));
        assert_eq!(auth2["sprints"][0]["name"], json!("Sprint 2"));
        assert_eq!(auth2["removed"], json!(false));
        let none = tasks.as_array().unwrap().iter().find(|t| t["key"] == "AUTH-1").unwrap();
        assert_eq!(none["usage"]["sessions"], json!(0));
        assert!(none["usage"]["active_secs"].is_null());

        let (_, b) = get(format!("/api/workspaces/{p}/tracker-tasks?sprint={s101}")).await;
        assert_eq!(json(&b).as_array().map(Vec::len), Some(2), "{b}");
        let unknown = uuid::Uuid::new_v4();
        let (st, b) = get(format!("/api/workspaces/{p}/tracker-tasks?sprint={unknown}")).await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::NOT_FOUND, json!("sprint_not_found"))
        );

        let (st, b) = get(format!("/api/workspaces/{p}/tracker-tasks/AUTH-2")).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        let detail = json(&b);
        assert_eq!(detail["usage"]["sessions"], json!(1));
        assert_eq!(detail["by_repo"].as_array().map(Vec::len), Some(1));
        assert_eq!(detail["by_repo"][0]["usage"]["sessions"], json!(1));

        let (st, b) = get(format!("/api/workspaces/{p}/sessions/unlinked")).await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::UNPROCESSABLE_ENTITY, json!("sprint_required"))
        );
        let (st, b) = get(format!("/api/workspaces/{p}/sessions/unlinked?sprint={s101}")).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert!(
            !json(&b).as_array().unwrap().iter().any(|s| s["id"] == json!(sess)),
            "a linked session is not unlinked"
        );
        let (st, b) = get(format!("/api/workspaces/{p}/sessions/unlinked?sprint={unknown}")).await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::NOT_FOUND, json!("sprint_not_found"))
        );
        let foreign = call(
            &h,
            "GET",
            &format!("/api/workspaces/{p}/sprints"),
            None,
            Some("https://evil.com"),
        )
        .await;
        assert_eq!(foreign.0, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn a_lookup_miss_in_the_board_shape_discovers_the_ticket_first() {
        let h = harness().await;
        let p = workspace(&h).await;
        let s = site();
        let board = h.factory.serve(
            &format!("https://{s}"),
            FakeBoard::standard().with_jql(
                "key in (AUTH-900)",
                json!([{"id": "10900", "key": "AUTH-900", "fields": {"summary": "New"}}]),
            ),
        );
        connect(&h, p, &s).await;
        settled(&h, p).await;

        let (st, b) =
            call(&h, "GET", &format!("/api/workspaces/{p}/tracker-tasks/auth-900"), None, None)
                .await;
        assert_eq!(
            (st, json(&b)["task"]["key"].clone()),
            (StatusCode::OK, json!("AUTH-900")),
            "{b}"
        );

        let (st, b) =
            call(&h, "GET", &format!("/api/workspaces/{p}/tracker-tasks/AUTH-404"), None, None)
                .await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{b}");
        assert_eq!(
            json(&b),
            json!({"error": "task_not_on_board", "key": "AUTH-404", "board_ref": "42"})
        );

        let asked = board.jql_calls().len();
        let (st, b) =
            call(&h, "GET", &format!("/api/workspaces/{p}/tracker-tasks/UTF-8"), None, None).await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::NOT_FOUND, json!("task_not_found"))
        );
        let (_, b) =
            call(&h, "GET", &format!("/api/workspaces/{p}/tracker-tasks/AUTH-404"), None, None)
                .await;
        assert_eq!(
            json(&b)["error"],
            json!("task_not_found"),
            "negative-cached: answered without asking"
        );
        assert_eq!(board.jql_calls().len(), asked, "neither asked the board");
    }

    #[tokio::test]
    async fn the_backlog_routes() {
        use crate::tracker::test_support::{repo_in, session_with};
        let h = harness().await;
        let p = workspace(&h).await;
        let get = |uri: String| {
            let h = &h;
            async move { call(h, "GET", &uri, None, None).await }
        };
        assert_eq!(
            json(&get(format!("/api/workspaces/{p}/backlog")).await.1)["error"],
            json!("not_connected")
        );
        let s = site();
        h.factory.serve(
            &format!("https://{s}"),
            FakeBoard::standard().with(
                "/rest/agile/1.0/board/42/issue",
                2,
                json!({"startAt": 2, "total": 4, "issues": [
                    {"id": "10003", "key": "AUTH-3", "fields": {}},
                    {"id": "10004", "key": "AUTH-4", "fields": {"summary": "no sprint"}}]}),
            ),
        );
        let repo = repo_in(&h.state.pg, p).await;
        session_with(&h.state.pg, Some(repo), &["fix/AUTH-4"], &[]).await;
        connect(&h, p, &s).await;
        settled(&h, p).await;

        let (st, b) = get(format!("/api/workspaces/{p}/backlog")).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(json(&b)["tickets"], json!(1));
        assert_eq!(json(&b)["usage"]["sessions"], json!(1));

        let (st, b) = get(format!("/api/workspaces/{p}/tracker-tasks?backlog=true")).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        let keys: Vec<Value> =
            json(&b).as_array().unwrap().iter().map(|t| t["key"].clone()).collect();
        assert_eq!(keys, vec![json!("AUTH-4")]);

        let sprint = h
            .state
            .pg
            .tracker_sprints(h.state.pg.tracker_for_workspace(p).await.unwrap().unwrap().id)
            .await
            .unwrap()[0]
            .id;
        let (st, b) =
            get(format!("/api/workspaces/{p}/tracker-tasks?sprint={sprint}&backlog=true")).await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::UNPROCESSABLE_ENTITY, json!("sprint_or_backlog"))
        );
    }
}
