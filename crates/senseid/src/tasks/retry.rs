//! Bounded-retry policy for the task engine (D6c).
//!
//! When a retryable task fails, it is re-enqueued with `retry_number + 1` after
//! an exponential backoff — up to [`MAX_RETRIES`], then the failure is terminal.
//! The queue is in-memory and has no delayed-task primitive, so the backoff is a
//! spawned `sleep → enqueue_unique` (never a blocking wait that would pin a
//! worker). Retries are scoped to the graph write pipeline; permanent-failure
//! kinds (bad payload / missing arg) are not retried.

use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

use super::queue::TaskQueue;
use super::{Task, TaskKind};

/// Maximum automatic retries after the initial attempt. Once a task's
/// `retry_number` reaches this, its failure is terminal.
pub const MAX_RETRIES: u32 = 3;

/// Backoff cap so an exhausted-but-large attempt count can't schedule a wildly
/// distant retry.
const BACKOFF_CAP_SECS: u64 = 60;

/// Whether a failed task of this kind is worth retrying — the graph write
/// pipeline (D6c). Other kinds fail for permanent reasons (a missing URL, a bad
/// project-id payload, a deleted root), so retrying them just burns cycles.
pub fn is_retryable(kind: &TaskKind) -> bool {
    matches!(
        kind,
        TaskKind::ProcessFile
            | TaskKind::ProcessGitFolder
            | TaskKind::ProcessFolder
            | TaskKind::BuildConnections
            | TaskKind::DetectCommunities
            // Metrics compute + health barrier fail for transient reasons (a DB
            // hiccup reading the window / writing project_metrics), so a bounded
            // retry re-drives them; the daily scheduler would otherwise not
            // re-attempt until the next day.
            | TaskKind::ComputeMetrics
            | TaskKind::ComputeHealth
    )
}

/// Exponential backoff before the given attempt. `attempt` is the
/// `retry_number` of the NEXT attempt (1 = first retry → 2s, 2 → 4s, 3 → 8s),
/// capped at [`BACKOFF_CAP_SECS`].
pub fn backoff(attempt: u32) -> Duration {
    let secs = 2u64.saturating_pow(attempt).min(BACKOFF_CAP_SECS);
    Duration::from_secs(secs)
}

/// The next attempt for a failed task, or `None` when it is not retryable or has
/// exhausted [`MAX_RETRIES`] (a terminal failure).
pub fn plan_retry(task: &Task) -> Option<Task> {
    if !is_retryable(&task.kind) || task.retry_number >= MAX_RETRIES {
        return None;
    }
    Some(task.retry())
}

/// Schedule a backed-off re-enqueue of `next` on a detached task — sleeps
/// `delay`, then `enqueue_unique` (single-writer safe, D6e). Never blocks the
/// caller's worker. Returns the join handle (ignored in production; awaited in
/// tests).
pub fn spawn_retry(queue: Arc<TaskQueue>, next: Task, delay: Duration) -> JoinHandle<Option<u64>> {
    tokio::spawn(async move {
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        queue.enqueue_unique(next).await
    })
}

/// The one call the worker's failure arm makes (D6c): if `task` is a retryable
/// kind that hasn't exhausted [`MAX_RETRIES`], schedule its next attempt after
/// the appropriate [`backoff`] and return the spawned handle; otherwise the
/// failure is terminal and this returns `None`. The handle is ignored in
/// production (the retry runs detached) and inspected in tests.
pub fn schedule_if_retryable(queue: Arc<TaskQueue>, task: &Task) -> Option<JoinHandle<Option<u64>>> {
    let next = plan_retry(task)?;
    let delay = backoff(next.retry_number);
    Some(spawn_retry(queue, next, delay))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_pipeline_kinds_are_retryable() {
        for k in [
            TaskKind::ProcessFile,
            TaskKind::ProcessGitFolder,
            TaskKind::ProcessFolder,
            TaskKind::BuildConnections,
            TaskKind::DetectCommunities,
        ] {
            assert!(is_retryable(&k), "{k} is a graph-pipeline write kind — retryable");
        }
    }

    #[test]
    fn metrics_compute_kinds_are_retryable() {
        // A transient DB fault mid-compute must be re-driven by the bounded retry,
        // not stranded until the next daily scheduler pass.
        for k in [TaskKind::ComputeMetrics, TaskKind::ComputeHealth] {
            assert!(is_retryable(&k), "{k} is a metrics-compute kind — retryable");
        }
    }

    #[test]
    fn permanent_failure_kinds_are_not_retryable() {
        for k in [
            TaskKind::ScanRoot,      // bad/deleted root path is permanent
            TaskKind::ImportLib,     // missing URL is permanent
            TaskKind::BranchSwitch,  // missing branch is permanent
            TaskKind::ScanDocDrift,  // bad project-id payload is permanent
        ] {
            assert!(!is_retryable(&k), "{k} fails for permanent reasons — not retried");
        }
    }

    #[test]
    fn backoff_is_monotonic_exponential_and_capped() {
        assert_eq!(backoff(1), Duration::from_secs(2));
        assert_eq!(backoff(2), Duration::from_secs(4));
        assert_eq!(backoff(3), Duration::from_secs(8));
        // Capped, never unbounded, and never panics on a large attempt count.
        assert_eq!(backoff(100), Duration::from_secs(BACKOFF_CAP_SECS));
        assert!(backoff(2) > backoff(1) && backoff(3) > backoff(2));
    }

    #[test]
    fn plan_retry_advances_until_the_cap_then_stops() {
        let mut t = Task::new(TaskKind::ProcessFile, "repo", "a.rs");
        // 0 → 1 → 2 → 3, then None at the cap.
        for expected in 1..=MAX_RETRIES {
            let next = plan_retry(&t).expect("retryable under the cap");
            assert_eq!(next.retry_number, expected);
            t = next;
        }
        assert_eq!(t.retry_number, MAX_RETRIES);
        assert!(plan_retry(&t).is_none(), "no retry once MAX_RETRIES is reached");
    }

    #[test]
    fn plan_retry_declines_non_retryable_kinds() {
        let t = Task::new(TaskKind::ImportLib, "repo", "react");
        assert!(plan_retry(&t).is_none(), "a non-retryable kind never retries");
    }

    #[tokio::test]
    async fn spawn_retry_reenqueues_the_next_attempt() {
        let queue = Arc::new(TaskQueue::new());
        let mut failed = Task::new(TaskKind::ProcessFile, "repo", "a.rs");
        failed.id = 5;
        let next = failed.retry(); // retry_number 1

        // ZERO delay → deterministic; await the handle for the enqueue result.
        let id = spawn_retry(queue.clone(), next, Duration::ZERO).await.unwrap();
        assert!(id.is_some(), "spawn_retry enqueues the retry");

        let task = queue.next_task().await;
        assert_eq!(task.kind, TaskKind::ProcessFile);
        assert_eq!(task.path, "a.rs");
        assert_eq!(task.retry_number, 1, "the re-enqueued task carries the bumped attempt count");
    }

    #[tokio::test]
    async fn schedule_if_retryable_schedules_for_a_retryable_under_cap() {
        let queue = Arc::new(TaskQueue::new());
        let task = Task::new(TaskKind::ProcessFile, "repo", "a.rs"); // retry_number 0
        let handle = schedule_if_retryable(queue.clone(), &task)
            .expect("a retryable task under the cap schedules a retry");
        // Don't wait out the real backoff — the enqueue mechanics are covered by
        // spawn_retry's ZERO-delay test; here we only assert the decision.
        handle.abort();
    }

    #[tokio::test]
    async fn schedule_if_retryable_declines_when_exhausted() {
        let queue = Arc::new(TaskQueue::new());
        let mut task = Task::new(TaskKind::ProcessFile, "repo", "a.rs");
        task.retry_number = MAX_RETRIES; // already at the cap
        assert!(schedule_if_retryable(queue, &task).is_none(), "an exhausted task is terminal");
    }

    #[tokio::test]
    async fn schedule_if_retryable_declines_non_retryable_kind() {
        let queue = Arc::new(TaskQueue::new());
        let task = Task::new(TaskKind::ScanRoot, "", "/nonexistent"); // not retryable
        assert!(schedule_if_retryable(queue, &task).is_none(), "a non-retryable kind is terminal");
    }

    #[tokio::test]
    async fn spawn_retry_is_single_writer_safe() {
        // A retry must not double-enqueue when the same (kind, folder, path) is
        // already in flight — enqueue_unique dedupes it.
        let queue = Arc::new(TaskQueue::new());
        queue.enqueue(Task::new(TaskKind::ProcessGitFolder, "repo", "repo")).await;
        let mut failed = Task::new(TaskKind::ProcessGitFolder, "repo", "repo");
        failed.id = 9;
        let id = spawn_retry(queue.clone(), failed.retry(), Duration::ZERO).await.unwrap();
        assert!(id.is_none(), "a retry of an already-pending scope is deduped");
    }
}
