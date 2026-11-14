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
// pending: wp8-daemon — the handlers move onto these in phase B:
// `PATCH /api/repositories/{key}` writes the dōjō's repository election
// (`PATCH /v1/you/repositories/election`) where the person may change it and
// answers 409 where they may not; `GET /api/dojo/repositories?persona=` gives the
// app the dōjō's rulings (`configurable_by_me`). The local `visibility` gate is
// retired: nothing local is gated on the plan, and nothing local is written.

/// What the dōjō answered an election.
#[allow(dead_code)] // pending: wp8-daemon
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ElectionOutcome {
    /// Recorded: the dōjō's own answer (`elected`, `sync_enabled`, …).
    Recorded(serde_json::Value),
    /// 403 — not this person's to change (an organisation's repository, and not
    /// its admin): the dōjō's reason.
    Refused(String),
    /// 404 — the dōjō does not know the repository (not registered there).
    NotRegistered(String),
    /// 409 — the dōjō cannot decide it yet (no authority: its forge visibility is
    /// not captured): the dōjō's reason.
    NotDecidable(String),
    /// 401 — the dōjō rejected the person's session: sign in again.
    SessionRejected(String),
    /// The dōjō could not be asked, or failed: the reason.
    Unavailable(String),
}

/// Why the dōjō's rulings could not be read.
#[allow(dead_code)] // pending: wp8-daemon
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ListFailure {
    /// 401 — sign in again.
    SessionRejected(String),
    /// Anything else, with the reason.
    Unavailable(String),
}

/// The two dōjō calls the repository surface makes — a seam, so a test can
/// script the dōjō: the person's repositories as the dōjō rules on them
/// (`GET /v1/you/repositories`: `elected`, `sync_enabled`, `configurable_by_me`, …)
/// and one election (`PATCH /v1/you/repositories/election`).
#[allow(dead_code)] // pending: wp8-daemon
#[async_trait::async_trait]
pub(crate) trait RepositoryPlane: Send + Sync {
    async fn list(&self, token: &str) -> Result<Vec<serde_json::Value>, ListFailure>;
    async fn set_election(&self, token: &str, repo_key: &str, elected: bool) -> ElectionOutcome;
}

/// Where a handler gets the person's session token — a seam, so a test can sign
/// out (the real one is `auth::live_access_token`).
#[allow(dead_code)] // pending: wp8-daemon
#[async_trait::async_trait]
pub(crate) trait SessionTokens: Send + Sync {
    async fn token(&self, persona: &str) -> Result<String, crate::api::handlers::auth::AuthError>;
}

/// The dōjō's HTTP status and body, as an election outcome. pending: wp8-daemon
#[allow(dead_code)] // pending: wp8-daemon
pub(crate) fn classify_election(_status: u16, _body: &str) -> ElectionOutcome {
    ElectionOutcome::Unavailable("pending: wp8-daemon".into())
}

/// The dōjō's HTTP status and body, as the person's repository rulings.
/// pending: wp8-daemon
#[allow(dead_code)] // pending: wp8-daemon
pub(crate) fn classify_list(
    _status: u16,
    _body: &str,
) -> Result<Vec<serde_json::Value>, ListFailure> {
    Err(ListFailure::Unavailable("pending: wp8-daemon".into()))
}

/// The election request: `PATCH {dojo_url}/v1/you/repositories/election`, the
/// person's bearer on this request only. pending: wp8-daemon
#[allow(dead_code)] // pending: wp8-daemon
pub(crate) fn election_request(
    client: &reqwest::Client,
    _dojo_url: &str,
    _token: &str,
    _repo_key: &str,
    _elected: bool,
) -> reqwest::Result<reqwest::Request> {
    client.get("http://pending.invalid/wp8-daemon").build()
}

/// The rulings request: `GET {dojo_url}/v1/you/repositories`, the person's bearer
/// on this request only. pending: wp8-daemon
#[allow(dead_code)] // pending: wp8-daemon
pub(crate) fn list_request(
    client: &reqwest::Client,
    _dojo_url: &str,
    _token: &str,
) -> reqwest::Result<reqwest::Request> {
    client.get("http://pending.invalid/wp8-daemon").build()
}

/// Record `elected` for the local repository `repo_key` in the dōjō, through
/// `plane`, as the person `token` signs for. pending: wp8-daemon
#[allow(dead_code)] // pending: wp8-daemon
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
#[allow(dead_code)] // pending: wp8-daemon
pub(crate) async fn my_repositories(
    _plane: &dyn RepositoryPlane,
    _token: &str,
) -> Result<Json<serde_json::Value>, ApiError> {
    Err(err(StatusCode::NOT_IMPLEMENTED, "pending: wp8-daemon"))
}

/// `PATCH /api/repositories/{key}` with its seams: the origin guard, the
/// `{persona, elected}` body, the person's session, then [`elect`]. A session that
/// cannot be had answers as `credential_status` does, and the dōjō is not asked.
/// pending: wp8-daemon
#[allow(dead_code)] // pending: wp8-daemon
pub(crate) async fn patch_repository_with(
    _state: &AppState,
    _repo_key: &str,
    _headers: &HeaderMap,
    _body: &serde_json::Value,
    _tokens: &dyn SessionTokens,
    _plane: &dyn RepositoryPlane,
) -> Result<Json<serde_json::Value>, ApiError> {
    Err(err(StatusCode::NOT_IMPLEMENTED, "pending: wp8-daemon"))
}

/// `GET /api/dojo/repositories?persona=` with its seams: the persona (required),
/// the person's session, then [`my_repositories`]. pending: wp8-daemon
#[allow(dead_code)] // pending: wp8-daemon
pub(crate) async fn my_repositories_with(
    _persona: Option<&str>,
    _tokens: &dyn SessionTokens,
    _plane: &dyn RepositoryPlane,
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

    // ── WP8 daemon side (spec: docs/plan/dojo-sync-handoff.md WP8; reviews
    // reviews/L3/wp8-tests-r1.md, reviews/L2/wp8-daemon-tests-r2.md + L0 rulings) ──

    use crate::api::handlers::auth::AuthError;

    /// A scripted dōjō: answers every election with `answer` and every list with
    /// `listed`, and records each call.
    struct ScriptedPlane {
        answer: ElectionOutcome,
        listed: Result<Vec<serde_json::Value>, ListFailure>,
        calls: std::sync::Mutex<Vec<(String, String, bool)>>,
        lists: std::sync::Mutex<u32>,
    }

    impl ScriptedPlane {
        fn answering(answer: ElectionOutcome) -> Self {
            Self {
                answer,
                listed: Ok(vec![]),
                calls: Default::default(),
                lists: Default::default(),
            }
        }
        fn listing(listed: Result<Vec<serde_json::Value>, ListFailure>) -> Self {
            Self { listed, ..Self::answering(ElectionOutcome::Unavailable("not asked".into())) }
        }
        fn calls(&self) -> Vec<(String, String, bool)> {
            self.calls.lock().unwrap().clone()
        }
        fn lists(&self) -> u32 {
            *self.lists.lock().unwrap()
        }
    }

    #[async_trait::async_trait]
    impl RepositoryPlane for ScriptedPlane {
        async fn list(&self, _token: &str) -> Result<Vec<serde_json::Value>, ListFailure> {
            *self.lists.lock().unwrap() += 1;
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

    /// A session source: every persona gets `answer`.
    struct FakeTokens(Result<String, AuthError>);

    #[async_trait::async_trait]
    impl SessionTokens for FakeTokens {
        async fn token(&self, _persona: &str) -> Result<String, AuthError> {
            match &self.0 {
                Ok(t) => Ok(t.clone()),
                Err(AuthError::SignedOut) => Err(AuthError::SignedOut),
                Err(AuthError::Unreachable(r)) => Err(AuthError::Unreachable(r.clone())),
                Err(_) => Err(AuthError::SignedOut),
            }
        }
    }

    /// A local repository row with `visibility` seeded; returns its key.
    async fn local_repo_with(pg: &crate::db::pg_store::PgStore, visibility: &str) -> String {
        let key = format!("ztest-host/acme/{}", uuid::Uuid::new_v4());
        sqlx_core::query::query(
            "INSERT INTO sensei.repositories(repo_key, name, visibility)
             VALUES($1, 'ztest-repo', $2::sensei.repo_visibility)",
        )
        .bind(&key)
        .bind(visibility)
        .execute(pg.pool())
        .await
        .unwrap();
        key
    }

    async fn local_repo(pg: &crate::db::pg_store::PgStore) -> String {
        local_repo_with(pg, "private").await
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
    /// back, and nothing local is written — in either direction (review r2 HIGH-1:
    /// electing `true` must not set the retired gate, electing `false` must not
    /// clear it).
    #[tokio::test]
    #[ignore = "pending: wp8-daemon"]
    async fn patch_visibility_writes_the_election_where_the_person_may() {
        let st = state().await;
        for (seeded, elected) in [("private", true), ("shared", false)] {
            let key = local_repo_with(&st.pg, seeded).await;
            let plane = ScriptedPlane::answering(ElectionOutcome::Recorded(serde_json::json!({
                "repo_key": key, "elected": elected, "sync_enabled": elected,
            })));
            let Json(out) = elect(&st.pg, &plane, "TOKEN", &key, elected).await.expect("200");
            assert_eq!(plane.calls(), vec![("TOKEN".to_string(), key.clone(), elected)]);
            assert_eq!(out["repo_key"], serde_json::json!(key));
            assert_eq!(out["elected"], serde_json::json!(elected));
            assert_eq!(out["sync_enabled"], serde_json::json!(elected));
            assert_eq!(local_visibility(&st.pg, &key).await, seeded, "nothing local is written");
        }
    }

    /// An organisation's public repository is the organisation's (D46): a member
    /// who is not its admin is refused by the dōjō, and the daemon answers 409 with
    /// the dōjō's reason — never a 200, and nothing local changes.
    #[tokio::test]
    #[ignore = "pending: wp8-daemon"]
    async fn patch_visibility_on_an_org_public_repo_is_refused() {
        let st = state().await;
        let key = local_repo_with(&st.pg, "shared").await;
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
        assert_eq!(local_visibility(&st.pg, &key).await, "shared", "nothing local changes");
    }

    /// A repository the dōjō has not registered cannot be elected: 409 that says
    /// so, and nothing local is written.
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
        assert_eq!(local_visibility(&st.pg, &key).await, "private");
    }

    /// The dōjō cannot decide it yet — no authority until its forge visibility is
    /// captured (L0 ruling r2 MED-1): 409 `not_decidable_yet` with the dōjō's
    /// reason, never a 502 outage.
    #[tokio::test]
    #[ignore = "pending: wp8-daemon"]
    async fn a_repository_the_dojo_cannot_decide_yet_is_409_not_decidable_yet() {
        let st = state().await;
        let key = local_repo(&st.pg).await;
        let reason = format!("{key} has no authority yet: forge visibility not captured");
        let plane = ScriptedPlane::answering(ElectionOutcome::NotDecidable(reason.clone()));
        let (status, Json(b)) = elect(&st.pg, &plane, "TOKEN", &key, true).await.expect_err("409");
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(
            b,
            serde_json::json!({"error": "not_decidable_yet", "repo_key": key, "reason": reason})
        );
        assert_eq!(local_visibility(&st.pg, &key).await, "private");
    }

    /// The dōjō rejected the person's session (its 401): the sign-in error, never
    /// a 502 (L0 ruling r2 MED-1).
    #[tokio::test]
    #[ignore = "pending: wp8-daemon"]
    async fn a_session_the_dojo_rejects_is_401_sign_in_again() {
        let st = state().await;
        let key = local_repo(&st.pg).await;
        let plane =
            ScriptedPlane::answering(ElectionOutcome::SessionRejected("token expired".into()));
        let (status, Json(b)) = elect(&st.pg, &plane, "TOKEN", &key, true).await.expect_err("401");
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            b,
            serde_json::json!({"error": "session_rejected", "repo_key": key, "reason": "token expired"})
        );
    }

    /// The dōjō could not be asked: 502 with a code and the reason — never a 200
    /// that reads as "recorded", and nothing local is written.
    #[tokio::test]
    #[ignore = "pending: wp8-daemon"]
    async fn an_unreachable_dojo_is_502() {
        let st = state().await;
        let key = local_repo(&st.pg).await;
        let plane =
            ScriptedPlane::answering(ElectionOutcome::Unavailable("connection refused".into()));
        let (status, Json(b)) = elect(&st.pg, &plane, "TOKEN", &key, true).await.expect_err("502");
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(
            b,
            serde_json::json!({"error": "dojo_unavailable", "repo_key": key, "reason": "connection refused"})
        );
        assert_eq!(local_visibility(&st.pg, &key).await, "private");
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

    /// The dōjō's status decides the outcome; the body's reason is carried, and a
    /// refusal is never reasonless.
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
        match classify_election(403, "{}") {
            ElectionOutcome::Refused(r) => {
                assert!(!r.trim().is_empty(), "a refusal carries a reason")
            }
            other => panic!("403 is a refusal, got {other:?}"),
        }
        assert_eq!(
            classify_election(404, r#"{"error":"unknown repository"}"#),
            ElectionOutcome::NotRegistered("unknown repository".into())
        );
        assert_eq!(
            classify_election(409, r#"{"error":"k has no authority yet: not captured"}"#),
            ElectionOutcome::NotDecidable("k has no authority yet: not captured".into())
        );
        assert_eq!(
            classify_election(401, r#"{"error":"token expired"}"#),
            ElectionOutcome::SessionRejected("token expired".into())
        );
        assert!(matches!(classify_election(500, "boom"), ElectionOutcome::Unavailable(_)));
        assert!(
            matches!(classify_election(200, "not json"), ElectionOutcome::Unavailable(_)),
            "an unreadable 200 is not a recorded election"
        );
    }

    /// The rulings read: the dōjō's `{repositories}` as given; a rejected session
    /// is a sign-in failure; anything else, or a body it cannot read, is unavailable.
    #[test]
    #[ignore = "pending: wp8-daemon"]
    fn the_dojo_s_status_decides_the_rulings_read() {
        let rows = serde_json::json!([{"repo_key": "k", "elected": true, "sync_enabled": true, "configurable_by_me": true}]);
        assert_eq!(
            classify_list(200, &serde_json::json!({ "repositories": rows }).to_string()),
            Ok(rows.as_array().unwrap().clone())
        );
        assert_eq!(
            classify_list(401, r#"{"error":"token expired"}"#),
            Err(ListFailure::SessionRejected("token expired".into()))
        );
        assert!(matches!(classify_list(500, "boom"), Err(ListFailure::Unavailable(_))));
        assert!(
            matches!(classify_list(200, "{}"), Err(ListFailure::Unavailable(_))),
            "no list is not an empty list"
        );
        assert!(matches!(classify_list(200, "not json"), Err(ListFailure::Unavailable(_))));
    }

    /// The person's session goes to its own dōjō only (review r2 MED-2): the
    /// requests address `{dojo_url}/v1/you/repositories[/election]` and carry the
    /// bearer, with the election's body exactly `{repo_key, elected}`.
    #[test]
    #[ignore = "pending: wp8-daemon"]
    fn the_requests_go_to_the_person_s_own_dojo_with_their_bearer() {
        let client = reqwest::Client::new();
        let r = election_request(
            &client,
            "https://dojo.example",
            "TOKEN",
            "github.com/me/notes",
            false,
        )
        .unwrap();
        assert_eq!(r.method(), reqwest::Method::PATCH);
        assert_eq!(r.url().as_str(), "https://dojo.example/v1/you/repositories/election");
        assert_eq!(r.headers().get("authorization").unwrap(), "Bearer TOKEN");
        let body: serde_json::Value =
            serde_json::from_slice(r.body().and_then(|b| b.as_bytes()).expect("a JSON body"))
                .unwrap();
        assert_eq!(body, serde_json::json!({"repo_key": "github.com/me/notes", "elected": false}));

        let l = list_request(&client, "https://dojo.example", "TOKEN").unwrap();
        assert_eq!(l.method(), reqwest::Method::GET);
        assert_eq!(l.url().as_str(), "https://dojo.example/v1/you/repositories");
        assert_eq!(l.headers().get("authorization").unwrap(), "Bearer TOKEN");
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

    /// A persona with no usable session (review r2 MED-2): signed out is 401,
    /// unreachable is 503 — as `credential_status` answers — and the dōjō is never
    /// asked.
    #[tokio::test]
    #[ignore = "pending: wp8-daemon"]
    async fn a_persona_with_no_session_is_refused_and_the_dojo_is_not_asked() {
        let st = state().await;
        let key = local_repo(&st.pg).await;
        let ask = serde_json::json!({"persona": "slot-1", "elected": true});
        for (tokens, want) in [
            (FakeTokens(Err(AuthError::SignedOut)), StatusCode::UNAUTHORIZED),
            (
                FakeTokens(Err(AuthError::Unreachable("offline".into()))),
                StatusCode::SERVICE_UNAVAILABLE,
            ),
        ] {
            let plane = ScriptedPlane::answering(ElectionOutcome::Recorded(serde_json::json!({})));
            let (status, _) =
                patch_repository_with(&st, &key, &HeaderMap::new(), &ask, &tokens, &plane)
                    .await
                    .expect_err("no session");
            assert_eq!(status, want);
            assert!(plane.calls().is_empty(), "the dōjō was not asked");

            let lister = ScriptedPlane::listing(Ok(vec![]));
            let (status, _) = my_repositories_with(Some("slot-1"), &tokens, &lister)
                .await
                .expect_err("no session");
            assert_eq!(status, want);
            assert_eq!(lister.lists(), 0, "the dōjō was not asked");
        }
        let lister = ScriptedPlane::listing(Ok(vec![]));
        let (status, _) = my_repositories_with(None, &FakeTokens(Ok("T".into())), &lister)
            .await
            .expect_err("400");
        assert_eq!(status, StatusCode::BAD_REQUEST, "a persona is required");
        assert_eq!(lister.lists(), 0);
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
    /// read as "nothing to share" and hide every toggle; a rejected session is the
    /// sign-in error.
    #[tokio::test]
    #[ignore = "pending: wp8-daemon"]
    async fn an_unreadable_repository_list_is_502_not_empty() {
        let plane =
            ScriptedPlane::listing(Err(ListFailure::Unavailable("dōjō returned 500".into())));
        let (status, Json(b)) = my_repositories(&plane, "TOKEN").await.expect_err("502");
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(
            b,
            serde_json::json!({"error": "dojo_unavailable", "reason": "dōjō returned 500"})
        );

        let plane =
            ScriptedPlane::listing(Err(ListFailure::SessionRejected("token expired".into())));
        let (status, Json(b)) = my_repositories(&plane, "TOKEN").await.expect_err("401");
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(b, serde_json::json!({"error": "session_rejected", "reason": "token expired"}));
    }
}
