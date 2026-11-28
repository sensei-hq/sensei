//! Relay janitor reads (#444 §10.2): the git checkouts to prune and clean,
//! the worktrees live lanes hold, and action-log compaction.

use super::PgStore;

impl PgStore {
    /// The local checkouts of repositories the relay has worked in (a live or
    /// archived task names their `repo_key`), for worktree pruning, scratch
    /// scavenging and manifest cleans. A repository sensei merely indexed is
    /// never the janitor's to touch (#543).
    pub async fn janitor_git_roots(&self) -> Result<Vec<String>, String> {
        sqlx_core::query_scalar::query_scalar(
            "select f.abs_path from sensei.folders f
               join sensei.repositories r on r.id = f.repository_id
              where f.kind = 'git'
                and r.repo_key in (select repo_key from relay.tasks
                                   union select repo_key from relay_archive.tasks)
              order by f.abs_path",
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

    /// Worktrees the relay recorded for lanes that have since unregistered:
    /// the only worktrees the janitor may remove. Absolute paths only — a
    /// relative one can never match what `git worktree list` prints.
    pub async fn janitor_dead_lane_worktrees(&self) -> Result<Vec<String>, String> {
        sqlx_core::query_scalar::query_scalar(
            "select distinct worktree from relay.lanes
              where worktree like '/%' and unregistered_at is not null",
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| format!("read dead lane worktrees: {e}"))
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

    /// Only a worktree the relay recorded for a lane that has since
    /// unregistered is the janitor's to remove. A live lane's, a relative
    /// path (it can never match what `git worktree list` prints) and a lane
    /// without one never are.
    #[tokio::test]
    async fn dead_lane_worktrees_are_only_absolute_paths_of_unregistered_lanes() {
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
        let tag = uuid::Uuid::now_v7();
        let dead = format!("/tmp/janitor-{tag}/dead");
        let live = format!("/tmp/janitor-{tag}/live");
        let relative = format!("lanes/janitor-{tag}");
        let gone = Some(chrono::Utc::now());
        for (code, worktree, unregistered) in [
            ("D", Some(dead.as_str()), gone),
            ("L", Some(live.as_str()), None),
            ("R", Some(relative.as_str()), gone),
            ("N", None, gone),
        ] {
            sqlx_core::query::query(
                "insert into relay.lanes (user_id, id, crew_id, code, family, token_hash, worktree, unregistered_at)
                 values ($1, $2, $3, $4, 'claude', 'h', $5, $6)",
            )
            .bind(user)
            .bind(uuid::Uuid::now_v7())
            .bind(crew)
            .bind(code)
            .bind(worktree)
            .bind(unregistered)
            .execute(pg.pool())
            .await
            .unwrap();
        }
        let got = pg.janitor_dead_lane_worktrees().await.unwrap();
        assert!(got.contains(&dead), "{got:?}");
        assert!(!got.contains(&live));
        assert!(!got.contains(&relative));
    }

    /// #543: the janitor's roots are the checkouts of repositories the relay
    /// has worked in, never every repository sensei indexed. A checkout of a
    /// repository no relay task names is someone else's, and stays untouched.
    #[tokio::test]
    async fn git_roots_are_only_checkouts_of_repositories_the_relay_worked_in() {
        let pg = PgStore::test_store().await;
        let tag = uuid::Uuid::now_v7().simple().to_string();
        let worked = format!("janitor-worked-{tag}");
        let other = format!("janitor-other-{tag}");
        crate::db::pg_store::tests::create_test_folder(&pg, &worked).await;
        crate::db::pg_store::tests::create_test_folder(&pg, &other).await;
        let (user, crew, task) = (uuid::Uuid::now_v7(), uuid::Uuid::now_v7(), uuid::Uuid::now_v7());
        sqlx_core::query::query(
            "insert into relay.crews (user_id, id, status) values ($1, $2, 'active')",
        )
        .bind(user)
        .bind(crew)
        .execute(pg.pool())
        .await
        .unwrap();
        sqlx_core::query::query(
            "insert into relay.tasks (user_id, id, crew_id, repo_key, title, phase, state)
             values ($1, $2, $3, $4, 'T', 'impl', 'cancelled')",
        )
        .bind(user)
        .bind(task)
        .bind(crew)
        .bind(format!("test/{worked}"))
        .execute(pg.pool())
        .await
        .unwrap();

        let roots = pg.janitor_git_roots().await.unwrap();

        sqlx_core::query::query("delete from relay.tasks where id = $1")
            .bind(task)
            .execute(pg.pool())
            .await
            .unwrap();
        sqlx_core::query::query("delete from relay.crews where id = $1")
            .bind(crew)
            .execute(pg.pool())
            .await
            .unwrap();
        assert!(roots.contains(&format!("/_test/{worked}")), "{roots:?}");
        assert!(
            !roots.contains(&format!("/_test/{other}")),
            "a repository no relay task names is never a janitor root: {roots:?}"
        );
    }

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
