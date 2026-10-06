//! `cost` group reads — what the window actually delivered.

use crate::db::pg_store::PgStore;

/// Merged runs and accepted recommendations over the window, in that order.
pub(crate) type DeliveredResults = (i64, i64);

impl PgStore {
    /// Results delivered for `project_id` within the trailing window: merged runs +
    /// accepted recommendations. Counted separately so the props can show which
    /// contributed — a cost that moved because recommendations dried up is a
    /// different story from one that moved because runs stopped merging.
    ///
    /// ## What counts as a result
    ///
    /// Merged runs + accepted recommendations. Both are things the user shipped or
    /// adopted — a completed *session* is activity, not delivery, and counting it
    /// would make the metric fall simply because someone worked more.
    ///
    /// Each half keys on its COMPLETION time (`completed_at`, `acted_at`), not on
    /// when the work began: a run that started inside the window and never finished
    /// delivered nothing. The window itself is a trailing interval anchored to
    /// `now()` rather than a [`super::day_filter`] bucket — there is no per-day
    /// grain to filter here, so `$1` is the only bind and `window_days` sizes the
    /// interval.
    pub(crate) async fn cost_results_in_window(
        &self,
        project_id: &uuid::Uuid,
        window_days: u32,
    ) -> Result<DeliveredResults, String> {
        let sql = format!(
            "SELECT (SELECT count(*) FROM activity.runs \
                      WHERE project_id = $1 AND status = 'done' \
                        AND completed_at > now() - interval '{window_days} days')::int8, \
                    (SELECT count(*) FROM inference.recommendations \
                      WHERE project_id = $1 AND status = 'accepted' \
                        AND acted_at > now() - interval '{window_days} days')::int8"
        );
        sqlx_core::query_as::query_as::<_, DeliveredResults>(&sql)
            .bind(project_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| e.to_string())
    }
}
