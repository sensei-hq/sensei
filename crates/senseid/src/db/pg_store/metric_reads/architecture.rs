//! `architecture` group reads — the shape of the code, read off the graph.
//!
//! ## Why every query carries its own guard
//!
//! A repository with no edges must produce NO ROW rather than a zero, because a
//! zero would read as "nothing resolves" when the truth is "nothing was
//! indexed". That is the difference between a measurement and a fabrication, and
//! it is why each query below ends in a `HAVING count(*) > 0` rather than
//! wrapping its ratio in a `coalesce(…, 0)`.
//!
//! ## Not windowed
//!
//! These three read the graph as it stands, not a span of days, so none of them
//! takes `window_days`/`as_of` or calls [`super::day_filter`]. The graph has no
//! per-edge occurrence time to anchor a window on; the computer stamps the row
//! with the day it ran.

use crate::db::pg_store::PgStore;

/// `(repository_id, placed, total)` — the counts, not the ratio, so the writer
/// can put both in props and a reader can see the denominator it was divided by.
pub(crate) type ConfidenceRow = (uuid::Uuid, i64, i64);
/// `(repository_id, exported, total)`.
pub(crate) type SurfaceRow = (uuid::Uuid, i64, i64);
/// `(repository_id, p95_lines, over_100, callables)`.
pub(crate) type SizeRow = (uuid::Uuid, f64, i64, i64);

impl PgStore {
    /// Placed vs total edges per repository.
    ///
    /// Keyed through `folders.repository_id` because edges carry `folder_id`, and a
    /// folder with no repository is excluded rather than pooled under a NULL key —
    /// an unattributed edge belongs to no repository's score.
    pub(crate) async fn architecture_edge_confidence_by_repo(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<ConfidenceRow>, String> {
        sqlx_core::query_as::query_as::<_, ConfidenceRow>(
            "SELECT f.repository_id \
                  , count(*) FILTER (WHERE e.target_id IS NOT NULL)::int8 \
                  , count(*)::int8 \
               FROM sensei.edges e \
               JOIN sensei.folders f ON f.id = e.folder_id \
               JOIN sensei.folder_projects fp ON fp.folder_id = e.folder_id \
              WHERE fp.project_id = $1 AND f.repository_id IS NOT NULL \
              GROUP BY 1 \
             HAVING count(*) > 0",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    /// Exported vs total DECLARED symbols per repository.
    ///
    /// `file_id IS NOT NULL` restricts this to declarations. Reference stubs and
    /// external `lib·` nodes have no file by definition, and counting them would put
    /// every symbol the repository merely MENTIONS into the denominator of a ratio
    /// about what it OWNS.
    pub(crate) async fn architecture_public_surface_by_repo(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<SurfaceRow>, String> {
        sqlx_core::query_as::query_as::<_, SurfaceRow>(
            "SELECT f.repository_id \
                  , count(*) FILTER (WHERE n.is_exported)::int8 \
                  , count(*)::int8 \
               FROM sensei.nodes n \
               JOIN sensei.folders f ON f.id = n.folder_id \
               JOIN sensei.folder_projects fp ON fp.folder_id = n.folder_id \
              WHERE fp.project_id = $1 AND f.repository_id IS NOT NULL \
                AND n.file_id IS NOT NULL \
                AND NOT n.is_test \
              GROUP BY 1 \
             HAVING count(*) > 0",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    /// p95 line span of callables, with the over-100 count beside it.
    ///
    /// Only rows with BOTH span bounds participate. A callable whose span was never
    /// captured is excluded, not treated as zero — a zero-length function would drag
    /// the percentile down and invent an improvement.
    pub(crate) async fn architecture_symbol_size_by_repo(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<SizeRow>, String> {
        sqlx_core::query_as::query_as::<_, SizeRow>(
            "SELECT f.repository_id \
                  , percentile_cont(0.95) WITHIN GROUP (ORDER BY (n.line_end - n.line_start + 1))::float8 \
                  , count(*) FILTER (WHERE n.line_end - n.line_start + 1 > 100)::int8 \
                  , count(*)::int8 \
               FROM sensei.nodes n \
               JOIN sensei.folders f ON f.id = n.folder_id \
               JOIN sensei.folder_projects fp ON fp.folder_id = n.folder_id \
              WHERE fp.project_id = $1 AND f.repository_id IS NOT NULL \
                AND n.kind IN ('function','method') \
                AND n.line_start IS NOT NULL AND n.line_end IS NOT NULL \
                AND n.line_end >= n.line_start \
                AND NOT n.is_test \
              GROUP BY 1 \
             HAVING count(*) > 0",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }
}
