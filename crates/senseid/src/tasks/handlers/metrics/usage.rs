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
use crate::db::pg_store::MetricRow;

const GRAIN_DAILY: &str = "daily";
const SOURCE_MEASURED: &str = "measured";
const SCOPE_USER: &str = "user";
const KEY_CACHE_REUSE: &str = "cache_reuse";

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
    let rows = pg.usage_cache_reuse_by_day(&project_id, window_days, as_of).await?;

    let mut written = 0u32;
    for (day, repository_id, mean_ratio, pooled_ratio, sessions) in rows {
        let props = serde_json::json!({
            "sessions": sessions,
            "pooled_ratio": pooled_ratio,
            "mean_of_session_ratios": mean_ratio,
        });
        pg.upsert_project_metric_repo(&MetricRow {
            metric_id: &mid,
            repository_id: &repository_id,
            scope: SCOPE_USER,
            identity: None,
            commit_sha: None,
            computed_on: day,
            grain: GRAIN_DAILY,
            value: mean_ratio,
            props: &props,
            source: SOURCE_MEASURED,
        })
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
