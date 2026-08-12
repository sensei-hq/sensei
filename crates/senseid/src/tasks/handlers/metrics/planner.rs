//! Timeline → day-task planner (Phase 4).
//!
//! `PlanMetricDays` runs ONCE per project and, for each ACTIVE **day-keyed** group,
//! turns the project's history into a set of per-day `ComputeMetrics{as_of=Some(D)}`
//! tasks. It is the single mechanism behind BOTH the first-install full backfill and
//! the daily incremental: it diffs the group's **data-day set** (the distinct
//! true-occurrence days its source rows carry) against the **covered-day set** (the
//! distinct `computed_on` days already stored in `sensei.project_metrics` for that
//! group's active metrics) and enqueues one compute per older MISSING day plus the
//! whole trailing [`metrics.window_days`] window (always re-enqueued so recent days
//! converge to their complete value — see [`plan_days`]). On a fresh install every
//! data day is missing → the whole history backfills; on a daily run only the
//! trailing window (and any older gap) is enqueued; a deleted/failed day self-heals
//! on the next plan.
//!
//! ## The complete per-project graph
//! `PlanMetricDays` is the SINGLE per-project metric planner (Phase 5): the metrics
//! scheduler enqueues exactly one `PlanMetricDays` per project per pass and this
//! handler enqueues the whole compute graph for that project.
//!
//! - **Day-keyed groups** — planned PER-DAY (backfill + today). Only groups whose
//!   source rows carry a real historical occurrence time qualify:
//!   - `session_outcomes` — anchored on `activity.sessions.started_at`.
//!   - `autonomy` — anchored on `activity.runs.started_at` UNION the
//!     `UserPromptSubmit` events' client `ts` day (`to_timestamp(ae.ts / 1000.0)`),
//!     the same true-time anchors those computers bucket on. (Only
//!     `UserPromptSubmit`, since `interruption_rate` writes no row on a day with no
//!     `UserPromptSubmit` — see [`DayKeyedGroup::data_days`].)
//! - **Snapshot / forward-only groups** (`churn`'s `rework_density`, `duplication`,
//!   `knowledge`, `tool`) reflect CURRENT state and have no honest historical source,
//!   so they are NOT backfilled — each gets ONE today-only `ComputeMetrics`
//!   (`as_of = None`, the rolling-window incremental compute). A historical `as_of`
//!   would make them skip anyway (see `super::is_historical`).
//! - **The `ComputeHealth` barrier** — one per project, `blocked_by` this pass's
//!   TODAY computes (the day-keyed today day + the snapshot computes) so the derived
//!   `project_health` roll-up runs only after the latest daily component values land.
//!   It blocks on TODAY's computes only, NOT the historical backfill days, so day-one
//!   health isn't stalled behind a months-long backfill (health reads each
//!   component's LATEST daily value — see [`super::health`]).
//!
//! ## Never fabricate
//! A day appears in the plan only when the group's source genuinely has a row on it
//! (or it is today). The diff is a pure function ([`plan_days`]); the day-set reads
//! propagate `Err` and are never masked into an empty success. No active base group
//! → no tasks (honest-empty).

use chrono::NaiveDate;

use crate::db::pg_store::PgStore;
use crate::tasks::executor::TaskContext;
use crate::tasks::queue::TaskQueue;
use crate::tasks::{Task, TaskKind};

use super::MetricGroup;

/// The groups the planner backfills per-day (see the module doc). Kept as a small
/// enum with a stable [`Self::ALL`] so the day-keyed set is explicit and the
/// per-group source query lives in exactly one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DayKeyedGroup {
    SessionOutcomes,
    Autonomy,
}

impl DayKeyedGroup {
    /// Every day-keyed group, in a stable order.
    pub(super) const ALL: [DayKeyedGroup; 2] =
        [DayKeyedGroup::SessionOutcomes, DayKeyedGroup::Autonomy];

    /// The base [`MetricGroup`] this day-keyed group plans compute tasks for — the
    /// single source of the group's registry `task_name` (reused so the label can't
    /// drift from the dispatch/scheduler).
    fn group(self) -> MetricGroup {
        match self {
            DayKeyedGroup::SessionOutcomes => MetricGroup::SessionOutcomes,
            DayKeyedGroup::Autonomy => MetricGroup::Autonomy,
        }
    }

    /// The registry `task_name` carried in the enqueued `ComputeMetrics` `task.path`
    /// (and used to resolve the group's active metric ids for the covered-day read).
    fn task_name(self) -> &'static str {
        self.group().as_str()
    }

    /// The DATA-day set for this group + project: the distinct TRUE-occurrence days
    /// its source rows carry (never an insert-time `created_at`). Matches the
    /// measurable base each computer writes over, so a planned day is one that can
    /// actually produce a row:
    /// - `session_outcomes` — days of measurable (`outcome is not null`) sessions,
    ///   bucketed on `sessions.started_at`.
    /// - `autonomy` — the UNION of run-started days (`runs.started_at`) and
    ///   `UserPromptSubmit`-event days (client `ts`, attributed via
    ///   `sessions.client_session_id`). Only `UserPromptSubmit` — NOT `Stop` —
    ///   anchors the event arm: `interruption_rate` is `Stop / UserPromptSubmit` and
    ///   emits NO row on a `UserPromptSubmit = 0` day (a 0/0 would be fabricated), so
    ///   a `Stop`-only day is not a measurable data day.
    async fn data_days(
        self,
        pg: &PgStore,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<NaiveDate>, String> {
        let sql = match self {
            DayKeyedGroup::SessionOutcomes => {
                "SELECT DISTINCT date_trunc('day', s.started_at)::date AS day
                   FROM activity.sessions s
                   JOIN sensei.folders    f ON f.id = s.folder_id
                  WHERE f.project_id = $1
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
        };
        let rows: Vec<(NaiveDate,)> = sqlx_core::query_as::query_as(sql)
            .bind(project_id)
            .fetch_all(pg.pool())
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(d,)| d).collect())
    }
}

/// The registry `task_name` of every day-keyed group — the single source of the
/// "a project-day is captured only when a DAY-KEYED (delivery) metric wrote its
/// row" set. Derived from [`DayKeyedGroup::ALL`] (each mapped through its
/// [`MetricGroup::as_str`]) so the pruner's capture-before-reclaim guard scopes to
/// exactly the groups the planner backfills per-day — never a second hardcoded
/// `["session_outcomes", "autonomy"]` list. Consumed by
/// [`crate::tasks::activity_pruner`] to scope `PgStore::prune_activity`'s capture
/// `EXISTS`.
pub(crate) fn day_keyed_task_names() -> Vec<&'static str> {
    DayKeyedGroup::ALL.iter().map(|g| g.task_name()).collect()
}

/// The day-keyed groups that are ACTIVE in the registry — the intersection of
/// [`DayKeyedGroup::ALL`] with the scheduler's active `task_name`s. Pure so the
/// honest-empty contract (empty active registry → no groups → no tasks) is
/// unit-testable without a DB.
pub(super) fn day_keyed_active(active: &[String]) -> Vec<DayKeyedGroup> {
    DayKeyedGroup::ALL
        .into_iter()
        .filter(|g| active.iter().any(|t| t == g.task_name()))
        .collect()
}

/// The ACTIVE base groups that are NOT day-keyed — the snapshot / forward-only
/// groups (`churn`, `duplication`, `knowledge`, `tool`) that reflect CURRENT state
/// and get ONE today-only `ComputeMetrics` (`as_of = None`) rather than a per-day
/// backfill. Derived from the scheduler's active `task_name`s via
/// [`MetricGroup::from_task_name`] (so the health barrier's own name and any unknown
/// group are excluded) minus the day-keyed groups. Pure so the day-keyed/snapshot
/// partition is unit-testable without a DB.
pub(super) fn snapshot_active(active: &[String]) -> Vec<MetricGroup> {
    let day_keyed: Vec<MetricGroup> = DayKeyedGroup::ALL.into_iter().map(|g| g.group()).collect();
    active
        .iter()
        .filter_map(|t| MetricGroup::from_task_name(t))
        .filter(|g| !day_keyed.contains(g))
        .collect()
}

/// The COVERED-day set: distinct `computed_on` days already stored in
/// `sensei.project_metrics` for this project and any of the group's active metric
/// ids. An empty id list (the group has no active metrics) trivially covers nothing.
async fn covered_days(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    metric_ids: &[uuid::Uuid],
) -> Result<Vec<NaiveDate>, String> {
    if metric_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(NaiveDate,)> = sqlx_core::query_as::query_as(
        "SELECT DISTINCT computed_on
           FROM sensei.project_metrics
          WHERE project_id = $1
            AND metric_id = ANY($2)",
    )
    .bind(project_id)
    .bind(metric_ids)
    .fetch_all(pg.pool())
    .await
    .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(d,)| d).collect())
}

/// The days to compute for one group: the trailing window
/// `[today - (window_days - 1) .. today]` UNIONed with the older backfill/gap-fill
/// days `(data − covered)`, sorted and deduplicated so each day is planned exactly
/// once. Pure — the diff core the planner is built on, tested without a DB.
///
/// The trailing window is ALWAYS re-enqueued REGARDLESS of covered-status (fixes
/// F1): a day computed once mid-day is stamped `computed_on` and would otherwise be
/// "covered" and dropped forever — freezing it to a partial-day sliver and losing
/// any session enriched after that first compute. Refreshing the whole trailing
/// window each pass lets recent days converge to their COMPLETE value and lets
/// late-enriched sessions land. Days OLDER than the window are settled: they are
/// enqueued once via `(data − covered)` and then dropped. `window_days` is the
/// configured [`metrics.window_days`] (default 14) the computers measure over.
pub(super) fn plan_days(
    data: &[NaiveDate],
    covered: &[NaiveDate],
    today: NaiveDate,
    window_days: u32,
) -> Vec<NaiveDate> {
    let covered_set: std::collections::HashSet<NaiveDate> = covered.iter().copied().collect();
    // Older-than-window backfill/gap-fill days: uncovered data days. In-window days
    // are added unconditionally below, so a covered in-window day is still refreshed.
    let mut days: std::collections::BTreeSet<NaiveDate> = data
        .iter()
        .copied()
        .filter(|d| !covered_set.contains(d))
        .collect();
    // The trailing window [today-(window_days-1) .. today], always re-enqueued so
    // recent days converge and late-enriched sessions land. `max(1)` guarantees today
    // is always planned even for a (guarded-against) window of 0.
    for i in 0..window_days.max(1) {
        days.insert(today - chrono::Duration::days(i as i64));
    }
    days.into_iter().collect()
}

/// The `ComputeMetrics` tasks enqueued for one project by [`enqueue_compute_plan`].
struct EnqueuedComputes {
    /// Total `ComputeMetrics` tasks enqueued — per-day day-keyed (backfill + today)
    /// PLUS one today-only per snapshot group.
    total: u32,
    /// The TODAY compute ids only. The health barrier blocks on THESE (not the
    /// historical backfill days) because the `project_health` roll-up reads each
    /// component's LATEST daily value — so day-one health must not wait out a
    /// months-long backfill.
    today_ids: Vec<u64>,
}

/// Enqueue the per-project `ComputeMetrics` plan across both group kinds. For each
/// DAY-KEYED group: read its data-day set + already-covered days, diff via
/// [`plan_days`], and enqueue one `ComputeMetrics{as_of=Some(D)}` per planned day
/// (backfill + today). For each SNAPSHOT group: enqueue one today-only
/// `ComputeMetrics` (`as_of = None`, the rolling-window incremental compute — no
/// honest historical source to backfill). The group rides in `task.path`, the
/// project id in `task.folder_path`. Returns the total enqueued + the TODAY compute
/// ids (the health-barrier deps). Extracted (like
/// `metrics_scheduler::enqueue_metrics_pass`) so the enqueue contract is
/// unit-testable against a real queue. Uses plain `enqueue` (NOT `enqueue_unique`,
/// whose identity ignores `as_of` and would collapse distinct days of the same group
/// into one); the downstream upsert makes a re-enqueued day idempotent.
async fn enqueue_compute_plan(
    queue: &TaskQueue,
    pg: &PgStore,
    project_id: &uuid::Uuid,
    day_keyed: &[DayKeyedGroup],
    snapshot: &[MetricGroup],
    today: NaiveDate,
    window_days: u32,
) -> Result<EnqueuedComputes, String> {
    let owner = project_id.to_string();
    let mut total = 0u32;
    let mut today_ids = Vec::new();
    for group in day_keyed {
        let data = group.data_days(pg, project_id).await?;
        let ids: Vec<uuid::Uuid> = pg
            .active_metric_ids(group.task_name())
            .await?
            .into_values()
            .collect();
        let covered = covered_days(pg, project_id, &ids).await?;
        for day in plan_days(&data, &covered, today, window_days) {
            let id = queue
                .enqueue(
                    Task::new(TaskKind::ComputeMetrics, &owner, group.task_name()).with_as_of(day),
                )
                .await;
            total += 1;
            if day == today {
                today_ids.push(id);
            }
        }
    }
    for group in snapshot {
        // `as_of = None` → the today-only rolling-window compute (snapshot groups have
        // no honest historical source, so they are never backfilled per-day).
        let id = queue
            .enqueue(Task::new(TaskKind::ComputeMetrics, &owner, group.as_str()))
            .await;
        total += 1;
        today_ids.push(id);
    }
    Ok(EnqueuedComputes { total, today_ids })
}

/// Plan the metric compute for one project (the `PlanMetricDays` body). Reads the
/// active registry, partitions the base groups into day-keyed (backfilled per-day)
/// and snapshot (today-only), enqueues the compute plan, and — when any base compute
/// was enqueued — enqueues the per-project `ComputeHealth` barrier `blocked_by` this
/// pass's TODAY computes. DB reads propagate `Err` (never masked into a fake empty
/// success). Returns the number of tasks enqueued (`0` = honest-empty: no active base
/// groups → nothing to compute, so no health barrier either).
pub(super) async fn plan(ctx: &TaskContext, project_raw: &str) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(project_raw)
        .map_err(|e| format!("plan_metric_days: bad project id {project_raw:?}: {e}"))?;
    let pg = ctx.pg();

    let active = pg.active_task_names().await?;
    let day_keyed = day_keyed_active(&active);
    let snapshot = snapshot_active(&active);
    if day_keyed.is_empty() && snapshot.is_empty() {
        tracing::debug!("plan_metric_days: no active base groups — nothing to plan");
        return Ok(0);
    }

    // `super::today` (SELECT current_date) — the same day source the snapshot
    // computers stamp, so "today" can't drift between the planner and the computers.
    let today = super::today(pg).await?;
    // The rolling window the day-keyed computers measure over — read via the shared
    // scheduler helper (config key + parser + default) so the planner's trailing
    // refresh window can't drift from the computers' own window.
    let window_days = crate::tasks::metrics_scheduler::window_days(pg).await;

    let computes =
        enqueue_compute_plan(&ctx.queue, pg, &project_id, &day_keyed, &snapshot, today, window_days)
            .await?;
    let mut enqueued = computes.total;
    // The per-project health barrier: blocked on THIS pass's TODAY computes so the
    // roll-up runs after the latest daily component values land. No base compute →
    // no barrier (honest-empty: health has nothing to roll up).
    if !computes.today_ids.is_empty() {
        ctx.queue
            .enqueue(
                Task::new(TaskKind::ComputeHealth, &project_id.to_string(), "")
                    .blocked_by(computes.today_ids),
            )
            .await;
        enqueued += 1;
    }
    tracing::info!(
        project = %project_raw,
        day_keyed = day_keyed.len(),
        snapshot = snapshot.len(),
        enqueued,
        "plan_metric_days: compute graph enqueued",
    );
    Ok(enqueued)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::test_support::{
        cleanup_metrics_fixture, make_ctx, purge_assistant_events, purge_runs,
        seed_assistant_event, seed_metrics_client_session, seed_metrics_project_folder,
        seed_metrics_session, seed_metrics_turn, seed_run,
    };

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    // ── Pure: plan_days ─────────────────────────────────────────────────────

    #[test]
    fn plan_days_is_missing_days_plus_today_sorted_unique() {
        // With `window_days = 1` the trailing window is just {today}, isolating the
        // older-missing-day diff: (data − covered) ∪ {today}, sorted ascending, each
        // day once. Nothing is covered here, so every data day plus today is planned
        // — the first-install backfill shape.
        let data = [d(2025, 6, 3), d(2025, 6, 1), d(2025, 6, 3)]; // duplicate 06-03
        let today = d(2025, 6, 5);
        assert_eq!(
            plan_days(&data, &[], today, 1),
            vec![d(2025, 6, 1), d(2025, 6, 3), d(2025, 6, 5)],
            "sorted, de-duplicated, today appended",
        );
    }

    #[test]
    fn plan_days_excludes_already_covered_days() {
        // `window_days = 1` → the trailing window is {today}, so a settled (covered)
        // day OLDER than the window is dropped; the uncovered data days and today
        // remain — the daily incremental / gap-fill shape.
        let data = [d(2025, 6, 1), d(2025, 6, 2), d(2025, 6, 3)];
        let covered = [d(2025, 6, 1), d(2025, 6, 3)];
        let today = d(2025, 6, 10);
        assert_eq!(
            plan_days(&data, &covered, today, 1),
            vec![d(2025, 6, 2), d(2025, 6, 10)],
            "covered days older than the window excluded; the one uncovered day + today remain",
        );
    }

    #[test]
    fn plan_days_always_includes_today_even_when_covered() {
        // Today is ALWAYS recomputed (it is inside the trailing window), so it appears
        // once even when today is in both data and covered.
        let today = d(2025, 6, 10);
        let out = plan_days(&[today], &[today], today, 1);
        assert_eq!(out, vec![today], "today is planned exactly once despite being covered");
    }

    #[test]
    fn plan_days_no_data_returns_just_today() {
        // Honest-empty source (no data days) with `window_days = 1` still recomputes
        // today — the daily run on a project with no history yet.
        assert_eq!(plan_days(&[], &[], d(2025, 6, 10), 1), vec![d(2025, 6, 10)]);
    }

    #[test]
    fn plan_days_refreshes_the_full_trailing_window_even_when_covered() {
        // F1: the trailing window [today-(window_days-1) .. today] is ALWAYS
        // re-enqueued REGARDLESS of covered-status, so a day computed once mid-day
        // converges to its complete value on the next pass (and a session enriched
        // after that first compute lands). Settled data days OLDER than the window are
        // enqueued once via (data − covered) and then dropped.
        let today = d(2025, 6, 30);
        let window_days = 14u32;
        let in_window_covered = d(2025, 6, 20); // 10 days back → inside the window
        let old_covered = d(2025, 6, 1); // 29 days back → outside the window
        let old_uncovered = d(2025, 5, 20); // outside the window, uncovered → backfill
        let data = [in_window_covered, old_covered, old_uncovered];
        let covered = [in_window_covered, old_covered];
        let out = plan_days(&data, &covered, today, window_days);

        // Every one of the last `window_days` days is present — even the covered one.
        for i in 0..window_days {
            let day = today - chrono::Duration::days(i as i64);
            assert!(out.contains(&day), "trailing-window day {day} must be planned");
        }
        assert!(out.contains(&in_window_covered), "a covered in-window day is refreshed (F1)");
        assert!(out.contains(&old_uncovered), "an uncovered old data day is still backfilled");
        assert!(!out.contains(&old_covered), "a covered day older than the window stays settled");

        // Sorted ascending + de-duplicated.
        let mut sorted = out.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(out, sorted, "sorted, de-duplicated");
    }

    // ── Pure: day-keyed group selection ─────────────────────────────────────

    #[test]
    fn day_keyed_active_keeps_only_active_day_keyed_groups() {
        // Both day-keyed groups active alongside non-day-keyed groups (churn/health):
        // only session_outcomes + autonomy are returned.
        let active = vec![
            "session_outcomes".to_string(),
            "churn".to_string(),
            "autonomy".to_string(),
            "health".to_string(),
        ];
        let groups = day_keyed_active(&active);
        assert_eq!(
            groups,
            vec![DayKeyedGroup::SessionOutcomes, DayKeyedGroup::Autonomy],
        );

        // Only one active → only that one is planned.
        assert_eq!(
            day_keyed_active(&["session_outcomes".to_string()]),
            vec![DayKeyedGroup::SessionOutcomes],
        );

        // Honest-empty: an empty active registry yields no day-keyed groups.
        assert!(day_keyed_active(&[]).is_empty());
        // A registry with only non-day-keyed groups yields nothing to plan.
        assert!(day_keyed_active(&["churn".to_string(), "health".to_string()]).is_empty());
    }

    // ── DB-backed: enqueue one task per missing day per group ────────────────

    #[tokio::test]
    async fn plan_enqueues_one_compute_per_missing_session_outcomes_day_plus_today() {
        // session_outcomes has measurable sessions on two historical days D1 and D2.
        // D1 is already covered (a project_metrics ftr row stamped computed_on=D1), so
        // the plan must enqueue D2 + today (D1 dropped) — one ComputeMetrics per day,
        // each carrying as_of and the group in path.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;

        let now = chrono::Utc::now();
        let today = now.date_naive();
        let ts1 = now - chrono::Duration::days(60);
        let ts2 = now - chrono::Duration::days(30);
        let d1 = ts1.date_naive();
        let d2 = ts2.date_naive();
        // A measurable session on D1, D2, and today (today is a data day too).
        for ts in [ts1, ts2, now - chrono::Duration::hours(2)] {
            let sid = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
            seed_metrics_turn(pg, &sid, 1, ts).await;
        }

        // Pre-cover D1: write an ftr project_metrics row stamped computed_on = D1.
        let ftr_id = *pg
            .active_metric_ids("session_outcomes")
            .await
            .unwrap()
            .get("ftr")
            .expect("ftr is an active session_outcomes metric");
        pg.upsert_project_metric(
            &ftr_id, &pid, None, None, d1, "daily", 1.0, &serde_json::json!({}), "measured",
        )
        .await
        .unwrap();

        // `window_days = 1` isolates the older-missing-day diff: the trailing window
        // is just {today}, so D1/D2 (60/30 days back) are governed purely by covered
        // status. (The trailing-window refresh is covered by
        // `enqueue_refreshes_a_covered_recent_day_within_the_window`.)
        let queue = crate::tasks::queue::TaskQueue::new();
        let computes = enqueue_compute_plan(&queue, pg, &pid, &[DayKeyedGroup::SessionOutcomes], &[], today, 1)
            .await
            .unwrap();
        assert_eq!(computes.total, 2, "D2 + today planned; the covered D1 is dropped");
        assert_eq!(computes.today_ids.len(), 1, "exactly one TODAY compute (the health-barrier dep)");

        // Drain and inspect: each is a ComputeMetrics for this project + group with
        // as_of set; the day-set is exactly {D2, today} and never the covered D1.
        let mut days = Vec::new();
        for _ in 0..computes.total {
            let t = queue.next_task().await;
            assert_eq!(t.kind, TaskKind::ComputeMetrics);
            assert_eq!(t.folder_path, pid.to_string(), "project id in folder_path");
            assert_eq!(t.path, "session_outcomes", "group task_name in path");
            days.push(t.as_of.expect("planner stamps as_of on every compute task"));
        }
        days.sort();
        assert_eq!(days, vec![d2, today], "one compute per missing day + today");
        assert!(!days.contains(&d1), "the already-covered day is not re-enqueued");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn plan_discovers_autonomy_data_days_from_runs_and_events() {
        // autonomy's data-day set is the UNION of run-started days and interruption
        // event days (client ts). Seed a run on day R and a UserPromptSubmit event on
        // a different day E — the plan must cover both (plus today), proving the union.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let csid = format!("_test:planner-autonomy:{uniq}");
        purge_assistant_events(pg, &[&csid]).await;
        purge_runs(pg, &[&pid]).await;

        let now = chrono::Utc::now();
        let today = now.date_naive();
        let run_ts = now - chrono::Duration::days(40);
        let event_ts = now - chrono::Duration::days(50);
        let run_day = run_ts.date_naive();
        let event_day = event_ts.date_naive();

        seed_metrics_client_session(pg, &fid, &pid, &csid, event_ts).await;
        seed_assistant_event(pg, &csid, "UserPromptSubmit", event_ts).await;
        seed_run(pg, &pid, "done", run_ts).await;

        // `window_days = 1` → trailing window {today}; the run/event days (40/50 days
        // back) are the pure union-of-sources discovery under test here.
        let queue = crate::tasks::queue::TaskQueue::new();
        let computes = enqueue_compute_plan(&queue, pg, &pid, &[DayKeyedGroup::Autonomy], &[], today, 1)
            .await
            .unwrap();
        assert_eq!(computes.total, 3, "run day + event day + today (union of both sources)");

        let mut days = Vec::new();
        for _ in 0..computes.total {
            let t = queue.next_task().await;
            assert_eq!(t.path, "autonomy", "group task_name in path");
            days.push(t.as_of.unwrap());
        }
        days.sort();
        assert_eq!(days, vec![event_day, run_day, today],
            "the plan spans both the run day and the (older) event day, plus today");

        purge_runs(pg, &[&pid]).await;
        purge_assistant_events(pg, &[&csid]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn enqueue_refreshes_a_covered_recent_day_within_the_window() {
        // F1 regression guard (enqueue level): a recent day already COVERED (a
        // project_metrics row) is STILL re-enqueued because it falls inside the
        // trailing window — so a day first computed mid-day converges to its complete
        // value instead of freezing to a partial-day sliver. Contrast the covered OLD
        // day in `plan_enqueues_one_compute_per_missing_session_outcomes_day_plus_today`,
        // which is dropped.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;

        let now = chrono::Utc::now();
        let today = now.date_naive();
        // A measurable session 3 days ago — well inside a 14-day window.
        let recent_ts = now - chrono::Duration::days(3);
        let recent_day = recent_ts.date_naive();
        let sid = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, recent_ts).await;
        seed_metrics_turn(pg, &sid, 1, recent_ts).await;

        // Pre-cover that recent day (as if a mid-day compute already ran).
        let ftr_id = *pg
            .active_metric_ids("session_outcomes")
            .await
            .unwrap()
            .get("ftr")
            .expect("ftr is an active session_outcomes metric");
        pg.upsert_project_metric(
            &ftr_id, &pid, None, None, recent_day, "daily", 1.0, &serde_json::json!({}), "measured",
        )
        .await
        .unwrap();

        // `with_max_repos` above the drain count: `next_task` bumps a per-folder
        // running count that only `complete` releases, so draining more than
        // max_concurrent_repos (default 3) same-project tasks without completing would
        // block. (In production the executor completes each task, freeing the slot.)
        let queue = crate::tasks::queue::TaskQueue::with_max_repos(64);
        let computes =
            enqueue_compute_plan(&queue, pg, &pid, &[DayKeyedGroup::SessionOutcomes], &[], today, 14)
                .await
                .unwrap();

        let mut days = Vec::new();
        for _ in 0..computes.total {
            days.push(queue.next_task().await.as_of.expect("planner stamps as_of"));
        }
        assert!(
            days.contains(&recent_day),
            "the covered recent day is refreshed because it is inside the trailing window (F1 fix)",
        );
        assert!(days.contains(&today), "today is always planned");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn plan_autonomy_ignores_stop_only_event_days() {
        // LOW-2: `interruption_rate` (Stop / UserPromptSubmit) writes NO row on a day
        // with no UserPromptSubmit (a 0/0 would be fabricated), so the planner's
        // autonomy data-day set scopes the event arm to UserPromptSubmit — a Stop-only
        // day is NOT a data day and gets no compute. (`window_days = 1` isolates the
        // data-day discovery from the trailing-window refresh.)
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let csid = format!("_test:planner-autonomy-stoponly:{uniq}");
        purge_assistant_events(pg, &[&csid]).await;
        purge_runs(pg, &[&pid]).await;

        let now = chrono::Utc::now();
        let today = now.date_naive();
        let stop_ts = now - chrono::Duration::days(40);
        let stop_day = stop_ts.date_naive();
        seed_metrics_client_session(pg, &fid, &pid, &csid, stop_ts).await;
        // A Stop-only day (no UserPromptSubmit) — must NOT become a data day.
        seed_assistant_event(pg, &csid, "Stop", stop_ts).await;

        let queue = crate::tasks::queue::TaskQueue::new();
        let computes = enqueue_compute_plan(&queue, pg, &pid, &[DayKeyedGroup::Autonomy], &[], today, 1)
            .await
            .unwrap();

        let mut days = Vec::new();
        for _ in 0..computes.total {
            days.push(queue.next_task().await.as_of.unwrap());
        }
        assert!(
            !days.contains(&stop_day),
            "a Stop-only day is not a data day (interruption_rate would write no row)",
        );
        assert_eq!(days, vec![today], "only today is planned (the Stop-only day contributes no data day)");

        purge_assistant_events(pg, &[&csid]).await;
        purge_runs(pg, &[&pid]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn plan_honest_empty_when_no_base_groups() {
        // Honest-empty: no active base groups (neither day-keyed nor snapshot) →
        // nothing enqueued, mirroring the scheduler's empty-registry contract.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;

        let queue = crate::tasks::queue::TaskQueue::new();
        let computes = enqueue_compute_plan(&queue, pg, &pid, &[], &[], chrono::Utc::now().date_naive(), 14)
            .await
            .unwrap();
        assert_eq!(computes.total, 0, "no base groups → no compute tasks");
        assert!(computes.today_ids.is_empty(), "no today computes → no health-barrier deps");
        assert_eq!(queue.status().await.pending, 0);

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[test]
    fn snapshot_active_keeps_only_active_non_day_keyed_base_groups() {
        // The snapshot partition is the active base groups MINUS the day-keyed ones
        // (session_outcomes, autonomy) and MINUS the health barrier's own name.
        let active = vec![
            "session_outcomes".to_string(), // day-keyed → excluded
            "churn".to_string(),
            "duplication".to_string(),
            "autonomy".to_string(), // day-keyed → excluded
            "knowledge".to_string(),
            "tool".to_string(),
            "health".to_string(), // the ComputeHealth barrier's name → not a base group
        ];
        let snap = snapshot_active(&active);
        assert_eq!(
            snap,
            vec![
                MetricGroup::Churn,
                MetricGroup::Duplication,
                MetricGroup::Knowledge,
                MetricGroup::Tool,
            ],
            "snapshot = churn/duplication/knowledge/tool; day-keyed + health excluded",
        );

        // Honest-empty: only day-keyed + health active → no snapshot groups.
        assert!(snapshot_active(&[
            "session_outcomes".to_string(),
            "autonomy".to_string(),
            "health".to_string(),
        ])
        .is_empty());
        // An unknown task_name never fabricates a group.
        assert!(snapshot_active(&["no_such_group".to_string()]).is_empty());
    }

    #[tokio::test]
    async fn enqueue_compute_plan_snapshot_groups_are_today_only() {
        // A snapshot group (churn) with historical session data is NOT backfilled per
        // day: it gets exactly ONE today-only ComputeMetrics (as_of = None), and its id
        // is a health-barrier dep. (Contrast the day-keyed session_outcomes test above,
        // which enqueues one per historical day.)
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;

        let now = chrono::Utc::now();
        let today = now.date_naive();
        // Sessions 60/30 days back — a day-keyed group would plan 3 days; a snapshot
        // group ignores history and plans today only.
        for ts in [now - chrono::Duration::days(60), now - chrono::Duration::days(30)] {
            let sid = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
            seed_metrics_turn(pg, &sid, 1, ts).await;
        }

        let queue = crate::tasks::queue::TaskQueue::new();
        let computes = enqueue_compute_plan(&queue, pg, &pid, &[], &[MetricGroup::Churn], today, 14)
            .await
            .unwrap();
        assert_eq!(computes.total, 1, "a snapshot group is today-only regardless of history");
        assert_eq!(computes.today_ids.len(), 1, "the one today compute is a health-barrier dep");

        let t = queue.next_task().await;
        assert_eq!(t.kind, TaskKind::ComputeMetrics);
        assert_eq!(t.path, "churn", "snapshot group task_name in path");
        assert_eq!(t.as_of, None, "snapshot compute is the today-only rolling-window run (as_of None)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn plan_enqueues_health_barrier_blocked_on_today_computes() {
        // plan() reads the REAL active registry (all groups active) and enqueues the
        // whole graph for one project: day-keyed per-day computes + snapshot today-only
        // computes + ONE ComputeHealth barrier blocked on this pass's TODAY computes
        // (NOT the historical backfill days, so day-one health isn't stalled behind the
        // backfill). A fresh project has only today's data, so the plan is small.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        // One measurable session TODAY so session_outcomes has a today data day.
        let now = chrono::Utc::now();
        let sid = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, now).await;
        seed_metrics_turn(pg, &sid, 1, now).await;

        let enqueued = plan(&ctx, &pid.to_string()).await.unwrap();
        assert!(enqueued >= 2, "at least one base compute + the health barrier, got {enqueued}");

        // Exactly one ComputeHealth barrier for this project, and it is BLOCKED (its
        // deps — this pass's today computes — are still pending).
        let owner = pid.to_string();
        let health = ctx
            .queue
            .snapshot()
            .await
            .into_iter()
            .filter(|(k, f, p)| *k == TaskKind::ComputeHealth && *f == owner && p.is_empty())
            .count();
        assert_eq!(health, 1, "exactly one ComputeHealth barrier per project");
        let status = ctx.queue.status().await;
        assert!(status.blocked >= 1, "the health barrier is blocked on its project's today computes");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }
}
