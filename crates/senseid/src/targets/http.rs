//! TG5 · the routes behind MCP `get_ticket` (D63, D71, D73): fetch a ticket live
//! by its key, start work on it (a `fetch` link with `started_at`), and hand it
//! over with the rules that apply. pending: tg5a — the handler lands in phase B.

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::{Value, json};
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::tracker::TrackerError;
    use crate::tracker::runtime::test_support::{FakeFactory, fake_runtime};
    use crate::tracker::test_support::{FakeBoard, repo_in, session_with};

    struct Harness {
        app: axum::Router,
        state: crate::api::state::AppState,
        factory: Arc<FakeFactory>,
    }

    async fn harness() -> Harness {
        let (rt, _store, factory) = fake_runtime();
        let ctx = crate::tasks::test_support::make_ctx_with_trackers(rt).await;
        let state = ctx.app_state.clone();
        Harness { app: crate::api::routes::create_router(state.clone()), state, factory }
    }

    async fn call(
        h: &Harness,
        method: &str,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let b = Request::builder().method(method).uri(uri).header("host", "127.0.0.1:7744");
        let req = match body {
            Some(v) => b
                .header("content-type", "application/json")
                .body(Body::from(v.to_string()))
                .unwrap(),
            None => b.body(Body::empty()).unwrap(),
        };
        let resp = h.app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    const AUTH_2: &str = "/rest/agile/1.0/issue/AUTH-2";

    fn auth_2() -> Value {
        json!({"id": "10002", "key": "AUTH-2", "fields": {
            "summary": "summary AUTH-2", "description": "Why it matters.",
            "status": {"name": "In Progress"}, "issuetype": {"name": "Story"}}})
    }

    /// A workspace with a connected Jira board (project AUTH, board 42) that has
    /// synced, a repository folder in it, and one session in that repository.
    struct World {
        folder: String,
        session: uuid::Uuid,
        client_session: String,
        board: Arc<FakeBoard>,
    }

    async fn world(h: &Harness, board: FakeBoard) -> World {
        let pg = &h.state.pg;
        let workspace = pg
            .create_workspace(&format!("_test:tg5:{}", uuid::Uuid::new_v4()), None, None)
            .await
            .unwrap();
        let site = format!("t{}.atlassian.net", uuid::Uuid::new_v4().simple());
        let board = h.factory.serve(&format!("https://{site}"), board);
        let (st, b) = call(
            h,
            "POST",
            &format!("/api/workspaces/{workspace}/trackers"),
            Some(json!({
                "kind": "jira", "level": "sprint",
                "board_url": format!("https://{site}/jira/software/projects/AUTH/boards/42"),
                "email": "dev@example.com", "api_token": "TOKEN",
            })),
        )
        .await;
        assert!(st.is_success(), "connect: {st} {b}");
        for _ in 0..300 {
            let (_, b) =
                call(h, "GET", &format!("/api/workspaces/{workspace}/trackers"), None).await;
            if b[0]["status"] != json!("pending") {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let repo = repo_in(pg, workspace).await;
        let (folder,): (String,) =
            sqlx_core::query_as::query_as("SELECT abs_path FROM sensei.folders WHERE id = $1")
                .bind(repo)
                .fetch_one(pg.pool())
                .await
                .unwrap();
        let session = session_with(pg, Some(repo), &[], &[]).await;
        let (client_session,): (String,) = sqlx_core::query_as::query_as(
            "SELECT client_session_id FROM activity.sessions WHERE id = $1",
        )
        .bind(session)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        World { folder, session, client_session, board }
    }

    /// `(source, matched_key, mentions, started_at is set)` of the session's links.
    async fn links_of(h: &Harness, session: uuid::Uuid) -> Vec<(String, String, i32, bool)> {
        sqlx_core::query_as::query_as(
            "SELECT source::text, matched_key, mentions, started_at IS NOT NULL
               FROM sensei.session_task_links WHERE session_id = $1 ORDER BY 1, 2",
        )
        .bind(session)
        .fetch_all(h.state.pg.pool())
        .await
        .unwrap()
    }

    fn fetch(key: &str) -> String {
        format!("/api/tickets/{key}/fetch")
    }

    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn a_fetch_links_the_session_and_records_start() {
        let h = harness().await;
        let w = world(&h, FakeBoard::standard().with(AUTH_2, 0, auth_2())).await;
        let (st, b) = call(
            &h,
            "POST",
            &fetch("AUTH-2"),
            Some(json!({"folder": w.folder, "session_id": w.client_session})),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(b["ticket"]["key"], json!("AUTH-2"));
        assert_eq!(b["ticket"]["title"], json!("summary AUTH-2"));
        assert_eq!(b["ticket"]["description"], json!("Why it matters."), "read live");
        assert_eq!(b["start_of_work"]["recorded"], json!(true), "{b}");
        assert_eq!(links_of(&h, w.session).await, vec![("fetch".into(), "AUTH-2".into(), 1, true)]);
        assert!(w.board.calls().iter().any(|c| c == AUTH_2), "the ticket was read live");
    }

    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn a_second_fetch_records_nothing_new() {
        let h = harness().await;
        let w = world(&h, FakeBoard::standard().with(AUTH_2, 0, auth_2())).await;
        let body = json!({"folder": w.folder, "session_id": w.client_session});
        call(&h, "POST", &fetch("AUTH-2"), Some(body.clone())).await;
        let (started,): (chrono::DateTime<chrono::Utc>,) = sqlx_core::query_as::query_as(
            "SELECT started_at FROM sensei.session_task_links WHERE session_id = $1 AND source = 'fetch'",
        )
        .bind(w.session)
        .fetch_one(h.state.pg.pool())
        .await
        .unwrap();

        let (st, b) = call(&h, "POST", &fetch("AUTH-2"), Some(body)).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(b["ticket"]["key"], json!("AUTH-2"), "the ticket is still handed over");
        assert_eq!(
            b["start_of_work"],
            json!({"recorded": false, "reason": "already_started", "started_at": started.to_rfc3339()})
        );
        assert_eq!(links_of(&h, w.session).await.len(), 1, "one fetch link, not two");
    }

    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn an_unknown_key_is_404_and_writes_no_link() {
        let h = harness().await;
        let w = world(&h, FakeBoard::standard()).await;
        let (st, b) = call(
            &h,
            "POST",
            &fetch("AUTH-404"),
            Some(json!({"folder": w.folder, "session_id": w.client_session})),
        )
        .await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{b}");
        assert_eq!(b, json!({"error": "ticket_not_found", "key": "AUTH-404"}));
        assert_eq!(links_of(&h, w.session).await, vec![], "no link for a ticket that is not there");
    }

    /// The ticket is still handed over when the session is unknown (or not
    /// given): the start of work is what is not recorded, and the answer says so.
    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn a_fetch_without_a_known_session_returns_the_ticket_and_says_not_recorded() {
        let h = harness().await;
        let w = world(&h, FakeBoard::standard().with(AUTH_2, 0, auth_2())).await;
        let (st, b) = call(
            &h,
            "POST",
            &fetch("AUTH-2"),
            Some(json!({"folder": w.folder, "session_id": "no-such-session"})),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(b["ticket"]["key"], json!("AUTH-2"));
        assert_eq!(b["start_of_work"], json!({"recorded": false, "reason": "session_not_found"}));

        let (st, b) = call(&h, "POST", &fetch("AUTH-2"), Some(json!({"folder": w.folder}))).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(b["start_of_work"], json!({"recorded": false, "reason": "no_session"}));
        assert_eq!(links_of(&h, w.session).await, vec![]);
    }

    /// A failed live read is a 502 naming the tracker's error — never a 200 with an
    /// empty or placeholder ticket, and no link.
    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn a_reader_failure_is_502_not_an_empty_ticket() {
        let h = harness().await;
        let w = world(&h, FakeBoard::standard().fail(AUTH_2, TrackerError::Unauthorized)).await;
        let (st, b) = call(
            &h,
            "POST",
            &fetch("AUTH-2"),
            Some(json!({"folder": w.folder, "session_id": w.client_session})),
        )
        .await;
        assert_eq!(st, StatusCode::BAD_GATEWAY, "{b}");
        assert_eq!(b, json!({"error": "unauthorized", "key": "AUTH-2"}));
        assert_eq!(links_of(&h, w.session).await, vec![]);
    }

    /// A key no tracker of the folder's workspace answers to is a 404 that says
    /// so — not "no such ticket". (GitHub Issues keys, `#12` and `owner/repo#12`,
    /// are answered by the GitHub reader, TG5b; until then no tracker answers.)
    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn a_key_no_tracker_answers_to_is_404_no_tracker_for_key() {
        let h = harness().await;
        let w = world(&h, FakeBoard::standard()).await;
        for key in ["ZZZ-1", "%2312"] {
            let (st, b) = call(&h, "POST", &fetch(key), Some(json!({"folder": w.folder}))).await;
            assert_eq!(st, StatusCode::NOT_FOUND, "{key}: {b}");
            assert_eq!(b["error"], json!("no_tracker_for_key"), "{key}: {b}");
        }
        assert!(
            !w.board.calls().iter().any(|c| c.starts_with("/rest/agile/1.0/issue/")),
            "nothing was asked"
        );
    }

    /// A ticket on the board that the mirror has not seen yet is discovered, then
    /// linked — the same discover-on-miss the tracker-task lookup uses.
    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn a_ticket_not_yet_mirrored_is_discovered_then_linked() {
        let h = harness().await;
        let fresh = json!({"id": "10900", "key": "AUTH-900", "fields": {"summary": "New"}});
        let w = world(
            &h,
            FakeBoard::standard()
                .with("/rest/agile/1.0/issue/AUTH-900", 0, fresh.clone())
                .with_jql("key in (AUTH-900)", json!([fresh])),
        )
        .await;
        let (st, b) = call(
            &h,
            "POST",
            &fetch("AUTH-900"),
            Some(json!({"folder": w.folder, "session_id": w.client_session})),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(b["start_of_work"]["recorded"], json!(true), "{b}");
        assert_eq!(
            links_of(&h, w.session).await,
            vec![("fetch".into(), "AUTH-900".into(), 1, true)]
        );
    }

    /// The ticket's rules (D73) are the repository's resolved rules — exactly what
    /// `get_rules` answers for the same folder — handed over with the ticket.
    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn the_ticket_s_rules_arrive_with_it() {
        let h = harness().await;
        let w = world(&h, FakeBoard::standard().with(AUTH_2, 0, auth_2())).await;
        let (_, rules) =
            call(&h, "GET", &format!("/api/knowledge/rules?folder={}", w.folder), None).await;
        let (st, b) = call(&h, "POST", &fetch("AUTH-2"), Some(json!({"folder": w.folder}))).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(
            b["rules"],
            json!({"total": rules["total"], "mandatory_count": rules["mandatory_count"], "rules": rules["rules"]})
        );
    }
}
