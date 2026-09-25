//! Transcript scheduler — keep assistant transcripts ingesting after boot.
//!
//! `IngestCaptures` is a dispatcher: it enqueues one `IngestCapture` per
//! changed or new transcript, then repairs sessions. Everything about it was
//! already right except WHEN it ran. It was enqueued from exactly one place —
//! the boot block in `api::server` — and named in no schedule, so ingestion
//! happened once per daemon start and never again.
//!
//! MEASURED before this module existed, on a daemon up for 1h39m:
//!
//! | | |
//! |---|---:|
//! | `max(activity.transcript_turns.created_at)` | 21:21:45 |
//! | `max(activity.capture_watermarks.updated_at)` | 21:21:45 |
//! | daemon start | 21:21:32 |
//!
//! Every watermark and every turn stopped thirteen seconds after boot. A
//! transcript written since was not late, it was never going to arrive.
//!
//! The boot pass lives HERE rather than in `api::server`, which is the shape
//! [`crate::tasks::reconcile_scheduler`] settled on: a scheduler owns both its
//! boot tick and its cadence, so there is one place that knows when this work
//! happens. Leaving the enqueue in the boot block and adding a cadence beside
//! it would have made a second site to keep in step — and a third copy of that
//! sequence is exactly what `transcript::backfill`'s own doc comment records
//! having already drifted once.

use std::sync::Arc;

use crate::db::pg_store::PgStore;
use crate::tasks::queue::TaskQueue;
use crate::tasks::ticker;
use crate::tasks::{Task, TaskKind};

/// Enqueue one `IngestCaptures` dispatcher, unless one is already in flight.
///
/// Overlap-guarded rather than deduplicated after the fact: the dispatcher
/// sweeps the transcript tree and enqueues per-file work, so two of them
/// running together do the sweep twice and race on the same watermarks. A
/// skipped tick costs nothing — the next one picks up whatever arrived, since
/// the dispatcher's unit of work is "changed since the watermark" and not
/// "since the last tick".
///
/// Returns whether one was enqueued, so a caller can say which happened
/// instead of logging an unconditional success.
async fn enqueue_backfill(queue: &TaskQueue) -> bool {
    if queue.has_pending_kind(TaskKind::IngestCaptures).await {
        tracing::debug!("transcript_scheduler: a backfill is already in flight — skipping");
        return false;
    }
    let id = queue.enqueue(Task::new(TaskKind::IngestCaptures, "", "")).await;
    tracing::debug!(task_id = id, "transcript_scheduler: enqueued transcript backfill");
    true
}

/// Spawn the transcript scheduler for the daemon's lifetime.
pub fn spawn(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    tokio::spawn(run(queue, pg));
}

async fn run(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    // THE BOOT PASS ALWAYS RUNS, and it is not the schedule's business: a
    // restart is the one moment a newly-tracked folder can make a previously
    // unresolvable cwd resolvable, so the session repair wants to happen then
    // whether or not the cadence says a run is due.
    if enqueue_backfill(&queue).await {
        tracing::info!("startup: enqueued transcript backfill for metric history");
    }

    // From here the schedule owns the cadence (name `ingest_captures`).
    ticker::run_scheduled(pg, "ingest_captures", move || {
        let queue = queue.clone();
        async move {
            enqueue_backfill(&queue).await;
            Ok(())
        }
    })
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tick enqueues the dispatcher, and a second tick while it is still
    /// in flight does NOT stack a duplicate.
    ///
    /// MUTATION: delete the `has_pending_kind` guard and the second assertion
    /// flips — two dispatchers sweep the same tree and race on one watermark.
    #[tokio::test]
    async fn a_tick_enqueues_one_backfill_and_will_not_stack_a_second() {
        let queue = TaskQueue::with_max_repos(64);

        assert!(enqueue_backfill(&queue).await, "the first tick enqueues");
        assert!(!enqueue_backfill(&queue).await, "a second tick must not stack a duplicate");

        let t = queue.next_task().await;
        assert_eq!(t.kind, TaskKind::IngestCaptures, "the dispatcher is what gets enqueued");
        queue.complete(t.id).await;

        // Once the in-flight one is done, the next tick enqueues again — the
        // guard suppresses an overlap, not the cadence.
        assert!(enqueue_backfill(&queue).await, "a later tick enqueues once the queue is clear");
        let t = queue.next_task().await;
        queue.complete(t.id).await;
    }
}
