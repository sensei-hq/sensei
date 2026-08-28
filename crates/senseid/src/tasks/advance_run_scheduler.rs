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

use crate::db::pg_store::PgStore;
use crate::runs::RunEventKind;
use crate::tasks::queue::TaskQueue;
use crate::tasks::ticker;
use crate::tasks::{Task, TaskKind};

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

/// Enqueue a `PublishRun` status-federation tick for one run id — de-duped, the
/// same contract as [`enqueue_advance`] but for the P1 run→relay bridge. Skips if
/// a `PublishRun` for this run is already pending so a backed-up queue never piles
/// up duplicate publishes. STATUS only — this federates the run's status/heartbeat
/// to Relay; it never drives the run.
pub(crate) async fn enqueue_publish_run(queue: &TaskQueue, run_id: &uuid::Uuid) {
    let path = run_id.to_string();
    if queue.has_pending_kind_path(TaskKind::PublishRun, &path).await {
        return;
    }
    queue.enqueue(Task::new(TaskKind::PublishRun, "", &path)).await;
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
            .append_run_event(
                &id,
                RunEventKind::Resumed,
                None,
                None,
                &serde_json::json!({ "auto": true }),
            )
            .await
        {
            tracing::warn!(run_id = %id, error = %e, "advance_run_scheduler: logging Resumed event failed");
        }
        enqueue_advance(queue, &id).await;
        // Federate the resume (paused → running) promptly, not just on the next
        // active-run sweep.
        enqueue_publish_run(queue, &id).await;
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
                // P1: federate the run's status/heartbeat to Relay each tick (and
                // thus on any status change) so Jerry can watch the build. STATUS
                // only — the publish never drives the run.
                enqueue_publish_run(queue, &run.id).await;
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
    // Cadence lives in `sensei.schedules` (name `advance_run`); the tick is
    // unchanged.
    let store = pg.clone();
    ticker::run_scheduled(pg, "advance_run", move || {
        let (queue, pg) = (queue.clone(), store.clone());
        async move {
            tick(&queue, &pg).await;
            Ok(())
        }
    })
    .await;
}

#[cfg(test)]
// `resume_test_guard()` is a blocking `std::sync::Mutex` held across awaits on
// purpose — see `crate::tasks::test_support::TestGate` for why an async mutex loses
// wakeups here. These are current-thread test runtimes, one per test, so
// blocking the thread costs nothing and cannot deadlock the runtime.
#[allow(clippy::await_holding_lock)]
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
    async fn enqueue_publish_run_carries_run_id_and_dedups() {
        let queue = TaskQueue::new();
        let id = uuid::Uuid::new_v4();
        enqueue_publish_run(&queue, &id).await;
        // A second enqueue with nothing drained is de-duped.
        enqueue_publish_run(&queue, &id).await;

        assert_eq!(queue.status().await.pending, 1, "publish enqueue is de-duped");
        let t = queue.next_task().await;
        assert_eq!(t.kind, TaskKind::PublishRun);
        assert_eq!(t.path, id.to_string(), "the run id rides in task.path");
        assert_eq!(t.folder_path, "", "no folder scoping for a run publish");
    }

    #[tokio::test]
    async fn tick_enqueues_one_per_active_run() {
        // DB-guarded: with a test DB, a tick over N active runs enqueues N ticks.
        // `tick` calls the global `resume_due_runs`, so hold the shared resume
        // lock to serialize with the other resume-race tests.
        let _guard = crate::runs::resume_test_guard();
        let Ok(pg) = PgStore::connect_test().await else {
            return;
        };
        let queue = TaskQueue::with_max_repos(16);

        use crate::runs::NewRun;
        let a = pg.create_run(&NewRun::default()).await.unwrap();
        let b = pg.create_run(&NewRun::default()).await.unwrap();

        tick(&queue, &pg).await;

        // Inspect the queue with `snapshot()` rather than draining it with
        // `next_task()`. Run tasks all carry folder_path "" (runs aren't folder
        // scoped), so the per-repo cap applies to them COLLECTIVELY: popping
        // max_repos of them without ever completing one leaves nothing
        // dispatchable, and `next_task()` then parks on `notified()` forever. On a
        // shared test DB `pending` routinely exceeds the cap, so the old
        // drain-`pending`-times loop deadlocked rather than flaked.
        // (Other runs from a shared test DB may also appear — we only assert ours
        // are present, and that every enqueued task is one of the two run kinds.)
        let mut advance_paths = std::collections::HashSet::new();
        let mut publish_paths = std::collections::HashSet::new();
        for (kind, _folder, path) in queue.snapshot().await {
            match kind {
                TaskKind::AdvanceRun => {
                    advance_paths.insert(path);
                }
                TaskKind::PublishRun => {
                    publish_paths.insert(path);
                }
                other => panic!("unexpected task kind enqueued by tick: {other}"),
            }
        }
        assert!(advance_paths.contains(&a.to_string()), "run a is ticked (advance)");
        assert!(advance_paths.contains(&b.to_string()), "run b is ticked (advance)");
        assert!(publish_paths.contains(&a.to_string()), "run a is published");
        assert!(publish_paths.contains(&b.to_string()), "run b is published");

        for id in [a, b] {
            sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
                .bind(id)
                .execute(pg.pool())
                .await
                .unwrap();
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
        let _guard = crate::runs::resume_test_guard();
        let Ok(pg) = PgStore::connect_test().await else {
            return;
        };
        let queue = TaskQueue::with_max_repos(16);

        use crate::runs::NewRun;
        let run = pg.create_run(&NewRun::default()).await.unwrap();

        // Two ticks, nothing dequeued in between.
        tick(&queue, &pg).await;
        tick(&queue, &pg).await;

        // Count the run's ticks across the whole queue, per kind. (Other runs from
        // a shared test DB may also be enqueued — we only assert ours.) Both the
        // AdvanceRun and the PublishRun enqueue are de-duped, so two ticks with
        // nothing drained enqueue each kind for our run exactly once.
        let want = run.to_string();
        let mut ours_advance = 0;
        let mut ours_publish = 0;
        for (kind, _folder, path) in queue.snapshot().await {
            if path == want {
                match kind {
                    TaskKind::AdvanceRun => ours_advance += 1,
                    TaskKind::PublishRun => ours_publish += 1,
                    other => panic!("unexpected task kind: {other}"),
                }
            }
        }
        assert_eq!(ours_advance, 1, "two ticks enqueue the AdvanceRun for a run only once");
        assert_eq!(ours_publish, 1, "two ticks enqueue the PublishRun for a run only once");

        sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
            .bind(run)
            .execute(pg.pool())
            .await
            .unwrap();
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
        let _guard = crate::runs::resume_test_guard();

        let Ok(pg) = PgStore::connect_test().await else {
            return;
        };
        let queue = TaskQueue::with_max_repos(16);

        use crate::runs::NewRun;
        use dojo_protocol::relay::RelayRunStatus;
        // Drain any pre-existing due pauses left by other tests so our resume_due
        // sees a clean slate and provably resumes only the run we create.
        pg.resume_due_runs().await.unwrap();

        let due = pg.create_run(&NewRun::default()).await.unwrap();
        pg.update_run_status(
            &due,
            RelayRunStatus::Paused,
            Some("2000-01-01T00:00:00Z"),
            Some("cap"),
        )
        .await
        .unwrap();

        resume_due(&queue, &pg).await;

        // Our run is now running with its pause cleared…
        let run = pg.get_run(&due).await.unwrap().unwrap();
        assert_eq!(run.status, RelayRunStatus::Running);
        assert!(run.paused_until.is_none());
        // …a Resumed event was logged for it (resume_due appends per resumed id)…
        let events = pg.list_run_events(&due, 10).await.unwrap();
        assert!(events.iter().any(|e| e.kind == RunEventKind::Resumed), "Resumed event logged");
        // …and both an AdvanceRun tick AND a PublishRun status-federation tick
        // were enqueued for it.
        let want = due.to_string();
        let mut saw_advance = false;
        let mut saw_publish = false;
        for (kind, _folder, path) in queue.snapshot().await {
            if path == want {
                match kind {
                    TaskKind::AdvanceRun => saw_advance = true,
                    TaskKind::PublishRun => saw_publish = true,
                    other => panic!("unexpected task kind: {other}"),
                }
            }
        }
        assert!(saw_advance, "resume_due enqueues an AdvanceRun tick for the resumed run");
        assert!(saw_publish, "resume_due enqueues a PublishRun tick for the resumed run");

        sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
            .bind(due)
            .execute(pg.pool())
            .await
            .unwrap();
    }
}
