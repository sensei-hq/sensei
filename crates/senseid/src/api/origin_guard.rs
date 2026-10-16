//! Request guards for routes a web page must not reach.
//!
//! The daemon serves `CorsLayer::allow_origin(Any)` (the desktop app is itself
//! cross-origin), so any page the user visits can call it. Routes that mutate
//! state or return private data opt into these checks.

use axum::extract::Request;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// Origins allowed to flip the sharing gate from a browser.
///
/// The daemon serves `CorsLayer::allow_origin(Any)` — deliberately, because the
/// desktop app is itself cross-origin and Safari rejects wildcard method/header
/// lists. That is tolerable for reads. It is NOT tolerable for this route, whose
/// effect is to start sending a repository's metrics off the machine: any page the
/// user visits could `fetch(...)` it, flip the gate, and read the reply — which
/// doubles as an inventory oracle over their local repositories. Verified live
/// against the running daemon before this guard existed.
pub(crate) const ALLOWED_ORIGINS: &[&str] = &[
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
pub(crate) fn origin_may_mutate(headers: &HeaderMap) -> bool {
    match headers.get(axum::http::header::ORIGIN) {
        None => true,
        Some(o) => o.to_str().map(|o| ALLOWED_ORIGINS.contains(&o)).unwrap_or(false),
    }
}

/// Whether the request was addressed to this machine's loopback, judged by `Host`.
///
/// Closes DNS rebinding: a page on `evil.example` that re-points its own name at
/// `127.0.0.1` makes requests the browser treats as same-origin, so they carry no
/// `Origin` on a GET — but they still say `Host: evil.example`. A missing `Host`
/// is refused: every HTTP/1.1 client sends one.
pub(crate) fn host_is_loopback(headers: &HeaderMap) -> bool {
    let Some(raw) = headers.get(axum::http::header::HOST).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let raw = raw.trim().to_ascii_lowercase();
    let is_port = |p: &str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
    let host = if let Some(rest) = raw.strip_prefix('[') {
        // `[v6]` or `[v6]:port`
        match rest.split_once(']') {
            Some((inside, "")) => inside.to_string(),
            Some((inside, tail)) if tail.strip_prefix(':').is_some_and(is_port) => {
                inside.to_string()
            }
            _ => return false,
        }
    } else {
        match raw.rsplit_once(':') {
            Some((name, port)) if is_port(port) => name.to_string(),
            Some(_) => return false,
            None => raw.clone(),
        }
    };
    matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1")
}

/// Middleware for the tracker routes: 403 for a foreign `Origin` or a
/// non-loopback `Host`. Reads included — they return client issue summaries.
pub(crate) async fn local_caller_guard(req: Request, next: Next) -> Response {
    if !origin_may_mutate(req.headers()) || !host_is_loopback(req.headers()) {
        // A distinct code: `forbidden` already means "the tracker refused this account".
        return crate::api::handlers::err(StatusCode::FORBIDDEN, "forbidden_caller")
            .into_response();
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(v: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(axum::http::header::HOST, v.parse().unwrap());
        h
    }

    #[test]
    fn loopback_hosts_with_or_without_a_port() {
        for v in [
            "127.0.0.1",
            "127.0.0.1:7744",
            "localhost",
            "localhost:7744",
            "[::1]",
            "[::1]:7744",
            "LOCALHOST:7744",
        ] {
            assert!(host_is_loopback(&host(v)), "{v}");
        }
    }

    #[test]
    fn anything_else_is_not_loopback() {
        for v in [
            "evil.example",
            "evil.example:7744",
            "127.0.0.1.evil.example",
            "localhost.evil.example",
            "10.0.0.5:7744",
            "0.0.0.0:7744",
            "[::2]:7744",
            "127.0.0.2",
        ] {
            assert!(!host_is_loopback(&host(v)), "{v}");
        }
        assert!(!host_is_loopback(&HeaderMap::new()), "a missing Host is refused");
    }

    async fn through_guard(headers: &[(&str, &str)]) -> StatusCode {
        use tower::ServiceExt;
        let app = axum::Router::new()
            .route("/x", axum::routing::get(|| async { "ok" }))
            .route_layer(axum::middleware::from_fn(local_caller_guard));
        let mut req = axum::http::Request::builder().uri("/x");
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        app.oneshot(req.body(axum::body::Body::empty()).unwrap()).await.unwrap().status()
    }

    #[tokio::test]
    async fn the_guard_admits_local_callers_and_the_app() {
        assert_eq!(through_guard(&[("host", "127.0.0.1:7744")]).await, StatusCode::OK, "the CLI");
        assert_eq!(
            through_guard(&[("host", "127.0.0.1:7744"), ("origin", "tauri://localhost")]).await,
            StatusCode::OK,
            "the desktop app"
        );
    }

    #[tokio::test]
    async fn the_guard_refuses_foreign_origins_and_rebound_hosts() {
        assert_eq!(
            through_guard(&[("host", "127.0.0.1:7744"), ("origin", "https://evil.example")]).await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(through_guard(&[("host", "evil.example:7744")]).await, StatusCode::FORBIDDEN);
        assert_eq!(through_guard(&[]).await, StatusCode::FORBIDDEN);
    }
}
