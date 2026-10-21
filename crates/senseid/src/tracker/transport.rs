//! The shared GET-only transport: status → error mapping and the 429 policy over
//! a one-method raw HTTP seam, so both are testable without a server. (Spec R4.)

use std::time::Duration;

use crate::tracker::{GetTransport, TrackerError};

/// One raw HTTP response, as much of it as the policy needs.
#[derive(Debug, Clone)]
pub struct RawResponse {
    pub status: u16,
    pub retry_after: Option<String>,
    pub body: Vec<u8>,
}

/// One GET, with no policy: the production impl is reqwest with redirects off.
#[async_trait::async_trait]
pub trait RawGet: Send + Sync {
    /// `Err` only for a network-level failure (no response at all).
    async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<RawResponse, ()>;
}

/// Applies the R4 policy over a [`RawGet`]: maps statuses onto [`TrackerError`]
/// and retries a 429 up to `retries` times, waiting [`crate::tracker::http::retry_after`].
pub struct PolicyTransport<R: RawGet> {
    raw: R,
    retries: u8,
}

impl<R: RawGet> PolicyTransport<R> {
    /// `retries = 3` for sync; `0` for connect's `validate` (fail fast).
    pub fn new(raw: R, retries: u8) -> Self {
        PolicyTransport { raw, retries }
    }
}

/// Map a non-429 response onto the result the caller sees.
pub fn map_response(resp: &RawResponse) -> Result<serde_json::Value, TrackerError> {
    match resp.status {
        200..=299 => serde_json::from_slice(&resp.body).map_err(|_| TrackerError::Decode),
        300..=399 => Err(TrackerError::Redirect),
        400 => Err(TrackerError::BadRequest),
        401 => Err(TrackerError::Unauthorized),
        403 => Err(TrackerError::Forbidden),
        404 => Err(TrackerError::NotFound),
        429 => Err(TrackerError::RateLimited),
        _ => Err(TrackerError::HttpError),
    }
}

#[async_trait::async_trait]
impl<R: RawGet> GetTransport for PolicyTransport<R> {
    async fn get_json(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<serde_json::Value, TrackerError> {
        let mut retried = 0u8;
        loop {
            let resp = self.raw.get(path, query).await.map_err(|()| TrackerError::Transport)?;
            if resp.status == 429 && retried < self.retries {
                retried += 1;
                let wait: Duration = crate::tracker::http::retry_after(
                    resp.retry_after.as_deref(),
                    chrono::Utc::now(),
                );
                tokio::time::sleep(wait).await;
                continue;
            }
            return map_response(&resp);
        }
    }
}

/// The production [`RawGet`]: reqwest with the shared bounded timeouts and
/// redirects disabled, so a 3xx is reported, never followed (a followed redirect
/// would carry the Basic credentials to another host).
pub struct ReqwestRaw {
    client: reqwest::Client,
    base_url: String,
    auth: crate::tracker::AuthScheme,
}

impl ReqwestRaw {
    pub fn new(base_url: &str, auth: crate::tracker::AuthScheme) -> Result<Self, TrackerError> {
        let client = crate::http::bounded_builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| TrackerError::Transport)?;
        Ok(ReqwestRaw { client, base_url: base_url.trim_end_matches('/').to_string(), auth })
    }
}

#[async_trait::async_trait]
impl RawGet for ReqwestRaw {
    async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<RawResponse, ()> {
        let req = self.client.get(format!("{}{}", self.base_url, path)).query(query);
        let req = match &self.auth {
            crate::tracker::AuthScheme::Basic { user, token } => {
                req.basic_auth(user, Some(&token.0))
            }
        };
        let resp = req.send().await.map_err(|_| ())?;
        let status = resp.status().as_u16();
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let body = resp.bytes().await.map_err(|_| ())?.to_vec();
        Ok(RawResponse { status, retry_after, body })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Plays back scripted responses and counts calls.
    struct Scripted {
        responses: Mutex<Vec<Result<RawResponse, ()>>>,
        calls: Mutex<u32>,
    }
    impl Scripted {
        fn new(mut rs: Vec<Result<RawResponse, ()>>) -> Self {
            rs.reverse();
            Scripted { responses: Mutex::new(rs), calls: Mutex::new(0) }
        }
        fn calls(&self) -> u32 {
            *self.calls.lock().unwrap()
        }
    }
    struct Ref<'a>(&'a Scripted);
    #[async_trait::async_trait]
    impl RawGet for Ref<'_> {
        async fn get(&self, _p: &str, _q: &[(&str, String)]) -> Result<RawResponse, ()> {
            *self.0.calls.lock().unwrap() += 1;
            self.0.responses.lock().unwrap().pop().expect("a scripted response")
        }
    }
    fn resp(status: u16, retry_after: Option<&str>, body: &str) -> Result<RawResponse, ()> {
        Ok(RawResponse {
            status,
            retry_after: retry_after.map(String::from),
            body: body.as_bytes().to_vec(),
        })
    }

    #[test]
    fn statuses_map_onto_the_error_vocabulary() {
        let ok = map_response(&resp(200, None, r#"{"a":1}"#).unwrap());
        assert_eq!(ok, Ok(serde_json::json!({"a": 1})));
        for (status, err) in [
            (401, TrackerError::Unauthorized),
            (403, TrackerError::Forbidden),
            (404, TrackerError::NotFound),
            (500, TrackerError::HttpError),
            (400, TrackerError::BadRequest),
            (301, TrackerError::Redirect),
            (302, TrackerError::Redirect),
            (429, TrackerError::RateLimited),
        ] {
            assert_eq!(map_response(&resp(status, None, "{}").unwrap()), Err(err), "{status}");
        }
        assert_eq!(map_response(&resp(200, None, "not json").unwrap()), Err(TrackerError::Decode));
    }

    #[tokio::test(start_paused = true)]
    async fn a_429_waits_retry_after_then_succeeds() {
        let raw = Scripted::new(vec![resp(429, Some("1"), ""), resp(200, None, "{}")]);
        let t = PolicyTransport::new(Ref(&raw), 3);
        let started = tokio::time::Instant::now();
        assert_eq!(t.get_json("/x", &[]).await, Ok(serde_json::json!({})));
        assert_eq!(started.elapsed(), Duration::from_secs(1));
        assert_eq!(raw.calls(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn a_429_without_retry_after_waits_the_default() {
        let raw = Scripted::new(vec![resp(429, None, ""), resp(200, None, "{}")]);
        let t = PolicyTransport::new(Ref(&raw), 3);
        let started = tokio::time::Instant::now();
        t.get_json("/x", &[]).await.unwrap();
        assert_eq!(started.elapsed(), crate::tracker::http::DEFAULT_RETRY_AFTER);
    }

    #[tokio::test(start_paused = true)]
    async fn the_retry_budget_is_three_then_rate_limited() {
        let raw = Scripted::new((0..4).map(|_| resp(429, Some("0"), "")).collect());
        let t = PolicyTransport::new(Ref(&raw), 3);
        assert_eq!(t.get_json("/x", &[]).await, Err(TrackerError::RateLimited));
        assert_eq!(raw.calls(), 4, "one try plus three retries");
    }

    #[tokio::test(start_paused = true)]
    async fn connect_mode_does_not_retry() {
        let raw = Scripted::new(vec![resp(429, Some("1"), "")]);
        let t = PolicyTransport::new(Ref(&raw), 0);
        let started = tokio::time::Instant::now();
        assert_eq!(t.get_json("/x", &[]).await, Err(TrackerError::RateLimited));
        assert_eq!(raw.calls(), 1);
        assert_eq!(started.elapsed(), Duration::ZERO);
    }

    #[tokio::test]
    async fn a_network_failure_is_transport_and_not_retried() {
        let raw = Scripted::new(vec![Err(())]);
        let t = PolicyTransport::new(Ref(&raw), 3);
        assert_eq!(t.get_json("/x", &[]).await, Err(TrackerError::Transport));
        assert_eq!(raw.calls(), 1);
    }

    // ── The production backend, against real local servers ──────────────────

    async fn serve(app: axum::Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}")
    }

    fn basic(user: &str, token: &str) -> crate::tracker::AuthScheme {
        crate::tracker::AuthScheme::Basic {
            user: user.into(),
            token: crate::tracker::ApiToken(token.into()),
        }
    }

    #[tokio::test]
    async fn redirects_are_reported_never_followed() {
        let hits = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let h = hits.clone();
        let target = serve(axum::Router::new().route(
            "/stolen",
            axum::routing::get(move || {
                h.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async { "{}" }
            }),
        ))
        .await;
        let location = format!("{target}/stolen");
        let origin = serve(axum::Router::new().route(
            "/x",
            axum::routing::get(move || {
                let location = location.clone();
                async move {
                    (axum::http::StatusCode::FOUND, [(axum::http::header::LOCATION, location)])
                }
            }),
        ))
        .await;
        let raw = ReqwestRaw::new(&origin, basic("u", "t")).unwrap();
        assert_eq!(raw.get("/x", &[]).await.unwrap().status, 302);
        let policy = PolicyTransport::new(ReqwestRaw::new(&origin, basic("u", "t")).unwrap(), 0);
        assert_eq!(policy.get_json("/x", &[]).await, Err(TrackerError::Redirect));
        assert_eq!(
            hits.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "the credential never reached the redirect target"
        );
    }

    #[tokio::test]
    async fn retry_after_auth_and_query_reach_the_wire() {
        let base = serve(
            axum::Router::new()
                .route(
                    "/limited",
                    axum::routing::get(|| async {
                        (
                            axum::http::StatusCode::TOO_MANY_REQUESTS,
                            [(axum::http::header::RETRY_AFTER, "7")],
                            "",
                        )
                    }),
                )
                .route(
                    "/echo",
                    axum::routing::get(
                        |headers: axum::http::HeaderMap, q: axum::extract::RawQuery| async move {
                            axum::Json(serde_json::json!({
                                "auth": headers.get("authorization").and_then(|v| v.to_str().ok()),
                                "query": q.0,
                            }))
                        },
                    ),
                ),
        )
        .await;
        let raw = ReqwestRaw::new(&base, basic("dev@example.com", "tok")).unwrap();
        let limited = raw.get("/limited", &[]).await.unwrap();
        assert_eq!((limited.status, limited.retry_after.as_deref()), (429, Some("7")));
        let echo =
            map_response(&raw.get("/echo", &[("startAt", "100".into())]).await.unwrap()).unwrap();
        use base64::Engine;
        let expected = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode("dev@example.com:tok")
        );
        assert_eq!(echo["auth"], serde_json::json!(expected));
        assert_eq!(echo["query"], serde_json::json!("startAt=100"));
    }
}
