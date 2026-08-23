//! `usage` metric group — how efficiently the work reuses context.
//!
//! ## `cache_reuse`: the share of input served from cache
//!
//! Every request re-sends the conversation prefix. Cached prefix tokens bill far
//! cheaper than fresh ones, so the share served from cache is a real efficiency
//! signal — but ONLY at the right grain, which took measuring to find.
//!
//! Per REQUEST it is a flat line: 99.8% median, 98.6% at p10 across 6,065 real
//! requests, because Claude Code caches the prefix on essentially every call. A
//! metric that reads 99.8% every day says nothing.
//!
//! Per SESSION it has genuine spread, and the driver is session LENGTH:
//!
//! ```text
//!   10+ turns   37 sessions   95.7% – 98.7%
//!   4-10 turns  28 sessions   88.8% – 99.2%
//!   2-3 turns    6 sessions   65.6% – 96.7%   (avg 82.7%)
//!   1 turn       3 sessions   92.7% – 93.9%
//! ```
//!
//! A short session never amortises its cold start, so it re-pays for context it
//! could have reused. That is the finding this metric exists to surface.
//!
//! ## Mean of per-session ratios, not a pooled total
//!
//! Deliberate, and it is the whole design. Pooling (Σcache_read / Σtotal) is
//! dominated by the longest session of the day, which HIDES the signal — a
//! handful of efficient marathons would mask a dozen wasteful restarts. Averaging
//! per-session ratios weights each session equally, so short sessions actually
//! move the number. Both are written to props so the reading stays transparent.

use crate::db::pg_store::PgStore;
use crate::tasks::executor::TaskContext;

use super::MetricGroup;

const GRAIN_DAILY: &str = "daily";
const SOURCE_MEASURED: &str = "measured";
const SCOPE_USER: &str = "user";
const KEY_CACHE_REUSE: &str = "cache_reuse";

/// One day's cache reuse for one repository: the mean of per-session ratios, the
/// pooled ratio for comparison, and the session count behind them.
type DayRow = (chrono::NaiveDate, uuid::Uuid, f64, f64, i64);

/// Per-(day, repository) cache reuse over the window.
///
/// Only turns with real token accounting participate (`tokens_in IS NOT NULL`):
/// the Zed and OpenCode adapters do not yet collect it, and treating an absent
/// reading as zero would fabricate a cache miss that never happened.
async fn daily_cache_reuse(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
    as_of: Option<chrono::NaiveDate>,
) -> Result<Vec<DayRow>, String> {
    let sql = format!(
        "WITH per_session AS ( \
             SELECT date_trunc('day', s.started_at)::date AS day \
                  , rf.repository_id \
                  , sum(tt.cache_read)::numeric AS cr \
                  , sum(tt.tokens_in + tt.cache_write + tt.cache_read)::numeric AS tot \
               FROM activity.transcript_turns tt \
               JOIN activity.sessions s  ON s.client_session_id = tt.session_id \
               JOIN sensei.folders    rf ON rf.id = s.repo_folder_id \
              WHERE s.project_id = $1 \
                AND rf.repository_id IS NOT NULL \
                AND tt.tokens_in IS NOT NULL \
                AND {} \
              GROUP BY 1, 2 \
             HAVING sum(tt.tokens_in + tt.cache_write + tt.cache_read) > 0 \
         ) \
         SELECT day, repository_id \
              , avg(cr / tot)::float8 \
              , (sum(cr) / sum(tot))::float8 \
              , count(*)::int8 \
           FROM per_session GROUP BY 1, 2 ORDER BY 1, 2",
        super::day_filter("s.started_at", as_of),
    );
    let q = sqlx_core::query_as::query_as::<_, DayRow>(&sql).bind(project_id);
    super::bind_day(q, window_days, as_of)
        .fetch_all(pg.pool())
        .await
        .map_err(|e| e.to_string())
}

pub(super) async fn compute(
    ctx: &TaskContext,
    project_raw: &str,
    as_of: Option<chrono::NaiveDate>,
) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(project_raw)
        .map_err(|e| format!("usage: bad project id {project_raw:?}: {e}"))?;
    let pg = ctx.pg();

    let ids = pg.active_metric_ids(MetricGroup::Usage.as_str()).await?;
    let Some(mid) = ids.get(KEY_CACHE_REUSE).copied() else {
        return Ok(0);
    };

    let window_days = crate::tasks::metrics_scheduler::window_days(pg).await;
    let rows = daily_cache_reuse(pg, &project_id, window_days, as_of).await?;

    let mut written = 0u32;
    for (day, repository_id, mean_ratio, pooled_ratio, sessions) in rows {
        let props = serde_json::json!({
            "sessions": sessions,
            "pooled_ratio": pooled_ratio,
            "mean_of_session_ratios": mean_ratio,
        });
        pg.upsert_project_metric_repo(
            &mid,
            &project_id,
            Some(&repository_id),
            SCOPE_USER,
            None,
            None,
            None,
            None,
            day,
            GRAIN_DAILY,
            mean_ratio,
            &props,
            SOURCE_MEASURED,
        )
        .await?;
        written += 1;
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_bad_project_id_is_an_error_not_a_silent_zero() {
        let ctx = crate::tasks::test_support::make_ctx().await;
        assert!(compute(&ctx, "not-a-uuid", None).await.is_err());
    }

    #[tokio::test]
    async fn a_project_with_no_token_accounted_turns_writes_no_row() {
        // Honest-empty: Zed/OpenCode turns carry no token split yet, and a project
        // with only those must produce NO row rather than a fabricated 0.0 — which
        // would read as "every request missed cache".
        let ctx = crate::tasks::test_support::make_ctx().await;
        let pid = uuid::Uuid::new_v4();
        assert_eq!(compute(&ctx, &pid.to_string(), None).await.unwrap(), 0);
    }
}
