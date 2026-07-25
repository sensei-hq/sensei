//! Relay-engine (P3.2) — the run observability API.
//!
//! Read-only endpoints over the daemon-owned `activity.runs` / `run_events`:
//!
//! - `GET /api/runs`       → `{ runs: [Run] }` — the active runs (running /
//!   paused / stalled), newest-started first.
//! - `GET /api/runs/{id}`  → `{ run: Run, events: [RunEvent] }` — one run plus
//!   its latest cadence events; `404` for a well-formed id with no row, `400`
//!   for a non-UUID id (mirrors `sessions::get_session`).
//! - `POST /api/runs`      → `{ run: Run }` (201) — create a daemon-owned run
//!   (P3.8 run-control; the MCP `start_run` tool + the desktop app kick runs off
//!   here). The scheduler picks it up on the next tick.
//!
//! The reads render the same durable state the scheduler/handler write, so the
//! console/phone can show a run's status + cadence without touching the queue.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};

use crate::api::state::AppState;
use crate::runs::NewRun;

/// Cap on the cadence events returned for a single run — the recent tail is what
/// the timeline renders; older events page in later if we need them.
const RUN_EVENTS_LIMIT: i64 = 20;

/// GET /api/runs — the active runs (running / paused / stalled), newest-started
/// first. Returns `{ runs: [Run] }`; a DB error is a 500.
pub(crate) async fn list_runs(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.pg.list_active_runs().await {
        Ok(runs) => Ok(Json(serde_json::json!({ "runs": runs }))),
        Err(e) => {
            tracing::error!(error = %e, "list_runs failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/runs/{id} — one run plus its latest ~20 cadence events. A well-formed
/// UUID with no row yields 404 (not 500); a non-UUID id yields 400. Returns
/// `{ run: Run, events: [RunEvent] }`.
pub(crate) async fn get_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let run = match state.pg.get_run(&uuid).await {
        Ok(Some(run)) => run,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!(error = %e, run = %id, "get_run failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let events = match state.pg.list_run_events(&uuid, RUN_EVENTS_LIMIT).await {
        Ok(events) => events,
        Err(e) => {
            tracing::error!(error = %e, run = %id, "get_run events lookup failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    Ok(Json(serde_json::json!({ "run": run, "events": events })))
}

/// POST /api/runs — create a daemon-owned run (P3.8 run-control; the MCP
/// `start_run` tool and the desktop app kick runs off here). Body:
/// `{ "goal": string (required, non-empty), "project"?: name-or-uuid,
///    "plan_ref"?: string, "max_concurrency"?: int }`.
///
/// - An empty/absent `goal` is a 400 — a run with nothing to drive is a no-op.
/// - `project` resolves name-OR-uuid → id ([`crate::api::util::resolve_project_uuid`]);
///   absent means no project (the drive Flags "no cwd" rather than spawning
///   loose), while a **given-but-unresolvable** project is a 400 so the caller's
///   intent is never silently dropped.
///
/// Returns `{ run: Run }` with `201 Created`. The scheduler advances it on the
/// next tick; whether it actually drives an agent still depends on the
/// OFF-by-default `SENSEI_RUN_DRIVE`.
pub(crate) async fn create_run(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let goal = body["goal"].as_str().map(str::trim).unwrap_or("");
    if goal.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let project_id = match body["project"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
        Some(p) => match crate::api::util::resolve_project_uuid(&state, p).await {
            Some(id) => Some(id),
            None => return Err(StatusCode::BAD_REQUEST),
        },
        None => None,
    };
    let plan_ref = body["plan_ref"].as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    let max_concurrency = body["max_concurrency"].as_i64().map(|n| n as i32);

    // Stamp the run with the git author of its project's repo root (user.name /
    // user.email, git's local→global precedence — the same identity as
    // get_user_for_project). This registers the run under who's doing the work,
    // matching the commit author + the Dōjō sign-in. Best-effort: an
    // unresolvable git identity (no project / not a repo) leaves the columns NULL.
    let (author_name, author_email) = match project_id {
        Some(pid) => match state.pg.project_root_path(&pid).await.ok().flatten() {
            Some(dir) => {
                let user = crate::git_identity::read_git_user(std::path::Path::new(&dir));
                (user.name, user.email)
            }
            None => (None, None),
        },
        None => (None, None),
    };

    let new = NewRun {
        project_id,
        plan_ref,
        goal: Some(goal.to_string()),
        dojo_session_id: None,
        max_concurrency,
        author_name,
        author_email,
    };
    let id = match state.pg.create_run(&new).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!(error = %e, "create_run failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    match state.pg.get_run(&id).await {
        Ok(Some(run)) => Ok((StatusCode::CREATED, Json(serde_json::json!({ "run": run })))),
        // Created then vanished between INSERT and read-back — shouldn't happen.
        Ok(None) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        Err(e) => {
            tracing::error!(error = %e, "create_run read-back failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::state::SharedState;
    use crate::runs::{NewRun, RunEventKind};
    use crate::tasks::queue::TaskQueue;
    use std::sync::Arc;

    async fn make_state() -> Option<AppState> {
        let queue = Arc::new(TaskQueue::new());
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let pg = crate::db::pg_store::PgStore::connect_test().await.ok()?;
        Some(Arc::new(SharedState {
            task_queue: queue,
            pg,
            gateway,
            event_tx: { let (tx, _) = tokio::sync::broadcast::channel(16); tx },
            breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            provisioning: None,
        }))
    }

    async fn del(state: &AppState, id: &uuid::Uuid) {
        sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
            .bind(id).execute(state.pg.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn get_run_bad_uuid_is_400() {
        let Some(state) = make_state().await else { return; };
        let err = get_run(State(state), Path("not-a-uuid".into())).await.unwrap_err();
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_run_unknown_is_404() {
        let Some(state) = make_state().await else { return; };
        let missing = uuid::Uuid::new_v4().to_string();
        let err = get_run(State(state), Path(missing)).await.unwrap_err();
        assert_eq!(err, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn create_run_empty_goal_is_400() {
        let Some(state) = make_state().await else { return; };
        // No DB row created — the guard rejects before any insert.
        let err = create_run(State(state), Json(serde_json::json!({ "goal": "   " })))
            .await.unwrap_err();
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_run_unresolvable_project_is_400() {
        let Some(state) = make_state().await else { return; };
        // A given project that resolves to nothing is a 400 (intent not dropped).
        let err = create_run(
            State(state),
            Json(serde_json::json!({ "goal": "do a thing", "project": "no-such-project-xyzzy" })),
        )
        .await.unwrap_err();
        assert_eq!(err, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_run_happy_path_returns_201_and_running_run() {
        let Some(state) = make_state().await else { return; };
        let (status, Json(body)) = create_run(
            State(state.clone()),
            Json(serde_json::json!({ "goal": "ship the thing", "plan_ref": "docs/plan/x.md" })),
        )
        .await.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["run"]["goal"], serde_json::json!("ship the thing"));
        assert_eq!(body["run"]["plan_ref"], serde_json::json!("docs/plan/x.md"));
        assert_eq!(body["run"]["status"], serde_json::json!("running"));
        // No project given → project_id is null (drive will Flag "no cwd").
        assert!(body["run"]["project_id"].is_null());

        let id = uuid::Uuid::parse_str(body["run"]["id"].as_str().unwrap()).unwrap();
        del(&state, &id).await;
    }

    #[tokio::test]
    async fn list_and_get_return_run_with_events() {
        let Some(state) = make_state().await else { return; };
        let id = state.pg.create_run(&NewRun::default()).await.unwrap();
        state.pg
            .append_run_event(&id, RunEventKind::Housekeeping, None, None, &serde_json::json!({ "tick": true }))
            .await.unwrap();

        // list_runs includes the active run.
        let Json(list) = list_runs(State(state.clone())).await.unwrap();
        let ids: Vec<&str> = list["runs"].as_array().unwrap()
            .iter().filter_map(|r| r["id"].as_str()).collect();
        assert!(ids.contains(&id.to_string().as_str()), "active run listed");

        // get_run returns the run + its events.
        let Json(one) = get_run(State(state.clone()), Path(id.to_string())).await.unwrap();
        assert_eq!(one["run"]["id"].as_str(), Some(id.to_string().as_str()));
        assert_eq!(one["run"]["status"], serde_json::json!("running"));
        let events = one["events"].as_array().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["kind"], serde_json::json!("housekeeping"));

        del(&state, &id).await;
    }
}
