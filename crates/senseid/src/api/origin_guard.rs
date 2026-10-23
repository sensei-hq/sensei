//! Request guards: who may call the daemon at all, and who may change it.
//!
//! Any page the user visits can `fetch` `http://127.0.0.1:7744`. CORS only keeps
//! a foreign page from READING the answer; a simple POST still arrives and runs
//! (#334: a page on `127.0.0.1:5180` added a watch root). So every request goes
//! through [`caller_guard`], and CORS answers only the app's own origins.

use axum::extract::Request;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// The browser origins that are the app: the only ones CORS answers and the
/// only ones whose writes are accepted.
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

/// Whether CORS answers this origin: the app's, and no other.
pub(crate) fn cors_allows(origin: &HeaderValue) -> bool {
    origin.to_str().is_ok_and(|o| ALLOWED_ORIGINS.contains(&o))
}

/// Middleware for the whole daemon: 403 for a non-loopback `Host` (DNS
/// rebinding), and for a write whose `Origin` is not the app's. A read from a
/// foreign page is let through; CORS keeps that page from seeing the answer.
pub(crate) async fn caller_guard(req: Request, next: Next) -> Response {
    let headers = req.headers();
    let reads = matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
    if !host_is_loopback(headers) || !(reads || origin_may_mutate(headers)) {
        tracing::warn!(
            method = %req.method(),
            path = req.uri().path(),
            origin = ?headers.get(axum::http::header::ORIGIN),
            host = ?headers.get(axum::http::header::HOST),
            "refused a request from outside the app (#334)"
        );
        return crate::api::handlers::err(StatusCode::FORBIDDEN, "forbidden_caller")
            .into_response();
    }
    next.run(req).await
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
            "evil-localhost",
            "notlocalhost:7744",
            "x.localhost",
            "my127.0.0.1",
            "127.0.0.1x:7744",
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

    // #334: the daemon-wide guard. Every request must be addressed to loopback;
    // every WRITE from a browser must come from the app.
    async fn through_caller_guard(method: &str, headers: &[(&str, &str)]) -> StatusCode {
        use tower::ServiceExt;
        let app = axum::Router::new()
            .route("/x", axum::routing::get(|| async { "ok" }).post(|| async { "ok" }))
            .layer(axum::middleware::from_fn(caller_guard));
        let mut req = axum::http::Request::builder().method(method).uri("/x");
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        app.oneshot(req.body(axum::body::Body::empty()).unwrap()).await.unwrap().status()
    }

    #[tokio::test]
    async fn a_write_from_a_foreign_page_is_refused_and_from_the_app_or_the_cli_is_not() {
        let local = ("host", "127.0.0.1:7744");
        for foreign in ["https://evil.example", "http://127.0.0.1:5180", "null"] {
            assert_eq!(
                through_caller_guard("POST", &[local, ("origin", foreign)]).await,
                StatusCode::FORBIDDEN,
                "{foreign}"
            );
        }
        assert_eq!(
            through_caller_guard("POST", &[local]).await,
            StatusCode::OK,
            "the CLI and hooks"
        );
        assert_eq!(
            through_caller_guard("POST", &[local, ("origin", "tauri://localhost")]).await,
            StatusCode::OK,
            "the desktop app"
        );
    }

    #[tokio::test]
    async fn a_read_needs_a_loopback_host_whatever_its_origin() {
        assert_eq!(
            through_caller_guard(
                "GET",
                &[("host", "127.0.0.1:7744"), ("origin", "https://evil.example")]
            )
            .await,
            StatusCode::OK,
            "CORS keeps a foreign page from reading the answer"
        );
        assert_eq!(
            through_caller_guard("GET", &[("host", "evil.example:7744")]).await,
            StatusCode::FORBIDDEN,
            "DNS rebinding"
        );
    }

    #[test]
    fn only_the_app_s_origins_get_cors() {
        for o in ALLOWED_ORIGINS {
            assert!(cors_allows(&axum::http::HeaderValue::from_static(o)), "{o}");
        }
        for o in ["https://evil.example", "http://127.0.0.1:5180", "null"] {
            assert!(!cors_allows(&axum::http::HeaderValue::from_static(o)), "{o}");
        }
    }
}
