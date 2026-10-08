//! `churn` group reads — the DB-sourced half of the group.
//!
//! Only `rework_density` reads Postgres. The group's two other metrics
//! (`churn_rate`, `churn_concentration`) are sourced from `git log` against each
//! repository checkout, so they have no statement here. Both reads below are
//! whole-project snapshots rather than windowed series — `rework_density` is a
//! forward-only count of the CURRENT signal against the CURRENT file set — so
//! neither takes `window_days`/`as_of` and neither uses [`super::day_filter`].

use crate::db::pg_store::PgStore;

impl PgStore {
    /// `# project files` — `kind = 'file'` nodes across the project's folders (the
    /// `rework_density` denominator). One scalar count; the former per-folder breakdown
    /// is gone with the per-module rows.
    pub(crate) async fn churn_project_file_count(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<i64, String> {
        let (total,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*)::int8
               FROM sensei.nodes   n
               JOIN sensei.folder_projects fp ON fp.folder_id = n.folder_id
              WHERE fp.project_id = $1
                AND n.kind        = 'file'::sensei.node_kind",
        )
        .bind(project_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(total)
    }

    /// `# rework-flagged files` — `inference.detected_patterns` rows the analyzer writes
    /// as `name = "rework: <file>"` (`is_anti_pattern`). One row per file (the table's
    /// uniqueness is `(project_id, name, is_anti_pattern)`), so a row count IS a
    /// distinct-file count — the `rework_density` numerator.
    pub(crate) async fn churn_rework_flagged_file_count(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<i64, String> {
        let (total,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*)::int8
               FROM inference.detected_patterns
              WHERE project_id      = $1
                AND is_anti_pattern
                AND name LIKE 'rework: %'",
        )
        .bind(project_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(total)
    }
}
