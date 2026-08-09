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
//! Parity: the daily `ftr` re-derives `database/ddl/view/sensei/ftr_daily.ddl`'s
//! `avg(case when s.ftr then 1 else 0 end)` as `ftr_count / session_count`, over
//! the same session base (`outcome is not null`, project scope via
//! `sensei.folders.project_id`) — but additionally stores the parts + counts the
//! roll-up views re-aggregate from.
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

/// The active `metric_id` for each key this group owns, resolved from the ACTIVE
/// registry (`active_metrics`) filtered to `task_name = "session_outcomes"`. A key
/// that is retired / not-yet-effective / not seeded resolves to `None` and is
/// therefore SKIPPED — the computer never writes a value for an inactive metric.
struct MetricIds {
    ftr:        Option<uuid::Uuid>,
    rework:     Option<uuid::Uuid>,
    throughput: Option<uuid::Uuid>,
}

impl MetricIds {
    /// Resolve key → `metric_id` from the active registry, scoped to this group's
    /// `task_name`. Propagates the `active_metrics` read error (never masks it).
    async fn resolve(pg: &PgStore) -> Result<Self, String> {
        let group = MetricGroup::SessionOutcomes.as_str();
        let mut ids = MetricIds { ftr: None, rework: None, throughput: None };
        for m in pg.active_metrics().await? {
            if m.task_name != group {
                continue;
            }
            match m.key.as_str() {
                KEY_FTR => ids.ftr = Some(m.id),
                KEY_REWORK => ids.rework = Some(m.id),
                KEY_THROUGHPUT => ids.throughput = Some(m.id),
                _ => {}
            }
        }
        Ok(ids)
    }

    /// True when none of this group's metrics are active — nothing to compute.
    fn is_empty(&self) -> bool {
        self.ftr.is_none() && self.rework.is_none() && self.throughput.is_none()
    }
}

/// One day's session-level aggregates for a project: `(day, session_count,
/// ftr_count, correction_count)`. Only days WITH ≥1 measurable session appear, so
/// `session_count` (the `ftr` denominator) is always ≥ 1.
type DayAgg = (chrono::NaiveDate, i64, i64, i64);

/// One day's turn-level aggregates for `rework_ratio`: `(day, corrected_tool_calls,
/// total_tool_calls)` summed from `activity.turns.tool_calls`.
type DayRework = (chrono::NaiveDate, i64, i64);

/// One session's first-try signal: `(session_id, day, ftr)`. `ftr` is nullable;
/// `NULL`/`false` both score 0.0 (matching `ftr_daily`'s `case when s.ftr`).
type SessionFtr = (uuid::Uuid, chrono::NaiveDate, Option<bool>);

/// Daily session-level aggregates over the window, project-scoped via
/// `sensei.folders.project_id` (the linkage `ftr_daily` uses). `outcome is not
/// null` restricts to measurable (analyzed) sessions — in-flight sessions whose
/// `ftr`/`outcome` are still `NULL` are excluded, as in `ftr_daily`.
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

    // Resolve which of this group's metrics are active; skip the rest.
    let ids = MetricIds::resolve(pg).await?;
    if ids.is_empty() {
        return Ok(0);
    }

    let mut written = 0u32;

    // Daily session-level metrics: ftr (pct) + throughput (count).
    if ids.ftr.is_some() || ids.throughput.is_some() {
        for (day, session_count, ftr_count, correction_count) in
            daily_session_aggregates(pg, &project_id, window_days).await?
        {
            if let Some(mid) = ids.ftr {
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
            if let Some(mid) = ids.throughput {
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
    if let Some(mid) = ids.rework {
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

    // Per-session ftr rows (grain=session, session_id set) — only if active.
    if let Some(mid) = ids.ftr {
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
    use crate::tasks::test_support::make_ctx;
    use sqlx_core::query::query;
    use sqlx_core::query_as::query_as;

    /// Create a project + a folder wired to it, mirroring the pg_store test
    /// helpers' fixed `/_test` watch root. Returns `(project_id, folder_id)`.
    async fn seed_project_folder(pg: &PgStore, uniq: &uuid::Uuid) -> (uuid::Uuid, uuid::Uuid) {
        let pid = pg
            .create_project(&format!("_test:so:{uniq}"), None, None)
            .await
            .unwrap();
        pg.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) \
             VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) \
             ON CONFLICT DO NOTHING",
        )
        .await
        .unwrap();
        let name = format!("so-{uniq}");
        let abs = format!("/_test/so-{uniq}");
        let (fid,): (uuid::Uuid,) = query_as(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path, project_id) \
             VALUES('00000000-0000-0000-0000-000000000001', 'git'::sensei.folder_kind, $1, $1, $2, $3) \
             ON CONFLICT(abs_path) DO UPDATE SET project_id = EXCLUDED.project_id RETURNING id",
        )
        .bind(&name)
        .bind(&abs)
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        (pid, fid)
    }

    /// Insert one session and return its id.
    async fn seed_session(
        pg: &PgStore,
        fid: &uuid::Uuid,
        pid: &uuid::Uuid,
        outcome: &str,
        ftr: bool,
        corrections: i32,
        started_at: chrono::DateTime<chrono::Utc>,
    ) -> uuid::Uuid {
        let (id,): (uuid::Uuid,) = query_as(
            "INSERT INTO activity.sessions (folder_id, project_id, outcome, ftr, corrections, started_at) \
             VALUES ($1, $2, $3::sensei.session_outcome, $4, $5, $6) RETURNING id",
        )
        .bind(fid)
        .bind(pid)
        .bind(outcome)
        .bind(ftr)
        .bind(corrections)
        .bind(started_at)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        id
    }

    /// Attach one turn (carrying the session's tool-calls) to a session.
    async fn seed_turn(
        pg: &PgStore,
        sid: &uuid::Uuid,
        tool_calls: i32,
        started_at: chrono::DateTime<chrono::Utc>,
    ) {
        query(
            "INSERT INTO activity.turns (session_id, turn_number, started_at, ended_at, tool_calls) \
             VALUES ($1, 1, $2, $2, $3)",
        )
        .bind(sid)
        .bind(started_at)
        .bind(tool_calls)
        .execute(pg.pool())
        .await
        .unwrap();
    }

    /// Remove everything a test seeded: the project (cascades its
    /// `project_metrics`) and the folder (cascades its `sessions` → `turns`).
    async fn cleanup(pg: &PgStore, pid: &uuid::Uuid, fid: Option<&uuid::Uuid>) {
        query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(pg.pool()).await.unwrap();
        if let Some(fid) = fid {
            query("DELETE FROM sensei.folders WHERE id = $1").bind(fid).execute(pg.pool()).await.unwrap();
        }
    }

    #[tokio::test]
    async fn session_outcomes_writes_ftr_rework_throughput() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_project_folder(pg, &uniq).await;

        // 4 sessions on ONE day (fixed instant so the inserts can't straddle
        // midnight): 3 first-try (completed, 0 corrections, 2 tool-calls each) and 1
        // corrected (ftr=false, 2 corrections, 6 tool-calls). Σ tool-calls = 12.
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        for _ in 0..3 {
            let sid = seed_session(pg, &fid, &pid, "completed", true, 0, ts).await;
            seed_turn(pg, &sid, 2, ts).await;
        }
        let corrected = seed_session(pg, &fid, &pid, "corrected", false, 2, ts).await;
        seed_turn(pg, &corrected, 6, ts).await;

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 7, "3 daily rows (ftr, rework_ratio, throughput) + 4 per-session ftr rows");

        // ── Daily rows ────────────────────────────────────────────────────
        let daily: Vec<(String, f64, serde_json::Value)> = query_as(
            "SELECT m.key, pm.value::float8, pm.props \
               FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND pm.grain = 'daily' AND pm.folder_id IS NULL \
              ORDER BY m.key",
        )
        .bind(pid)
        .fetch_all(pg.pool())
        .await
        .unwrap();

        let ftr = daily.iter().find(|r| r.0 == "ftr").expect("ftr daily row present");
        assert!((ftr.1 - 0.75).abs() < 1e-9, "ftr value = 3/4 = 0.75 (matches ftr_daily's avg(case when ftr))");
        assert_eq!(ftr.2["numerator"].as_i64(), Some(3), "ftr numerator = # first-try sessions");
        assert_eq!(ftr.2["denominator"].as_i64(), Some(4), "ftr denominator = session_count");
        assert_eq!(ftr.2["correction_count"].as_i64(), Some(2), "correction_count = Σ corrections (display)");

        let rework = daily.iter().find(|r| r.0 == "rework_ratio").expect("rework_ratio daily row present");
        assert!((rework.1 - 0.5).abs() < 1e-9, "rework value = 6/12 = 0.5");
        assert_eq!(rework.2["numerator"].as_i64(), Some(6), "rework numerator = corrected-session tool-calls");
        assert_eq!(rework.2["denominator"].as_i64(), Some(12), "rework denominator = all tool-calls");

        let throughput = daily.iter().find(|r| r.0 == "throughput").expect("throughput daily row present");
        assert!((throughput.1 - 4.0).abs() < 1e-9, "throughput value = 4 sessions that day");

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
        assert_eq!(sess.len(), 4, "one per-session ftr row per session");
        let ones = sess.iter().filter(|(_, v)| (*v - 1.0).abs() < 1e-9).count();
        let zeros = sess.iter().filter(|(_, v)| v.abs() < 1e-9).count();
        assert_eq!(ones, 3, "three first-try sessions score 1.0");
        assert_eq!(zeros, 1, "the corrected session scores 0.0");
        let zero_sid = sess.iter().find(|(_, v)| v.abs() < 1e-9).map(|(id, _)| *id);
        assert_eq!(zero_sid, Some(corrected), "the 0.0 per-session row is the corrected session");

        // ── Idempotency: re-run backfills in place, never duplicates ──────
        let again = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(again, 7, "re-run recomputes the same rows");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 7, "idempotent upsert — still 7 rows after a second run");

        cleanup(pg, &pid, Some(&fid)).await;
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

        cleanup(pg, &pid, None).await;
    }
}
