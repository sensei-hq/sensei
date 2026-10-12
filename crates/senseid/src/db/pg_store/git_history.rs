//! Git history persistence — `sensei.commits`, `commit_files`, `commit_scans`.
//!
//! INSERT-ONLY. Nothing here deletes a commit because it stopped being
//! reachable. A first design did, and it was unsound: the facts are
//! repository-grain while a scan runs per CHECKOUT, and 10 repositories on this
//! machine have more than one anchor folder. Two checkouts at different refs
//! would each compute "unreachable" from their own HEAD and delete the other's
//! commits, forever, every reconcile tick. A shallow clone would delete a full
//! clone's history; an empty reachable set would delete all of it.
//!
//! The prune was also redundant. 4,298 of the 6,654 paths ever touched in this
//! repository (65%) no longer exist on disk, so they have no `sensei.files` row
//! and cannot reach a diagram however many commits mention them. EXISTENCE is
//! the filter the read path already applies; reachability never was.
//!
//! So a rewritten history simply means the next walk inserts whatever it finds
//! and `ON CONFLICT DO NOTHING` absorbs the overlap. Two checkouts CONTRIBUTE to
//! one set instead of fighting over it, and the union is a superset of either —
//! which is the honest answer.
//!
//! The one deletion that does happen is RETENTION, by age, which is a policy
//! about storage rather than a claim about what is true.

use super::*;

impl PgStore {
    /// Insert one walk's commits and their file touches. Idempotent.
    ///
    /// `ON CONFLICT DO NOTHING` on both tables is what makes a re-walk free and
    /// a rewritten history harmless — re-reading a commit already stored is a
    /// no-op rather than a conflict to resolve.
    ///
    /// Written in ONE transaction so a commit row and its file rows arrive
    /// together: a `commit_files` row whose parent is missing is a foreign-key
    /// error, and a `commits` row with no touches is indistinguishable from an
    /// empty commit. Half a walk is worse than none.
    ///
    /// Returns `(commits_inserted, touches_inserted)` — the rows this call
    /// actually added, not the rows it was handed, so a caller can tell a walk
    /// that found new history from one that re-read what was already there.
    pub async fn insert_commits(
        &self,
        repository_id: &uuid::Uuid,
        commits: &[crate::indexer::git_history::Commit],
    ) -> Result<(u64, u64), String> {
        if commits.is_empty() {
            return Ok((0, 0));
        }
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        let (mut n_commits, mut n_touches) = (0u64, 0u64);

        for c in commits {
            let res = sqlx_core::query::query(
                "INSERT INTO sensei.commits(repository_id, sha, authored_at, author_email) \
                 VALUES($1, $2, $3, $4) ON CONFLICT DO NOTHING",
            )
            .bind(repository_id)
            .bind(&c.sha)
            .bind(c.authored_at)
            .bind(c.author_email.as_ref().and_then(|e| e.as_str()))
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
            n_commits += res.rows_affected();

            for f in &c.files {
                // The path is written from BYTES, not from a lossy decode. A
                // path that is not UTF-8 is still a real path, and turning its
                // bytes into U+FFFD would store a different file's name.
                let Some(path) = f.path.as_str() else {
                    tracing::warn!(
                        sha = %c.sha,
                        "insert_commits: non-utf8 path skipped — Postgres text cannot hold it"
                    );
                    continue;
                };
                let res = sqlx_core::query::query(
                    "INSERT INTO sensei.commit_files(repository_id, sha, path, lines_changed) \
                     VALUES($1, $2, $3, $4) ON CONFLICT DO NOTHING",
                )
                .bind(repository_id)
                .bind(&c.sha)
                .bind(path)
                .bind(f.lines_changed)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
                n_touches += res.rows_affected();
            }
        }

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok((n_commits, n_touches))
    }

    /// The tips this CHECKOUT's last walk covered, or `None` if never scanned.
    ///
    /// Keyed on the folder, not the repository: the cursor says "where did I
    /// stop", and two checkouts of one repository stop in different places. The
    /// facts they produce are shared; their positions are not.
    pub async fn get_commit_scan_tips(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Option<Vec<String>>, String> {
        let row: Option<(Vec<String>,)> = sqlx_core::query_as::query_as(
            "SELECT tips FROM sensei.commit_scans WHERE folder_id = $1",
        )
        .bind(folder_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(t,)| t))
    }

    /// Record where this checkout's walk stopped. Idempotent per folder.
    pub async fn upsert_commit_scan(
        &self,
        folder_id: &uuid::Uuid,
        repository_id: &uuid::Uuid,
        tips: &[String],
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.commit_scans(folder_id, repository_id, tips, scanned_at) \
             VALUES($1, $2, $3, now()) \
             ON CONFLICT(folder_id) DO UPDATE SET \
               repository_id = EXCLUDED.repository_id, \
               tips = EXCLUDED.tips, \
               scanned_at = now()",
        )
        .bind(folder_id)
        .bind(repository_id)
        .bind(tips)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Drop commits older than `retention_days`. Returns rows removed.
    ///
    /// THE ONLY DELETION. It is a storage policy, not a claim that the history
    /// did not happen — which is why it is by AGE and never by reachability.
    ///
    /// The caller is responsible for the floor `retention_days >= max window`;
    /// this refuses a non-positive value rather than interpreting it, because
    /// "retain for 0 days" and "retain nothing" are the same instruction and
    /// neither is one a caller should be able to give by accident.
    ///
    /// `commit_files` follows by `ON DELETE CASCADE`, so the two tables cannot
    /// drift into a state where a touch outlives its commit.
    pub async fn prune_commits_older_than(&self, retention_days: i32) -> Result<u64, String> {
        if retention_days <= 0 {
            return Err(format!(
                "prune_commits_older_than: retention_days must be positive, got {retention_days}"
            ));
        }
        let res = sqlx_core::query::query(
            "DELETE FROM sensei.commits \
              WHERE authored_at < now() - make_interval(days => $1)",
        )
        .bind(retention_days)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(res.rows_affected())
    }
}
