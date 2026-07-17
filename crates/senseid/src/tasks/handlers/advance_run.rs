//! Relay-engine (P3.2) — advance one autonomous run by a single tick.
//!
//! The scheduler ([`crate::tasks::advance_run_scheduler`]) enqueues one
//! `AdvanceRun` task per active run each tick, carrying the run id in
//! `task.path`. This handler is the per-run tick body.
//!
//! **P3.2 scope:** the tick scaffolding + the lifecycle bits doable *without*
//! an agent — the liveness heartbeat and a housekeeping cadence event. The
//! actual agent spawn/drive (which emits `phase_started`/`feature_started`/
//! `feature_done`/`gate_raised` and updates `current_phase`/`current_feature`/
//! `status`) is P3.3 and plugs in at the `// P3.3 SEAM` below.
//!
//! Resume is NOT handled here: the scheduler's `resume_due_runs` flips a
//! `paused` run whose `paused_until` has elapsed back to `running` (SQL-side)
//! before enqueueing this tick, so by the time a run reaches this handler a due
//! pause is already cleared. A still-`paused` run is a no-op tick.

use super::super::executor::TaskContext;
use super::super::Task;
use crate::runs::RunEventKind;

/// Handler for `TaskKind::AdvanceRun`: advance one run by a tick. Returns `1`
/// when the run was ticked (heartbeat + housekeeping event), `0` for empty work
/// — an unknown/invalid run id, a run that no longer exists, a terminal run
/// (`Done`/`Failed`/`Crashed`), or a still-`paused` run.
pub async fn advance_run(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    // Run id rides in task.path. Empty / non-UUID → empty work, not an error.
    let run_id_str = task.path.as_str();
    if run_id_str.is_empty() {
        return Ok(0);
    }
    let Ok(run_id) = uuid::Uuid::parse_str(run_id_str) else {
        return Ok(0);
    };

    // A run that was completed/deleted between enqueue and dispatch is empty work.
    let Some(run) = ctx
        .pg()
        .get_run(&run_id)
        .await
        .map_err(|e| format!("get_run failed: {e}"))?
    else {
        return Ok(0);
    };

    use dojo_protocol::relay::RelayRunStatus;
    match run.status {
        // Terminal — no further progress. (Crashed is unexpected-death, also
        // terminal for the tick: recovery is a later, deliberate action.)
        RelayRunStatus::Done | RelayRunStatus::Failed | RelayRunStatus::Crashed => Ok(0),
        // Still paused: the scheduler's resume_due_runs flips a *due* pause to
        // running before this handler ever sees it, so a run that reaches here
        // still paused is not due yet — a no-op tick. The handler never resumes.
        RelayRunStatus::Paused => Ok(0),
        // Blocked: waiting on a hard-block gate (a human reply). No autonomous
        // progress until the gate clears, so the tick is a no-op — but we still
        // want a live heartbeat so a blocked run isn't mistaken for a crash.
        // Fall through to the heartbeat path.
        RelayRunStatus::Running | RelayRunStatus::Stalled | RelayRunStatus::Blocked => {
            // Liveness: keep the heartbeat fresh so stall detection sees an
            // advancing (or at least alive) run.
            ctx.pg()
                .touch_run_heartbeat(&run_id)
                .await
                .map_err(|e| format!("touch_run_heartbeat failed: {e}"))?;

            // Cadence log: a lightweight housekeeping marker so the observability
            // API/console shows the run is being serviced each tick. Never a diff
            // or raw tool output (relay-engine D10) — just a tick marker.
            ctx.pg()
                .append_run_event(
                    &run_id,
                    RunEventKind::Housekeeping,
                    run.current_phase.as_deref(),
                    run.current_feature.as_deref(),
                    &serde_json::json!({ "tick": true }),
                )
                .await
                .map_err(|e| format!("append_run_event failed: {e}"))?;

            // P3.3 SEAM: spawn/supervise the agent for this run's next feature
            // here; emit phase_started/feature_started/feature_done/gate_raised +
            // update current_phase/current_feature/status. P3.2 only heartbeats.

            Ok(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::state::SharedState;
    use crate::runs::NewRun;
    use crate::tasks::queue::TaskQueue;
    use crate::tasks::TaskKind;
    use dojo_protocol::relay::RelayRunStatus;
    use std::sync::Arc;

    async fn make_ctx() -> Option<Arc<TaskContext>> {
        let queue = Arc::new(TaskQueue::new());
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let pg = crate::db::pg_store::PgStore::connect_test().await.ok()?;
        let app_state = Arc::new(SharedState {
            task_queue: queue.clone(),
            pg,
            gateway,
            event_tx: { let (tx, _) = tokio::sync::broadcast::channel(16); tx },
            breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        });
        Some(Arc::new(TaskContext { queue, app_state, _graph_path: None, logger: sensei_logger::Logger::noop() }))
    }

    #[tokio::test]
    async fn empty_run_id_is_empty_work() {
        // No DB needed — the empty-path guard short-circuits before any query.
        let Some(ctx) = make_ctx().await else { return; };
        let task = Task::new(TaskKind::AdvanceRun, "", "");
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn non_uuid_run_id_is_empty_work() {
        let Some(ctx) = make_ctx().await else { return; };
        let task = Task::new(TaskKind::AdvanceRun, "", "not-a-uuid");
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn unknown_run_is_empty_work() {
        let Some(ctx) = make_ctx().await else { return; };
        let id = uuid::Uuid::new_v4().to_string();
        let task = Task::new(TaskKind::AdvanceRun, "", &id);
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn running_run_heartbeats_and_logs() {
        let Some(ctx) = make_ctx().await else { return; };
        let pg = ctx.pg();
        let id = pg.create_run(&NewRun::default()).await.unwrap(); // defaults to running

        let task = Task::new(TaskKind::AdvanceRun, "", &id.to_string());
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 1, "a running run is ticked");

        // Heartbeat was stamped.
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert!(run.heartbeat_at.as_deref().unwrap().contains('T'), "heartbeat set");

        // A housekeeping tick event was appended.
        let events = pg.list_run_events(&id, 10).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, RunEventKind::Housekeeping);
        assert_eq!(events[0].detail["tick"], serde_json::json!(true));

        pg_delete_run(pg, &id).await;
    }

    #[tokio::test]
    async fn paused_run_is_noop() {
        let Some(ctx) = make_ctx().await else { return; };
        let pg = ctx.pg();
        let id = pg.create_run(&NewRun::default()).await.unwrap();
        pg.update_run_status(&id, RelayRunStatus::Paused, Some("2999-01-01T00:00:00Z"), Some("cap"))
            .await.unwrap();

        let task = Task::new(TaskKind::AdvanceRun, "", &id.to_string());
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 0, "a paused run is a no-op tick");

        // No heartbeat, no event.
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert!(run.heartbeat_at.is_none(), "paused run is not heartbeated");
        assert!(pg.list_run_events(&id, 10).await.unwrap().is_empty(), "no event for a paused tick");

        pg_delete_run(pg, &id).await;
    }

    #[tokio::test]
    async fn terminal_run_is_noop() {
        let Some(ctx) = make_ctx().await else { return; };
        let pg = ctx.pg();
        let id = pg.create_run(&NewRun::default()).await.unwrap();
        pg.complete_run(&id, RelayRunStatus::Done).await.unwrap();

        let task = Task::new(TaskKind::AdvanceRun, "", &id.to_string());
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 0, "a done run is a no-op tick");
        assert!(pg.list_run_events(&id, 10).await.unwrap().is_empty());

        pg_delete_run(pg, &id).await;
    }

    async fn pg_delete_run(pg: &crate::db::pg_store::PgStore, id: &uuid::Uuid) {
        sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
            .bind(id).execute(pg.pool()).await.unwrap();
    }
}
