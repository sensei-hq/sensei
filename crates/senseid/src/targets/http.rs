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
    use crate::tracker::secret::test_support::MemoryStore;
    use crate::tracker::test_support::{FakeBoard, repo_in, session_with};

    struct Harness {
        app: axum::Router,
        state: crate::api::state::AppState,
        factory: Arc<FakeFactory>,
        store: Arc<MemoryStore>,
    }

    async fn harness() -> Harness {
        let (rt, store, factory) = fake_runtime();
        let ctx = crate::tasks::test_support::make_ctx_with_trackers(rt).await;
        let state = ctx.app_state.clone();
        Harness { app: crate::api::routes::create_router(state.clone()), state, factory, store }
    }

    const API_TOKEN: &str = "TOKEN-TG5-SENTINEL";
    const ACCOUNT: &str = "dev@example.com";

    /// A fetch through the route. Whatever it answers — 200, 404 or 502 — no body
    /// carries the tracker's credential: not the API token, not the Keychain
    /// service it is filed under, not the account (review r1 MED-3).
    async fn post_fetch(h: &Harness, key: &str, body: Value) -> (StatusCode, Value) {
        let (st, b) = call(h, "POST", &fetch(key), Some(body)).await;
        let text = b.to_string();
        let services: Vec<(String,)> =
            sqlx_core::query_as::query_as("SELECT secret_service FROM sensei.trackers")
                .fetch_all(h.state.pg.pool())
                .await
                .unwrap();
        for secret in [API_TOKEN, ACCOUNT].into_iter().chain(services.iter().map(|(s,)| s.as_str()))
        {
            assert!(
                !text.contains(secret),
                "a fetch answered with a credential ({secret}): {text}"
            );
        }
        (st, b)
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
        let board = connect_board(h, workspace, board).await;
        let mut w = repository_in(h, workspace).await;
        w.board = board;
        w
    }

    /// Connect a Jira board (project AUTH, board 42) to `workspace` and wait for
    /// its first sync to finish.
    async fn connect_board(h: &Harness, workspace: uuid::Uuid, board: FakeBoard) -> Arc<FakeBoard> {
        let site = format!("t{}.atlassian.net", uuid::Uuid::new_v4().simple());
        let board = h.factory.serve(&format!("https://{site}"), board);
        let (st, b) = call(
            h,
            "POST",
            &format!("/api/workspaces/{workspace}/trackers"),
            Some(json!({
                "kind": "jira", "level": "sprint",
                "board_url": format!("https://{site}/jira/software/projects/AUTH/boards/42"),
                "email": ACCOUNT, "api_token": API_TOKEN,
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
        board
    }

    /// A repository folder in `workspace` and one session in it; `board` is an
    /// unserved, empty board until a caller connects one.
    async fn repository_in(h: &Harness, workspace: uuid::Uuid) -> World {
        let pg = &h.state.pg;
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
        World { folder, session, client_session, board: Arc::new(FakeBoard::standard()) }
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
        let (st, b) =
            post_fetch(&h, "AUTH-2", json!({"folder": w.folder, "session_id": w.client_session}))
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
        post_fetch(&h, "AUTH-2", body.clone()).await;
        let (started,): (chrono::DateTime<chrono::Utc>,) = sqlx_core::query_as::query_as(
            "SELECT started_at FROM sensei.session_task_links WHERE session_id = $1 AND source = 'fetch'",
        )
        .bind(w.session)
        .fetch_one(h.state.pg.pool())
        .await
        .unwrap();

        let (st, b) = post_fetch(&h, "AUTH-2", body).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(b["ticket"]["key"], json!("AUTH-2"), "the ticket is still handed over");
        let sow = &b["start_of_work"];
        assert_eq!(
            (&sow["recorded"], &sow["reason"]),
            (&json!(false), &json!("already_started")),
            "{b}"
        );
        // The instant, not its spelling (review r1 MED-2).
        let said: chrono::DateTime<chrono::Utc> =
            serde_json::from_value(sow["started_at"].clone()).expect("started_at is a timestamp");
        assert_eq!(said, started, "the first fetch's start, unchanged");
        assert_eq!(links_of(&h, w.session).await.len(), 1, "one fetch link, not two");
    }

    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn an_unknown_key_is_404_and_writes_no_link() {
        let h = harness().await;
        let w = world(&h, FakeBoard::standard()).await;
        let (st, b) =
            post_fetch(&h, "AUTH-404", json!({"folder": w.folder, "session_id": w.client_session}))
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
        let (st, b) =
            post_fetch(&h, "AUTH-2", json!({"folder": w.folder, "session_id": "no-such-session"}))
                .await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(b["ticket"]["key"], json!("AUTH-2"));
        assert_eq!(b["start_of_work"], json!({"recorded": false, "reason": "session_not_found"}));

        let (st, b) = post_fetch(&h, "AUTH-2", json!({"folder": w.folder})).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(b["start_of_work"], json!({"recorded": false, "reason": "no_session"}));
        assert_eq!(links_of(&h, w.session).await, vec![]);
    }

    /// A failed live read is a 502 naming the tracker's error — never a 200 with an
    /// empty or placeholder ticket, and no link: refused, unreachable, throttled.
    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn a_reader_failure_is_502_not_an_empty_ticket() {
        let h = harness().await;
        for e in [TrackerError::Unauthorized, TrackerError::Transport, TrackerError::RateLimited] {
            let w = world(&h, FakeBoard::standard().fail(AUTH_2, e)).await;
            let (st, b) = post_fetch(
                &h,
                "AUTH-2",
                json!({"folder": w.folder, "session_id": w.client_session}),
            )
            .await;
            assert_eq!(st, StatusCode::BAD_GATEWAY, "{e:?}: {b}");
            assert_eq!(b, json!({"error": e.code(), "key": "AUTH-2"}), "{e:?}");
            assert_eq!(links_of(&h, w.session).await, vec![], "{e:?}");
        }
    }

    /// The board's credential is gone from the Keychain (review r1 MED-1): a 502
    /// that says so, never a read without it, and no link.
    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn a_missing_token_is_502_token_missing() {
        let h = harness().await;
        let w = world(&h, FakeBoard::standard().with(AUTH_2, 0, auth_2())).await;
        h.store.entries.lock().unwrap().clear();
        let asked = w.board.calls().len();
        let (st, b) =
            post_fetch(&h, "AUTH-2", json!({"folder": w.folder, "session_id": w.client_session}))
                .await;
        assert_eq!(st, StatusCode::BAD_GATEWAY, "{b}");
        assert_eq!(b, json!({"error": "token_missing", "key": "AUTH-2"}));
        assert_eq!(w.board.calls().len(), asked, "nothing was asked without the credential");
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
        for (path_key, key) in [("ZZZ-1", "ZZZ-1"), ("%2312", "#12")] {
            let (st, b) = post_fetch(&h, path_key, json!({"folder": w.folder})).await;
            assert_eq!(st, StatusCode::NOT_FOUND, "{key}: {b}");
            assert_eq!(b, json!({"error": "no_tracker_for_key", "key": key}));
        }
        assert!(
            !w.board.calls().iter().any(|c| c.starts_with("/rest/agile/1.0/issue/")),
            "nothing was asked"
        );
    }

    /// A key is resolved against the trackers of the folder's OWN workspace only
    /// (review r1 HIGH-1): another workspace's board that answers to the key is
    /// never asked, and the folder's workspace has no tracker for it.
    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn a_key_resolves_only_against_the_folder_s_own_workspace() {
        let h = harness().await;
        let pg = &h.state.pg;
        let mine = pg
            .create_workspace(&format!("_test:tg5:a:{}", uuid::Uuid::new_v4()), None, None)
            .await
            .unwrap();
        let theirs = pg
            .create_workspace(&format!("_test:tg5:b:{}", uuid::Uuid::new_v4()), None, None)
            .await
            .unwrap();
        let their_board =
            connect_board(&h, theirs, FakeBoard::standard().with(AUTH_2, 0, auth_2())).await;
        let w = repository_in(&h, mine).await;
        let (st, b) =
            post_fetch(&h, "AUTH-2", json!({"folder": w.folder, "session_id": w.client_session}))
                .await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{b}");
        assert_eq!(b, json!({"error": "no_tracker_for_key", "key": "AUTH-2"}));
        assert!(
            !their_board.calls().iter().any(|c| c.starts_with("/rest/agile/1.0/issue/")),
            "another workspace's board was never asked"
        );
        assert_eq!(links_of(&h, w.session).await, vec![]);
    }

    /// A folder the daemon has not indexed is the folder resolver's own 404 —
    /// never a fall-back to some workspace.
    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn an_unindexed_folder_is_404() {
        let h = harness().await;
        world(&h, FakeBoard::standard().with(AUTH_2, 0, auth_2())).await;
        let (st, b) = post_fetch(&h, "AUTH-2", json!({"folder": "/no/such/folder"})).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{b}");
        assert_eq!(b, json!({"error": "folder not indexed"}));
    }

    /// A folder that belongs to no workspace has no trackers to answer: an
    /// explicit 404 naming it — never a 200, never another workspace's tracker.
    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn a_folder_in_no_workspace_is_404() {
        let h = harness().await;
        world(&h, FakeBoard::standard().with(AUTH_2, 0, auth_2())).await;
        let loose = crate::db::pg_store::tests::create_test_folder(
            &h.state.pg,
            &format!("tg5-loose-{}", uuid::Uuid::new_v4().simple()),
        )
        .await;
        let (path,): (String,) =
            sqlx_core::query_as::query_as("SELECT abs_path FROM sensei.folders WHERE id = $1")
                .bind(loose)
                .fetch_one(h.state.pg.pool())
                .await
                .unwrap();
        let (st, b) = post_fetch(&h, "AUTH-2", json!({"folder": path})).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{b}");
        assert_eq!(b, json!({"error": "folder_not_in_workspace", "folder": path}));
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
        let (st, b) =
            post_fetch(&h, "AUTH-900", json!({"folder": w.folder, "session_id": w.client_session}))
                .await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(b["start_of_work"]["recorded"], json!(true), "{b}");
        assert_eq!(
            links_of(&h, w.session).await,
            vec![("fetch".into(), "AUTH-900".into(), 1, true)]
        );
    }

    /// The live read succeeded but the mirror could not learn the ticket (review r1
    /// LOW-1): the ticket is still handed over, and the start of work says why it
    /// was not recorded — never `recorded: true` without a link.
    #[tokio::test]
    #[ignore = "pending: tg5a"]
    async fn a_discovery_failure_after_a_read_returns_the_ticket_not_recorded() {
        let h = harness().await;
        let fresh = json!({"id": "10901", "key": "AUTH-901", "fields": {"summary": "Newer"}});
        let w = world(
            &h,
            FakeBoard::standard()
                .with("/rest/agile/1.0/issue/AUTH-901", 0, fresh)
                .fail_jql("key in (AUTH-901)", TrackerError::Transport),
        )
        .await;
        let (st, b) =
            post_fetch(&h, "AUTH-901", json!({"folder": w.folder, "session_id": w.client_session}))
                .await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(b["ticket"]["key"], json!("AUTH-901"));
        assert_eq!(b["start_of_work"], json!({"recorded": false, "reason": "transport"}));
        assert_eq!(links_of(&h, w.session).await, vec![]);
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
        let (st, b) = post_fetch(&h, "AUTH-2", json!({"folder": w.folder})).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(
            b["rules"],
            json!({"total": rules["total"], "mandatory_count": rules["mandatory_count"], "rules": rules["rules"]})
        );
    }
}
