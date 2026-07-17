//! Relay-engine (P3.2) — the AdvanceRun scheduler.
//!
//! A long-lived tokio task (mirroring [`crate::tasks::analyzer_scheduler::spawn`])
//! that wakes on a short interval and drives every daemon-owned run forward:
//!
//! 1. **Resume due pauses** — [`PgStore::resume_due_runs`] flips every `paused`
//!    run whose `paused_until` has elapsed back to `running` (SQL-side clock
//!    compare). For each resumed run we log a [`RunEventKind::Resumed`] cadence
//!    event and enqueue an immediate `AdvanceRun` tick so it advances this tick
//!    rather than waiting for the next one.
//! 2. **Tick every active run** — [`PgStore::list_active_runs`] returns the
//!    `running`/`paused`/`stalled`/`blocked` runs; we enqueue one `AdvanceRun`
//!    task per run (id in `task.path`). The enqueue is de-duped: if an
//!    `AdvanceRun` for that run is already pending/blocked/running we skip it, so
//!    a backed-up queue (esp. P3.3's slow agent spawn) never piles up duplicate
//!    ticks for the same run in the unbounded queue. The handler is also a cheap
//!    no-op for the runs that aren't actually advanceable (still-paused /
//!    terminal-between-ticks), so an occasional over-enqueue is still safe.
//!
//! **Fault tolerance:** a DB error on any step is logged (`tracing::warn`) and
//! the loop continues — it never panics the scheduler. In particular, if the
//! `activity.runs` table isn't deployed yet, each tick warns and keeps looping
//! rather than crashing the daemon.

use std::sync::Arc;
use std::time::Duration;

use crate::db::pg_store::PgStore;
use crate::runs::RunEventKind;
use crate::tasks::queue::TaskQueue;
use crate::tasks::{Task, TaskKind};

/// How often to sweep runs. Short — a tick is a DB read + a few enqueues, and a
/// stalled/heartbeat-driven run wants a tight liveness cadence.
const INTERVAL_SECS: u64 = 15;

/// Enqueue an `AdvanceRun` tick for one run id — de-duped. Skips the enqueue if
/// an `AdvanceRun` for this run (id in `task.path`) is already pending, blocked,
/// or running, so a backed-up queue (esp. P3.3's slow agent spawn) never piles
/// up duplicate ticks for the same run in the unbounded `VecDeque`. Kept as a
/// tiny helper so the "what an active run enqueues" contract is unit-testable
/// against a real queue without driving the scheduler's infinite `run` loop.
/// `pub(crate)` so the watchdog scheduler (P3.6) reuses the exact same de-duped
/// enqueue when it recovers a stalled run (DRY — one enqueue contract).
pub(crate) async fn enqueue_advance(queue: &TaskQueue, run_id: &uuid::Uuid) {
    let path = run_id.to_string();
    if queue.has_pending_kind_path(TaskKind::AdvanceRun, &path).await {
        return;
    }
    queue.enqueue(Task::new(TaskKind::AdvanceRun, "", &path)).await;
}

/// Resume every due pause, logging a `Resumed` event + enqueueing a tick for
/// each. Tolerant: a resume-query failure is logged and treated as "nothing
/// resumed" so the tick still proceeds to the active-run sweep. Extracted so the
/// resume→event→enqueue contract is testable without the infinite loop.
async fn resume_due(queue: &TaskQueue, pg: &PgStore) {
    let resumed = match pg.resume_due_runs().await {
        Ok(ids) => ids,
        Err(e) => {
            // A missing runs table lands here (and in list_active_runs below):
            // warn once per tick and keep looping — never crash the scheduler.
            tracing::warn!(error = %e, "advance_run_scheduler: resume_due_runs failed");
            return;
        }
    };
    for id in resumed {
        // Log the resume so the observability API/console shows the auto-resume.
        // A per-run event failure is logged and skipped — it must not stop us
        // from ticking the run (the state is already flipped to running).
        if let Err(e) = pg
            .append_run_event(&id, RunEventKind::Resumed, None, None, &serde_json::json!({ "auto": true }))
            .await
        {
            tracing::warn!(run_id = %id, error = %e, "advance_run_scheduler: logging Resumed event failed");
        }
        enqueue_advance(queue, &id).await;
    }
}

/// One scheduler tick: resume due pauses, then enqueue an `AdvanceRun` per
/// active run. Pulled out of `run` so a single tick is testable directly.
async fn tick(queue: &TaskQueue, pg: &PgStore) {
    resume_due(queue, pg).await;

    match pg.list_active_runs().await {
        Ok(runs) => {
            for run in &runs {
                enqueue_advance(queue, &run.id).await;
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "advance_run_scheduler: list_active_runs failed");
        }
    }
}

/// Spawn the scheduler for the daemon's lifetime.
pub fn spawn(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    tokio::spawn(run(queue, pg));
}

async fn run(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    let mut ticker = tokio::time::interval(Duration::from_secs(INTERVAL_SECS));
    loop {
        ticker.tick().await;
        tick(&queue, &pg).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn enqueue_advance_carries_run_id() {
        let queue = TaskQueue::new();
        let id = uuid::Uuid::new_v4();
        enqueue_advance(&queue, &id).await;

        assert_eq!(queue.status().await.pending, 1);
        let t = queue.next_task().await;
        assert_eq!(t.kind, TaskKind::AdvanceRun);
        assert_eq!(t.path, id.to_string(), "the run id rides in task.path");
        assert_eq!(t.folder_path, "", "no folder scoping for a run tick");
    }

    #[tokio::test]
    async fn tick_enqueues_one_per_active_run() {
        // DB-guarded: with a test DB, a tick over N active runs enqueues N ticks.
        // `tick` calls the global `resume_due_runs`, so hold the shared resume
        // lock to serialize with the other resume-race tests.
        let _guard = crate::runs::resume_test_lock().lock().await;
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let queue = TaskQueue::with_max_repos(16);

        use crate::runs::NewRun;
        let a = pg.create_run(&NewRun::default()).await.unwrap();
        let b = pg.create_run(&NewRun::default()).await.unwrap();

        tick(&queue, &pg).await;

        // Drain the queue and confirm both runs got an AdvanceRun tick. (Other
        // runs from a shared test DB may also appear — we only assert ours are
        // present, and that every enqueued task is an AdvanceRun.)
        let n = queue.status().await.pending;
        let mut paths = std::collections::HashSet::new();
        for _ in 0..n {
            let t = queue.next_task().await;
            assert_eq!(t.kind, TaskKind::AdvanceRun);
            paths.insert(t.path);
        }
        assert!(paths.contains(&a.to_string()), "run a is ticked");
        assert!(paths.contains(&b.to_string()), "run b is ticked");

        for id in [a, b] {
            sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
                .bind(id).execute(pg.pool()).await.unwrap();
        }
    }

    #[tokio::test]
    async fn tick_dedups_advance_across_ticks() {
        // Two consecutive ticks over the SAME active run, with nothing draining
        // the queue, must enqueue the AdvanceRun only once — a backed-up queue
        // (P3.3's slow agent spawn) must not pile up duplicate ticks per run.
        //
        // `tick` calls the global `resume_due_runs`, so hold the shared resume
        // lock to serialize with the other resume-race tests.
        let _guard = crate::runs::resume_test_lock().lock().await;
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let queue = TaskQueue::with_max_repos(16);

        use crate::runs::NewRun;
        let run = pg.create_run(&NewRun::default()).await.unwrap();

        // Two ticks, nothing dequeued in between.
        tick(&queue, &pg).await;
        tick(&queue, &pg).await;

        // Count AdvanceRun ticks for OUR run across the whole queue. (Other runs
        // from a shared test DB may also be enqueued — we only assert ours.)
        let want = run.to_string();
        let n = queue.status().await.pending;
        let mut ours = 0;
        for _ in 0..n {
            let t = queue.next_task().await;
            assert_eq!(t.kind, TaskKind::AdvanceRun);
            if t.path == want { ours += 1; }
        }
        assert_eq!(ours, 1, "two ticks enqueue the AdvanceRun for a run only once");

        sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
            .bind(run).execute(pg.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn resume_due_logs_event_and_enqueues_tick() {
        // Exercises the resume_due helper: a due pause is flipped to running, a
        // Resumed event is logged, and an AdvanceRun tick is enqueued for it.
        //
        // `resume_due_runs` is a *global* UPDATE and the test DB is shared, so a
        // concurrently-running test could flip THIS run first — leaving our
        // resume_due with an empty set and nothing to log. Serialize with the
        // pg_store resume test on the shared lock (production has a single
        // scheduler, so there is no such race there).
        let _guard = crate::runs::resume_test_lock().lock().await;

        let Ok(pg) = PgStore::connect_test().await else { return; };
        let queue = TaskQueue::with_max_repos(16);

        use crate::runs::NewRun;
        use dojo_protocol::relay::RelayRunStatus;
        // Drain any pre-existing due pauses left by other tests so our resume_due
        // sees a clean slate and provably resumes only the run we create.
        pg.resume_due_runs().await.unwrap();

        let due = pg.create_run(&NewRun::default()).await.unwrap();
        pg.update_run_status(&due, RelayRunStatus::Paused, Some("2000-01-01T00:00:00Z"), Some("cap"))
            .await.unwrap();

        resume_due(&queue, &pg).await;

        // Our run is now running with its pause cleared…
        let run = pg.get_run(&due).await.unwrap().unwrap();
        assert_eq!(run.status, RelayRunStatus::Running);
        assert!(run.paused_until.is_none());
        // …a Resumed event was logged for it (resume_due appends per resumed id)…
        let events = pg.list_run_events(&due, 10).await.unwrap();
        assert!(events.iter().any(|e| e.kind == RunEventKind::Resumed), "Resumed event logged");
        // …and an AdvanceRun tick was enqueued for it.
        let n = queue.status().await.pending;
        let mut saw_due = false;
        for _ in 0..n {
            let t = queue.next_task().await;
            assert_eq!(t.kind, TaskKind::AdvanceRun);
            if t.path == due.to_string() { saw_due = true; }
        }
        assert!(saw_due, "resume_due enqueues a tick for the resumed run");

        sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
            .bind(due).execute(pg.pool()).await.unwrap();
    }
}
