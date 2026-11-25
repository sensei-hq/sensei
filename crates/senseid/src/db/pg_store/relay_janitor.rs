//! Relay janitor reads (#444 §10.2): the git checkouts to prune and clean,
//! the worktrees live lanes hold, and action-log compaction.

use super::PgStore;

impl PgStore {
    /// Every git checkout's local path, for worktree pruning and manifest
    /// cleans. Oldest first — hygiene starts where rot is likeliest.
    pub async fn janitor_git_roots(&self) -> Result<Vec<String>, String> {
        sqlx_core::query_scalar::query_scalar(
            "select abs_path from sensei.folders where kind = 'git' order by abs_path",
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| format!("read git checkouts: {e}"))
    }

    /// Worktrees live lanes hold (registered, with a path): anything else a
    /// repo lists is pruneable.
    pub async fn janitor_live_worktrees(&self) -> Result<Vec<String>, String> {
        sqlx_core::query_scalar::query_scalar(
            "select worktree from relay.lanes where worktree is not null and unregistered_at is null",
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| format!("read live worktrees: {e}"))
    }

    /// Compact action logs (#444 §10.2.4): null the raw stdout/stderr
    /// attachments (`body_md`, `report_md`) on actions of tasks completed
    /// more than 7 days ago, live and archived alike. Structured metadata
    /// (kinds, rounds, props) stays. Returns actions compacted.
    pub async fn janitor_compact_action_logs(
        &self,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, String> {
        let mut compacted = 0u64;
        for schema in ["relay", "relay_archive"] {
            let done = sqlx_core::query::query(&format!(
                "update {schema}.actions a set body_md = null, report_md = null
                  where (a.body_md is not null or a.report_md is not null)
                    and exists (
                      select 1 from {schema}.tasks t
                       where t.user_id = a.user_id and t.id = a.task_id
                         and t.completed_at < $1)"
            ))
            .bind(now - chrono::Duration::days(7))
            .execute(self.pool())
            .await
            .map_err(|e| format!("compact action logs: {e}"))?;
            compacted += done.rows_affected();
        }
        Ok(compacted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn compacting_nulls_old_attachments_and_keeps_metadata() {
        let pg = PgStore::test_store().await;
        let user = uuid::Uuid::now_v7();
        let crew = uuid::Uuid::now_v7();
        sqlx_core::query::query(
            "insert into relay.crews (user_id, id, status) values ($1, $2, 'active')",
        )
        .bind(user)
        .bind(crew)
        .execute(pg.pool())
        .await
        .unwrap();
        let old = chrono::Utc::now() - chrono::Duration::days(8);
        let fresh = chrono::Utc::now() - chrono::Duration::days(1);
        let mut mine = Vec::new();
        for (tag, completed) in [("old", old), ("fresh", fresh)] {
            let (task, action) = (uuid::Uuid::now_v7(), uuid::Uuid::now_v7());
            sqlx_core::query::query(
                "insert into relay.tasks (user_id, id, crew_id, repo_key, title, phase, state, completed_at)
                 values ($1, $2, $3, 'github.com/o/r', 'T', 'impl', 'cancelled', $4)",
            )
            .bind(user)
            .bind(task)
            .bind(crew)
            .bind(completed)
            .execute(pg.pool())
            .await
            .unwrap();
            sqlx_core::query::query(
                "insert into relay.actions (user_id, id, crew_id, task_id, kind, body_md, report_md, props)
                 values ($1, $2, $3, $4, 'reported', $5, 'rep', '{}')",
            )
            .bind(user)
            .bind(action)
            .bind(crew)
            .bind(task)
            .bind(format!("body-{tag}"))
            .execute(pg.pool())
            .await
            .unwrap();
            mine.push((task, action));
        }

        let n = pg.janitor_compact_action_logs(chrono::Utc::now()).await.unwrap();
        assert!(n >= 1, "at least our old action compacts");

        for (i, (task, action)) in mine.iter().enumerate() {
            let row: (Option<String>, Option<String>, serde_json::Value) =
                sqlx_core::query_as::query_as(
                    "select body_md, report_md, props from relay.actions where id = $1",
                )
                .bind(action)
                .fetch_one(pg.pool())
                .await
                .unwrap();
            if i == 0 {
                assert_eq!(
                    row,
                    (None, None, serde_json::json!({})),
                    "old: attachments nulled, metadata kept"
                );
            } else {
                assert_eq!(row.0.as_deref(), Some("body-fresh"), "fresh: untouched");
            }
            sqlx_core::query::query("delete from relay.actions where id = $1")
                .bind(action)
                .execute(pg.pool())
                .await
                .unwrap();
            sqlx_core::query::query("delete from relay.tasks where id = $1")
                .bind(task)
                .execute(pg.pool())
                .await
                .unwrap();
        }
        sqlx_core::query::query("delete from relay.crews where id = $1")
            .bind(crew)
            .execute(pg.pool())
            .await
            .unwrap();
    }
}
