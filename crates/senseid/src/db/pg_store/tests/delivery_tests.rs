//! The reads and marks behind a board's delivery push (tracker-delivery-metrics D3; #352).
//!
//! What must never happen here: a removed ticket travelling; a ticket's sprints read
//! as its current sprint only; a removed or update-less ticket counted as pending
//! for ever; a consistency token that does not move when the history task writes; a
//! mark that loses its refusal or keeps a stale one.
use super::pg_store;
use crate::db::pg_store::PgStore;
use crate::db::pg_store::delivery::PartKey;

async fn exec(pg: &PgStore, sql: &str, t: uuid::Uuid) {
    sqlx_core::query::query(sql)
        .bind(t)
        .execute(pg.pool())
        .await
        .unwrap_or_else(|e| panic!("{sql}: {e}"));
}

/// The S5 board (sprints 100, 101 closed, 102 future; tickets A-D, B in 100 and 101),
/// connected and synced, with sprint 99 removed and ticket E removed.
async fn board(pg: &PgStore) -> uuid::Uuid {
    let t = crate::sdlc::sprint_replay::store_tests::seeded(pg).await;
    for sql in [
        "UPDATE sensei.trackers SET last_success_at = now(), params = '{\"board_name\": \"AUTH Scrum\"}' WHERE id = $1",
        "INSERT INTO sensei.sprints (tracker_id, external_id, seq, name, state, removed_at)
         VALUES ($1, '99', 99, 'S99', 'closed', now())",
        "INSERT INTO sensei.tracker_tasks (tracker_id, external_id, key, summary, removed_at)
         VALUES ($1, '5', 'E', 'gone', now())",
        "UPDATE sensei.tracker_tasks SET summary = 'Login' WHERE tracker_id = $1 AND key = 'A'",
    ] {
        exec(pg, sql, t).await;
    }
    t
}

#[tokio::test]
async fn a_synced_sprint_board_is_listed_with_its_canonical_connection() {
    let s = pg_store().await;
    let t = board(&s).await;
    let mine: Vec<_> =
        s.delivery_trackers().await.unwrap().into_iter().filter(|d| d.id == t).collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].kind, "jira");
    assert_eq!(mine[0].board_ref, "42");
    assert!(mine[0].base_url.starts_with("https://"));
    assert_eq!(mine[0].board_name.as_deref(), Some("AUTH Scrum"));

    // Never synced, or not a sprint board: not listed.
    exec(&s, "UPDATE sensei.trackers SET last_success_at = NULL WHERE id = $1", t).await;
    assert!(!s.delivery_trackers().await.unwrap().iter().any(|d| d.id == t));
    exec(&s, "UPDATE sensei.trackers SET last_success_at = now(), level = 'epic' WHERE id = $1", t)
        .await;
    assert!(!s.delivery_trackers().await.unwrap().iter().any(|d| d.id == t));
}

#[tokio::test]
async fn sprints_include_the_removed_and_tickets_exclude_them_with_their_membership() {
    let s = pg_store().await;
    let t = board(&s).await;
    let sprints = s.delivery_sprints(t).await.unwrap();
    let ids: Vec<(&str, bool)> =
        sprints.iter().map(|x| (x.external_id.as_str(), x.removed)).collect();
    assert_eq!(ids, [("99", true), ("100", false), ("101", false), ("102", false)]);

    let tickets = s.delivery_tickets(t).await.unwrap();
    let keys: Vec<&str> = tickets.iter().map(|x| x.key.as_str()).collect();
    assert_eq!(keys, ["A", "B", "C", "D"], "the removed ticket E does not travel");
    let b = tickets.iter().find(|x| x.key == "B").unwrap();
    assert_eq!(b.sprints, ["100", "101"], "every sprint that lists B, not only its current one");
    let a = tickets.iter().find(|x| x.key == "A").unwrap();
    assert_eq!(a.title.as_deref(), Some("Login"));
    assert_eq!(a.points, Some(3.0));
    assert_eq!(tickets.iter().find(|x| x.key == "B").unwrap().title, None, "no summary stays None");

    // A ticket only in a removed sprint lists none.
    exec(&s, "UPDATE sensei.sprints SET removed_at = now() WHERE tracker_id = $1 AND external_id = '100'", t).await;
    let a = s.delivery_tickets(t).await.unwrap().into_iter().find(|x| x.key == "A").unwrap();
    assert!(a.sprints.is_empty());
}

#[tokio::test]
async fn pending_counts_what_the_history_task_would_still_read() {
    let s = pg_store().await;
    let t = board(&s).await;
    // Everything read; the removed ticket E has no history and is not pending.
    assert_eq!(s.delivery_token(t).await.unwrap().pending, 0);
    // A ticket with no `updated` from the tracker is never read, so never pending.
    exec(
        &s,
        "INSERT INTO sensei.tracker_tasks (tracker_id, external_id, key) VALUES ($1, '6', 'F')",
        t,
    )
    .await;
    assert_eq!(s.delivery_token(t).await.unwrap().pending, 0);
    // An update past its history is.
    exec(&s, "UPDATE sensei.tracker_tasks SET issue_updated_at = '2026-10-05Z' WHERE tracker_id = $1 AND key = 'C'", t).await;
    let token = s.delivery_token(t).await.unwrap();
    assert_eq!(token.pending, 1);
    assert_eq!(
        token.pending as usize,
        s.history_candidates(t, 1000).await.unwrap().len(),
        "one predicate, two readers"
    );
}

#[tokio::test]
async fn the_token_moves_when_the_history_task_or_a_sync_writes() {
    let s = pg_store().await;
    let t = board(&s).await;
    let before = s.delivery_token(t).await.unwrap();
    assert_eq!(before, s.delivery_token(t).await.unwrap());
    exec(&s, "UPDATE sensei.tracker_tasks SET history_through = '2026-10-02Z' WHERE tracker_id = $1 AND key = 'A'", t).await;
    let after_history = s.delivery_token(t).await.unwrap();
    assert_ne!(before, after_history);
    exec(&s, "UPDATE sensei.trackers SET last_success_at = last_success_at + interval '1 minute' WHERE id = $1", t).await;
    assert_ne!(after_history, s.delivery_token(t).await.unwrap());
}

#[tokio::test]
async fn a_workspace_lists_its_repositories_keys() {
    let s = pg_store().await;
    let t = board(&s).await;
    let (w,): (uuid::Uuid,) =
        sqlx_core::query_as::query_as("SELECT workspace_id FROM sensei.trackers WHERE id = $1")
            .bind(t)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(s.workspace_repo_keys(w).await.unwrap().is_empty());
    let key = format!("github.com/ztest/{}", uuid::Uuid::new_v4().simple());
    let (r,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.repositories (name, repo_key) VALUES ('r', $1) RETURNING id",
    )
    .bind(&key)
    .fetch_one(s.pool())
    .await
    .unwrap();
    sqlx_core::query::query("INSERT INTO sensei.repositories_in_workspaces (workspace_id, repository_id) VALUES ($1, $2)")
        .bind(w)
        .bind(r)
        .execute(s.pool())
        .await
        .unwrap();
    // A local-only repository (no repo_key) cannot be shared and is not listed.
    let (local,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.repositories (name) VALUES ('local') RETURNING id",
    )
    .fetch_one(s.pool())
    .await
    .unwrap();
    sqlx_core::query::query("INSERT INTO sensei.repositories_in_workspaces (workspace_id, repository_id) VALUES ($1, $2)")
        .bind(w)
        .bind(local)
        .execute(s.pool())
        .await
        .unwrap();
    assert_eq!(s.workspace_repo_keys(w).await.unwrap(), [key]);
}

#[tokio::test]
async fn a_mark_records_attempts_acknowledgements_and_refusals() {
    use crate::dojo::delivery::MarkState;
    let s = pg_store().await;
    let t = board(&s).await;
    let (p,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.personas (label) VALUES ('ztest-delivery') RETURNING id",
    )
    .fetch_one(s.pool())
    .await
    .unwrap();
    let tenant = uuid::Uuid::new_v4();

    assert_eq!(
        s.push_mark(&PartKey { tracker_id: t, persona_id: p, tenant_id: tenant, part: "facts" })
            .await
            .unwrap(),
        None
    );
    s.mark_push_attempt(&PartKey {
        tracker_id: t,
        persona_id: p,
        tenant_id: tenant,
        part: "facts",
    })
    .await
    .unwrap();
    assert_eq!(
        s.push_mark(&PartKey { tracker_id: t, persona_id: p, tenant_id: tenant, part: "facts" })
            .await
            .unwrap(),
        Some(MarkState::default()),
        "attempted, nothing acknowledged"
    );

    s.mark_push_refused(
        &PartKey { tracker_id: t, persona_id: p, tenant_id: tenant, part: "facts" },
        "h1",
        "dōjō returned 400",
    )
    .await
    .unwrap();
    let refused = s
        .push_mark(&PartKey { tracker_id: t, persona_id: p, tenant_id: tenant, part: "facts" })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(refused.failed_hash.as_deref(), Some("h1"));
    assert!(refused.failed_at.is_some());
    assert_eq!(refused.hash, None);

    s.mark_push_acknowledged(
        &PartKey { tracker_id: t, persona_id: p, tenant_id: tenant, part: "facts" },
        "h2",
    )
    .await
    .unwrap();
    let acked = s
        .push_mark(&PartKey { tracker_id: t, persona_id: p, tenant_id: tenant, part: "facts" })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(acked.hash.as_deref(), Some("h2"));
    assert!(acked.pushed_at.is_some());
    assert_eq!(
        (acked.failed_hash, acked.failed_at),
        (None, None),
        "an acknowledgement clears the refusal"
    );

    // Another tenant or part is another mark; the persona's marks go with it.
    assert_eq!(
        s.push_mark(&PartKey {
            tracker_id: t,
            persona_id: p,
            tenant_id: uuid::Uuid::new_v4(),
            part: "facts"
        })
        .await
        .unwrap(),
        None
    );
    assert_eq!(
        s.push_mark(&PartKey { tracker_id: t, persona_id: p, tenant_id: tenant, part: "effort" })
            .await
            .unwrap(),
        None
    );
    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = $1")
        .bind(p)
        .execute(s.pool())
        .await
        .unwrap();
    assert_eq!(
        s.push_mark(&PartKey { tracker_id: t, persona_id: p, tenant_id: tenant, part: "facts" })
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn board_figures_answer_every_sprint_and_predictability_from_one_replay() {
    let s = pg_store().await;
    let t = board(&s).await;
    let now = chrono::Utc::now();
    let figures = crate::sdlc::sprint_replay::board_figures(&s, t, now).await.unwrap();
    let sprints = crate::sdlc::sprint_replay::sprint_sdlc(&s, t, now).await.unwrap();
    assert_eq!(figures.by_sprint.len(), sprints.len());
    for (id, f) in &sprints {
        assert_eq!(
            serde_json::to_value(figures.by_sprint.get(id).unwrap()).unwrap(),
            serde_json::to_value(f).unwrap()
        );
    }
    let project = crate::sdlc::sprint_replay::project_sdlc(&s, t, now).await.unwrap();
    assert_eq!(figures.predictability.ok(), project.predictability);
    assert_eq!(figures.predictability.err(), project.reasons.get("predictability").copied());
}
