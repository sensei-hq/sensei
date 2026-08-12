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
//! Phase 5 fills in the real per-group computation (read the window, write
//! `sensei.project_metrics`) for the six base groups; Phase 6 fills in
//! [`compute_health`] (the derived `project_health` roll-up in [`health`]). An
//! UNKNOWN `task_name` is an intentional logged no-op that returns `Ok` — a registry
//! entry the daemon doesn't yet know about degrades to a warning, never a panic or a
//! stuck queue.

use super::super::executor::TaskContext;
use super::super::Task;
use crate::db::pg_store::PgStore;

/// Per-group computers. `session_outcomes` is the first real one (Phase 5.1) and
/// the template the remaining groups follow: `churn` (Phase 5.2), `autonomy`
/// (Phase 5.4), `knowledge` (Phase 5.5), and `tool` (Phase 5.6) complete the base
/// groups; `quality` (Phase 8) is the git-worktree + `qlty` code-quality group that
/// superseded the former own-graph `duplication` snapshot.
mod autonomy;
mod churn;
mod explainer;
mod health;
mod knowledge;
pub(crate) mod planner;
mod quality;
mod session_outcomes;
mod tool;

/// Today's date (DB `current_date`) — the `computed_on` for the SNAPSHOT metrics
/// (`churn`'s `rework_density`) that store a point-in-time value rather than a
/// windowed per-day series. Read from the DB so
/// the day boundary matches the `date_trunc('day', started_at)::date` the windowed
/// computers use (same session TZ). Shared by every snapshot computer so the day
/// source can't drift between groups.
pub(super) async fn today(pg: &PgStore) -> Result<chrono::NaiveDate, String> {
    let (d,): (chrono::NaiveDate,) = sqlx_core::query_as::query_as("SELECT current_date")
        .fetch_one(pg.pool())
        .await
        .map_err(|e| e.to_string())?;
    Ok(d)
}

/// Whether a forward-only SNAPSHOT computer must SKIP for this `as_of` (Phase 3).
/// The snapshot metrics (`churn`'s `rework_density`, `knowledge`, `tool`, `health`)
/// reflect CURRENT state — they cannot be reconstructed for a past
/// day from present data — so a historical target day (`Some(D)` with `D != today`)
/// has no honest value: the computer writes NO row (never a fabricated historical
/// snapshot). `None` (today's incremental run) or `Some(today)` → compute as normal.
/// Shared so the "past day → skip" rule (and its `today` source) can't drift between
/// the forward-only groups.
pub(super) async fn is_historical(
    pg: &PgStore,
    as_of: Option<chrono::NaiveDate>,
) -> Result<bool, String> {
    match as_of {
        Some(d) => Ok(d != today(pg).await?),
        None => Ok(false),
    }
}

/// The `$2`-anchored single-day-or-window SQL filter shared by the per-day base
/// computers. `anchor` is the group's occurrence-time expression — the timestamptz
/// it buckets/windows on (`s.started_at`, `r.started_at`,
/// `to_timestamp(ae.ts / 1000.0)`, …). `as_of = None` → the rolling window
/// (`$2 = window_days::int`, the incremental behavior); `Some(_)` → a single
/// historical day (`$2 = D::date`, the backfill/gap-fill path). Kept in ONE place so
/// the window/day SQL (and its `$2` contract with [`bind_day`]) can't drift between
/// groups. The `day` SELECT column each computer emits must use the SAME `anchor`
/// (`date_trunc('day', <anchor>)::date`) so `computed_on` matches the filter.
pub(super) fn day_filter(anchor: &str, as_of: Option<chrono::NaiveDate>) -> String {
    match as_of {
        Some(_) => format!("date_trunc('day', {anchor})::date = $2::date"),
        None => format!("{anchor} >= now() - make_interval(days => $2::int)"),
    }
}

/// Bind `$2` for [`day_filter`]: the target day on the `Some` path, else the window
/// length. Consumes and returns the query so callers stay one-liners. Anchor-agnostic
/// — the same for every per-day computer, so the `$2` binding lives in ONE place.
pub(super) fn bind_day<'q, O>(
    q: sqlx_core::query_as::QueryAs<'q, sqlx_postgres::Postgres, O, sqlx_postgres::PgArguments>,
    window_days: u32,
    as_of: Option<chrono::NaiveDate>,
) -> sqlx_core::query_as::QueryAs<'q, sqlx_postgres::Postgres, O, sqlx_postgres::PgArguments> {
    match as_of {
        Some(d) => q.bind(d),
        None => q.bind(window_days as i32),
    }
}

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
    Quality,
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
            "quality" => Some(Self::Quality),
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
            Self::Quality => "quality",
            Self::Autonomy => "autonomy",
            Self::Knowledge => "knowledge",
            Self::Tool => "tool",
        }
    }
}

/// `ComputeMetrics` handler: dispatch by the metric group in `task.path` (project
/// id in `task.folder_path`). A known group runs its computer; an unknown
/// `task_name` is a logged no-op that returns `Ok` — never a panic, never a queue
/// error. Returns the number of `project_metrics` rows the group wrote.
///
/// Phases 5.1–5.6 wire all six v1 base groups — `session_outcomes`, `churn`,
/// `duplication`, `autonomy`, `knowledge`, and `tool` — to their real computers;
/// an UNKNOWN `task_name` remains an honest logged no-op, never a fabricated value.
pub async fn compute(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    #[cfg(test)]
    probe::record("compute");
    match MetricGroup::from_task_name(&task.path) {
        Some(group) => {
            tracing::info!(
                group = group.as_str(),
                project = %task.folder_path,
                "compute_metrics: computing {}",
                group.as_str(),
            );
            // `task.as_of` is the target `computed_on` day: `None` = today's
            // incremental (rolling-window) run, `Some(D)` = the single historical
            // day D (backfill/gap-fill). Threaded into every computer so the crate
            // compiles uniformly; only `session_outcomes` implements per-day
            // behavior in this phase — the rest accept the param and keep their
            // current behavior (later phases wire them up).
            let as_of = task.as_of;
            let written = match group {
                MetricGroup::SessionOutcomes => {
                    session_outcomes::compute(ctx, &task.folder_path, as_of).await
                }
                MetricGroup::Churn => churn::compute(ctx, &task.folder_path, as_of).await,
                MetricGroup::Quality => quality::compute(ctx, &task.folder_path, as_of).await,
                MetricGroup::Autonomy => autonomy::compute(ctx, &task.folder_path, as_of).await,
                MetricGroup::Knowledge => knowledge::compute(ctx, &task.folder_path, as_of).await,
                MetricGroup::Tool => tool::compute(ctx, &task.folder_path, as_of).await,
            }?;
            // Compute-time per-datapoint EXPLAINER enrichment: generate each
            // datapoint's "why this day's value is what it is" line WITH the value
            // and merge it into props.explainer. Best-effort — the value is already
            // persisted, so a failed/absent-model explainer never fails the compute.
            // Cache-guarded (unchanged value ⇒ no model call). Rides the planner's
            // per-day Some(D) tasks; a bare None (rolling) run enriches today.
            // Decision record: docs/analysis/metric-explainability-generation.md.
            if let Ok(project_id) = uuid::Uuid::parse_str(&task.folder_path) {
                let day = match as_of {
                    Some(d) => Some(d),
                    None => today(ctx.pg()).await.ok(),
                };
                if let Some(day) = day {
                    explainer::enrich_day(ctx, &project_id, group.as_str(), day).await;
                }
            }
            Ok(written)
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

/// `ComputeHealth` handler (Phase 6): the per-project barrier that runs after the
/// base `ComputeMetrics` tasks land (wired via `blocked_by` in the scheduler). It
/// rolls the project's latest daily component values into the derived
/// `project_health` score — see [`health::compute`] for the normalization and the
/// never-fabricate rules. The project id rides in `task.folder_path`. Returns the
/// number of `project_metrics` rows written (`1` when ≥1 component is included, else
/// `0` — no components ⇒ NO row, never a fabricated score).
pub async fn compute_health(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    #[cfg(test)]
    probe::record("compute_health");
    tracing::info!(
        project = %task.folder_path,
        "compute_health: computing project_health",
    );
    // Thread the target day through the barrier too (accepted now, per-day wiring
    // in a later phase); `None` keeps the current snapshot-on-today behavior.
    health::compute(ctx, &task.folder_path, task.as_of).await
}

/// `PlanMetricDays` handler (Phase 4): the per-project timeline → day-task planner.
/// The project id rides in `task.folder_path`. Delegates to [`planner::plan`],
/// which — for each ACTIVE day-keyed group — diffs the project's data-day set
/// against the already-covered `computed_on` days and enqueues one
/// `ComputeMetrics{as_of=Some(D)}` per missing day plus today. Returns the number
/// of `ComputeMetrics` tasks enqueued (`0` = honest-empty: no active day-keyed
/// groups). Never fabricates: a day with no source data simply yields no task
/// (and the downstream compute would write no row).
pub async fn plan_metric_days(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    #[cfg(test)]
    probe::record("plan_metric_days");
    tracing::info!(
        project = %task.folder_path,
        "plan_metric_days: planning per-day metric compute",
    );
    planner::plan(ctx, &task.folder_path).await
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
        assert_eq!(MetricGroup::from_task_name("quality"), Some(MetricGroup::Quality));
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

        // A known task_name routes to its computer and returns Ok. `pid` is a
        // random (nonexistent) project, so session_outcomes finds no sessions and
        // honestly writes 0 rows — the routing is what's under test here.
        let known = Task::new(TaskKind::ComputeMetrics, &pid, "session_outcomes");
        assert_eq!(
            compute(&ctx, &known).await.unwrap(),
            0,
            "a known group routes to its computer and returns Ok (0 rows for an empty project)",
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
