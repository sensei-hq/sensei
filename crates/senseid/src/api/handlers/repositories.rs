//! Repository sharing — the surface that turns gate 1 on and off.
//!
//! `sensei.repositories.visibility` is private-by-default, deliberately: signing
//! in must not start sharing a repository the user never offered. But until this
//! handler existed, **nothing could set it at all** — no API, no CLI flag, no
//! toggle — so gate 1 was unreachable, every repository sat at the default, and
//! the metric push would have moved zero rows while reporting success
//! (`docs/spec/dojo/daemon-sync.md` claim C3).
//!
//! What sharing means, and why the default lives here rather than at sign-in
//! (D8 / §2a): the value of pushing metrics appears at two or more people on one
//! repository, where "me vs the rest" becomes a comparison and governance and
//! insight sharing have someone to share with. So the sensible default is ON for
//! a repository whose code is already public, and OFF (subscription-gated) for a
//! private one. That default is applied when the user configures sharing — it is
//! never inferred behind their back, which is what would make signing in start
//! sharing.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};

use crate::api::handlers::err;
use crate::api::origin_guard::origin_may_mutate;
use crate::api::state::AppState;

type ApiError = (StatusCode, Json<serde_json::Value>);

/// The legal values, owned by `sensei.repo_visibility`.
///
/// Listed here only to give a 400 with a readable message instead of a 500 from a
/// failed enum cast. The database remains the authority — a value that passes
/// this check and fails the cast still errors rather than being coerced.
const VISIBILITIES: &[&str] = &["private", "shared"];

/// `PATCH /api/repositories/{repo_key}` — opt a repository into or out of sharing.
///
/// `{ "visibility": "shared" | "private" }`
///
/// The `repo_key` is the durable cross-install identity (`host/org/repo`), not a
/// local uuid: the same repository has different ids on different machines, and
/// this is the name the dōjō knows it by.
///
/// A repository this database does not have is a 404, never an insert. Creating a
/// row here would invent a repository the scanner never saw.
pub(crate) async fn patch_repository(
    State(state): State<AppState>,
    Path(repo_key): Path<String>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !origin_may_mutate(&headers) {
        // 403 before any lookup: the 404-vs-200 distinction is itself the
        // inventory oracle, so a foreign origin must not learn it either.
        return Err(err(StatusCode::FORBIDDEN, "this origin may not change repository sharing"));
    }
    let visibility = body
        .get("visibility")
        .and_then(|v| v.as_str())
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "visibility is required"))?;
    if !VISIBILITIES.contains(&visibility) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            format!("visibility must be one of {VISIBILITIES:?}, got {visibility:?}"),
        ));
    }

    let n = state
        .pg
        .set_repository_visibility(&repo_key, visibility)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    if n == 0 {
        return Err(err(
            StatusCode::NOT_FOUND,
            format!("no repository with repo_key {repo_key:?}"),
        ));
    }
    Ok(Json(serde_json::json!({ "repo_key": repo_key, "visibility": visibility })))
}

// ── WP8 (D46): the election, written to the dōjō ─────────────────────────────
//
// pending: wp8-daemon — `patch_repository` moves onto these in phase B: it
// writes the dōjō's repository election (`PATCH /v1/you/repositories/election`)
// where the person may change it and answers 409 where they may not; the local
// `visibility` gate is retired (nothing local is gated on the plan).

/// What the dōjō answered an election.
#[allow(dead_code)] // pending: wp8-daemon (used by patch_repository in phase B)
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ElectionOutcome {
    /// Recorded: the dōjō's own answer (`elected`, `sync_enabled`, …).
    Recorded(serde_json::Value),
    /// The person may not change it (an organisation's repository and not its
    /// admin): the dōjō's reason.
    Refused(String),
    /// The dōjō does not know the repository (not registered there).
    NotRegistered(String),
    /// The dōjō could not be asked, or failed: the reason.
    Unavailable(String),
}

/// The two dōjō calls the repository surface makes — a seam, so a test can
/// script the dōjō: the person's repositories as the dōjō rules on them
/// (`GET /v1/you/repositories`: `elected`, `sync_enabled`, `configurable_by_me`, …)
/// and one election (`PATCH /v1/you/repositories/election`).
#[allow(dead_code)] // pending: wp8-daemon (used by the handlers in phase B)
#[async_trait::async_trait]
pub(crate) trait RepositoryPlane: Send + Sync {
    async fn list(&self, token: &str) -> Result<Vec<serde_json::Value>, String>;
    async fn set_election(&self, token: &str, repo_key: &str, elected: bool) -> ElectionOutcome;
}

/// The dōjō's HTTP status and body, as an election outcome. pending: wp8-daemon
#[allow(dead_code)] // pending: wp8-daemon (used by patch_repository in phase B)
pub(crate) fn classify_election(_status: u16, _body: &str) -> ElectionOutcome {
    ElectionOutcome::Unavailable("pending: wp8-daemon".into())
}

/// Record `elected` for the local repository `repo_key` in the dōjō, through
/// `plane`, as the person `token` signs for. pending: wp8-daemon
#[allow(dead_code)] // pending: wp8-daemon (used by patch_repository in phase B)
pub(crate) async fn elect(
    _pg: &crate::db::pg_store::PgStore,
    _plane: &dyn RepositoryPlane,
    _token: &str,
    _repo_key: &str,
    _elected: bool,
) -> Result<Json<serde_json::Value>, ApiError> {
    Err(err(StatusCode::NOT_IMPLEMENTED, "pending: wp8-daemon"))
}

/// The person's repositories as the dōjō rules on them, for the app's sync
/// toggle (shown only where `configurable_by_me`). pending: wp8-daemon
#[allow(dead_code)] // pending: wp8-daemon (used by GET /api/dojo/repositories in phase B)
pub(crate) async fn my_repositories(
    _plane: &dyn RepositoryPlane,
    _token: &str,
) -> Result<Json<serde_json::Value>, ApiError> {
    Err(err(StatusCode::NOT_IMPLEMENTED, "pending: wp8-daemon"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(v: serde_json::Value) -> Json<serde_json::Value> {
        Json(v)
    }

    /// The shared `AppState` fixture. DB-backed: `make_ctx` connects to
    /// `sensei_test` and panics if it cannot, so these tests need the daemon
    /// database running — there is no graceful skip.
    async fn state() -> AppState {
        crate::tasks::test_support::make_ctx().await.app_state.clone()
    }

    fn origin(v: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(axum::http::header::ORIGIN, v.parse().unwrap());
        h
    }

    #[tokio::test]
    async fn a_foreign_origin_cannot_flip_the_sharing_gate() {
        // The whole daemon serves allow_origin(Any), so before this guard any page
        // the user visited could start sharing a repository's metrics and read the
        // reply. Verified live against the running daemon.
        let st = state().await;
        let key = format!("ztest-host/acme/{}", uuid::Uuid::new_v4());
        sqlx_core::query::query(
            "INSERT INTO sensei.repositories(repo_key, name) VALUES($1, 'ztest-repo')",
        )
        .bind(&key)
        .execute(st.pg.pool())
        .await
        .unwrap();

        let (status, _) = patch_repository(
            State(st.clone()),
            Path(key.clone()),
            origin("https://evil.example"),
            body(serde_json::json!({ "visibility": "shared" })),
        )
        .await
        .unwrap_err();
        let still: (String,) = sqlx_core::query_as::query_as(
            "SELECT visibility::text FROM sensei.repositories WHERE repo_key = $1",
        )
        .bind(&key)
        .fetch_one(st.pg.pool())
        .await
        .unwrap();

        sqlx_core::query::query("DELETE FROM sensei.repositories WHERE repo_key = $1")
            .bind(&key)
            .execute(st.pg.pool())
            .await
            .unwrap();

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(still.0, "private", "a refused request must not have changed anything");
    }

    #[test]
    fn a_native_client_sends_no_origin_and_is_allowed() {
        // The CLI, the MCP server and hooks use reqwest/curl — no Origin at all.
        // Rejecting a missing Origin would break every non-browser caller, and
        // forging its absence buys nothing: the attack needs a browser.
        assert!(origin_may_mutate(&HeaderMap::new()));
        assert!(origin_may_mutate(&origin("tauri://localhost")), "the bundled app");
        assert!(origin_may_mutate(&origin("http://localhost:5173")), "the dev app");
        assert!(!origin_may_mutate(&origin("https://evil.example")));
        assert!(!origin_may_mutate(&origin("http://localhost:5174")), "a near-miss port");
    }

    #[tokio::test]
    async fn a_visibility_outside_the_enum_is_a_400_not_a_500() {
        // `public` is a real value of the OTHER visibility enum
        // (`namespace_visibility`), so it is the exact mistake a caller makes.
        // Reaching the database would answer 500 with a cast error; the user
        // needs to read "must be one of private, shared".
        let (status, b) = patch_repository(
            State(state().await),
            Path("host/org/repo".to_string()),
            HeaderMap::new(),
            body(serde_json::json!({ "visibility": "public" })),
        )
        .await
        .unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let msg = b.0["error"].as_str().unwrap_or_default();
        assert!(msg.contains("private"), "the message must name the legal values, got {msg}");
    }

    #[tokio::test]
    async fn a_missing_visibility_is_rejected_rather_than_defaulted() {
        // Defaulting an absent field would silently share (or unshare) a
        // repository on an empty PATCH.
        let (status, _) = patch_repository(
            State(state().await),
            Path("host/org/repo".to_string()),
            HeaderMap::new(),
            body(serde_json::json!({})),
        )
        .await
        .unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn an_unknown_repository_is_a_404_and_creates_nothing() {
        let key = format!("ztest-host/never/{}", uuid::Uuid::new_v4());
        let st = state().await;
        let (status, _) = patch_repository(
            State(st.clone()),
            Path(key.clone()),
            HeaderMap::new(),
            body(serde_json::json!({ "visibility": "shared" })),
        )
        .await
        .unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);

        let n: (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.repositories WHERE repo_key = $1",
        )
        .bind(&key)
        .fetch_one(st.pg.pool())
        .await
        .unwrap();
        assert_eq!(n.0, 0, "a 404 must not have invented a repository");
    }

    #[tokio::test]
    async fn sharing_a_repository_is_what_gate_1_reads() {
        // The end-to-end point of the handler: after this call
        // `shared_repositories` (gate 1) yields the repository, and after the
        // reverse call it does not.
        let st = state().await;
        let key = format!("ztest-host/acme/{}", uuid::Uuid::new_v4());
        sqlx_core::query::query(
            "INSERT INTO sensei.repositories(repo_key, name) VALUES($1, 'ztest-repo')",
        )
        .bind(&key)
        .execute(st.pg.pool())
        .await
        .unwrap();

        let shared = patch_repository(
            State(st.clone()),
            Path(key.clone()),
            HeaderMap::new(),
            body(serde_json::json!({ "visibility": "shared" })),
        )
        .await;
        let gate_after_share = st.pg.shared_repositories(1000).await.unwrap();
        let revoked = patch_repository(
            State(st.clone()),
            Path(key.clone()),
            HeaderMap::new(),
            body(serde_json::json!({ "visibility": "private" })),
        )
        .await;
        let gate_after_revoke = st.pg.shared_repositories(1000).await.unwrap();

        sqlx_core::query::query("DELETE FROM sensei.repositories WHERE repo_key = $1")
            .bind(&key)
            .execute(st.pg.pool())
            .await
            .unwrap();

        assert_eq!(shared.expect("200").0["visibility"], "shared");
        assert!(gate_after_share.iter().any(|r| r.repo_key == key), "gate 1 yields it");
        assert_eq!(revoked.expect("200").0["visibility"], "private");
        assert!(!gate_after_revoke.iter().any(|r| r.repo_key == key), "and stops yielding it");
    }

    // ── WP8 daemon side (spec: docs/plan/dojo-sync-handoff.md WP8; review
    // reviews/L3/wp8-tests-r1.md, L0 rulings) ───────────────────────────────

    /// A scripted dōjō: answers every election with `answer`, and records each call.
    struct ScriptedPlane {
        answer: ElectionOutcome,
        listed: Result<Vec<serde_json::Value>, String>,
        calls: std::sync::Mutex<Vec<(String, String, bool)>>,
    }

    impl ScriptedPlane {
        fn answering(answer: ElectionOutcome) -> Self {
            Self { answer, listed: Ok(vec![]), calls: Default::default() }
        }
        fn listing(listed: Result<Vec<serde_json::Value>, String>) -> Self {
            Self { listed, ..Self::answering(ElectionOutcome::Unavailable("not asked".into())) }
        }
        fn calls(&self) -> Vec<(String, String, bool)> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl RepositoryPlane for ScriptedPlane {
        async fn list(&self, _token: &str) -> Result<Vec<serde_json::Value>, String> {
            self.listed.clone()
        }
        async fn set_election(
            &self,
            token: &str,
            repo_key: &str,
            elected: bool,
        ) -> ElectionOutcome {
            self.calls.lock().unwrap().push((token.into(), repo_key.into(), elected));
            self.answer.clone()
        }
    }

    /// A local repository row; returns its key.
    async fn local_repo(pg: &crate::db::pg_store::PgStore) -> String {
        let key = format!("ztest-host/acme/{}", uuid::Uuid::new_v4());
        sqlx_core::query::query(
            "INSERT INTO sensei.repositories(repo_key, name) VALUES($1, 'ztest-repo')",
        )
        .bind(&key)
        .execute(pg.pool())
        .await
        .unwrap();
        key
    }

    async fn local_visibility(pg: &crate::db::pg_store::PgStore, key: &str) -> String {
        let (v,): (String,) = sqlx_core::query_as::query_as(
            "SELECT visibility::text FROM sensei.repositories WHERE repo_key = $1",
        )
        .bind(key)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        v
    }

    /// The person may change it (their own repository, or an org's as its admin):
    /// the election goes to the dōjō with their session, the dōjō's answer comes
    /// back, and nothing local is written — the local visibility gate is retired.
    #[tokio::test]
    #[ignore = "pending: wp8-daemon"]
    async fn patch_visibility_writes_the_election_where_the_person_may() {
        let st = state().await;
        let key = local_repo(&st.pg).await;
        let plane = ScriptedPlane::answering(ElectionOutcome::Recorded(serde_json::json!({
            "repo_key": key, "elected": false, "sync_enabled": false,
        })));
        let Json(out) = elect(&st.pg, &plane, "TOKEN", &key, false).await.expect("200");
        assert_eq!(plane.calls(), vec![("TOKEN".to_string(), key.clone(), false)]);
        assert_eq!(out["repo_key"], serde_json::json!(key));
        assert_eq!(out["elected"], serde_json::json!(false));
        assert_eq!(out["sync_enabled"], serde_json::json!(false));
        assert_eq!(local_visibility(&st.pg, &key).await, "private", "nothing local is written");
    }

    /// An organisation's public repository is the organisation's (D46): a member
    /// who is not its admin is refused by the dōjō, and the daemon answers 409 with
    /// the dōjō's reason — never a 200, and nothing local changes.
    #[tokio::test]
    #[ignore = "pending: wp8-daemon"]
    async fn patch_visibility_on_an_org_public_repo_is_refused() {
        let st = state().await;
        let key = local_repo(&st.pg).await;
        let plane = ScriptedPlane::answering(ElectionOutcome::Refused(
            "only the organisation's admin may change this".into(),
        ));
        let (status, Json(b)) = elect(&st.pg, &plane, "TOKEN", &key, false).await.expect_err("409");
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(
            b,
            serde_json::json!({
                "error": "not_yours_to_change", "repo_key": key,
                "reason": "only the organisation's admin may change this",
            })
        );
        assert_eq!(local_visibility(&st.pg, &key).await, "private");
    }

    /// A repository the dōjō has not registered cannot be elected: 409 that says so.
    #[tokio::test]
    #[ignore = "pending: wp8-daemon"]
    async fn a_repository_the_dojo_does_not_know_is_409_not_registered() {
        let st = state().await;
        let key = local_repo(&st.pg).await;
        let plane =
            ScriptedPlane::answering(ElectionOutcome::NotRegistered("unknown repository".into()));
        let (status, Json(b)) = elect(&st.pg, &plane, "TOKEN", &key, true).await.expect_err("409");
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(b, serde_json::json!({"error": "not_registered", "repo_key": key}));
    }

    /// The dōjō could not be asked: 502 with the reason — never a 200 that reads as
    /// "recorded".
    #[tokio::test]
    #[ignore = "pending: wp8-daemon"]
    async fn an_unreachable_dojo_is_502() {
        let st = state().await;
        let key = local_repo(&st.pg).await;
        let plane =
            ScriptedPlane::answering(ElectionOutcome::Unavailable("connection refused".into()));
        let (status, Json(b)) = elect(&st.pg, &plane, "TOKEN", &key, true).await.expect_err("502");
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(b["error"], serde_json::json!("connection refused"));
    }

    /// A repository this daemon has never seen is a 404, and the dōjō is not asked.
    #[tokio::test]
    #[ignore = "pending: wp8-daemon"]
    async fn an_unknown_local_repository_is_404_and_asks_no_dojo() {
        let st = state().await;
        let plane = ScriptedPlane::answering(ElectionOutcome::Recorded(serde_json::json!({})));
        let key = format!("ztest-host/never/{}", uuid::Uuid::new_v4());
        let (status, _) = elect(&st.pg, &plane, "TOKEN", &key, true).await.expect_err("404");
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(plane.calls().is_empty(), "the dōjō was not asked");
    }

    /// The dōjō's status decides the outcome; the body's reason is carried.
    #[test]
    #[ignore = "pending: wp8-daemon"]
    fn the_dojo_s_status_decides_the_election_outcome() {
        let ok = classify_election(200, r#"{"repo_key":"k","elected":true,"sync_enabled":true}"#);
        assert_eq!(
            ok,
            ElectionOutcome::Recorded(
                serde_json::json!({"repo_key":"k","elected":true,"sync_enabled":true})
            )
        );
        assert_eq!(
            classify_election(403, r#"{"error":"only an admin may change this"}"#),
            ElectionOutcome::Refused("only an admin may change this".into())
        );
        assert_eq!(
            classify_election(404, r#"{"error":"unknown repository"}"#),
            ElectionOutcome::NotRegistered("unknown repository".into())
        );
        assert!(matches!(classify_election(500, "boom"), ElectionOutcome::Unavailable(_)));
        assert!(
            matches!(classify_election(200, "not json"), ElectionOutcome::Unavailable(_)),
            "an unreadable 200 is not a recorded election"
        );
    }

    /// The request body is `{persona, elected}`: `elected` strictly a boolean (a
    /// string "false" must not turn sharing on), `persona` required — and the old
    /// `{visibility}` body is refused, since the local gate is retired.
    #[tokio::test]
    #[ignore = "pending: wp8-daemon"]
    async fn the_body_is_persona_and_a_strict_boolean() {
        let st = state().await;
        let key = local_repo(&st.pg).await;
        for bad in [
            serde_json::json!({"persona": "p", "elected": "false"}),
            serde_json::json!({"elected": true}),
            serde_json::json!({"visibility": "shared"}),
        ] {
            let (status, _) = patch_repository(
                State(st.clone()),
                Path(key.clone()),
                HeaderMap::new(),
                body(bad.clone()),
            )
            .await
            .expect_err("400");
            assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
        }
        assert_eq!(
            local_visibility(&st.pg, &key).await,
            "private",
            "a refused body writes nothing"
        );
    }

    /// The person's repositories, as the dōjō rules on them, reach the app as they
    /// are — `configurable_by_me` included, which is what shows or hides the
    /// toggle (L0 ruling: the app toggle is L2's, inside WP8).
    #[tokio::test]
    #[ignore = "pending: wp8-daemon"]
    async fn the_dojo_s_repositories_reach_the_app_with_who_may_change_them() {
        let rows = vec![
            serde_json::json!({"repo_key": "github.com/acme/web", "elected": true, "sync_enabled": true, "configurable_by_me": false}),
            serde_json::json!({"repo_key": "github.com/me/notes", "elected": false, "sync_enabled": false, "configurable_by_me": true}),
        ];
        let plane = ScriptedPlane::listing(Ok(rows.clone()));
        let Json(out) = my_repositories(&plane, "TOKEN").await.expect("200");
        assert_eq!(out, serde_json::json!({ "repositories": rows }));
    }

    /// A list the dōjō could not give is a 502 — never an empty list, which would
    /// read as "nothing to share" and hide every toggle.
    #[tokio::test]
    #[ignore = "pending: wp8-daemon"]
    async fn an_unreadable_repository_list_is_502_not_empty() {
        let plane = ScriptedPlane::listing(Err("dōjō returned 500".into()));
        let (status, Json(b)) = my_repositories(&plane, "TOKEN").await.expect_err("502");
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(b["error"], serde_json::json!("dōjō returned 500"));
    }
}
