//! `usage` group reads — how efficiently the work reuses context.

use super::{bind_day, day_filter};
use crate::db::pg_store::PgStore;

/// One day's cache reuse for one repository: the mean of per-session ratios, the
/// pooled ratio for comparison, and the session count behind them.
pub(crate) type CacheReuseDay = (chrono::NaiveDate, uuid::Uuid, f64, f64, i64);

impl PgStore {
    /// Per-(day, repository) cache reuse over the window.
    ///
    /// Only turns with real token accounting participate
    /// (`tokens_in IS NOT NULL`): the Zed and OpenCode adapters do not collect
    /// it yet, and treating an absent reading as zero would fabricate a cache
    /// miss that never happened.
    ///
    /// The MEAN OF PER-SESSION RATIOS is the headline, with the pooled ratio
    /// carried beside it. Pooling is dominated by the longest session of the
    /// day, which hides the signal the metric exists for — a handful of
    /// efficient marathons masking a dozen wasteful restarts. Both are returned
    /// so the computer can write both and the reading stays transparent.
    pub(crate) async fn usage_cache_reuse_by_day(
        &self,
        project_id: &uuid::Uuid,
        window_days: u32,
        as_of: Option<chrono::NaiveDate>,
    ) -> Result<Vec<CacheReuseDay>, String> {
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
            day_filter("s.started_at", as_of),
        );
        let q = sqlx_core::query_as::query_as::<_, CacheReuseDay>(&sql).bind(project_id);
        bind_day(q, window_days, as_of).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }
}
