//! TG5 · the routes behind MCP `get_ticket` (D63, D71, D73): fetch a ticket live
//! by its key, start work on it (a `fetch` link with `started_at`), and hand it
//! over with the rules that apply. Nothing read from the tracker is stored (D71),
//! and a failed read is an error the caller sees — never an empty or placeholder
//! ticket, and never a link.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use crate::api::state::AppState;
use crate::db::pg_store::targets::FetchStart;
use crate::tracker::ticket::{JiraTicketReader, TicketReader};
use crate::tracker::{TrackerKind, adapter_for_kind, gitlab, jira, runtime};

/// Body of `POST /api/tickets/{key}/fetch`.
#[derive(serde::Deserialize)]
pub(crate) struct FetchBody {
    /// The folder the work is in (the MCP call's cwd).
    pub folder: String,
    /// The assistant's own session id, to link the start of work to.
    pub session_id: Option<String>,
}

/// A refusal: `status` with `{ "error": code, …extra }`.
fn refuse(status: StatusCode, code: &str, extra: Value) -> Response {
    let mut body = json!({ "error": code });
    if let (Some(b), Some(e)) = (body.as_object_mut(), extra.as_object()) {
        b.extend(e.clone());
    }
    (status, Json(body)).into_response()
}

/// `POST /api/tickets/{key}/fetch` — read the ticket live, start work on it, and
/// hand it over with the repository's rules.
pub(crate) async fn fetch_ticket(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(body): Json<FetchBody>,
) -> Response {
    let (folder_path, folder_id) =
        match crate::api::handlers::knowledge::resolve_folder(&state, Some(&body.folder), None)
            .await
        {
            Ok(f) => f,
            Err(e) => return e.into_response(),
        };

    let pg = &state.pg;
    // The folder's workspace, exactly one: none and several are different answers.
    let workspace = match pg.workspaces_of_folder(&folder_id).await {
        Ok(ws) => match ws.as_slice() {
            [one] => *one,
            [] => {
                return refuse(
                    StatusCode::NOT_FOUND,
                    "folder_not_in_workspace",
                    json!({"folder": folder_path}),
                );
            }
            _ => {
                return refuse(
                    StatusCode::CONFLICT,
                    "folder_in_many_workspaces",
                    json!({"folder": folder_path}),
                );
            }
        },
        Err(e) => {
            tracing::error!(error = %e, "fetch_ticket: folder workspaces read failed");
            return refuse(StatusCode::INTERNAL_SERVER_ERROR, "internal", json!({}));
        }
    };

    let trackers = match pg.trackers_for_workspace(workspace).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "fetch_ticket: trackers read failed");
            return refuse(StatusCode::INTERNAL_SERVER_ERROR, "internal", json!({}));
        }
    };
    // The key in its canonical form and the tracker that answers to it. A Jira key
    // is upper-cased; a forge key (`acme/platform#12`, or a bare `#12` read in the
    // folder's own repository) keeps a lowercased path (§G9).
    let (canonical, tracker) = if key.contains('#') {
        let (canonical, host) = match forge_key(pg, &folder_id, &key).await {
            Ok(ForgeKey::Key(k, host)) => (k, host),
            Ok(ForgeKey::Invalid) => {
                return refuse(StatusCode::NOT_FOUND, "ticket_not_found", json!({"key": key}));
            }
            Ok(ForgeKey::NoRepository) => {
                return refuse(StatusCode::NOT_FOUND, "no_tracker_for_key", json!({"key": key}));
            }
            Err(e) => {
                tracing::error!(error = %e, "fetch_ticket: the folder's repository read failed");
                return refuse(StatusCode::INTERNAL_SERVER_ERROR, "internal", json!({}));
            }
        };
        let tracker =
            crate::tracker::forge::tracker_for_key(&trackers, &canonical, host.as_deref());
        (canonical, tracker)
    } else {
        // One of THIS workspace's Jira trackers whose project key is the key's prefix.
        let upper = key.to_ascii_uppercase();
        let prefix = upper.split_once('-').map(|(p, _)| p);
        let tracker = trackers.iter().find(|t| {
            TrackerKind::parse(&t.kind) == Some(TrackerKind::Jira)
                && jira::is_jira_key(&upper)
                && prefix.is_some_and(|p| p.eq_ignore_ascii_case(&t.external_scope))
        });
        (upper, tracker)
    };
    let Some(tracker) = tracker else {
        return refuse(StatusCode::NOT_FOUND, "no_tracker_for_key", json!({"key": key}));
    };
    let Some(adapter) = TrackerKind::parse(&tracker.kind).and_then(adapter_for_kind) else {
        return refuse(StatusCode::BAD_GATEWAY, "unsupported_tracker", json!({"key": key}));
    };

    // The live read, with the board's own credential.
    let transport = match runtime::transport_for(&state.trackers, adapter, tracker, 0).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(code = e.code(), "fetch_ticket: no transport for the tracker");
            return refuse(StatusCode::BAD_GATEWAY, e.code(), json!({"key": key}));
        }
    };
    let read = match adapter.kind() {
        TrackerKind::Jira => {
            JiraTicketReader {
                transport: transport.as_ref(),
                base_url: tracker.base_url.clone(),
                acceptance_criteria_field: tracker.params["acceptance_criteria_field"]
                    .as_str()
                    .map(String::from),
            }
            .read_ticket(&canonical)
            .await
        }
        TrackerKind::Gitlab => {
            gitlab::GitlabTicketReader {
                transport: transport.as_ref(),
                board: crate::tracker::BoardScope::of(tracker),
            }
            .read_ticket(&canonical)
            .await
        }
        TrackerKind::Github => {
            crate::tracker::github::GithubTicketReader {
                transport: transport.as_ref(),
                board: crate::tracker::BoardScope::of(tracker),
            }
            .read_ticket(&canonical)
            .await
        }
    };
    let ticket = match read {
        Ok(Some(t)) => t,
        Ok(None) => return refuse(StatusCode::NOT_FOUND, "ticket_not_found", json!({"key": key})),
        Err(e) => {
            tracing::warn!(code = e.code(), "fetch_ticket: the live read failed");
            return refuse(StatusCode::BAD_GATEWAY, e.code(), json!({"key": key}));
        }
    };

    // The ticket's rules (D73): the repository's resolved rules.
    let rules =
        match crate::api::handlers::knowledge::resolve_repo_ruleset(&state, &folder_id).await {
            // As `get_rules` answers it, including whether the dōjō's rules are missing:
            // a ticket handed over with incomplete rules says so.
            Ok((r, dojo)) => json!({
                "total": r.total,
                "mandatory_count": r.mandatory_count,
                "rules": r.rules,
                "rules_incomplete": dojo.rules_incomplete(),
                "dojo_rules": dojo,
            }),
            Err(e) => {
                tracing::error!(error = %e, "fetch_ticket: rules could not be resolved");
                return refuse(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "rules_unavailable",
                    json!({"key": key}),
                );
            }
        };

    let start_of_work = start_work(&state, workspace, &canonical, body.session_id.as_deref()).await;
    Json(json!({ "ticket": ticket, "rules": rules, "start_of_work": start_of_work }))
        .into_response()
}

/// A forge key, as the route reads it.
enum ForgeKey {
    /// Canonical: the lowercased path, `#`, the number; and the host it was read on,
    /// known only for a bare `#n` (the folder's `repo_key`), so selection can keep
    /// to that host (spec tracker-github §H5).
    Key(String, Option<String>),
    /// Not a key (an empty, `.` or `..` path segment, no number): nothing is fetched.
    Invalid,
    /// A bare `#n` from a folder with no repository key to read it in.
    NoRepository,
}

/// `key` in canonical form. A bare `#n` is read in the folder's own repository:
/// its `repo_key` (`host/org/repo`) gives the path.
async fn forge_key(
    pg: &crate::db::pg_store::PgStore,
    folder_id: &uuid::Uuid,
    key: &str,
) -> Result<ForgeKey, String> {
    use crate::tracker::forge::{is_key_path, parse_key};
    if let Some(n) = key.strip_prefix('#') {
        if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
            return Ok(ForgeKey::Invalid);
        }
        let Some(repo_key) = pg.repo_key_of_folder(folder_id).await? else {
            return Ok(ForgeKey::NoRepository);
        };
        // `host/org/repo`: the path is everything after the host.
        return Ok(match repo_key.split_once('/') {
            Some((host, path)) if is_key_path(path) => {
                ForgeKey::Key(format!("{path}#{n}"), Some(host.to_ascii_lowercase()))
            }
            _ => ForgeKey::NoRepository,
        });
    }
    Ok(match parse_key(key) {
        Some((path, n)) => ForgeKey::Key(format!("{path}#{n}"), None),
        None => ForgeKey::Invalid,
    })
}

/// Link the session to the ticket as the start of work, or say why not. The
/// ticket is handed over either way; only `recorded: true` means a link exists.
async fn start_work(
    state: &AppState,
    workspace: uuid::Uuid,
    key: &str,
    client: Option<&str>,
) -> Value {
    let not = |reason: &str| json!({"recorded": false, "reason": reason});
    let Some(client) = client.filter(|c| !c.is_empty()) else { return not("no_session") };
    let pg = &state.pg;
    let session = match pg.session_for_client_id(client).await {
        Ok(Some(s)) => s,
        Ok(None) => return not("session_not_found"),
        Err(e) => {
            tracing::error!(error = %e, "fetch_ticket: session read failed");
            return not("internal");
        }
    };
    // The mirror row: as synced, else discovered now (the tracker-task lookup's path).
    let task = match pg.tracker_task_detail(workspace, key).await {
        Ok(Some(t)) => t.id,
        Ok(None) => {
            match crate::api::handlers::trackers::discover_on_miss(state, workspace, key).await {
                Ok(t) => t.id,
                Err((_, Json(e))) => {
                    let code = e["error"].as_str().unwrap_or("internal").to_string();
                    tracing::warn!(code = %code, "fetch_ticket: the ticket could not be mirrored");
                    return not(&code);
                }
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "fetch_ticket: mirror read failed");
            return not("internal");
        }
    };
    match pg.record_fetch_link(&session, &task, key).await {
        Ok(FetchStart::Recorded) => json!({"recorded": true}),
        Ok(FetchStart::AlreadyStarted(at)) => {
            json!({"recorded": false, "reason": "already_started", "started_at": at})
        }
        Err(e) => {
            tracing::error!(error = %e, "fetch_ticket: the start of work could not be recorded");
            not("internal")
        }
    }
}

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
    async fn an_unindexed_folder_is_404() {
        let h = harness().await;
        world(&h, FakeBoard::standard().with(AUTH_2, 0, auth_2())).await;
        let (st, b) = post_fetch(&h, "AUTH-2", json!({"folder": "/no/such/folder"})).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{b}");
        assert_eq!(b, json!({"error": "folder not indexed"}));
    }

    /// A folder whose repository serves two workspaces has two answers (review r2
    /// MED-1, L0 ruling): an explicit 409, distinct from "in no workspace", with no
    /// board asked — picking one workspace would be a fabrication
    /// (`sole_workspace_of`'s contract).
    #[tokio::test]
    async fn a_folder_in_two_workspaces_is_409() {
        let h = harness().await;
        let w = world(&h, FakeBoard::standard().with(AUTH_2, 0, auth_2())).await;
        let other = h
            .state
            .pg
            .create_workspace(&format!("_test:tg5:other:{}", uuid::Uuid::new_v4()), None, None)
            .await
            .unwrap();
        let (folder_id,): (uuid::Uuid,) =
            sqlx_core::query_as::query_as("SELECT id FROM sensei.folders WHERE abs_path = $1")
                .bind(&w.folder)
                .fetch_one(h.state.pg.pool())
                .await
                .unwrap();
        h.state.pg.set_folder_workspace(&folder_id, &other, "root", None).await.unwrap();
        let asked = w.board.calls().len();
        let (st, b) =
            post_fetch(&h, "AUTH-2", json!({"folder": w.folder, "session_id": w.client_session}))
                .await;
        assert_eq!(st, StatusCode::CONFLICT, "{b}");
        assert_eq!(b, json!({"error": "folder_in_many_workspaces", "folder": w.folder}));
        assert_eq!(w.board.calls().len(), asked, "no board was asked");
        assert_eq!(links_of(&h, w.session).await, vec![]);
    }

    /// A folder that belongs to no workspace has no trackers to answer: an
    /// explicit 404 naming it — never a 200, never another workspace's tracker.
    #[tokio::test]
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
    async fn the_ticket_s_rules_arrive_with_it() {
        let h = harness().await;
        let w = world(&h, FakeBoard::standard().with(AUTH_2, 0, auth_2())).await;
        let (_, rules) =
            call(&h, "GET", &format!("/api/knowledge/rules?folder={}", w.folder), None).await;
        let (st, b) = post_fetch(&h, "AUTH-2", json!({"folder": w.folder})).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        // `get_rules`' own answer for the folder, without its `folder` echo — so the
        // dōjō-rules state (`dojo_rules`, `rules_incomplete`) arrives too.
        let mut expected = rules.clone();
        expected.as_object_mut().unwrap().remove("folder");
        assert_eq!(b["rules"], expected);
    }

    // ── §G9: GitLab keys, against the recorded CE ───────────────────────────

    /// A workspace with the recorded GitLab group `acme` (group:857) connected and
    /// synced, a repository folder whose `repo_key` is `acme/backend`, and a session.
    /// Issue 44 is the recorded one; 99 is missing; 7 lives in a project outside the
    /// group; 8 answers 500.
    async fn gitlab_world(h: &Harness) -> World {
        use crate::tracker::gitlab::test_support::{RecordedGitlab, body};
        let outside = {
            let mut i = body("ce-closed-milestone", "project_issue_one");
            i["project_id"] = json!(999);
            i
        };
        let gl = RecordedGitlab::load("ce-closed-milestone")
            .with(
                "projects/acme%2Fbackend/issues/44",
                200,
                body("ce-closed-milestone", "project_issue_one"),
            )
            .with("projects/acme%2Fbackend/issues/99", 404, json!({}))
            .with("projects/acme%2Fbackend/issues/7", 200, outside)
            .with("projects/999", 200, json!({"id": 999, "namespace": {"full_path": "elsewhere"}}))
            .with("projects/acme%2Fbackend/issues/8", 500, json!({}));
        h.factory.serve_transport("https://gitlab.example.com", Arc::new(gl));
        let pg = &h.state.pg;
        let workspace = pg
            .create_workspace(&format!("_test:g9:{}", uuid::Uuid::new_v4()), None, None)
            .await
            .unwrap();
        let (st, b) = call(
            h,
            "POST",
            &format!("/api/workspaces/{workspace}/trackers"),
            Some(json!({
                "kind": "gitlab", "level": "sprint",
                "board_url": "https://gitlab.example.com/groups/acme",
                "confirm_host": "gitlab.example.com", "api_token": API_TOKEN,
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
        let w = repository_in(h, workspace).await;
        sqlx_core::query::query(
            "UPDATE sensei.repositories SET repo_key = 'gitlab.example.com/acme/backend'
              WHERE id = (SELECT repository_id FROM sensei.folders WHERE abs_path = $1)",
        )
        .bind(&w.folder)
        .execute(pg.pool())
        .await
        .unwrap();
        w
    }

    #[tokio::test]
    async fn a_gitlab_key_reads_the_issue_and_links_the_session() {
        let h = harness().await;
        let w = gitlab_world(&h).await;
        let (st, b) = post_fetch(
            &h,
            "Acme%2FBackend%2344",
            json!({"folder": w.folder, "session_id": w.client_session}),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{b}");
        let t = &b["ticket"];
        assert_eq!(
            (t["key"].clone(), t["tracker"].clone()),
            (json!("acme/backend#44"), json!("gitlab"))
        );
        assert_eq!(t["title"], json!("Issue 44"));
        assert_eq!(t["acceptance_criteria_source"], json!("not_configured"));
        assert_eq!(t["url"], json!("https://gitlab.example.com/acme/backend/-/work_items/44"));
        assert_eq!(
            b["start_of_work"],
            json!({"recorded": true}),
            "the canonical key finds the mirror"
        );
        assert_eq!(
            links_of(&h, w.session).await,
            [("fetch".into(), "acme/backend#44".into(), 1, true)]
        );
    }

    #[tokio::test]
    async fn a_bare_key_is_read_in_the_folder_s_repository() {
        let h = harness().await;
        let w = gitlab_world(&h).await;
        let (st, b) = post_fetch(&h, "%2344", json!({"folder": w.folder})).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert_eq!(b["ticket"]["key"], json!("acme/backend#44"));
    }

    #[tokio::test]
    async fn a_gitlab_issue_outside_the_board_is_not_returned() {
        // The token can see it; the board does not hold it.
        let h = harness().await;
        let w = gitlab_world(&h).await;
        let (st, b) = post_fetch(&h, "acme%2Fbackend%237", json!({"folder": w.folder})).await;
        assert_eq!((st, b["error"].clone()), (StatusCode::NOT_FOUND, json!("ticket_not_found")));
        assert!(b.get("ticket").is_none() && !b.to_string().contains("Issue 44"), "{b}");
    }

    #[tokio::test]
    async fn gitlab_misses_failures_and_bad_keys_answer_with_their_codes() {
        let h = harness().await;
        let w = gitlab_world(&h).await;
        let folder = json!({"folder": w.folder});
        let (st, b) = post_fetch(&h, "acme%2Fbackend%2399", folder.clone()).await;
        assert_eq!((st, b["error"].clone()), (StatusCode::NOT_FOUND, json!("ticket_not_found")));
        let (st, b) = post_fetch(&h, "acme%2Fbackend%238", folder.clone()).await;
        assert_eq!((st, b["error"].clone()), (StatusCode::BAD_GATEWAY, json!("http_error")));
        let (st, b) = post_fetch(&h, "acme%2F..%2Fx%231", folder.clone()).await;
        assert_eq!((st, b["error"].clone()), (StatusCode::NOT_FOUND, json!("ticket_not_found")));
        let (st, b) = post_fetch(&h, "other%2Frepo%231", folder).await;
        assert_eq!((st, b["error"].clone()), (StatusCode::NOT_FOUND, json!("no_tracker_for_key")));
    }
}
