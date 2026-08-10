//! `session_outcomes` metric group computer (Phase 5.1).
//!
//! The FIRST real base-metric computer and the TEMPLATE the other five groups
//! follow. It reads the configured rolling window ([`metrics.window_days`],
//! default 14) and writes daily + per-session rows to `sensei.project_metrics`
//! (via [`PgStore::upsert_project_metric`]) for ONE project.
//!
//! v1 registry keys (all `task_name = "session_outcomes"`):
//! - `ftr` (pct): first-try-right rate — daily (`numerator` = #`ftr` sessions,
//!   `denominator` = # sessions) AND per-session (1.0/0.0).
//! - `rework_ratio` (ratio): Σ tool-calls in `corrected` sessions / Σ tool-calls
//!   across all sessions — daily.
//! - `throughput` (count): sessions per day — daily.
//!
//! FTR definition: the daily `ftr` is `avg(case when s.ftr then 1 else 0 end)`
//! computed as `ftr_count / session_count` over the measurable session base
//! (`outcome is not null`, project scope via `sensei.folders.project_id`) — and
//! additionally stores the parts + counts the roll-up views re-aggregate from.
//! These `ftr` rows are now the single FTR source of truth (Phase 8 retired the
//! legacy `sensei.ftr_daily` / `sensei.project_ftr_metrics` views).
//!
//! Never-fabricate: every DB call propagates `Err`; a day/metric with no data
//! writes NO row (a `0` value is written only when a real denominator exists).
//! `tool_calls` live on `activity.turns` (per-turn), never on `sessions`.

use crate::db::pg_store::PgStore;
use crate::tasks::executor::TaskContext;

use super::MetricGroup;

/// `sensei.metric_grain` text values.
const GRAIN_DAILY: &str = "daily";
const GRAIN_SESSION: &str = "session";
/// `sensei.metric_source` text value — these are measured, not estimated.
const SOURCE_MEASURED: &str = "measured";

/// The registry `key`s this computer produces.
const KEY_FTR: &str = "ftr";
const KEY_REWORK: &str = "rework_ratio";
const KEY_THROUGHPUT: &str = "throughput";
const KEY_TTUR: &str = "time_to_useful_result";

/// One day's session-level aggregates for a project: `(day, session_count,
/// ftr_count, correction_count)`. Only days WITH ≥1 measurable session appear, so
/// `session_count` (the `ftr` denominator) is always ≥ 1.
type DayAgg = (chrono::NaiveDate, i64, i64, i64);

/// One day's turn-level aggregates for `rework_ratio`: `(day, corrected_tool_calls,
/// total_tool_calls)` summed from `activity.turns.tool_calls`.
type DayRework = (chrono::NaiveDate, i64, i64);

/// One session's first-try signal: `(session_id, day, ftr)`. `ftr` is nullable;
/// `NULL`/`false` both score 0.0 (the `case when s.ftr` FTR convention).
type SessionFtr = (uuid::Uuid, chrono::NaiveDate, Option<bool>);

/// One day's `time_to_useful_result`: `(day, median_seconds, n)`. `n` = the number
/// of sessions that contributed a first-useful latency that day.
type DayTtur = (chrono::NaiveDate, f64, i64);

/// Daily session-level aggregates over the window, project-scoped via
/// `sensei.folders.project_id`. `outcome is not null` restricts to measurable
/// (analyzed) sessions — in-flight sessions whose `ftr`/`outcome` are still
/// `NULL` are excluded from the FTR base.
async fn daily_session_aggregates(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
) -> Result<Vec<DayAgg>, String> {
    sqlx_core::query_as::query_as(
        "SELECT date_trunc('day', s.started_at)::date              AS day
              , count(*)::int8                                     AS session_count
              , count(*) FILTER (WHERE s.ftr)::int8                AS ftr_count
              , coalesce(sum(s.corrections), 0)::int8              AS correction_count
           FROM activity.sessions s
           JOIN sensei.folders    f ON f.id = s.folder_id
          WHERE f.project_id  = $1
            AND s.outcome    IS NOT NULL
            AND s.started_at >= now() - make_interval(days => $2::int)
          GROUP BY 1
          ORDER BY 1",
    )
    .bind(project_id)
    .bind(window_days as i32)
    .fetch_all(pg.pool())
    .await
    .map_err(|e| e.to_string())
}

/// Daily tool-call sums for `rework_ratio`: `corrected_tool_calls` (numerator) over
/// sessions with `outcome = 'corrected'`, and `total_tool_calls` (denominator) over
/// all measurable sessions that day. Tool-calls come from `activity.turns`; a
/// session with no turns contributes 0 either way.
async fn daily_rework(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
) -> Result<Vec<DayRework>, String> {
    sqlx_core::query_as::query_as(
        "SELECT date_trunc('day', s.started_at)::date                                         AS day
              , coalesce(sum(t.tool_calls) FILTER (WHERE s.outcome = 'corrected'::sensei.session_outcome), 0)::int8 AS corrected_tool_calls
              , coalesce(sum(t.tool_calls), 0)::int8                                           AS total_tool_calls
           FROM activity.sessions s
           JOIN sensei.folders    f ON f.id = s.folder_id
           JOIN activity.turns    t ON t.session_id = s.id
          WHERE f.project_id  = $1
            AND s.outcome    IS NOT NULL
            AND s.started_at >= now() - make_interval(days => $2::int)
          GROUP BY 1
          ORDER BY 1",
    )
    .bind(project_id)
    .bind(window_days as i32)
    .fetch_all(pg.pool())
    .await
    .map_err(|e| e.to_string())
}

/// Per-session first-try rows over the window, project-scoped, same measurable
/// base as [`daily_session_aggregates`]. `computed_on` is the session's own day.
async fn session_ftr(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
) -> Result<Vec<SessionFtr>, String> {
    sqlx_core::query_as::query_as(
        "SELECT s.id                                       AS session_id
              , date_trunc('day', s.started_at)::date      AS day
              , s.ftr                                      AS ftr
           FROM activity.sessions s
           JOIN sensei.folders    f ON f.id = s.folder_id
          WHERE f.project_id  = $1
            AND s.outcome    IS NOT NULL
            AND s.started_at >= now() - make_interval(days => $2::int)
          ORDER BY s.started_at",
    )
    .bind(project_id)
    .bind(window_days as i32)
    .fetch_all(pg.pool())
    .await
    .map_err(|e| e.to_string())
}

/// Daily median `time_to_useful_result` (seconds) per project. For each measurable
/// session, the latency is `started_at → ended_at of the FIRST non-correction turn`
/// (the first usable output). `percentile_cont(0.5)` medians those per-session
/// latencies within each day. Sessions whose only turns are corrections — or that
/// have no turns — produce no usable output and are dropped by the inner `LIMIT 1`
/// join (never a fabricated 0). `n` is the contributing session count that day.
async fn daily_time_to_useful(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
) -> Result<Vec<DayTtur>, String> {
    sqlx_core::query_as::query_as(
        "WITH first_useful AS ( \
             SELECT date_trunc('day', s.started_at)::date                        AS day \
                  , EXTRACT(EPOCH FROM (fu.ended_at - s.started_at))::float8      AS secs \
               FROM activity.sessions s \
               JOIN sensei.folders    f ON f.id = s.folder_id \
               JOIN LATERAL ( \
                      SELECT t.ended_at \
                        FROM activity.turns t \
                       WHERE t.session_id     = s.id \
                         AND t.is_correction  = false \
                       ORDER BY t.turn_number \
                       LIMIT 1 \
                    ) fu ON true \
              WHERE f.project_id  = $1 \
                AND s.outcome    IS NOT NULL \
                AND s.started_at >= now() - make_interval(days => $2::int) \
         ) \
         SELECT day \
              , percentile_cont(0.5) WITHIN GROUP (ORDER BY secs)::float8         AS median_secs \
              , count(*)::int8                                                     AS n \
           FROM first_useful \
          WHERE secs >= 0 \
          GROUP BY day \
          ORDER BY day",
    )
    .bind(project_id)
    .bind(window_days as i32)
    .fetch_all(pg.pool())
    .await
    .map_err(|e| e.to_string())
}

/// Compute the `session_outcomes` group for one project over the configured window.
/// `project_raw` is the project uuid carried in `task.folder_path`. Returns the
/// number of `project_metrics` rows written (`0` = honest-empty: no measurable
/// sessions, or none of the group's metrics active). Idempotent — re-running
/// backfills in place via the upsert identity.
pub(super) async fn compute(ctx: &TaskContext, project_raw: &str) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(project_raw)
        .map_err(|e| format!("session_outcomes: bad project id {project_raw:?}: {e}"))?;
    let pg = ctx.pg();

    // Reuse the scheduler's window reader (config key + parser + default) — DRY.
    let window_days = crate::tasks::metrics_scheduler::window_days(pg).await;

    // Resolve key → metric_id for this group's ACTIVE metrics via the shared store
    // helper. A key absent from the map is inactive (retired / not-yet-effective /
    // unseeded) → skipped: the computer never writes a value for an inactive metric.
    let ids = pg.active_metric_ids(MetricGroup::SessionOutcomes.as_str()).await?;
    if ids.is_empty() {
        return Ok(0);
    }
    let ftr_id = ids.get(KEY_FTR).copied();
    let rework_id = ids.get(KEY_REWORK).copied();
    let throughput_id = ids.get(KEY_THROUGHPUT).copied();
    let ttur_id = ids.get(KEY_TTUR).copied();

    let mut written = 0u32;

    // Daily session-level metrics: ftr (pct) + throughput (count).
    if ftr_id.is_some() || throughput_id.is_some() {
        for (day, session_count, ftr_count, correction_count) in
            daily_session_aggregates(pg, &project_id, window_days).await?
        {
            if let Some(mid) = ftr_id {
                // denominator (session_count) is ≥ 1 for any returned day.
                let value = ftr_count as f64 / session_count as f64;
                let props = serde_json::json!({
                    "numerator": ftr_count,
                    "denominator": session_count,
                    "correction_count": correction_count,
                });
                pg.upsert_project_metric(
                    &mid, &project_id, None, None, day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
                )
                .await?;
                written += 1;
            }
            if let Some(mid) = throughput_id {
                // count-type: value IS the count; no numerator/denominator needed.
                let props = serde_json::json!({});
                pg.upsert_project_metric(
                    &mid, &project_id, None, None, day, GRAIN_DAILY, session_count as f64, &props,
                    SOURCE_MEASURED,
                )
                .await?;
                written += 1;
            }
        }
    }

    // Daily rework_ratio (ratio) — only if active.
    if let Some(mid) = rework_id {
        for (day, corrected_tool_calls, total_tool_calls) in
            daily_rework(pg, &project_id, window_days).await?
        {
            if total_tool_calls == 0 {
                // No tool-call data that day → no denominator → NO row (a 0/0 would
                // be a fabricated zero, not a measured one).
                continue;
            }
            let value = corrected_tool_calls as f64 / total_tool_calls as f64;
            let props = serde_json::json!({
                "numerator": corrected_tool_calls,
                "denominator": total_tool_calls,
            });
            pg.upsert_project_metric(
                &mid, &project_id, None, None, day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }
    }

    // Daily time_to_useful_result (duration, median seconds) — only if active. A day
    // with no session that produced a usable turn writes NO row (honest-empty).
    if let Some(mid) = ttur_id {
        for (day, median_secs, n) in daily_time_to_useful(pg, &project_id, window_days).await? {
            let props = serde_json::json!({ "n": n });
            pg.upsert_project_metric(
                &mid, &project_id, None, None, day, GRAIN_DAILY, median_secs, &props, SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }
    }

    // Per-session ftr rows (grain=session, session_id set) — only if active.
    if let Some(mid) = ftr_id {
        for (session_id, day, ftr) in session_ftr(pg, &project_id, window_days).await? {
            let hit: i64 = if ftr.unwrap_or(false) { 1 } else { 0 };
            // Keep the ratio/pct props contract uniform: value = numerator/denominator
            // = hit/1. (Session-grain rows are excluded from the daily roll-up views,
            // which read grain='daily' only, so this never double-counts.)
            let props = serde_json::json!({ "numerator": hit, "denominator": 1 });
            pg.upsert_project_metric(
                &mid, &project_id, None, Some(&session_id), day, GRAIN_SESSION, hit as f64, &props,
                SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::test_support::{
        cleanup_metrics_fixture, make_ctx, seed_metrics_project_folder, seed_metrics_session,
        seed_metrics_turn, seed_metrics_turn_ex,
    };
    use sqlx_core::query_as::query_as;

    /// The daily project-scope rows keyed by metric `(key → (value, props))`.
    async fn daily_rows(pg: &PgStore, pid: &uuid::Uuid) -> Vec<(String, f64, serde_json::Value)> {
        query_as(
            "SELECT m.key, pm.value::float8, pm.props \
               FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND pm.grain = 'daily' AND pm.folder_id IS NULL \
              ORDER BY m.key",
        )
        .bind(pid)
        .fetch_all(pg.pool())
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn session_outcomes_writes_ftr_rework_throughput() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;

        // Sessions on ONE day (fixed instant so inserts can't straddle midnight): 3
        // first-try (completed, 0 corrections, 2 tool-calls each) + 1 corrected
        // (ftr=false, 2 corrections, 6 tool-calls). Σ measurable tool-calls = 12.
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        for _ in 0..3 {
            let sid = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
            seed_metrics_turn(pg, &sid, 2, ts).await;
        }
        let corrected = seed_metrics_session(pg, &fid, &pid, Some("corrected"), Some(false), 2, ts).await;
        seed_metrics_turn(pg, &corrected, 6, ts).await;
        // An in-flight session (outcome NULL, ftr NULL) on the SAME day, WITH
        // tool-calls — it is not yet measurable and MUST be excluded from every
        // session_outcomes metric (the `outcome is not null` FTR base). If the
        // filter regressed, session_count→5, ftr→3/5, throughput→5, rework→6/17.
        let inflight = seed_metrics_session(pg, &fid, &pid, None, None, 0, ts).await;
        seed_metrics_turn(pg, &inflight, 5, ts).await;

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 8, "4 daily rows (ftr, rework, throughput, time_to_useful) + 4 per-session ftr rows (in-flight excluded)");

        // ── Daily rows ────────────────────────────────────────────────────
        let daily = daily_rows(pg, &pid).await;

        let ftr = daily.iter().find(|r| r.0 == "ftr").expect("ftr daily row present");
        assert!((ftr.1 - 0.75).abs() < 1e-9, "ftr value = 3/4 = 0.75 (avg(case when ftr) over the measurable base)");
        assert_eq!(ftr.2["numerator"].as_i64(), Some(3), "ftr numerator = # first-try sessions (in-flight excluded)");
        assert_eq!(ftr.2["denominator"].as_i64(), Some(4), "ftr denominator = session_count (in-flight excluded)");
        assert_eq!(ftr.2["correction_count"].as_i64(), Some(2), "correction_count = Σ corrections (display)");

        let rework = daily.iter().find(|r| r.0 == "rework_ratio").expect("rework_ratio daily row present");
        assert!((rework.1 - 0.5).abs() < 1e-9, "rework value = 6/12 = 0.5 (in-flight's 5 tool-calls excluded)");
        assert_eq!(rework.2["numerator"].as_i64(), Some(6), "rework numerator = corrected-session tool-calls");
        assert_eq!(rework.2["denominator"].as_i64(), Some(12), "rework denominator = all measurable tool-calls");

        let throughput = daily.iter().find(|r| r.0 == "throughput").expect("throughput daily row present");
        assert!((throughput.1 - 4.0).abs() < 1e-9, "throughput value = 4 measurable sessions (in-flight excluded)");

        // ── FTR parity (Phase-8 consolidation check) ──────────────────────
        // Asserted against the ARITHMETIC the retired `sensei.ftr_daily` view
        // computed — `avg(case when s.ftr then 1 else 0)` and `count(*)` over the
        // measurable base — read straight from the base tables, NOT the view
        // object, so this survives the view being dropped in Phase 8.5.
        let (fd_rate, fd_count): (f64, i64) = query_as(
            "SELECT avg(CASE WHEN s.ftr THEN 1.0 ELSE 0.0 END)::float8, count(*)::int8 \
               FROM activity.sessions s JOIN sensei.folders f ON f.id = s.folder_id \
              WHERE f.project_id = $1 AND s.outcome IS NOT NULL",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert!((fd_rate - ftr.1).abs() < 1e-9, "computed daily ftr value == avg(case when ftr) over the measurable base (old ftr_daily arithmetic)");
        assert_eq!(
            Some(fd_count), ftr.2["denominator"].as_i64(),
            "ftr denominator == count(*) over the measurable base (old ftr_daily.session_count)",
        );

        // ── Per-session ftr rows ─────────────────────────────────────────
        let sess: Vec<(uuid::Uuid, f64)> = query_as(
            "SELECT pm.session_id, pm.value::float8 \
               FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND pm.grain = 'session' AND m.key = 'ftr'",
        )
        .bind(pid)
        .fetch_all(pg.pool())
        .await
        .unwrap();
        assert_eq!(sess.len(), 4, "one per-session ftr row per MEASURABLE session");
        assert!(!sess.iter().any(|(id, _)| *id == inflight), "the in-flight session gets NO per-session row");
        let ones = sess.iter().filter(|(_, v)| (*v - 1.0).abs() < 1e-9).count();
        let zeros = sess.iter().filter(|(_, v)| v.abs() < 1e-9).count();
        assert_eq!(ones, 3, "three first-try sessions score 1.0");
        assert_eq!(zeros, 1, "the corrected session scores 0.0");
        let zero_sid = sess.iter().find(|(_, v)| v.abs() < 1e-9).map(|(id, _)| *id);
        assert_eq!(zero_sid, Some(corrected), "the 0.0 per-session row is the corrected session");

        // ── Idempotency: re-run backfills in place, never duplicates ──────
        let again = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(again, 8, "re-run recomputes the same rows");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 8, "idempotent upsert — still 8 rows after a second run");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    /// Phase 8.4 parity proof, written as formula-equivalence so it survives the
    /// `sensei.ftr_daily` / `sensei.project_ftr_metrics` drop: the store-derived
    /// FTR (from `project_metrics`, read back through the getters) equals the
    /// ARITHMETIC the retired views computed, expressed directly over the seeded
    /// sessions — NOT queried from any view object.
    #[tokio::test]
    async fn ftr_parity_store_vs_views() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;

        // Seed a measurable set on ONE day (all `outcome is not null`, so the old
        // per-session-average view formula and the store's Σnum/Σden coincide):
        // 3 first-try + 1 corrected. Direct arithmetic over these seeds:
        //   ftr_daily      : avg(case when ftr) = (1+1+1+0)/4 = 0.75; session_count = 4
        //   project_ftr    : Σnumerator/Σdenominator = 3/4 = 0.75; sessions_7d = 4
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        for _ in 0..3 {
            seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
        }
        seed_metrics_session(pg, &fid, &pid, Some("corrected"), Some(false), 1, ts).await;

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert!(written > 0, "compute wrote store rows for the seeded sessions");

        // Store-derived daily FTR (get_ftr_daily reads project_metric_daily) ==
        // the old ftr_daily arithmetic over the seed.
        let daily = pg.get_ftr_daily(Some(&pid), 14).await.unwrap();
        let today = (chrono::Utc::now()).date_naive().to_string();
        let row = daily.iter().find(|r| r["day"].as_str() == Some(today.as_str()))
            .expect("today's daily ftr row present");
        assert!((row["ftr_rate"].as_f64().unwrap() - 0.75).abs() < 1e-9,
            "store daily ftr_rate == avg(case when ftr) over seeds = 0.75");
        assert_eq!(row["session_count"].as_i64(), Some(4),
            "store session_count == count(*) over the measurable base = 4");

        // Store-derived 14d headline (get_project_ftr reads project_metric_daily)
        // == the old project_ftr_metrics formula (Σnum/Σden over 14d) over the seed.
        let ftr = pg.get_project_ftr(&pid).await.unwrap();
        assert!((ftr["ftr14d"].as_f64().unwrap() - 0.75).abs() < 1e-9,
            "store ftr14d == Σnumerator/Σdenominator over 14d = 3/4 = 0.75");
        assert_eq!(ftr["sessions7d"].as_i64(), Some(4),
            "store sessions7d == Σdenominator over 7d = 4");

        // And the shared rate helper the legacy surfaces call agrees.
        let rate = pg.get_project_ftr_rate(&pid).await.unwrap();
        assert!((rate.expect("rate present for a project with ftr rows") - 0.75).abs() < 1e-9,
            "get_project_ftr_rate == the same 14d headline number");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    /// FIX 2: `get_project_ftr`'s inline trend must use the same analyzed base as
    /// the headline (`outcome is not null`), so the sparkline's last point agrees
    /// with `ftr14d` even when the day has in-flight sessions. Before the fix the
    /// unfiltered trend scored the in-flight session as 0 and dragged the point
    /// below the headline (last=0.5 vs ftr14d=2/3). Mutation guard: dropping the
    /// `outcome is not null` filter from the trend query fails the last assert.
    #[tokio::test]
    async fn trend_last_point_matches_headline_with_inflight_sessions() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        // 2 first-try + 1 corrected (measurable) + 1 in-flight (outcome NULL) today.
        for _ in 0..2 {
            seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
        }
        seed_metrics_session(pg, &fid, &pid, Some("corrected"), Some(false), 1, ts).await;
        seed_metrics_session(pg, &fid, &pid, None, None, 0, ts).await; // in-flight, excluded

        compute(&ctx, &pid.to_string()).await.unwrap();

        let ftr = pg.get_project_ftr(&pid).await.unwrap();
        let headline = ftr["ftr14d"].as_f64().expect("ftr14d present");
        assert!((headline - 2.0 / 3.0).abs() < 1e-9,
            "headline = 2/3 over the 3 measurable sessions (in-flight excluded)");
        let last = ftr["ftrTrend"].as_array().and_then(|a| a.last()).and_then(|v| v.as_f64())
            .expect("trend has a last point");
        assert!((last - headline).abs() < 1e-9,
            "trend's last point agrees with the headline (same `outcome is not null` base)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn session_outcomes_no_data_writes_zero_rows() {
        // Never-fabricate: a project with zero sessions in the window writes NO rows
        // (not a defaulted 0). Honest-empty is the correct output for no data.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let pid = pg
            .create_project(&format!("_test:so-empty:{uniq}"), None, None)
            .await
            .unwrap();

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 0, "no sessions in the window → zero rows written");

        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no project_metrics rows for an empty project (never fabricated)");

        cleanup_metrics_fixture(pg, &pid, None, &[]).await;
    }

    #[tokio::test]
    async fn session_outcomes_zero_tool_calls_writes_no_rework_row() {
        // A day with measurable sessions whose turns carry ZERO tool-calls has a real
        // rework denominator of 0 → NO rework_ratio row (a 0/0 would be a fabricated
        // zero). ftr + throughput still compute (they don't depend on tool-calls).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        // 2 completed sessions, each with a turn but ZERO tool-calls → the day appears
        // in the rework aggregate with total_tool_calls = 0 (exercises the skip path).
        for _ in 0..2 {
            let sid = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
            seed_metrics_turn(pg, &sid, 0, ts).await;
        }

        let written = compute(&ctx, &pid.to_string()).await.unwrap();

        let (rework_rows,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'rework_ratio'",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(rework_rows, 0, "no rework_ratio row when total tool-calls is 0 (never a fabricated 0/0)");
        assert_eq!(written, 5, "ftr daily + throughput daily + time_to_useful daily + 2 per-session ftr; rework skipped");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn session_outcomes_all_corrected_writes_zero_ftr() {
        // A day where EVERY session is corrected → ftr numerator 0 but a REAL
        // denominator → value 0.0 is WRITTEN (a real zero, never skipped).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        for _ in 0..2 {
            let sid = seed_metrics_session(pg, &fid, &pid, Some("corrected"), Some(false), 1, ts).await;
            seed_metrics_turn(pg, &sid, 3, ts).await;
        }

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 6, "ftr daily (0.0) + rework daily + throughput daily + time_to_useful daily + 2 per-session ftr");

        let daily = daily_rows(pg, &pid).await;
        let ftr = daily.iter().find(|r| r.0 == "ftr").expect("ftr daily row present (a real zero is still written)");
        assert!(ftr.1.abs() < 1e-9, "ftr value is a real 0.0 (0 numerator over a real denominator)");
        assert_eq!(ftr.2["numerator"].as_i64(), Some(0), "ftr numerator = 0 (no first-try sessions)");
        assert_eq!(ftr.2["denominator"].as_i64(), Some(2), "ftr denominator = 2 (real denominator → row written, not skipped)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn time_to_useful_result_is_median_of_first_non_correction_turn_latency() {
        // Definition (B): latency = session.started_at → ended_at of the FIRST turn
        // that is not a correction. Median over the day's measurable sessions.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;

        let t = chrono::Utc::now() - chrono::Duration::hours(2);
        let at = |n: i64| t + chrono::Duration::seconds(n);

        // A: only turn is useful at +10s → 10s.
        let a = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, t).await;
        seed_metrics_turn_ex(pg, &a, 1, t, at(10), false, 1).await;
        // B: turn 1 is a correction (+3s), turn 2 is the first useful one (+30s) → 30s.
        let b = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, t).await;
        seed_metrics_turn_ex(pg, &b, 1, t, at(3), true, 1).await;
        seed_metrics_turn_ex(pg, &b, 2, at(3), at(30), false, 1).await;
        // C: session-outcome 'corrected' but its first TURN is useful at +20s → 20s
        // (session-level correction != turn-level correction).
        let c = seed_metrics_session(pg, &fid, &pid, Some("corrected"), Some(false), 1, t).await;
        seed_metrics_turn_ex(pg, &c, 1, t, at(20), false, 1).await;
        // D: ONLY a correction turn → produced no usable output → excluded.
        let d = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, t).await;
        seed_metrics_turn_ex(pg, &d, 1, t, at(5), true, 1).await;
        // In-flight (outcome NULL) with a useful turn → not measurable → excluded.
        let inflight = seed_metrics_session(pg, &fid, &pid, None, None, 0, t).await;
        seed_metrics_turn_ex(pg, &inflight, 1, t, at(1), false, 1).await;

        compute(&ctx, &pid.to_string()).await.unwrap();

        let daily = daily_rows(pg, &pid).await;
        let ttur = daily.iter().find(|r| r.0 == "time_to_useful_result")
            .expect("time_to_useful_result daily row present");
        // median([10, 20, 30]) = 20; D (correction-only) + in-flight excluded.
        assert!((ttur.1 - 20.0).abs() < 1e-6,
            "median first-useful latency = median(10,20,30) = 20s, got {}", ttur.1);
        assert_eq!(ttur.2["n"].as_i64(), Some(3),
            "n = 3 sessions with a usable turn (correction-only + in-flight excluded)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn time_to_useful_result_no_usable_turn_writes_no_row() {
        // Never-fabricate: if every measurable session's only turn is a correction,
        // no session produced a usable output → NO time_to_useful_result row (not a 0).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let t = chrono::Utc::now() - chrono::Duration::hours(2);
        for _ in 0..2 {
            let s = seed_metrics_session(pg, &fid, &pid, Some("corrected"), Some(false), 1, t).await;
            seed_metrics_turn_ex(pg, &s, 1, t, t + chrono::Duration::seconds(4), true, 1).await;
        }

        compute(&ctx, &pid.to_string()).await.unwrap();

        let (rows,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'time_to_useful_result'",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(rows, 0, "no usable turn in any session → no time_to_useful_result row (never a fabricated 0)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }
}
