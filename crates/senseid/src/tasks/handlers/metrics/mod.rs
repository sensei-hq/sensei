//! Metrics compute handlers (Phase 4 skeleton).
//!
//! `ComputeMetrics` handles EVERY base metric group with ONE task kind — the
//! specific group (the registry `task_name`, e.g. `"session_outcomes"`) travels
//! in `task.path`, and the project id rides in `task.folder_path`. Keeping the
//! group in the payload (not one `TaskKind` per group) is deliberate: it avoids
//! per-group enum plumbing across the queue/retry/watchdog/executor surface.
//!
//! `ComputeHealth` is the SEPARATE barrier kind that runs AFTER a project's base
//! metrics land (the scheduler wires it `blocked_by` the project's
//! `ComputeMetrics` ids). Its own registry `task_name` is [`HEALTH_TASK_NAME`],
//! which is therefore NOT a base group.
//!
//! Phase 4 is the skeleton: routing + stubs. Each known group logs
//! `computing <group>` and returns `Ok(0)`; Phase 5 fills in the real per-group
//! computation (read the window, write `sensei.project_metrics`) and Phase 6
//! fills in [`compute_health`]. An UNKNOWN `task_name` is an intentional logged
//! no-op that returns `Ok` — a registry entry the daemon doesn't yet know about
//! degrades to a warning, never a panic or a stuck queue.

use super::super::executor::TaskContext;
use super::super::Task;

/// Test-only routing probe: records WHICH handler last ran on the current thread,
/// so the executor's dispatch test can assert `ComputeMetrics → compute` and
/// `ComputeHealth → compute_health` by IDENTITY (both stubs return `Ok(0)`, so a
/// swapped match arm would otherwise stay green until Phase 5/6 make the paths
/// diverge). Thread-local so it's isolated per `#[tokio::test]` (each runs on its
/// own current-thread runtime); guarded by `#[cfg(test)]` so it compiles out of
/// production entirely.
#[cfg(test)]
pub(crate) mod probe {
    use std::cell::Cell;

    thread_local! {
        static LAST: Cell<Option<&'static str>> = const { Cell::new(None) };
    }

    /// Record that handler `which` ran on this thread.
    pub(crate) fn record(which: &'static str) {
        LAST.with(|c| c.set(Some(which)));
    }
    /// Take (and clear) the last-recorded handler identity for this thread.
    pub(crate) fn take() -> Option<&'static str> {
        LAST.with(|c| c.take())
    }
    /// Clear any prior recording so a fresh assertion starts from a clean slate.
    pub(crate) fn reset() {
        LAST.with(|c| c.set(None));
    }
}

/// The registry `task_name` of the health barrier. It is computed by the separate
/// [`ComputeHealth`](crate::tasks::TaskKind::ComputeHealth) kind, NOT dispatched
/// as a base `ComputeMetrics` group, so the metrics scheduler filters it out of
/// the `ComputeMetrics` enumeration. Shared with `metrics_scheduler` so the two
/// can't drift.
pub(crate) const HEALTH_TASK_NAME: &str = "health";

/// The base metric groups `ComputeMetrics` dispatches to — the registry
/// `task_name` (carried in `task.path`) mapped to a typed group. Kept as an enum
/// with a pure [`MetricGroup::from_task_name`] so routing is unit-testable
/// without a DB and the dispatch can't silently drift from the registry. `health`
/// is deliberately absent (it is the `ComputeHealth` barrier, not a base group).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MetricGroup {
    SessionOutcomes,
    Churn,
    Duplication,
    Autonomy,
    Knowledge,
    Tool,
}

impl MetricGroup {
    /// Map a registry `task_name` to its base group, or `None` for the health
    /// barrier's own name and for any group the daemon doesn't (yet) know.
    pub(crate) fn from_task_name(task_name: &str) -> Option<Self> {
        match task_name {
            "session_outcomes" => Some(Self::SessionOutcomes),
            "churn" => Some(Self::Churn),
            "duplication" => Some(Self::Duplication),
            "autonomy" => Some(Self::Autonomy),
            "knowledge" => Some(Self::Knowledge),
            "tool" => Some(Self::Tool),
            _ => None,
        }
    }

    /// The registry `task_name` for this group — the stable label used in logs.
    fn as_str(self) -> &'static str {
        match self {
            Self::SessionOutcomes => "session_outcomes",
            Self::Churn => "churn",
            Self::Duplication => "duplication",
            Self::Autonomy => "autonomy",
            Self::Knowledge => "knowledge",
            Self::Tool => "tool",
        }
    }
}

/// `ComputeMetrics` handler: dispatch by the metric group in `task.path` (project
/// id in `task.folder_path`). A known group runs its computer; an unknown
/// `task_name` is a logged no-op that returns `Ok` — never a panic, never a queue
/// error.
///
/// Phase 4 stub: a known group only logs `computing <group>` and returns `Ok(0)`.
/// `ctx` is unused here but is the seam Phase 5 computes from (`ctx.pg()` + the
/// configured window → `upsert_project_metric`).
pub async fn compute(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    #[cfg(test)]
    probe::record("compute");
    let _ = ctx; // Phase 5 seam: read the window + write sensei.project_metrics.
    match MetricGroup::from_task_name(&task.path) {
        Some(group) => {
            tracing::info!(
                group = group.as_str(),
                project = %task.folder_path,
                "compute_metrics: computing {} (stub — Phase 5 fills in)",
                group.as_str(),
            );
            Ok(0)
        }
        None => {
            // Intentional logged no-op (NOT fabrication): a registry entry whose
            // task_name the daemon doesn't map degrades to a warning instead of
            // panicking the worker or wedging the queue.
            tracing::warn!(
                task_name = %task.path,
                project = %task.folder_path,
                "compute_metrics: unknown metrics task_name — no-op",
            );
            Ok(0)
        }
    }
}

/// `ComputeHealth` handler (Phase 4 stub): the per-project barrier that runs after
/// the base `ComputeMetrics` tasks land (wired via `blocked_by` in the scheduler).
/// Phase 6 fills in the derived `project_health` score; the stub logs and returns
/// `Ok` so the barrier completes and the pipeline is exercised end-to-end. The
/// project id rides in `task.folder_path`.
pub async fn compute_health(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    #[cfg(test)]
    probe::record("compute_health");
    let _ = ctx; // Phase 6 seam: aggregate the project's base metrics into a score.
    tracing::info!(
        project = %task.folder_path,
        "compute_health: (stub — Phase 6 fills in)",
    );
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::test_support::make_ctx;
    use crate::tasks::{Task, TaskKind};

    #[test]
    fn metric_group_routes_known_task_names() {
        // Every v1 base group maps to its typed group — the routing seam the
        // dispatch relies on.
        assert_eq!(MetricGroup::from_task_name("session_outcomes"), Some(MetricGroup::SessionOutcomes));
        assert_eq!(MetricGroup::from_task_name("churn"), Some(MetricGroup::Churn));
        assert_eq!(MetricGroup::from_task_name("duplication"), Some(MetricGroup::Duplication));
        assert_eq!(MetricGroup::from_task_name("autonomy"), Some(MetricGroup::Autonomy));
        assert_eq!(MetricGroup::from_task_name("knowledge"), Some(MetricGroup::Knowledge));
        assert_eq!(MetricGroup::from_task_name("tool"), Some(MetricGroup::Tool));
    }

    #[test]
    fn metric_group_rejects_unknown_and_health() {
        // A genuinely-unknown task_name has no group → handler logs a no-op.
        assert_eq!(MetricGroup::from_task_name("no_such_group"), None);
        // `health` is the ComputeHealth barrier's name, NOT a base ComputeMetrics
        // group — so it never routes to a base computer.
        assert_eq!(MetricGroup::from_task_name(HEALTH_TASK_NAME), None);
    }

    #[tokio::test]
    async fn compute_metrics_dispatches_by_task_name() {
        let ctx = make_ctx().await;
        let pid = uuid::Uuid::new_v4().to_string();

        // A known task_name routes to its computer (stub) and returns Ok.
        let known = Task::new(TaskKind::ComputeMetrics, &pid, "session_outcomes");
        assert_eq!(
            compute(&ctx, &known).await.unwrap(),
            0,
            "a known group computes (stub) and returns Ok",
        );

        // An UNKNOWN task_name is a logged no-op — Ok, never a panic, never a
        // queue error (so a registry entry the daemon doesn't know degrades to a
        // warning, not a stuck queue).
        let unknown = Task::new(TaskKind::ComputeMetrics, &pid, "no_such_group");
        assert_eq!(
            compute(&ctx, &unknown).await.unwrap(),
            0,
            "an unknown task_name is a no-op that returns Ok",
        );
    }

    #[tokio::test]
    async fn compute_health_stub_returns_ok() {
        let ctx = make_ctx().await;
        let pid = uuid::Uuid::new_v4().to_string();
        let task = Task::new(TaskKind::ComputeHealth, &pid, "");
        assert_eq!(
            compute_health(&ctx, &task).await.unwrap(),
            0,
            "the health barrier stub completes with Ok",
        );
    }
}
