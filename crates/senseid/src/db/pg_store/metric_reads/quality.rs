//! `quality` group reads (#227) — what a qlty scan has already recorded.

use crate::db::pg_store::PgStore;

impl PgStore {
    /// Whether `day` already has a `scope = repo` daily row for EVERY id in
    /// `metric_ids` ON `repository_id`.
    ///
    /// Keyed on the REPOSITORY rather than the project (D2 — a repository is
    /// global, shared by every project that spans it), and restricted to `scope
    /// = 'repo'` + `grain = 'daily'`. The whole-tree twin is always written on a
    /// successful scan, so it is the authoritative coverage signal: a `scope =
    /// user` row can legitimately be absent when git resolves no local author,
    /// and counting it would read an honest-empty as a gap forever.
    ///
    /// Counting DISTINCT `metric_id` against the caller's id set — rather than a
    /// bare row count — is what lets a metric activated AFTER an earlier scan
    /// still read as uncovered and backfill. Empty `metric_ids` → trivially not
    /// covered. Propagates the read error; never masks it.
    pub(crate) async fn quality_repo_day_fully_covered(
        &self,
        metric_ids: &[uuid::Uuid],
        repository_id: &uuid::Uuid,
        day: chrono::NaiveDate,
    ) -> Result<bool, String> {
        if metric_ids.is_empty() {
            return Ok(false);
        }
        let (present,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(DISTINCT metric_id)
           FROM sensei.project_metrics
          WHERE repository_id = $1
            AND metric_id = ANY($2)
            AND scope = 'repo'
            AND grain = 'daily'
            AND computed_on = $3",
        )
        .bind(repository_id)
        .bind(metric_ids)
        .bind(day)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(present as usize == metric_ids.len())
    }
}
