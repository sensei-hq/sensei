//! Issue-tracker connections, their sprints and tickets (spec
//! `docs/spec/daemon/tracker-jira-linkage.md`, R2/R3/R5). Every SQL statement the
//! tracker integration runs lives here; `crate::tracker` holds the provider
//! logic and calls these methods.

#[cfg(test)]
mod schema_tests {
    use crate::db::pg_store::PgStore;

    async fn project(pg: &PgStore, tag: &str) -> uuid::Uuid {
        pg.create_project(&format!("_test:tracker:{tag}:{}", uuid::Uuid::new_v4()), None, None)
            .await
            .unwrap()
    }

    async fn insert_tracker(
        pg: &PgStore,
        project_id: uuid::Uuid,
        base_url: &str,
        board_ref: &str,
        secret_service: &str,
    ) -> Result<uuid::Uuid, String> {
        sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
            "INSERT INTO sensei.trackers
                 (project_id, kind, base_url, account, external_scope, board_ref, secret_service)
             VALUES ($1, 'jira', $2, 'dev@example.com', 'AUTH', $3, $4)
             RETURNING id",
        )
        .bind(project_id)
        .bind(base_url)
        .bind(board_ref)
        .bind(secret_service)
        .fetch_one(pg.pool())
        .await
        .map(|(id,)| id)
        .map_err(|e| e.to_string())
    }

    async fn tombstone(pg: &PgStore, secret_service: &str) -> Option<(String, bool)> {
        sqlx_core::query_as::query_as::<_, (String, bool)>(
            "SELECT account, not_before <= now()
               FROM sensei.tracker_secret_tombstones WHERE secret_service = $1",
        )
        .bind(secret_service)
        .fetch_optional(pg.pool())
        .await
        .unwrap()
    }

    async fn cleanup(pg: &PgStore, project_id: uuid::Uuid, services: &[&str]) {
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(project_id)
            .execute(pg.pool())
            .await
            .ok();
        for s in services {
            sqlx_core::query::query(
                "DELETE FROM sensei.tracker_secret_tombstones WHERE secret_service = $1",
            )
            .bind(s)
            .execute(pg.pool())
            .await
            .ok();
        }
    }

    fn unique(tag: &str) -> String {
        format!("com.sensei.tracker.jira.test-{tag}-{}", uuid::Uuid::new_v4())
    }

    #[tokio::test]
    async fn a_project_holds_at_most_one_tracker() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let p = project(&pg, "one-per-project").await;
        let (s1, s2) = (unique("a"), unique("b"));
        insert_tracker(&pg, p, "https://a.atlassian.net", "1", &s1).await.unwrap();
        let second = insert_tracker(&pg, p, "https://b.atlassian.net", "2", &s2).await;
        assert!(second.is_err(), "a second tracker on one project must violate unique(project_id)");
        cleanup(&pg, p, &[&s1, &s2]).await;
    }

    #[tokio::test]
    async fn a_board_belongs_to_at_most_one_project() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let (p1, p2) = (project(&pg, "board-a").await, project(&pg, "board-b").await);
        let (s1, s2) = (unique("a"), unique("b"));
        let base = format!("https://t{}.atlassian.net", uuid::Uuid::new_v4().simple());
        insert_tracker(&pg, p1, &base, "42", &s1).await.unwrap();
        let second = insert_tracker(&pg, p2, &base, "42", &s2).await;
        assert!(
            second.is_err(),
            "the same (kind, base_url, board_ref) on two projects must be rejected"
        );
        cleanup(&pg, p1, &[&s1]).await;
        cleanup(&pg, p2, &[&s2]).await;
    }

    #[tokio::test]
    async fn deleting_a_tracker_tombstones_its_secret_immediately() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let p = project(&pg, "delete").await;
        let s = unique("del");
        let t = insert_tracker(&pg, p, "https://d.atlassian.net", "7", &s).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.trackers WHERE id = $1")
            .bind(t)
            .execute(pg.pool())
            .await
            .unwrap();
        assert_eq!(tombstone(&pg, &s).await, Some(("dev@example.com".into(), true)));
        cleanup(&pg, p, &[&s]).await;
    }

    #[tokio::test]
    async fn deleting_the_project_cascades_and_still_tombstones_the_secret() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let p = project(&pg, "cascade").await;
        let s = unique("cascade");
        insert_tracker(&pg, p, "https://c.atlassian.net", "8", &s).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(p)
            .execute(pg.pool())
            .await
            .unwrap();
        assert_eq!(tombstone(&pg, &s).await, Some(("dev@example.com".into(), true)));
        cleanup(&pg, p, &[&s]).await;
    }

    #[tokio::test]
    async fn swapping_the_secret_tombstones_the_old_one_not_the_new_one() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let p = project(&pg, "swap").await;
        let (old, new) = (unique("old"), unique("new"));
        let t = insert_tracker(&pg, p, "https://s.atlassian.net", "9", &old).await.unwrap();
        sqlx_core::query::query("UPDATE sensei.trackers SET secret_service = $2 WHERE id = $1")
            .bind(t)
            .bind(&new)
            .execute(pg.pool())
            .await
            .unwrap();
        assert_eq!(tombstone(&pg, &old).await, Some(("dev@example.com".into(), true)));
        assert_eq!(tombstone(&pg, &new).await, None, "the live secret must never be tombstoned");
        cleanup(&pg, p, &[&old, &new]).await;
    }

    #[tokio::test]
    async fn updating_other_columns_writes_no_tombstone() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let p = project(&pg, "noop").await;
        let s = unique("noop");
        let t = insert_tracker(&pg, p, "https://n.atlassian.net", "10", &s).await.unwrap();
        sqlx_core::query::query("UPDATE sensei.trackers SET last_attempt_at = now() WHERE id = $1")
            .bind(t)
            .execute(pg.pool())
            .await
            .unwrap();
        assert_eq!(tombstone(&pg, &s).await, None);
        cleanup(&pg, p, &[&s]).await;
    }

    #[tokio::test]
    async fn sprints_tasks_and_memberships_are_keyed_and_cascade() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let p = project(&pg, "children").await;
        let s = unique("children");
        let t = insert_tracker(&pg, p, "https://k.atlassian.net", "11", &s).await.unwrap();

        let (sprint,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.sprints (tracker_id, external_id, seq, name, state)
             VALUES ($1, '100', 100, 'Sprint 1', 'active') RETURNING id",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        let dup_sprint = sqlx_core::query::query(
            "INSERT INTO sensei.sprints (tracker_id, external_id, seq, name, state)
             VALUES ($1, '100', 100, 'dup', 'active')",
        )
        .bind(t)
        .execute(pg.pool())
        .await;
        assert!(dup_sprint.is_err(), "sprints are unique by (tracker_id, external_id)");

        let (task,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.tracker_tasks (tracker_id, external_id, key, current_sprint_id)
             VALUES ($1, '10001', 'AUTH-420', $2) RETURNING id",
        )
        .bind(t)
        .bind(sprint)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        let dup_task = sqlx_core::query::query(
            "INSERT INTO sensei.tracker_tasks (tracker_id, external_id, key) VALUES ($1, '10001', 'AUTH-999')",
        )
        .bind(t)
        .execute(pg.pool())
        .await;
        assert!(dup_task.is_err(), "tickets are unique by (tracker_id, external_id)");

        sqlx_core::query::query(
            "INSERT INTO sensei.tracker_task_sprints (task_id, sprint_id) VALUES ($1, $2)",
        )
        .bind(task)
        .bind(sprint)
        .execute(pg.pool())
        .await
        .unwrap();

        // Deleting the sprint drops its memberships and un-sets the ticket's current sprint.
        sqlx_core::query::query("DELETE FROM sensei.sprints WHERE id = $1")
            .bind(sprint)
            .execute(pg.pool())
            .await
            .unwrap();
        let (members, current): (i64, Option<uuid::Uuid>) = sqlx_core::query_as::query_as(
            "SELECT (SELECT count(*) FROM sensei.tracker_task_sprints WHERE task_id = $1),
                    (SELECT current_sprint_id FROM sensei.tracker_tasks WHERE id = $1)",
        )
        .bind(task)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!((members, current), (0, None));

        // Deleting the tracker removes its tickets.
        sqlx_core::query::query("DELETE FROM sensei.trackers WHERE id = $1")
            .bind(t)
            .execute(pg.pool())
            .await
            .unwrap();
        let (tasks,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.tracker_tasks WHERE tracker_id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(tasks, 0);
        cleanup(&pg, p, &[&s]).await;
    }
}
