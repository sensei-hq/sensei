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
use crate::tracker::{TrackerKind, ValidateError, adapter_for_kind};

/// One-second waits a first sync spends on a running sync before leaving it to the tick.
const FIRST_SYNC_BUSY_RETRIES: u32 = 15 * 60;

fn fail(status: StatusCode, code: &str) -> Response {
    err(status, code).into_response()
}

fn internal() -> Response {
    fail(StatusCode::INTERNAL_SERVER_ERROR, "internal")
}

/// The project's uuid, or the response to return (404 for an unknown project:
/// `resolve_project_uuid` accepts any well-formed uuid without checking it).
async fn known_project(state: &AppState, id: &str) -> Result<uuid::Uuid, Response> {
    let uuid = crate::api::util::resolve_project_uuid(state, id)
        .await
        .map_err(|s| s.into_response())?
        .ok_or_else(|| fail(StatusCode::NOT_FOUND, "project_not_found"))?;
    match state.pg.get_project(&uuid).await {
        Ok(Some(_)) => Ok(uuid),
        Ok(None) => Err(fail(StatusCode::NOT_FOUND, "project_not_found")),
        Err(_) => Err(internal()),
    }
}

/// The connection as the API shows it. Never the token, never the keyring name.
async fn view(state: &AppState, row: &TrackerRow) -> Result<serde_json::Value, Response> {
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

/// `GET /api/projects/{id}/trackers`
pub(crate) async fn list_trackers(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let project = match known_project(&state, &id).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    match state.pg.tracker_for_project(project).await {
        Ok(None) => Json(serde_json::json!([])).into_response(),
        Ok(Some(row)) => match view(&state, &row).await {
            Ok(v) => Json(serde_json::json!([v])).into_response(),
            Err(r) => r,
        },
        Err(_) => internal(),
    }
}

/// `POST /api/projects/{id}/trackers/{kind}/connect`
pub(crate) async fn connect_tracker(
    State(state): State<AppState>,
    Path((id, kind)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let project = match known_project(&state, &id).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    let Some(adapter) = TrackerKind::parse(&kind).and_then(adapter_for_kind) else {
        return fail(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_tracker");
    };
    let params = match adapter.connect_params(&body) {
        Ok(p) => p,
        Err(e) => return fail(StatusCode::UNPROCESSABLE_ENTITY, e.code()),
    };
    // Connect stays interactive: no 429 retries during validation.
    let transport = match state.trackers.transports.transport(
        &params.base_url,
        adapter.auth(&params.account, &params.token),
        0,
    ) {
        Ok(t) => t,
        Err(e) => return fail(StatusCode::BAD_GATEWAY, e.code()),
    };
    let (base_url, board_ref) = (params.base_url.clone(), params.board_ref.clone());
    let rt = state.trackers.clone();
    let connected =
        match connect(&state.pg, rt.secrets.clone(), adapter, transport.as_ref(), project, params)
            .await
        {
            Ok(c) => c,
            Err(e) => return connect_failure(e, &base_url, &board_ref),
        };
    let row = match state.pg.tracker_by_id(connected.tracker_id).await {
        Ok(Some(row)) => row,
        _ => return internal(),
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
        Err(r) => r,
    }
}

fn connect_failure(e: ConnectError, base_url: &str, board_ref: &str) -> Response {
    use crate::tracker::TrackerError;
    match e {
        ConnectError::BoardAlreadyConnected { project_id, project_name } => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "board_already_connected",
                "project": {"id": project_id, "name": project_name},
            })),
        )
            .into_response(),
        ConnectError::Validate(ValidateError::BoardNotFound) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "board_not_found", "base_url": base_url, "board_ref": board_ref})),
        )
            .into_response(),
        ConnectError::Validate(ValidateError::BoardProjectMismatch) => {
            fail(StatusCode::UNPROCESSABLE_ENTITY, "board_project_mismatch")
        }
        ConnectError::Validate(ValidateError::Tracker(TrackerError::Unauthorized)) => {
            fail(StatusCode::UNAUTHORIZED, "unauthorized")
        }
        ConnectError::Validate(ValidateError::Tracker(TrackerError::Forbidden)) => {
            fail(StatusCode::FORBIDDEN, "forbidden")
        }
        ConnectError::Validate(ValidateError::Tracker(e)) => fail(StatusCode::BAD_GATEWAY, e.code()),
        ConnectError::CredentialStore => fail(StatusCode::INTERNAL_SERVER_ERROR, "credential_store"),
        ConnectError::Db(_) => internal(),
    }
}

/// `DELETE /api/projects/{id}/trackers`
pub(crate) async fn disconnect_tracker(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let project = match known_project(&state, &id).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    match disconnect(&state.pg, state.trackers.secrets.clone(), project).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => fail(StatusCode::NOT_FOUND, "not_connected"),
        Err(_) => internal(),
    }
}

/// `POST /api/projects/{id}/trackers/sync`
pub(crate) async fn sync_tracker(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let project = match known_project(&state, &id).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    let row = match state.pg.tracker_for_project(project).await {
        Ok(Some(row)) => row,
        Ok(None) => return fail(StatusCode::NOT_FOUND, "not_connected"),
        Err(_) => return internal(),
    };
    match sync_stored(&state.pg, &state.trackers, &row).await {
        Ok(c) => Json(serde_json::json!({"counts": {
            "sprints": c.sprints, "tasks": c.tasks, "removed": c.removed,
            "links": c.links, "skipped_members": c.skipped_members,
        }}))
        .into_response(),
        Err(SyncError::Busy) => fail(StatusCode::CONFLICT, "busy"),
        Err(SyncError::Gone) => fail(StatusCode::NOT_FOUND, "not_connected"),
        Err(SyncError::Failed(e)) => fail(StatusCode::BAD_GATEWAY, e.code()),
        Err(SyncError::Db(_)) => internal(),
    }
}

/// `GET /api/projects/{id}/tracker-tasks/{key}`
pub(crate) async fn get_tracker_task(
    State(state): State<AppState>,
    Path((id, key)): Path<(String, String)>,
) -> Response {
    let project = match known_project(&state, &id).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    let task = match state.pg.tracker_task_detail(project, &key).await {
        Ok(Some(task)) => task,
        Ok(None) => return fail(StatusCode::NOT_FOUND, "task_not_found"),
        Err(_) => return internal(),
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
        "sessions": sessions,
    }))
    .into_response()
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

    async fn project(h: &Harness) -> uuid::Uuid {
        h.state
            .pg
            .create_project(&format!("_test:tracker-api:{}", uuid::Uuid::new_v4()), None, None)
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
            &format!("/api/projects/{p}/trackers/jira/connect"),
            Some(connect_body(site)),
            None,
        )
        .await
    }

    /// Poll `GET …/trackers` until the status leaves `pending` (≤ 3 s).
    async fn settled(h: &Harness, p: uuid::Uuid) -> Value {
        for _ in 0..300 {
            let (_, body) =
                call(h, "GET", &format!("/api/projects/{p}/trackers"), None, None).await;
            let v = json(&body);
            if v[0]["status"] != json!("pending") {
                return v;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("first sync never finished");
    }

    #[tokio::test]
    async fn a_project_without_a_tracker_lists_empty_and_an_unknown_project_is_404() {
        let h = harness().await;
        let p = project(&h).await;
        assert_eq!(
            call(&h, "GET", &format!("/api/projects/{p}/trackers"), None, None).await,
            (StatusCode::OK, "[]".into())
        );
        let ghost = uuid::Uuid::new_v4();
        assert_eq!(
            call(&h, "GET", &format!("/api/projects/{ghost}/trackers"), None, None).await.0,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn connect_returns_201_pending_then_the_first_sync_lands() {
        let h = harness().await;
        let p = project(&h).await;
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
        let p = project(&h).await;
        let uri = format!("/api/projects/{p}/trackers/jira/connect");
        let (st, b) = call(
            &h,
            "POST",
            &format!("/api/projects/{p}/trackers/gitlab/connect"),
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
        let p = project(&h).await;
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
            call(&h, "GET", &format!("/api/projects/{p}/trackers"), None, None).await.1,
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
    async fn a_board_owned_by_another_project_is_409_with_that_project() {
        let h = harness().await;
        let (p1, p2) = (project(&h).await, project(&h).await);
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        assert_eq!(connect(&h, p1, &s).await.0, StatusCode::CREATED);
        let (st, b) = connect(&h, p2, &s).await;
        assert_eq!(st, StatusCode::CONFLICT);
        let v = json(&b);
        assert_eq!(
            (v["error"].clone(), v["project"]["id"].clone()),
            (json!("board_already_connected"), json!(p1.to_string()))
        );
        assert!(v["project"]["name"].as_str().is_some_and(|n| n.starts_with("_test:tracker-api:")));
    }

    #[tokio::test]
    async fn a_keyring_failure_is_500_credential_store() {
        let h = harness().await;
        let p = project(&h).await;
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
        let p = project(&h).await;
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        let uri = format!("/api/projects/{p}/trackers/sync");
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
        let id = h.state.pg.tracker_for_project(p).await.unwrap().unwrap().id;
        let _held = h.rt.locks.try_lock(id).unwrap();
        let (st, b) = call(&h, "POST", &uri, None, None).await;
        assert_eq!((st, json(&b)["error"].clone()), (StatusCode::CONFLICT, json!("busy")));
    }

    #[tokio::test]
    async fn a_failed_sync_is_502_with_its_code() {
        let h = harness().await;
        let p = project(&h).await;
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        connect(&h, p, &s).await;
        settled(&h, p).await;
        h.factory.serve(
            &format!("https://{s}"),
            FakeBoard::standard().fail("/rest/agile/1.0/board/42/sprint", TrackerError::Forbidden),
        );
        let (st, b) =
            call(&h, "POST", &format!("/api/projects/{p}/trackers/sync"), None, None).await;
        assert_eq!((st, json(&b)["error"].clone()), (StatusCode::BAD_GATEWAY, json!("forbidden")));
        let listed =
            json(&call(&h, "GET", &format!("/api/projects/{p}/trackers"), None, None).await.1);
        assert_eq!(
            (listed[0]["status"].clone(), listed[0]["last_error"].clone()),
            (json!("error"), json!("forbidden"))
        );
    }

    #[tokio::test]
    async fn disconnect_is_204_then_404() {
        let h = harness().await;
        let p = project(&h).await;
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        connect(&h, p, &s).await;
        settled(&h, p).await;
        let uri = format!("/api/projects/{p}/trackers");
        assert_eq!(call(&h, "DELETE", &uri, None, None).await.0, StatusCode::NO_CONTENT);
        assert_eq!(call(&h, "GET", &uri, None, None).await.1, "[]");
        assert_eq!(call(&h, "DELETE", &uri, None, None).await.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn the_routes_are_guarded_against_foreign_origins_and_hosts() {
        let h = harness().await;
        let p = project(&h).await;
        let uri = format!("/api/projects/{p}/trackers");
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
        let p = project(&h).await;
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        let mut bodies = vec![connect(&h, p, &s).await.1];
        settled(&h, p).await;
        bodies.push(call(&h, "GET", &format!("/api/projects/{p}/trackers"), None, None).await.1);
        bodies.push(
            call(&h, "POST", &format!("/api/projects/{p}/trackers/sync"), None, None).await.1,
        );
        bodies.push(connect(&h, project(&h).await, &s).await.1); // 409
        *h.store.fail_set.lock().unwrap() = true;
        bodies.push(connect(&h, p, &s).await.1); // 500
        for b in bodies {
            assert!(!b.contains(SENTINEL), "token leaked: {b}");
        }
        let row = h.state.pg.tracker_for_project(p).await.unwrap().unwrap();
        assert!(!format!("{row:?}").contains(SENTINEL));
    }

    #[tokio::test]
    async fn the_same_board_typed_differently_is_still_409() {
        let h = harness().await;
        let (p1, p2) = (project(&h).await, project(&h).await);
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
            &format!("/api/projects/{p2}/trackers/jira/connect"),
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
        let p = project(&h).await;
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        connect(&h, p, &s).await;
        settled(&h, p).await;
        // A sync is running when the user re-connects.
        let id = h.state.pg.tracker_for_project(p).await.unwrap().unwrap().id;
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
        let p = project(&h).await;
        let s = site();
        h.factory.serve(&format!("https://{s}"), FakeBoard::standard());
        let repo = repo_in(&h.state.pg, p).await;
        let sess = session_with(&h.state.pg, Some(repo), &["fix/auth-3"], &["AUTH-3 please"]).await;
        connect(&h, p, &s).await;
        settled(&h, p).await;

        let (st, b) =
            call(&h, "GET", &format!("/api/projects/{p}/tracker-tasks/auth-3"), None, None).await;
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
            call(&h, "POST", &format!("/api/projects/{p}/trackers/sync"), None, None).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        let (st, b) =
            call(&h, "GET", &format!("/api/projects/{p}/tracker-tasks/AUTH-3"), None, None).await;
        assert_eq!((st, json(&b)["task"]["key"].clone()), (StatusCode::OK, json!("PLAT-9")), "{b}");

        let missing =
            call(&h, "GET", &format!("/api/projects/{p}/tracker-tasks/AUTH-404"), None, None).await;
        assert_eq!(
            (missing.0, json(&missing.1)["error"].clone()),
            (StatusCode::NOT_FOUND, json!("task_not_found"))
        );
        let foreign = call(
            &h,
            "GET",
            &format!("/api/projects/{p}/tracker-tasks/AUTH-3"),
            None,
            Some("https://evil.com"),
        )
        .await;
        assert_eq!(foreign.0, StatusCode::FORBIDDEN, "reads are guarded too");
        let unknown = uuid::Uuid::new_v4();
        let gone =
            call(&h, "GET", &format!("/api/projects/{unknown}/tracker-tasks/AUTH-3"), None, None)
                .await;
        assert_eq!(json(&gone.1)["error"], json!("project_not_found"));
    }
}
