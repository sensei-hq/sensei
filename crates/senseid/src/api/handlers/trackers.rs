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
use crate::tracker::{TrackerError, TrackerKind, TrackerLevel, ValidateError, adapter_for_kind};

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
        "level": row.level,
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
    let rows = match state.pg.trackers_for_workspace(workspace).await {
        Ok(rows) => rows,
        Err(_) => return internal().into_response(),
    };
    let mut views = Vec::with_capacity(rows.len());
    for row in &rows {
        match view(&state, row).await {
            Ok(v) => views.push(v),
            Err(r) => return r.into_response(),
        }
    }
    Json(views).into_response()
}

/// `POST /api/workspaces/{id}/trackers` — `{kind, level, …adapter fields}`.
pub(crate) async fn connect_tracker(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let workspace = match known_workspace(&state, &id).await {
        Ok(p) => p,
        Err(r) => return r.into_response(),
    };
    let field = |k: &str| body.get(k).and_then(|v| v.as_str());
    let Some(adapter) = field("kind").and_then(TrackerKind::parse).and_then(adapter_for_kind)
    else {
        return fail(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_tracker").into_response();
    };
    let level = match field("level").map(TrackerLevel::parse) {
        Some(Some(level)) => level,
        Some(None) => {
            return fail(StatusCode::UNPROCESSABLE_ENTITY, "invalid_level").into_response();
        }
        None => return fail(StatusCode::UNPROCESSABLE_ENTITY, "level_required").into_response(),
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
        level,
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
    // The first sync runs in the background: connect never waits on a sync. The
    // site's other boards moved to this token, so they sync again too.
    for id in std::iter::once(row.id).chain(connected.rekeyed.iter().copied()) {
        spawn_first_sync(state.pg.clone(), rt.clone(), id);
    }
    match view(&state, &row).await {
        Ok(v) => (
            StatusCode::CREATED,
            Json(serde_json::json!({"tracker": v, "first_sync": {"pending": true}})),
        )
            .into_response(),
        Err(r) => r.into_response(),
    }
}

/// Sync a just-connected (or just re-keyed) board in the background. A sync
/// already running (e.g. on the old credential) is waited out, bounded, rather
/// than dropping the first sync.
fn spawn_first_sync(
    pg: crate::db::pg_store::PgStore,
    rt: std::sync::Arc<crate::tracker::runtime::TrackerRuntime>,
    id: uuid::Uuid,
) {
    tokio::spawn(async move {
        for _ in 0..FIRST_SYNC_BUSY_RETRIES {
            match sync_current(&pg, &rt, id).await {
                Err(SyncError::Busy) => tokio::time::sleep(std::time::Duration::from_secs(1)).await,
                Err(e) => {
                    tracing::warn!(tracker_id = %id, "tracker first sync: {e:?}");
                    return;
                }
                Ok(_) => return,
            }
        }
        tracing::warn!(tracker_id = %id, "tracker first sync: still busy; the scheduled sync will run it");
    });
}

/// Times a sync re-reads its row after the credential changed under it.
const SUPERSEDED_RETRIES: u32 = 3;

/// Sync the tracker on the credential its row names now: a sync superseded by a
/// re-key re-reads the row and runs again, a bounded number of times.
async fn sync_current(
    pg: &crate::db::pg_store::PgStore,
    rt: &crate::tracker::runtime::TrackerRuntime,
    id: uuid::Uuid,
) -> Result<crate::db::pg_store::trackers::ApplyCounts, SyncError> {
    let mut outcome = Err(SyncError::Superseded);
    for _ in 0..SUPERSEDED_RETRIES {
        let row = pg.tracker_by_id(id).await.map_err(SyncError::Db)?.ok_or(SyncError::Gone)?;
        outcome = sync_stored(pg, rt, &row).await;
        if outcome != Err(SyncError::Superseded) {
            break;
        }
    }
    outcome
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

/// `DELETE /api/workspaces/{id}/trackers/{tracker}`
pub(crate) async fn disconnect_tracker(
    State(state): State<AppState>,
    Path((id, tracker)): Path<(String, String)>,
) -> Response {
    let row = match known_tracker(&state, &id, &tracker).await {
        Ok(row) => row,
        Err(r) => return r.into_response(),
    };
    match disconnect(&state.pg, state.trackers.secrets.clone(), row.workspace_id, row.id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => fail(StatusCode::NOT_FOUND, "tracker_not_found").into_response(),
        Err(_) => internal().into_response(),
    }
}

/// `POST /api/workspaces/{id}/trackers/{tracker}/sync`
pub(crate) async fn sync_tracker(
    State(state): State<AppState>,
    Path((id, tracker)): Path<(String, String)>,
) -> Response {
    let row = match known_tracker(&state, &id, &tracker).await {
        Ok(row) => row,
        Err(r) => return r.into_response(),
    };
    match sync_current(&state.pg, &state.trackers, row.id).await {
        Ok(c) => Json(serde_json::json!({"counts": {
            "sprints": c.sprints, "tasks": c.tasks, "removed": c.removed,
            "links": c.links, "skipped_members": c.skipped_members,
        }}))
        .into_response(),
        Err(SyncError::Busy) => fail(StatusCode::CONFLICT, "busy").into_response(),
        Err(SyncError::Gone) => fail(StatusCode::NOT_FOUND, "tracker_not_found").into_response(),
        Err(SyncError::Superseded) => fail(StatusCode::CONFLICT, "superseded").into_response(),
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
    let usage = match ticket_usages(&state.pg, task.tracker_id, Some(&[task.id])).await {
        Ok(mut u) => u.remove(&task.id).unwrap_or_default(),
        Err(_) => return internal().into_response(),
    };
    let by_repo = match ticket_usage_by_repo(&state.pg, task.tracker_id, task.id).await {
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
            "tracker_id": task.tracker_id, "key": task.key, "summary": task.summary, "issue_type": task.issue_type,
            "status": task.status, "removed": task.removed,
        },
        "usage": usage,
        "by_repo": by_repo,
        "sessions": sessions,
    }))
    .into_response()
}

/// Longest a lookup waits on discovery, across all of the workspace's boards, before answering.
const LOOKUP_DISCOVERY: std::time::Duration = std::time::Duration::from_secs(30);

/// R11 lookup: a key in the shape of one of the workspace's boards that sensei
/// has not synced is fetched once before answering, from each board in level
/// order until one has it, all within one [`LOOKUP_DISCOVERY`]. A board that is
/// busy or failing does not stop the others: a found ticket wins, then a board
/// that answered it has no such ticket, then the first failure.
async fn discover_on_miss(
    state: &AppState,
    workspace: uuid::Uuid,
    key: &str,
) -> Result<crate::db::pg_store::trackers::TaskDetail, ApiErr> {
    let trackers = state.pg.trackers_for_workspace(workspace).await.map_err(|_| internal())?;
    let until = tokio::time::Instant::now() + LOOKUP_DISCOVERY;
    let (mut not_on_board, mut failure) = (None, None);
    for tracker in &trackers {
        match discover_on(state, workspace, tracker, key, until).await {
            Ok(Discovered::Task(task)) => return Ok(task),
            Ok(Discovered::NotOnBoard(miss)) => {
                not_on_board.get_or_insert(miss);
            }
            Ok(Discovered::Nothing) => {}
            Err(e) => {
                failure.get_or_insert(e);
            }
        }
    }
    Err(not_on_board.or(failure).unwrap_or_else(|| fail(StatusCode::NOT_FOUND, "task_not_found")))
}

/// What asking one board for a missing key found.
enum Discovered {
    Task(crate::db::pg_store::trackers::TaskDetail),
    /// The key has this board's shape, and the board answered it has no such ticket.
    NotOnBoard(ApiErr),
    /// The key is not in this board's shape, or the board was not asked.
    Nothing,
}

async fn discover_on(
    state: &AppState,
    workspace: uuid::Uuid,
    tracker: &TrackerRow,
    key: &str,
    until: tokio::time::Instant,
) -> Result<Discovered, ApiErr> {
    use crate::tracker::discover::{DiscoverError, Scope};
    let keys = [key.to_string()];
    let run = crate::tracker::runtime::discover_stored(
        &state.pg,
        &state.trackers,
        tracker,
        Scope::Keys(&keys),
    );
    match tokio::time::timeout_at(until, run).await {
        Err(_) => Err(fail(StatusCode::BAD_GATEWAY, TrackerError::Transport.code())),
        Ok(Ok(out)) if !out.inserted.is_empty() => {
            match state.pg.tracker_task_detail(workspace, key).await {
                Ok(Some(task)) => Ok(Discovered::Task(task)),
                Ok(None) => Ok(Discovered::Nothing),
                Err(_) => Err(internal()),
            }
        }
        Ok(Ok(out)) => Ok(match out.not_on_board.first() {
            Some(k) => Discovered::NotOnBoard((
                StatusCode::NOT_FOUND,
                Json(
                    serde_json::json!({"error": "task_not_on_board", "key": k, "board_ref": tracker.board_ref}),
                ),
            )),
            None => Discovered::Nothing,
        }),
        Ok(Err(DiscoverError::Busy)) => Err(fail(StatusCode::CONFLICT, "busy")),
        Ok(Err(DiscoverError::Failed(e))) => Err(fail(StatusCode::BAD_GATEWAY, e.code())),
        Ok(Err(DiscoverError::Db(_))) => Err(internal()),
        // Paused after a recent failure, or re-connected meanwhile: answer from what is stored.
        Ok(Err(DiscoverError::Paused | DiscoverError::Gone)) => Ok(Discovered::Nothing),
    }
}

/// One of the workspace's trackers, or the response to return: 404
/// `workspace_not_found`, or 404 `tracker_not_found` for an id that is not a
/// tracker of this workspace.
async fn known_tracker(state: &AppState, id: &str, tracker: &str) -> Result<TrackerRow, ApiErr> {
    let workspace = known_workspace(state, id).await?;
    let not_found = || fail(StatusCode::NOT_FOUND, "tracker_not_found");
    let tracker: uuid::Uuid = tracker.parse().map_err(|_| not_found())?;
    match state.pg.tracker_by_id(tracker).await {
        Ok(Some(row)) if row.workspace_id == workspace => Ok(row),
        Ok(_) => Err(not_found()),
        Err(_) => Err(internal()),
    }
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
    include_removed: bool,
}

/// `GET /api/workspaces/{id}/trackers/{tracker}/sprints` — one Sprints lane (D40).
pub(crate) async fn list_sprints(
    State(state): State<AppState>,
    Path((id, tracker)): Path<(String, String)>,
) -> Response {
    let tracker = match known_tracker(&state, &id, &tracker).await {
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

/// `GET /api/workspaces/{id}/trackers/{tracker}/tasks?sprint=&include_removed=`
pub(crate) async fn list_tracker_tasks(
    State(state): State<AppState>,
    Path((id, tracker)): Path<(String, String)>,
    axum::extract::Query(q): axum::extract::Query<SprintQuery>,
) -> Response {
    let tracker = match known_tracker(&state, &id, &tracker).await {
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
    let tasks = match state.pg.tracker_task_rows(tracker.id, sprint, q.include_removed).await {
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

/// `GET /api/workspaces/{id}/trackers/{tracker}/sessions/unlinked?sprint=`
pub(crate) async fn list_unlinked_sessions(
    State(state): State<AppState>,
    Path((id, tracker)): Path<(String, String)>,
    axum::extract::Query(q): axum::extract::Query<SprintQuery>,
) -> Response {
    let tracker = match known_tracker(&state, &id, &tracker).await {
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
        board_body(site, "42", "sprint")
    }

    fn board_body(site: &str, board: &str, level: &str) -> Value {
        json!({
            "kind": "jira",
            "level": level,
            "board_url": format!("https://{site}/jira/software/projects/AUTH/boards/{board}"),
            "email": "dev@example.com",
            "api_token": SENTINEL,
        })
    }

    async fn connect(h: &Harness, p: uuid::Uuid, site: &str) -> (StatusCode, String) {
        call(h, "POST", &format!("/api/workspaces/{p}/trackers"), Some(connect_body(site)), None)
            .await
    }

    /// The workspace's first tracker, as `GET …/trackers` lists it.
    async fn tracker(h: &Harness, p: uuid::Uuid) -> String {
        let (_, body) = call(h, "GET", &format!("/api/workspaces/{p}/trackers"), None, None).await;
        json(&body)[0]["id"].as_str().unwrap().to_string()
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
        let uri = format!("/api/workspaces/{p}/trackers");
        let mut gitlab = connect_body(&site());
        gitlab["kind"] = json!("gitlab");
        let mut no_level = connect_body(&site());
        no_level.as_object_mut().unwrap().remove("level");
        for (body, code) in [
            (gitlab, "unsupported_tracker"),
            (board_body(&site(), "42", "portfolio"), "invalid_level"),
            (no_level, "level_required"),
        ] {
            let (st, b) = call(&h, "POST", &uri, Some(body), None).await;
            assert_eq!(
                (st, json(&b)["error"].clone()),
                (StatusCode::UNPROCESSABLE_ENTITY, json!(code))
            );
        }
        let bad_url = json!({"kind": "jira", "level": "sprint", "board_url": "https://evil.example/boards/42", "email": "dev@example.com", "api_token": "t"});
        let (st, b) = call(&h, "POST", &uri, Some(bad_url), None).await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::UNPROCESSABLE_ENTITY, json!("invalid_base_url"))
        );
        let no_email = json!({"kind": "jira", "level": "sprint", "board_url": format!("https://{}/jira/software/projects/AUTH/boards/42", site()), "api_token": "t"});
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
        let ghost = uuid::Uuid::new_v4();
        let (st, b) =
            call(&h, "POST", &format!("/api/workspaces/{p}/trackers/{ghost}/sync"), None, None)
                .await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::NOT_FOUND, json!("tracker_not_found")),
            "no such tracker"
        );
        connect(&h, p, &s).await;
        settled(&h, p).await;
        let id = tracker(&h, p).await;
        let uri = format!("/api/workspaces/{p}/trackers/{id}/sync");
        let (st, b) = call(&h, "POST", &uri, None, None).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(
            json(&b)["counts"],
            json!({"sprints": 2, "tasks": 3, "removed": 0, "links": 0, "skipped_members": 0})
        );
        let _held = h.rt.locks.try_lock(id.parse().unwrap()).unwrap();
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
        let id = tracker(&h, p).await;
        let (st, b) =
            call(&h, "POST", &format!("/api/workspaces/{p}/trackers/{id}/sync"), None, None).await;
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
        let list = format!("/api/workspaces/{p}/trackers");
        let uri = format!("{list}/{}", tracker(&h, p).await);
        let foreign =
            format!("/api/workspaces/{}/trackers/{}", workspace(&h).await, tracker(&h, p).await);
        assert_eq!(
            call(&h, "DELETE", &foreign, None, None).await.0,
            StatusCode::NOT_FOUND,
            "another workspace cannot remove it"
        );
        assert_eq!(call(&h, "DELETE", &uri, None, None).await.0, StatusCode::NO_CONTENT);
        assert_eq!(call(&h, "GET", &list, None, None).await.1, "[]");
        let (st, b) = call(&h, "DELETE", &uri, None, None).await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::NOT_FOUND, json!("tracker_not_found"))
        );
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
        let id = tracker(&h, p).await;
        bodies.push(
            call(&h, "POST", &format!("/api/workspaces/{p}/trackers/{id}/sync"), None, None)
                .await
                .1,
        );
        bodies.push(connect(&h, workspace(&h).await, &s).await.1); // 409
        *h.store.fail_set.lock().unwrap() = true;
        bodies.push(connect(&h, p, &s).await.1); // 500
        for b in bodies {
            assert!(!b.contains(SENTINEL), "token leaked: {b}");
        }
        let rows = h.state.pg.trackers_for_workspace(p).await.unwrap();
        assert!(!format!("{rows:?}").contains(SENTINEL));
    }

    #[tokio::test]
    async fn the_same_board_typed_differently_is_still_409() {
        let h = harness().await;
        let (p1, p2) = (workspace(&h).await, workspace(&h).await);
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        assert_eq!(connect(&h, p1, &s).await.0, StatusCode::CREATED);
        let shouted = json!({
            "kind": "jira",
            "level": "sprint",
            "board_url": format!("https://{}/jira/software/projects/AUTH/boards/042?selectedIssue=AUTH-1", s.to_uppercase()),
            "email": "dev@example.com",
            "api_token": SENTINEL,
        });
        let (st, b) =
            call(&h, "POST", &format!("/api/workspaces/{p2}/trackers"), Some(shouted), None).await;
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
        let id = tracker(&h, p).await.parse().unwrap();
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
        let id = tracker(&h, p).await;
        let (st, b) =
            call(&h, "POST", &format!("/api/workspaces/{p}/trackers/{id}/sync"), None, None).await;
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
        let ghost = uuid::Uuid::new_v4();
        assert_eq!(
            json(&get(format!("/api/workspaces/{p}/trackers/{ghost}/sprints")).await.1)["error"],
            json!("tracker_not_found")
        );
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        let repo = repo_in(&h.state.pg, p).await;
        let sess = session_with(&h.state.pg, Some(repo), &["fix/AUTH-2"], &[]).await;
        connect(&h, p, &s).await;
        settled(&h, p).await;
        let t = format!("/api/workspaces/{p}/trackers/{}", tracker(&h, p).await);

        let (st, b) = get(format!("{t}/sprints")).await;
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

        let (st, b) = get(format!("{t}/tasks")).await;
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

        let (_, b) = get(format!("{t}/tasks?sprint={s101}")).await;
        assert_eq!(json(&b).as_array().map(Vec::len), Some(2), "{b}");
        let unknown = uuid::Uuid::new_v4();
        let (st, b) = get(format!("{t}/tasks?sprint={unknown}")).await;
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

        let (st, b) = get(format!("{t}/sessions/unlinked")).await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::UNPROCESSABLE_ENTITY, json!("sprint_required"))
        );
        let (st, b) = get(format!("{t}/sessions/unlinked?sprint={s101}")).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert!(
            !json(&b).as_array().unwrap().iter().any(|s| s["id"] == json!(sess)),
            "a linked session is not unlinked"
        );
        let (st, b) = get(format!("{t}/sessions/unlinked?sprint={unknown}")).await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::NOT_FOUND, json!("sprint_not_found"))
        );
        let foreign =
            call(&h, "GET", &format!("{t}/sprints"), None, Some("https://evil.com")).await;
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
    async fn a_workspace_lists_its_boards_by_level_and_each_has_its_own_sprints() {
        let h = harness().await;
        let p = workspace(&h).await;
        let s = site();
        h.factory.serve(
            &format!("https://{s}"),
            FakeBoard::standard().with(
                "/rest/agile/1.0/board/43",
                0,
                json!({"id": 43, "name": "Portfolio", "location": {"projectKey": "AUTH"}}),
            ),
        );
        let uri = format!("/api/workspaces/{p}/trackers");
        let (st, b) = call(&h, "POST", &uri, Some(board_body(&s, "42", "sprint")), None).await;
        assert_eq!(st, StatusCode::CREATED, "{b}");
        let (st, b) = call(&h, "POST", &uri, Some(board_body(&s, "43", "epic")), None).await;
        assert_eq!(st, StatusCode::CREATED, "{b}");
        assert_eq!(json(&b)["tracker"]["level"], json!("epic"));

        let listed = json(&call(&h, "GET", &uri, None, None).await.1);
        let boards: Vec<(Value, Value)> = listed
            .as_array()
            .unwrap()
            .iter()
            .map(|t| (t["level"].clone(), t["board_ref"].clone()))
            .collect();
        assert_eq!(boards, [(json!("epic"), json!("43")), (json!("sprint"), json!("42"))]);

        let sprint_board = listed[1]["id"].as_str().unwrap();
        let ghost = workspace(&h).await;
        let (st, b) = call(
            &h,
            "GET",
            &format!("/api/workspaces/{ghost}/trackers/{sprint_board}/sprints"),
            None,
            None,
        )
        .await;
        assert_eq!(
            (st, json(&b)["error"].clone()),
            (StatusCode::NOT_FOUND, json!("tracker_not_found")),
            "a tracker is read through its own workspace only"
        );
    }

    #[tokio::test]
    async fn a_lookup_asks_the_next_board_when_one_is_busy() {
        let h = harness().await;
        let p = workspace(&h).await;
        let s = site();
        h.factory.serve(
            &format!("https://{s}"),
            FakeBoard::standard().with_board(43, "Portfolio").with_jql(
                "key in (AUTH-900)",
                json!([{"id": "10900", "key": "AUTH-900", "fields": {"summary": "New"}}]),
            ),
        );
        let uri = format!("/api/workspaces/{p}/trackers");
        for (board, level) in [("42", "sprint"), ("43", "epic")] {
            let (st, b) = call(&h, "POST", &uri, Some(board_body(&s, board, level)), None).await;
            assert_eq!(st, StatusCode::CREATED, "{b}");
        }
        let mut listed = json!(null);
        for _ in 0..300 {
            listed = json(&call(&h, "GET", &uri, None, None).await.1);
            if listed
                .as_array()
                .is_some_and(|l| l.iter().all(|t| t["status"] == json!("connected")))
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let (epic, sprint) = (listed[0]["id"].as_str().unwrap(), listed[1]["id"].as_str().unwrap());
        assert_eq!(listed[0]["level"], json!("epic"), "{listed}");
        // The epic board, asked first, is mid-sync.
        let _held = h.rt.locks.try_lock(epic.parse().unwrap()).unwrap();
        let (st, b) =
            call(&h, "GET", &format!("/api/workspaces/{p}/tracker-tasks/AUTH-900"), None, None)
                .await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(json(&b)["task"]["tracker_id"], json!(sprint));
    }
}
