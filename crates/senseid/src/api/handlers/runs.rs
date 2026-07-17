//! Relay-engine (P3.2) — the run observability API.
//!
//! Read-only endpoints over the daemon-owned `activity.runs` / `run_events`:
//!
//! - `GET /api/runs`       → `{ runs: [Run] }` — the active runs (running /
//!   paused / stalled), newest-started first.
//! - `GET /api/runs/{id}`  → `{ run: Run, events: [RunEvent] }` — one run plus
//!   its latest cadence events; `404` for a well-formed id with no row, `400`
//!   for a non-UUID id (mirrors `sessions::get_session`).
//!
//! These read the same durable state the scheduler/handler write, so the
//! console/phone can render a run's status + cadence without touching the queue.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};

use crate::api::state::AppState;

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
