//! Repo-grain metric compute ENGINE (spec §4/§5 — the watermark engine).
//!
//! This replaces the old timeline → day-task planner. The compute ORCHESTRATION is
//! now a two-level task graph keyed on the per-`(repository × group)` watermark
//! (`sensei.metric_watermarks`) instead of the old `covered_days` diff + global
//! `metrics.last_run` clock. The per-group COMPUTERS are unchanged — this file only
//! changes HOW days are scheduled and sealed.
//!
//! ## The per-project graph
//! - [`compute_project`] (`ComputeProjectMetrics`, the PARENT): freezes one `as_of`
//!   for the whole run (spec test 5), then enqueues ONE `ComputeGroupMetrics`
//!   child per active base group (the active registry `task_name`s minus
//!   [`HEALTH_TASK_NAME`]) and — when any child was enqueued — one
//!   [`ComputeHealth`](crate::tasks::TaskKind::ComputeHealth) barrier `blocked_by`
//!   the children, carrying the same frozen `as_of`. Honest-empty: no base groups →
//!   nothing enqueued, no barrier.
//! - [`compute_group`] (`ComputeGroupMetrics`, the CHILD): owns cadence for ONE
//!   group. It partitions by cadence:
//!   - **SNAPSHOT** (`knowledge`, `tool`) — forward-only, no honest historical
//!     source. Runs the computer once with `as_of = None` (the rolling-window
//!     today-incremental compute), enriches today, and advances NO watermark.
//!   - **DAY-KEYED** (`session_outcomes`, `autonomy`, `churn`, `quality`) — reads the
//!     group's DATA-day set ([`DayKeyedGroup::data_days`], the distinct
//!     true-occurrence source days), reads the MIN `sealed_through` across the
//!     project's repositories for the group, plans the days via
//!     [`watermark_plan_days`], computes + enriches each planned day, and — ON
//!     SUCCESS ONLY — seals every repo's watermark to `as_of − 1` (today is NEVER
//!     sealed). A compute error propagates and advances NO watermark (fail-closed:
//!     the group holds its cursor and retries next run — spec test 6).
//!
//! ## Watermark, not covered-days
//! The per-`(repository, group)` `sealed_through` IS coverage. An unset watermark
//! (`None`) fills the whole DATA history (its earliest day is already the `min_date`
//! — spec test 1). A settled day (`d ≤ sealed_through`, outside the trailing window)
//! is skipped, so an immutable commit-day (churn/quality) is computed once and never
//! recomputed (spec test 4). Today is never sealed (`sealed_through = as_of − 1`), so
//! the open day always recomputes and in-window late/late-enriched data lands (spec
//! tests 2, 3). Out-of-window backdated data is served by the explicit backfill
//! re-plan (which resets the watermark), not by the trailing window.
//!
//! ## Never fabricate
//! A day is planned only when the group's source genuinely has a row on it (or it is
//! in the trailing window / today). A project with no repository is an honest no-op
//! (no rows, no watermark). Every day-set read propagates `Err`. A failed group never
//! silently skips a day (fail-closed).

use chrono::NaiveDate;

use crate::db::pg_store::PgStore;
use crate::tasks::executor::TaskContext;
use crate::tasks::{Task, TaskKind};

use super::{HEALTH_TASK_NAME, MetricGroup};

/// The groups the engine fills PER-DAY (day cadence). Kept as a small enum with a
/// stable [`Self::ALL`] so the day-keyed set is explicit and each group's
/// true-occurrence source query lives in exactly one place. (`knowledge`/`tool` are
/// snapshot/forward-only and are NOT here — they never carry a watermark.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DayKeyedGroup {
    SessionOutcomes,
    Autonomy,
    /// GIT-derived churn (`churn_rate`/`churn_concentration`): filled per commit-day.
    /// Its registry `capture_source` is `git`, so it does NOT authorize the pruner's
    /// capture-before-reclaim (only `session` does).
    Churn,
    /// GIT-worktree + `qlty`-derived code quality (`duplication_ratio`/`module_quality`):
    /// filled per SAMPLED commit-day (one anchor per ISO week — see
    /// [`super::quality::sample_commit_days`]) to bound the heavy scan cost. Also
    /// git/source-derived, so it does NOT authorize the pruner's capture-before-reclaim.
    Quality,
    /// LLM process-quality (`spec_depth` + the three occurrence rates): filled per
    /// day of sessions the LLM analyzer has scored (`sessions.props ? 'process'`),
    /// bucketed on `sessions.started_at`. Session-source, so it DOES ride the same
    /// day cadence as `session_outcomes`.
    SessionProcess,
    /// Context-reuse (`cache_reuse`): one ratio per (day, repository) from the
    /// per-turn token split, bucketed on `sessions.started_at`. Session-source, so
    /// it rides the same day cadence as `session_outcomes` — a past day's sessions
    /// are settled and can be honestly recomputed, unlike a snapshot.
    Usage,
}

impl DayKeyedGroup {
    /// Every day-keyed group, in a stable order.
    pub(super) const ALL: [DayKeyedGroup; 6] = [
        DayKeyedGroup::SessionOutcomes,
        DayKeyedGroup::Autonomy,
        DayKeyedGroup::Churn,
        DayKeyedGroup::Quality,
        DayKeyedGroup::SessionProcess,
        DayKeyedGroup::Usage,
    ];

    /// The base [`MetricGroup`] this day-keyed group computes — the single source of
    /// the group's registry `task_name` (reused so the label can't drift from the
    /// dispatch/scheduler) and the group used to route to the computer.
    pub(super) fn group(self) -> MetricGroup {
        match self {
            DayKeyedGroup::SessionOutcomes => MetricGroup::SessionOutcomes,
            DayKeyedGroup::Autonomy => MetricGroup::Autonomy,
            DayKeyedGroup::Churn => MetricGroup::Churn,
            DayKeyedGroup::Quality => MetricGroup::Quality,
            DayKeyedGroup::SessionProcess => MetricGroup::SessionProcess,
            DayKeyedGroup::Usage => MetricGroup::Usage,
        }
    }

    /// The registry `task_name` this day-keyed group is known by.
    fn task_name(self) -> &'static str {
        self.group().as_str()
    }

    /// The DATA-day set for this group + project: the distinct TRUE-occurrence days
    /// its source rows carry (never an insert-time `created_at`). Matches the
    /// measurable base each computer writes over, so a planned day is one that can
    /// actually produce a row. This is the source-day DISCOVERY — its earliest day is
    /// the group's `min_date`, so an unset watermark fills from real history:
    /// - `session_outcomes` — days of measurable (`outcome is not null`) sessions,
    ///   bucketed on `sessions.started_at`.
    /// - `autonomy` — the UNION of run-started days (`runs.started_at`) and
    ///   `UserPromptSubmit`-event days (client `ts`, attributed via
    ///   `sessions.client_session_id`). Only `UserPromptSubmit` — NOT `Stop` — anchors
    ///   the event arm: `interruption_rate` (`Stop / UserPromptSubmit`) emits NO row on
    ///   a `UserPromptSubmit = 0` day (a 0/0 would be fabricated), so a `Stop`-only day
    ///   is not a measurable data day.
    /// - `churn` — the distinct GIT committer-days in the project's repo (via
    ///   [`super::churn::git_commit_days`] on the repo root from
    ///   [`PgStore::project_root_path`]). Not a SQL read: churn is git-derived. A
    ///   project with no repo-root folder / a non-git root has no git churn days →
    ///   honest-empty (no fill, no fabricated day).
    /// - `quality` — the SAMPLED git commit-days (one anchor per ISO week, via
    ///   [`super::quality::sample_commit_days`] over `git_commit_days`), so the heavy
    ///   per-worktree `qlty` scan runs at a bounded cadence rather than every commit-day.
    async fn data_days(
        self,
        pg: &PgStore,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<NaiveDate>, String> {
        if matches!(self, DayKeyedGroup::Churn | DayKeyedGroup::Quality) {
            let Some(root) = pg.project_root_path(project_id).await? else {
                return Ok(Vec::new());
            };
            let commit_days = super::churn::git_commit_days(&root);
            return Ok(match self {
                // Quality samples the commit-days (weekly anchor) to bound scan cost;
                // churn fills every commit-day.
                DayKeyedGroup::Quality => super::quality::sample_commit_days(&commit_days),
                _ => commit_days,
            });
        }
        let sql = match self {
            DayKeyedGroup::SessionOutcomes => {
                "SELECT DISTINCT date_trunc('day', s.started_at)::date AS day
                   FROM activity.sessions s
                  WHERE s.project_id = $1
                    AND s.outcome   IS NOT NULL"
            }
            DayKeyedGroup::Autonomy => {
                "SELECT DISTINCT day FROM (
                     SELECT date_trunc('day', r.started_at)::date AS day
                       FROM activity.runs r
                      WHERE r.project_id = $1
                     UNION
                     SELECT date_trunc('day', to_timestamp(ae.ts / 1000.0))::date AS day
                       FROM activity.assistant_events ae
                       JOIN activity.sessions        s ON s.client_session_id = ae.session_id
                      WHERE s.project_id   = $1
                        AND ae.event_type  = 'UserPromptSubmit'
                 ) u"
            }
            DayKeyedGroup::Usage => {
                // Days with token-accounted turns. Only claude_code carries the
                // split so far, so a day of Zed/OpenCode-only work has no data day
                // and is not planned — which is honest: we cannot measure reuse we
                // never captured.
                "SELECT DISTINCT date_trunc('day', s.started_at)::date AS day
                   FROM activity.transcript_turns tt
                   JOIN activity.sessions         s ON s.client_session_id = tt.session_id
                  WHERE s.project_id  = $1
                    AND tt.tokens_in IS NOT NULL"
            }
            DayKeyedGroup::SessionProcess => {
                // Days of sessions the LLM analyzer has SCORED (props ? 'process'),
                // bucketed on started_at — the base the process computer writes over.
                "SELECT DISTINCT date_trunc('day', s.started_at)::date AS day
                   FROM activity.sessions s
                  WHERE s.project_id = $1
                    AND s.props ? 'process'"
            }
            // Handled by the git early-return above (churn/quality are not SQL day-sets).
            DayKeyedGroup::Churn | DayKeyedGroup::Quality => {
                unreachable!("churn/quality data_days are git-sourced above")
            }
        };
        let rows: Vec<(NaiveDate,)> = sqlx_core::query_as::query_as(sql)
            .bind(project_id)
            .fetch_all(pg.pool())
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(d,)| d).collect())
    }
}

/// The day-keyed groups that are ACTIVE — the intersection of [`DayKeyedGroup::ALL`]
/// with the given `task_name`s. Pure so the cadence partition is unit-testable
/// without a DB. [`compute_group`] passes a single-element slice to classify the one
/// group it handles.
pub(super) fn day_keyed_active(active: &[String]) -> Vec<DayKeyedGroup> {
    DayKeyedGroup::ALL.into_iter().filter(|g| active.iter().any(|t| t == g.task_name())).collect()
}

/// The ACTIVE base groups that are NOT day-keyed — the SNAPSHOT / forward-only groups
/// (`knowledge`, `tool`) that reflect CURRENT state and are computed today-only
/// (`as_of = None`, no watermark). Derived from the given `task_name`s via
/// [`MetricGroup::from_task_name`] (so the health barrier's own name and any unknown
/// group are excluded) minus the day-keyed groups. Pure so the day-keyed/snapshot
/// partition is unit-testable without a DB. [`compute_group`] passes a single-element
/// slice to classify the one group it handles.
pub(super) fn snapshot_active(active: &[String]) -> Vec<MetricGroup> {
    let day_keyed: Vec<MetricGroup> = DayKeyedGroup::ALL.into_iter().map(|g| g.group()).collect();
    active
        .iter()
        .filter_map(|t| MetricGroup::from_task_name(t))
        .filter(|g| !day_keyed.contains(g))
        .collect()
}

/// The days to compute for one day-keyed group under the watermark. The union of:
/// - the UNSEALED data days — `{ d ∈ data : sealed is None OR d > sealed }` (an unset
///   watermark fills the whole history, i.e. from `data`'s earliest / `min_date`
///   day; a set watermark keeps only days after the sealed cursor),
/// - the trailing window `[as_of − (window_days − 1) .. as_of]` (ALWAYS reopened, so
///   in-window late/late-enriched data converges — out-of-window backdated data is
///   served by the explicit backfill re-plan, which resets the watermark),
/// - and `{as_of}` (today is always planned).
///
/// Sorted ascending and de-duplicated so each day is computed exactly once. Pure —
/// the fast-path is the watermark, so this drops the old covered-days argument.
/// `window_days` is the configured [`metrics.window_days`] (default 14) the computers
/// measure over; `max(1)` guarantees today is planned even for a (guarded) window of 0.
pub(super) fn watermark_plan_days(
    data: &[NaiveDate],
    sealed: Option<NaiveDate>,
    as_of: NaiveDate,
    window_days: u32,
) -> Vec<NaiveDate> {
    // Unsealed data days: everything on an unset watermark, else strictly after the
    // sealed cursor (settled days are skipped — an immutable commit-day never recomputes).
    let mut days: std::collections::BTreeSet<NaiveDate> = data
        .iter()
        .copied()
        .filter(|d| match sealed {
            None => true,
            Some(s) => *d > s,
        })
        .collect();
    // The trailing window [as_of-(window-1) .. as_of], always reopened so in-window
    // late data lands and today converges to its complete value.
    for i in 0..window_days.max(1) {
        days.insert(as_of - chrono::Duration::days(i as i64));
    }
    // Today is always planned (guaranteed by the window; explicit for clarity).
    days.insert(as_of);
    days.into_iter().collect()
}

/// A boxed, `Send` per-day compute future — the injected unit [`fill_and_seal`] runs.
type DayFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<u32, String>> + Send + 'a>>;

/// Route a base group to its computer (the UNCHANGED per-group `compute`). Threads the
/// target `as_of` through: `Some(day)` for a day-keyed backfill/gap-fill day; `None`
/// for a snapshot group's rolling-window today compute. The single dispatch seam so
/// the group→computer mapping lives in one place.
async fn run_computer(
    ctx: &TaskContext,
    group: MetricGroup,
    project_raw: &str,
    as_of: Option<NaiveDate>,
) -> Result<u32, String> {
    match group {
        MetricGroup::SessionOutcomes => {
            super::session_outcomes::compute(ctx, project_raw, as_of).await
        }
        MetricGroup::Churn => super::churn::compute(ctx, project_raw, as_of).await,
        MetricGroup::Quality => super::quality::compute(ctx, project_raw, as_of).await,
        MetricGroup::Autonomy => super::autonomy::compute(ctx, project_raw, as_of).await,
        MetricGroup::Knowledge => super::knowledge::compute(ctx, project_raw, as_of).await,
        MetricGroup::Cost => super::cost::compute(ctx, project_raw, as_of).await,
        MetricGroup::Usage => super::usage::compute(ctx, project_raw, as_of).await,
        MetricGroup::Coverage => super::coverage::compute(ctx, project_raw, as_of).await,
        MetricGroup::SessionProcess => {
            super::session_process::compute(ctx, project_raw, as_of).await
        }
    }
}

/// Fill the day-keyed `plan` then SEAL. For each planned day: run `compute_day` (the
/// per-group computer) and run the per-day explainer enrichment. THEN — only after
/// every day computed without error — advance each repo's watermark to `as_of − 1`
/// (today is NEVER sealed). Fail-closed: the FIRST `compute_day` error propagates and
/// NO watermark advances, so a failed group holds its cursor and retries next run
/// (spec test 6). `compute_day` is injected so the seal / fail-closed logic is
/// directly unit-testable with a forced failure (mirrors `quality::compute_with_scanner`),
/// while production passes the real [`run_computer`].
#[allow(clippy::too_many_arguments)] // engine state + the injected compute_day closure (for fail-closed testability)
async fn fill_and_seal<'a, F>(
    ctx: &TaskContext,
    pg: &PgStore,
    project_id: &uuid::Uuid,
    group: &str,
    repos: &[uuid::Uuid],
    plan: &[NaiveDate],
    as_of: NaiveDate,
    compute_day: F,
) -> Result<u32, String>
where
    F: Fn(NaiveDate) -> DayFuture<'a>,
{
    let mut written = 0u32;
    for &day in plan {
        written += compute_day(day).await?;
        // Per-day explainer enrichment — the SAME wiring the old dispatcher ran, moved
        // here so each computed (group, day) is still enriched exactly as before.
        super::explainer::enrich_day(ctx, project_id, group, day).await;
    }
    // Success: seal every repo through as_of - 1 (today is never sealed). A read/write
    // error here propagates (never a fabricated / partial advance).
    let sealed_through = as_of - chrono::Duration::days(1);
    for repo in repos {
        pg.advance_metric_watermark(repo, group, sealed_through).await?;
    }
    Ok(written)
}

/// `ComputeProjectMetrics` — the per-project PARENT. Freezes one `as_of` for the whole
/// run (spec test 5), enqueues one [`TaskKind::ComputeGroupMetrics`] child per active
/// base group (active registry `task_name`s minus [`HEALTH_TASK_NAME`]), and — when
/// any child was enqueued — one [`TaskKind::ComputeHealth`] barrier `blocked_by` those
/// children (same frozen `as_of`) so the roll-up runs after the components land. The
/// project id rides in `task.folder_path`. Returns the number of tasks enqueued
/// (`0` = honest-empty: no active base groups → nothing to compute, no barrier).
pub(super) async fn compute_project(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(&task.folder_path)
        .map_err(|e| format!("compute_project: bad project id {:?}: {e}", task.folder_path))?;
    let pg = ctx.pg();

    // Freeze `as_of` ONCE (SELECT current_date) so every child in this run shares one
    // clock — the same day source the computers stamp (spec test 5).
    let as_of = super::today(pg).await?;

    let active = pg.active_task_names().await?;
    let owner = project_id.to_string();
    let mut child_ids = Vec::new();
    // One child per active base group — the health barrier's own name is not a base
    // group. An unknown group still gets a child that degrades to a logged no-op in
    // `compute_group` (never a panic / stuck queue).
    for task_name in active.into_iter().filter(|t| t != HEALTH_TASK_NAME) {
        let id = ctx
            .queue
            .enqueue(Task::new(TaskKind::ComputeGroupMetrics, &owner, &task_name).with_as_of(as_of))
            .await;
        child_ids.push(id);
    }

    if child_ids.is_empty() {
        tracing::debug!(
            project = %task.folder_path,
            "compute_project: no active base groups — nothing to compute",
        );
        return Ok(0);
    }

    let n_children = child_ids.len();
    // The per-project health barrier: blocked on THIS run's group children (same frozen
    // as_of) so the derived roll-up runs only after the components land.
    ctx.queue
        .enqueue(
            Task::new(TaskKind::ComputeHealth, &owner, "").with_as_of(as_of).blocked_by(child_ids),
        )
        .await;
    let enqueued = n_children as u32 + 1;
    tracing::info!(
        project = %task.folder_path,
        groups = n_children,
        enqueued,
        "compute_project: metric compute graph enqueued",
    );
    Ok(enqueued)
}

/// `ComputeGroupMetrics` — the per-`(project, group)` CHILD that owns cadence. The
/// project id rides in `task.folder_path`, the registry `task_name` in `task.path`,
/// and the frozen `as_of` in `task.as_of`. Partitions by cadence (see the module doc):
/// a SNAPSHOT group runs today-only with no watermark; a DAY-KEYED group plans days
/// against the watermark, computes + enriches each, and seals on success. An unknown
/// `task_name` is a logged no-op (`Ok(0)`) — the same honest degrade as before.
/// Returns the number of `project_metrics` rows written.
pub(super) async fn compute_group(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let pg = ctx.pg();
    let project_id = uuid::Uuid::parse_str(&task.folder_path)
        .map_err(|e| format!("compute_group: bad project id {:?}: {e}", task.folder_path))?;
    // `as_of` = the frozen run clock (Some from the parent), else today.
    let as_of = match task.as_of {
        Some(d) => d,
        None => super::today(pg).await?,
    };

    let one = std::slice::from_ref(&task.path);
    if let Some(dk) = day_keyed_active(one).into_iter().next() {
        // ── DAY-KEYED (session_outcomes / autonomy / churn / quality) ──
        // The repositories this project spans — the watermark grain. No repo → no
        // watermark, no fill: honest no-op (never a fabricated repository).
        let repos = pg.repositories_for_project(&project_id).await?;
        if repos.is_empty() {
            tracing::debug!(
                project = %task.folder_path,
                group = %task.path,
                "compute_group: project spans no repository — honest no-op",
            );
            return Ok(0);
        }
        // The group's DATA-day discovery (unchanged) — its earliest day is `min_date`.
        let data = dk.data_days(pg, &project_id).await?;
        // The MIN sealed_through across the project's repos for this group. `None` if
        // ANY repo is unset → full-history fill (the min_date fill, spec test 1).
        let sealed = pg.min_sealed_through_for_repos(&repos, &task.path).await?;
        // The rolling window the computers measure over — shared reader (DRY) so the
        // trailing-refresh window can't drift from the computers' own window.
        let window_days = crate::tasks::metrics_scheduler::window_days(pg).await;
        let plan = watermark_plan_days(&data, sealed, as_of, window_days);

        let group_metric = dk.group();
        let project_owned = task.folder_path.clone();
        fill_and_seal(ctx, pg, &project_id, &task.path, &repos, &plan, as_of, move |day| {
            let proj = project_owned.clone();
            // The `DayFuture<'_>` annotation forces the unsize coercion from the
            // concrete future to the boxed trait object (the `F` bound's Output).
            let fut: DayFuture<'_> =
                Box::pin(async move { run_computer(ctx, group_metric, &proj, Some(day)).await });
            fut
        })
        .await
    } else if let Some(group) = snapshot_active(one).into_iter().next() {
        // ── SNAPSHOT (knowledge / tool) ──
        // Forward-only: the rolling-window today compute (`as_of = None`), no watermark.
        // Then enrich today's datapoints exactly as the old dispatcher did.
        let written = run_computer(ctx, group, &task.folder_path, None).await?;
        super::explainer::enrich_day(ctx, &project_id, &task.path, as_of).await;
        Ok(written)
    } else {
        // Intentional logged no-op (NOT fabrication): a registry entry whose task_name
        // the daemon doesn't map (or the health barrier's own name) degrades to a
        // warning instead of panicking the worker or wedging the queue.
        tracing::warn!(
            task_name = %task.path,
            project = %task.folder_path,
            "compute_group: unknown metrics task_name — no-op",
        );
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::test_support::{
        cleanup_metrics_fixture, git_commit_on_day, make_ctx, seed_git_project_folder,
        seed_metrics_project_folder, seed_metrics_session, seed_metrics_turn,
    };

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    // ── Pure: watermark_plan_days ────────────────────────────────────────────

    #[test]
    fn watermark_plan_days_unset_fills_all_data_plus_window_and_today() {
        // sealed = None → the whole DATA history (from the earliest / min_date day)
        // PLUS the trailing window PLUS today; sorted + de-duplicated.
        let today = d(2025, 6, 30);
        let window = 14u32;
        let old = d(2025, 5, 1); // outside the window → only reached via the unset fill
        let mid = d(2025, 6, 10);
        let out = watermark_plan_days(&[mid, old, mid], None, today, window);
        assert!(
            out.contains(&old),
            "an unset watermark fills from the earliest data day (min_date)"
        );
        assert!(out.contains(&mid));
        for i in 0..window {
            assert!(
                out.contains(&(today - chrono::Duration::days(i as i64))),
                "trailing window present"
            );
        }
        assert!(out.contains(&today), "today is always planned");
        let mut sorted = out.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(out, sorted, "sorted, de-duplicated");
    }

    #[test]
    fn watermark_plan_days_sealed_keeps_only_days_after_cursor_plus_window_and_today() {
        // sealed = Some(s) → only data days STRICTLY after the cursor, plus the window,
        // plus today. A settled day older than the window is dropped (never recomputed).
        let today = d(2025, 6, 30);
        let sealed = d(2025, 6, 15);
        let settled_old = d(2025, 5, 1); // ≤ sealed AND outside window → dropped
        let after_cursor = d(2025, 6, 20); // > sealed (also inside window) → kept
        let out = watermark_plan_days(&[settled_old, after_cursor], Some(sealed), today, 14);
        assert!(!out.contains(&settled_old), "a settled day (≤ sealed, outside window) is skipped");
        assert!(out.contains(&after_cursor), "a day after the cursor is planned");
        assert!(out.contains(&today), "today is always planned");
        for i in 0..14u32 {
            assert!(
                out.contains(&(today - chrono::Duration::days(i as i64))),
                "trailing window present"
            );
        }
    }

    #[test]
    fn watermark_plan_days_today_always_present_even_window_zero() {
        // `max(1)` guarantees today even for a (guarded-against) window of 0, sealed or not.
        let today = d(2025, 6, 30);
        assert_eq!(watermark_plan_days(&[], None, today, 0), vec![today]);
        assert_eq!(watermark_plan_days(&[], Some(d(2025, 6, 29)), today, 0), vec![today]);
    }

    // ── Pure: cadence partition ──────────────────────────────────────────────

    #[test]
    fn day_keyed_active_keeps_only_active_day_keyed_groups() {
        let active = vec![
            "session_outcomes".to_string(),
            "knowledge".to_string(),
            "autonomy".to_string(),
            "churn".to_string(),
            "quality".to_string(),
            "health".to_string(),
        ];
        assert_eq!(
            day_keyed_active(&active),
            vec![
                DayKeyedGroup::SessionOutcomes,
                DayKeyedGroup::Autonomy,
                DayKeyedGroup::Churn,
                DayKeyedGroup::Quality,
            ],
        );
        assert!(day_keyed_active(&[]).is_empty(), "honest-empty");
        assert!(day_keyed_active(&["knowledge".to_string(), "health".to_string()]).is_empty());
    }

    #[test]
    fn snapshot_active_keeps_only_active_non_day_keyed_base_groups() {
        let active = vec![
            "session_outcomes".to_string(),
            "churn".to_string(),
            "quality".to_string(),
            "autonomy".to_string(),
            "knowledge".to_string(),
            "coverage".to_string(),
            "health".to_string(),
        ];
        assert_eq!(
            snapshot_active(&active),
            vec![MetricGroup::Knowledge, MetricGroup::Coverage],
            "snapshot = knowledge/coverage; day-keyed + health excluded",
        );
        assert!(
            snapshot_active(&["no_such_group".to_string()]).is_empty(),
            "unknown never fabricates"
        );
        assert!(snapshot_active(&["health".to_string()]).is_empty());
    }

    // ── Test-only injected per-day computes (for fill_and_seal fail-closed) ──

    fn ok_zero(_day: NaiveDate) -> DayFuture<'static> {
        Box::pin(async { Ok(0u32) })
    }
    fn boom(_day: NaiveDate) -> DayFuture<'static> {
        Box::pin(async { Err::<u32, String>("forced compute failure".to_string()) })
    }

    /// The `ftr` daily value for a project on `day` (scope=user, one repo) — `None`
    /// when no row. The read-back the day-keyed engine tests assert against.
    async fn ftr_value_on(pg: &PgStore, pid: &uuid::Uuid, day: NaiveDate) -> Option<f64> {
        let row: Option<(f64,)> = sqlx_core::query_as::query_as(
            "SELECT pm.value::float8 FROM sensei.project_metrics pm \
               JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'ftr' AND pm.computed_on = $2 AND pm.grain = 'daily' \
              LIMIT 1",
        )
        .bind(pid)
        .bind(day)
        .fetch_optional(pg.pool())
        .await
        .unwrap();
        row.map(|(v,)| v)
    }

    // ── #1 min_date fill: an unset watermark computes the full history ──

    #[tokio::test]
    async fn unset_watermark_fills_the_full_history_and_no_data_is_a_noop() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;

        let now = chrono::Utc::now();
        let today = now.date_naive();
        // Measurable sessions on two historical days (60 + 30 back — both outside the
        // 14-day window, so ONLY the unset-watermark full fill reaches them).
        let (ts1, ts2) = (now - chrono::Duration::days(60), now - chrono::Duration::days(30));
        let (d1, d2) = (ts1.date_naive(), ts2.date_naive());
        for ts in [ts1, ts2] {
            let sid =
                seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
            seed_metrics_turn(pg, &sid, 1, ts).await;
        }

        let task = Task::new(TaskKind::ComputeGroupMetrics, &pid.to_string(), "session_outcomes")
            .with_as_of(today);
        let written = compute_group(&ctx, &task).await.unwrap();
        assert!(written > 0, "an unset watermark fills the historical data days");

        // The earliest computed day is the earliest data day (min_date fill).
        let (min_day,): (Option<NaiveDate>,) = sqlx_core::query_as::query_as(
            "SELECT min(computed_on) FROM sensei.project_metrics WHERE project_id = $1 AND grain = 'daily'",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(min_day, Some(d1), "the fill starts at the earliest data day (min_date)");
        assert!(
            ftr_value_on(pg, &pid, d2).await.is_some(),
            "the later historical day is filled too"
        );

        // A repo with NO data is an honest no-op (no rows written).
        let uniq2 = uuid::Uuid::new_v4();
        let (pid2, fid2) = seed_metrics_project_folder(pg, &uniq2).await;
        let task2 = Task::new(TaskKind::ComputeGroupMetrics, &pid2.to_string(), "session_outcomes")
            .with_as_of(today);
        assert_eq!(
            compute_group(&ctx, &task2).await.unwrap(),
            0,
            "no session data → no row (honest no-op)"
        );

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
        cleanup_metrics_fixture(pg, &pid2, Some(&fid2), &[]).await;
    }

    // ── #2 watermark fill + today never sealed; a re-run recomputes ONLY today ──

    #[tokio::test]
    async fn seals_through_as_of_minus_one_and_reruns_only_today() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;

        let now = chrono::Utc::now();
        let today = now.date_naive();
        let old_ts = now - chrono::Duration::days(30); // outside the window
        let old_day = old_ts.date_naive();
        // A first-try session on the old day (ftr → 1.0) and a NOT-first-try session
        // today (ftr → 0.0), so the two days carry distinct values.
        let sid_old =
            seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, old_ts).await;
        seed_metrics_turn(pg, &sid_old, 1, old_ts).await;
        let sid_today =
            seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(false), 1, now).await;
        seed_metrics_turn(pg, &sid_today, 1, now).await;

        let task = Task::new(TaskKind::ComputeGroupMetrics, &pid.to_string(), "session_outcomes")
            .with_as_of(today);
        compute_group(&ctx, &task).await.unwrap();

        // Watermark sealed through as_of - 1 for each repo (today is NEVER sealed).
        let repos = pg.repositories_for_project(&pid).await.unwrap();
        assert!(!repos.is_empty());
        for repo in &repos {
            assert_eq!(
                pg.metric_watermark_sealed_through(repo, "session_outcomes").await.unwrap(),
                Some(today - chrono::Duration::days(1)),
                "sealed_through = as_of - 1 (today never sealed)",
            );
        }
        assert_eq!(ftr_value_on(pg, &pid, old_day).await, Some(1.0), "old day computed");
        assert_eq!(ftr_value_on(pg, &pid, today).await, Some(0.0), "today computed");

        // Tamper BOTH days, then re-run. The settled old day must NOT recompute (its
        // tamper survives); today MUST recompute (its tamper is overwritten).
        sqlx_core::query::query(
            "UPDATE sensei.project_metrics pm SET value = 0.123 \
               FROM sensei.metrics m \
              WHERE m.id = pm.metric_id AND m.key = 'ftr' AND pm.project_id = $1 AND pm.grain = 'daily'",
        )
        .bind(pid)
        .execute(pg.pool())
        .await
        .unwrap();

        compute_group(&ctx, &task).await.unwrap();
        assert_eq!(
            ftr_value_on(pg, &pid, old_day).await,
            Some(0.123),
            "a settled day is NOT recomputed"
        );
        assert_eq!(
            ftr_value_on(pg, &pid, today).await,
            Some(0.0),
            "today is always recomputed (reopened)"
        );

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    // ── #4 commit immutable: a churn commit-day computed once, never recomputed ──

    #[tokio::test]
    async fn churn_commit_day_is_computed_once_and_never_recomputed() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;

        let today = chrono::Utc::now().date_naive();
        // A commit 60 days back — well outside the 14-day window, so once sealed it is
        // never reopened.
        let commit_day = today - chrono::Duration::days(60);
        git_commit_on_day(
            repo.path(),
            &commit_day.format("%Y-%m-%d").to_string(),
            &[("a.rs", "1\n2\n3\n")],
        );

        let task =
            Task::new(TaskKind::ComputeGroupMetrics, &pid.to_string(), "churn").with_as_of(today);
        compute_group(&ctx, &task).await.unwrap();

        // The churn_rate rows on the commit-day (scope=repo + scope=user).
        let (c0,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'churn_rate' AND pm.computed_on = $2",
        )
        .bind(pid)
        .bind(commit_day)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert!(c0 > 0, "the churn commit-day is computed on the first run");

        // Tamper every churn_rate row on the commit-day (real value is the touched-file
        // count = 1, so 999 is unmistakably synthetic), then re-run. A sealed commit's
        // code is immutable → it is NOT recomputed → the tamper survives.
        sqlx_core::query::query(
            "UPDATE sensei.project_metrics pm SET value = 999 \
               FROM sensei.metrics m \
              WHERE m.id = pm.metric_id AND m.key = 'churn_rate' AND pm.project_id = $1 AND pm.computed_on = $2",
        )
        .bind(pid)
        .bind(commit_day)
        .execute(pg.pool())
        .await
        .unwrap();

        compute_group(&ctx, &task).await.unwrap();
        let (c999,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'churn_rate' AND pm.computed_on = $2 AND pm.value = 999",
        )
        .bind(pid)
        .bind(commit_day)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(
            c999, c0,
            "the tampered value survives — the sealed commit-day is not recomputed"
        );

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    // ── #5 frozen as_of shared across all children of one ComputeProjectMetrics run ──

    #[tokio::test]
    async fn compute_project_freezes_one_as_of_across_all_children() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let today = super::super::today(pg).await.unwrap();

        let parent = Task::new(TaskKind::ComputeProjectMetrics, &pid.to_string(), "");
        let enqueued = compute_project(&ctx, &parent).await.unwrap();
        assert!(enqueued >= 2, "at least one group child + the health barrier, got {enqueued}");

        // Drain every enqueued task (completing each to free its per-folder slot and to
        // unblock the health barrier). Assert EVERY group child AND the barrier carry
        // the SAME frozen as_of == today.
        let mut health = 0u32;
        let mut group_children = 0u32;
        for _ in 0..enqueued {
            let t = ctx.queue.next_task().await;
            assert_eq!(t.as_of, Some(today), "every child shares the one frozen as_of");
            match t.kind {
                TaskKind::ComputeGroupMetrics => group_children += 1,
                TaskKind::ComputeHealth => health += 1,
                other => panic!("compute_project enqueued an unexpected kind: {other}"),
            }
            ctx.queue.complete(t.id).await;
        }
        assert_eq!(health, 1, "exactly one ComputeHealth barrier per project");
        assert_eq!(group_children, enqueued - 1, "the rest are group children");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn compute_project_is_honest_empty_with_no_active_base_groups() {
        // A bad project id fails fast (never a masked empty success).
        let ctx = make_ctx().await;
        let bad = Task::new(TaskKind::ComputeProjectMetrics, "not-a-uuid", "");
        assert!(compute_project(&ctx, &bad).await.is_err(), "a bad project id propagates Err");
    }

    // ── #6 failed group holds its watermark; a healthy sibling advances ──

    #[tokio::test]
    async fn failed_group_holds_its_watermark_while_a_healthy_sibling_advances() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let repos = pg.repositories_for_project(&pid).await.unwrap();
        assert!(!repos.is_empty());
        let as_of = super::super::today(pg).await.unwrap();
        let plan = vec![as_of];

        // HEALTHY group: every day computes Ok → the watermark seals through as_of - 1.
        fill_and_seal(&ctx, pg, &pid, "session_outcomes", &repos, &plan, as_of, ok_zero)
            .await
            .unwrap();
        for repo in &repos {
            assert_eq!(
                pg.metric_watermark_sealed_through(repo, "session_outcomes").await.unwrap(),
                Some(as_of - chrono::Duration::days(1)),
                "a healthy group seals through as_of - 1",
            );
        }

        // FAILED sibling: a compute error propagates → NO watermark advances (the group
        // holds its cursor and retries next run — fail-closed, per repo/group).
        let res = fill_and_seal(&ctx, pg, &pid, "autonomy", &repos, &plan, as_of, boom).await;
        assert!(res.is_err(), "a failing compute propagates Err");
        for repo in &repos {
            assert_eq!(
                pg.metric_watermark_sealed_through(repo, "autonomy").await.unwrap(),
                None,
                "a FAILED group advances NO watermark (retry isolation is per repo/group)",
            );
        }

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }
}
