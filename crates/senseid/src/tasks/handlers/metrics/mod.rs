//! Metrics compute handlers — the watermark engine dispatch.
//!
//! Three task kinds drive the per-project metric compute:
//! - [`ComputeProjectMetrics`](crate::tasks::TaskKind::ComputeProjectMetrics) — the
//!   per-project PARENT. [`compute_project`] freezes one `as_of` for the wave and
//!   enqueues one [`ComputeGroupMetrics`](crate::tasks::TaskKind::ComputeGroupMetrics)
//!   child per active base group plus the
//!   [`ComputeHealth`](crate::tasks::TaskKind::ComputeHealth) barrier.
//! - `ComputeGroupMetrics` — the per-(project, group) CHILD. The specific group (the
//!   registry `task_name`, e.g. `"session_outcomes"`) travels in `task.path`, the
//!   project id in `task.folder_path`, and the frozen `as_of` in `task.as_of`.
//!   [`compute_group`] partitions the group by cadence, runs its computer for each
//!   planned day, enriches each computed day, and advances the per-repo watermark.
//! - `ComputeHealth` — the SEPARATE barrier kind that runs AFTER a project's base
//!   metrics land (the scheduler wires it `blocked_by` the `ComputeGroupMetrics`
//!   children). Its own registry `task_name` is [`HEALTH_TASK_NAME`], which is
//!   therefore NOT a base group.
//!
//! The engine (day scheduling, watermark sealing, per-day explainer enrichment) lives
//! in [`planner`]; this module owns the dispatch entry points and the shared helpers
//! ([`today`], [`is_historical`], [`day_filter`], [`bind_day`], [`MetricGroup`]). An
//! UNKNOWN `task_name` is an intentional logged no-op that returns `Ok` — a registry
//! entry the daemon doesn't yet know about degrades to a warning, never a panic or a
//! stuck queue.

use super::super::Task;
use super::super::executor::TaskContext;
use crate::db::pg_store::PgStore;

/// Per-group computers. `session_outcomes` is the first real one (Phase 5.1) and
/// the template the remaining groups follow: `churn` (Phase 5.2), `autonomy`
/// (Phase 5.4), `knowledge` (Phase 5.5), and `tool` (Phase 5.6) complete the base
/// groups; `quality` (Phase 8) is the git-worktree + `qlty` code-quality group that
/// superseded the former own-graph `duplication` snapshot.
mod autonomy;
mod churn;
mod cost;
pub(crate) mod coverage;
mod explainer;
mod health;
mod knowledge;
pub(crate) mod planner;
mod quality;
mod session_outcomes;
mod session_process;
mod usage;

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
/// so the executor's dispatch test can assert `ComputeProjectMetrics → compute_project`,
/// `ComputeGroupMetrics → compute_group`, and `ComputeHealth → compute_health` by
/// IDENTITY (the day-keyed/honest-empty paths return `Ok(0)` for an empty project, so a
/// swapped match arm would otherwise stay green). Thread-local so it's isolated per
/// `#[tokio::test]` (each runs on its own current-thread runtime); guarded by
/// `#[cfg(test)]` so it compiles out of production entirely.
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
/// as a base `ComputeGroupMetrics` group, so `compute_project` filters it out of
/// the base-group enumeration. Shared with `metrics_scheduler` so the two
/// can't drift.
pub(crate) const HEALTH_TASK_NAME: &str = "health";

/// The base metric groups `ComputeGroupMetrics` dispatches to — the registry
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
    /// Test coverage, INGESTED from an lcov report the project's test run produced
    /// (the daemon never runs tests). Forward-only snapshot.
    Coverage,
    /// LLM process-quality judgments (spec 2026-08-20): day-keyed metrics derived
    /// from `sessions.props.process` (spec_depth + the three occurrence rates).
    SessionProcess,
    /// Real money: the configured subscription divided by what shipped in the
    /// trailing window. SNAPSHOT cadence — anchored to now, forward-only.
    Cost,
    /// Context-reuse efficiency (`cache_reuse`) from the per-turn token split.
    /// DAY-KEYED: each day's sessions are settled once the day is past.
    Usage,
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
            "cost" => Some(Self::Cost),
            "usage" => Some(Self::Usage),
            "coverage" => Some(Self::Coverage),
            "session_process" => Some(Self::SessionProcess),
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
            Self::Cost => "cost",
            Self::Usage => "usage",
            Self::Coverage => "coverage",
            Self::SessionProcess => "session_process",
        }
    }
}

/// `ComputeGroupMetrics` handler: the per-(project, group) CHILD. The metric group
/// rides in `task.path`, the project id in `task.folder_path`, and the frozen
/// `as_of` (shared by every child of one `ComputeProjectMetrics` wave) in
/// `task.as_of`. Delegates to [`planner::compute_group`], which partitions the group
/// by cadence — SNAPSHOT (`knowledge`/`tool`) → one today-only rolling compute (no
/// watermark); DAY-KEYED (`session_outcomes`/`autonomy`/`churn`/`quality`) →
/// watermark-planned per-day backfill — runs the computer for each planned day, runs
/// the per-day explainer enrichment (moved here from the old `compute`), and, ON
/// SUCCESS ONLY, advances each repo's `sensei.metric_watermarks` cursor to
/// `as_of - 1` (today is never sealed). A failed group propagates `Err` and holds its
/// watermark, so it retries next run (fail-closed — never a silent skipped day). An
/// UNKNOWN `task_name` is a logged no-op that returns `Ok(0)`. Returns the number of
/// `sensei.project_metrics` rows the group wrote.
pub async fn compute_group(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    #[cfg(test)]
    probe::record("compute_group");
    tracing::info!(
        group = %task.path,
        project = %task.folder_path,
        "compute_group: computing metric group",
    );
    planner::compute_group(ctx, task).await
}

/// `ComputeHealth` handler (Phase 6): the per-project barrier that runs after the
/// base `ComputeGroupMetrics` tasks land (wired via `blocked_by` in the scheduler). It
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

/// `ComputeProjectMetrics` handler: the per-project PARENT that schedules the compute
/// wave. The project id rides in `task.folder_path`. Delegates to
/// [`planner::compute_project`], which freezes a single `as_of` for the whole wave
/// (so every child sees the same day), enqueues ONE `ComputeGroupMetrics` child per
/// active base group (each carrying the frozen `as_of`), and — when any child was
/// enqueued — enqueues the per-project `ComputeHealth` barrier `blocked_by` those
/// children. Returns the number of tasks enqueued (`0` = honest-empty: no active base
/// groups → no children and no barrier). Never fabricates: a group with no source
/// data still enqueues a child that honestly writes no row and advances no watermark.
pub async fn compute_project(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    #[cfg(test)]
    probe::record("compute_project");
    tracing::info!(
        project = %task.folder_path,
        "compute_project: scheduling per-project metric compute wave",
    );
    planner::compute_project(ctx, task).await
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
        assert_eq!(
            MetricGroup::from_task_name("session_outcomes"),
            Some(MetricGroup::SessionOutcomes)
        );
        assert_eq!(MetricGroup::from_task_name("churn"), Some(MetricGroup::Churn));
        assert_eq!(MetricGroup::from_task_name("quality"), Some(MetricGroup::Quality));
        assert_eq!(MetricGroup::from_task_name("autonomy"), Some(MetricGroup::Autonomy));
        assert_eq!(MetricGroup::from_task_name("knowledge"), Some(MetricGroup::Knowledge));
        assert_eq!(
            MetricGroup::from_task_name("session_process"),
            Some(MetricGroup::SessionProcess)
        );
        assert_eq!(MetricGroup::from_task_name("coverage"), Some(MetricGroup::Coverage));
        // `tool` is retired (unused_tools → the tool_usage_by_repository view), no longer a group.
        assert_eq!(MetricGroup::from_task_name("tool"), None);
    }

    #[test]
    fn metric_group_rejects_unknown_and_health() {
        // A genuinely-unknown task_name has no group → handler logs a no-op.
        assert_eq!(MetricGroup::from_task_name("no_such_group"), None);
        // `health` is the ComputeHealth barrier's name, NOT a base ComputeGroupMetrics
        // group — so it never routes to a base computer.
        assert_eq!(MetricGroup::from_task_name(HEALTH_TASK_NAME), None);
    }

    #[tokio::test]
    async fn compute_group_dispatches_by_task_name() {
        let ctx = make_ctx().await;
        let pid = uuid::Uuid::new_v4().to_string();

        // A known task_name routes to its computer and returns Ok. `pid` is a
        // random (nonexistent) project with no repositories, so the day-keyed engine
        // finds no repos and honestly writes 0 rows — the routing is what's under
        // test here.
        let known = Task::new(TaskKind::ComputeGroupMetrics, &pid, "session_outcomes");
        assert_eq!(
            compute_group(&ctx, &known).await.unwrap(),
            0,
            "a known group routes to its computer and returns Ok (0 rows for an empty project)",
        );

        // An UNKNOWN task_name is a logged no-op — Ok, never a panic, never a
        // queue error (so a registry entry the daemon doesn't know degrades to a
        // warning, not a stuck queue).
        let unknown = Task::new(TaskKind::ComputeGroupMetrics, &pid, "no_such_group");
        assert_eq!(
            compute_group(&ctx, &unknown).await.unwrap(),
            0,
            "an unknown task_name is a no-op that returns Ok",
        );
    }

    #[tokio::test]
    async fn compute_project_schedules_the_wave() {
        // The per-project PARENT delegates to the engine, which enqueues one
        // ComputeGroupMetrics child per active base group plus (when any child was
        // enqueued) the ComputeHealth barrier. `pid` is a random project, so the
        // wave is driven purely by the active registry. The returned count must equal
        // the tasks actually enqueued onto the queue — routing + delegation under test
        // (the watermark/day-scheduling behavior is covered by the engine tests in
        // `planner`).
        let ctx = make_ctx().await;
        let pid = uuid::Uuid::new_v4().to_string();
        let task = Task::new(TaskKind::ComputeProjectMetrics, &pid, "");
        let enqueued = compute_project(&ctx, &task).await.unwrap();
        let status = ctx.queue.status().await;
        assert_eq!(
            enqueued as usize,
            status.pending + status.blocked,
            "compute_project returns the count of tasks it enqueued (children pending + barrier blocked)",
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

/// Which repo-scope metrics are still worth computing, per repository.
///
/// ## Why a gate exists at all
///
/// `scope=repo` metrics are produced by a whole-tree `git log` over ALL authors
/// — a separate subprocess from the per-identity `git log --author=` that feeds
/// the local view — and NOTHING outside the dōjō reads them. So a repository
/// whose every consuming tenant has switched a metric off is paying real money
/// for a value no one will look at. That is what
/// `dojo.metric_activations` promised to prevent and never did, because nothing
/// read it.
///
/// ## Fail OPEN, always
///
/// Every uncertainty resolves to "compute it": no config row, unparseable JSON,
/// a repository the plan never mentioned, a metric no tenant named. Skipping is
/// the destructive direction — a metric not computed leaves a hole in a series
/// that only a backfill can fill, while computing one nobody wanted costs a
/// single `git log`. The asymmetry decides the default.
#[derive(Debug, Clone, Default)]
pub(super) struct MetricGate {
    /// repo_key → metric keys EVERY consuming tenant has switched off.
    /// Absent repository = nothing disabled = compute everything.
    disabled: std::collections::HashMap<String, Vec<String>>,
}

impl MetricGate {
    /// Read the last plan's ruling. Never fails: see "fail OPEN".
    ///
    /// Reads `sensei.metric_deactivations`. An earlier version kept the same map
    /// as JSON under one `sensei.config` key, which served the gate and nothing
    /// else: `sensei.metric_status` cannot join a blob, and an enable/disable
    /// screen had nothing to read.
    pub(super) async fn load(pg: &PgStore) -> Self {
        match pg.metric_deactivations().await {
            Ok(disabled) => Self { disabled },
            // No sync has run, or the read failed. Either way nothing is KNOWN to
            // be unwanted, and the safe answer is to compute.
            Err(e) => {
                tracing::warn!(error = %e, "could not read metric deactivations — computing everything");
                Self::default()
            }
        }
    }

    /// Whether ANY of `keys` is still wanted for `repo_key`.
    ///
    /// Used to decide whether a whole group's repo-scope work is worth doing: the
    /// `git log` is shared by every metric in the group, so it pays for itself if
    /// even one survives.
    pub(super) fn wants_any(&self, repo_key: Option<&str>, keys: &[&str]) -> bool {
        let Some(repo_key) = repo_key else {
            // Repository not registered with any dōjō — no tenant has an opinion,
            // so nothing has been switched off.
            return true;
        };
        match self.disabled.get(repo_key) {
            None => true,
            Some(off) => keys.iter().any(|k| !off.iter().any(|d| d == k)),
        }
    }
}

#[cfg(test)]
mod metric_gate {
    use super::MetricGate;

    fn gate(pairs: &[(&str, &[&str])]) -> MetricGate {
        let disabled = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.iter().map(|s| (*s).to_string()).collect()))
            .collect();
        // Constructed directly rather than through `load`, so these tests cover
        // the DECISION without a database.
        MetricGate { disabled }
    }

    #[test]
    fn an_empty_gate_wants_everything() {
        // The state every install is in until a tenant switches something off,
        // and the state a failed read falls back to.
        let g = MetricGate::default();
        assert!(g.wants_any(Some("github.com/a/b"), &["churn_rate", "churn_concentration"]));
    }

    #[test]
    fn a_group_is_still_worth_running_while_one_of_its_metrics_survives() {
        // The `git log` is shared across the group, so it pays for itself if even
        // one metric is wanted. Skipping on "some are off" would silently drop
        // the survivors.
        let g = gate(&[("github.com/a/b", &["churn_rate"])]);
        assert!(g.wants_any(Some("github.com/a/b"), &["churn_rate", "churn_concentration"]));
    }

    #[test]
    fn the_group_is_skipped_only_when_every_one_of_its_metrics_is_off() {
        let g = gate(&[("github.com/a/b", &["churn_rate", "churn_concentration"])]);
        assert!(!g.wants_any(Some("github.com/a/b"), &["churn_rate", "churn_concentration"]));
    }

    #[test]
    fn a_repository_the_plan_never_mentioned_is_computed() {
        // Not shared with any dōjō, so no tenant has an opinion. Treating silence
        // as "off" would stop computing for every unshared repository.
        let g = gate(&[("github.com/a/b", &["churn_rate"])]);
        assert!(g.wants_any(Some("github.com/other/repo"), &["churn_rate"]));
    }

    #[test]
    fn an_unregistered_repository_is_computed() {
        // No repo_key at all — never registered with a dōjō.
        let g = gate(&[("github.com/a/b", &["churn_rate"])]);
        assert!(g.wants_any(None, &["churn_rate"]));
    }

    #[test]
    fn a_disabled_metric_outside_the_group_does_not_skip_the_group() {
        // `ftr` is off, but it is not this group's metric. Matching loosely here
        // would have one group's deactivation silence another's work.
        let g = gate(&[("github.com/a/b", &["ftr"])]);
        assert!(g.wants_any(Some("github.com/a/b"), &["churn_rate"]));
    }
}
