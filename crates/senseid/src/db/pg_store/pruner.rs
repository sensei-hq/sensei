//! The pruner (#247): remove a repository root and everything that belongs to
//! it, and nothing that does not.
//!
//! Before this, three code paths each removed PART of a repository and left the
//! rest: the exclusion handler deleted folders, `exclude_project` deleted nodes
//! and one folder row, and deleting a watch root cascaded folders. None removed
//! the `repositories` rows those folders pointed at. Cutting the production DB
//! to one repository on 2026-10-07 left 13,504 of them behind.
//!
//! ## What "everything" means
//!
//! 1. Every folder at or under the path, and through the folder cascades its
//!    files, nodes, edges, commit scans and the rest.
//! 2. The repositories those folders pointed at, UNLESS another folder still
//!    points at one. A second checkout of the same repository keeps it, and its
//!    commits, metrics and project membership with it.
//! 3. Projects left empty, through `prune_empty_projects_among` — which owns the
//!    rule for what an empty project is, history included — SCOPED to the
//!    projects those repositories belonged to. Never a global sweep.
//!
//! Steps 1 and 2 are one transaction, so a failure cannot leave folders gone
//! and their repositories behind. A failure is an `Err`, never a count of zero.

use super::*;

/// What one prune removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct PruneReport {
    pub folders: u64,
    pub repositories: u64,
    pub projects: u64,
}

impl PgStore {
    /// Remove the repository rooted at `path` (absolute; a trailing slash is
    /// ignored) and every association. See the module docs.
    ///
    /// `starts_with(abs_path, path || '/')`, never a bare prefix: `/x/a` must not
    /// take `/x/ab` with it.
    pub async fn prune_repository_root(&self, path: &str) -> Result<PruneReport, String> {
        let p = path.trim_end_matches('/');
        let mut tx = self.pool.begin().await.map_err(|e| format!("prune: begin: {e}"))?;

        let (folders, repositories): (i64, Vec<uuid::Uuid>) = sqlx_core::query_as::query_as(
            "WITH gone AS (
                 DELETE FROM sensei.folders f
                  WHERE f.abs_path = $1 OR starts_with(f.abs_path, $1 || '/')
              RETURNING f.repository_id
             )
             SELECT count(*)::bigint
                  , coalesce(array_agg(DISTINCT repository_id)
                               FILTER (WHERE repository_id IS NOT NULL), '{}')
               FROM gone",
        )
        .bind(p)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("prune: folders under {p}: {e}"))?;

        // The projects these repositories belonged to, read BEFORE the delete
        // cascades the membership away — they are the only projects this prune
        // can have emptied, and the only ones it may remove.
        let (projects_touched,): (Vec<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT coalesce(array_agg(DISTINCT project_id), '{}')
               FROM sensei.repositories_in_projects WHERE repository_id = ANY($1)",
        )
        .bind(&repositories)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("prune: projects under {p}: {e}"))?;

        let repos = sqlx_core::query::query(
            "DELETE FROM sensei.repositories r
              WHERE r.id = ANY($1)
                AND NOT EXISTS (SELECT 1 FROM sensei.folders f WHERE f.repository_id = r.id)",
        )
        .bind(&repositories)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("prune: repositories under {p}: {e}"))?;

        tx.commit().await.map_err(|e| format!("prune: commit: {e}"))?;

        // After the commit, and deliberately outside it: `prune_empty_projects`
        // owns its own rule and its own statement, and a project it keeps is not
        // a reason to restore the repository just removed.
        let projects = self.prune_empty_projects_among(0, Some(&projects_touched)).await?;

        Ok(PruneReport { folders: folders as u64, repositories: repos.rows_affected(), projects })
    }

    /// The repository roots under a watch root — what stops syncing if it goes.
    /// `(name, abs_path, kind)`, git and standalone only: a nested `folder` is
    /// part of its repository, not a thing to ask about separately.
    pub async fn repositories_under_root(
        &self,
        root_id: &uuid::Uuid,
    ) -> Result<Vec<(String, String, String)>, String> {
        sqlx_core::query_as::query_as(
            "SELECT name, abs_path, kind::text FROM sensei.folders
              WHERE root_id = $1 AND kind IN ('git'::sensei.folder_kind, 'standalone'::sensei.folder_kind)
              ORDER BY abs_path",
        )
        .bind(root_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("repositories_under_root: {e}"))
    }
}
