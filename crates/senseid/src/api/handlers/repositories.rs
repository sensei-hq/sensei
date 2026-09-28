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
use crate::api::state::AppState;

type ApiError = (StatusCode, Json<serde_json::Value>);

/// Origins allowed to flip the sharing gate from a browser.
///
/// The daemon serves `CorsLayer::allow_origin(Any)` — deliberately, because the
/// desktop app is itself cross-origin and Safari rejects wildcard method/header
/// lists. That is tolerable for reads. It is NOT tolerable for this route, whose
/// effect is to start sending a repository's metrics off the machine: any page the
/// user visits could `fetch(...)` it, flip the gate, and read the reply — which
/// doubles as an inventory oracle over their local repositories. Verified live
/// against the running daemon before this guard existed.
const ALLOWED_ORIGINS: &[&str] = &[
    // Bundled desktop app (Tauri).
    "tauri://localhost",
    // `devUrl` from app/src-tauri/tauri.conf.json, both host spellings.
    "http://localhost:5173",
    "http://127.0.0.1:5173",
];

/// Whether a caller may mutate, judged only by `Origin`.
///
/// **No `Origin` header means ALLOW.** That is not a loophole: browsers always
/// send `Origin` on a cross-origin request, and the daemon's non-browser clients
/// — the CLI, the MCP server, hooks, `curl` — send none. A forged absence buys an
/// attacker nothing, because the attack requires a browser to make the request.
fn origin_may_mutate(headers: &HeaderMap) -> bool {
    match headers.get(axum::http::header::ORIGIN) {
        None => true,
        Some(o) => o.to_str().map(|o| ALLOWED_ORIGINS.contains(&o)).unwrap_or(false),
    }
}

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
}
