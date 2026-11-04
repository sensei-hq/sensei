//! The shared GET transport, and the write seam that only `writeback.rs` uses:
//! status → error mapping and the 429 policy over one-method raw HTTP seams, so
//! both are testable without a server. (Spec R4; write-back W10.)

use std::time::Duration;

use crate::tracker::{GetTransport, TrackerError, WriteMethod, WriteTransport};

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

/// One write, with no policy: the production impl is reqwest with redirects off (W10).
#[async_trait::async_trait]
pub trait RawWrite: Send + Sync {
    /// `Err` only for a network-level failure (no response at all).
    async fn send(
        &self,
        method: WriteMethod,
        path: &str,
        query: &[(&str, String)],
        body: &[u8],
    ) -> Result<RawResponse, ()>;
}

/// Applies the R4 policy over a [`RawGet`] or a [`RawWrite`]: maps statuses onto [`TrackerError`]
/// and retries a 429 up to `retries` times, waiting [`crate::tracker::http::retry_after`].
pub struct PolicyTransport<R> {
    raw: R,
    retries: u8,
}

impl<R> PolicyTransport<R> {
    /// `retries = 3` for sync and write-back (`WRITE_RETRIES`); `0` for connect's `validate` (fail fast).
    pub fn new(raw: R, retries: u8) -> Self {
        PolicyTransport { raw, retries }
    }
    /// The backend, for a test to read its call log.
    #[cfg(test)]
    pub fn raw(&self) -> &R {
        &self.raw
    }

    /// The R4 policy, shared by reads and writes (W10): retry a 429 up to
    /// `retries` times after its `Retry-After`, then map the response.
    async fn with_retries<F, Fut>(
        &self,
        send: F,
        map: fn(&RawResponse) -> Result<serde_json::Value, TrackerError>,
    ) -> Result<serde_json::Value, TrackerError>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<RawResponse, ()>> + Send,
    {
        let mut retried = 0u8;
        loop {
            let resp = send().await.map_err(|()| TrackerError::Transport)?;
            if resp.status == 429 && retried < self.retries {
                retried += 1;
                let wait: Duration = crate::tracker::http::retry_after(
                    resp.retry_after.as_deref(),
                    chrono::Utc::now(),
                );
                tokio::time::sleep(wait).await;
                continue;
            }
            return map(&resp);
        }
    }
}

/// [`map_response`] for a write: an empty 2xx (Jira's 204) is `Null`, not `Decode`.
pub fn map_write_response(resp: &RawResponse) -> Result<serde_json::Value, TrackerError> {
    match resp.status {
        200..=299 if resp.body.is_empty() => Ok(serde_json::Value::Null),
        _ => map_response(resp),
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
        self.with_retries(|| self.raw.get(path, query), map_response).await
    }
}

#[async_trait::async_trait]
impl<R: RawWrite> WriteTransport for PolicyTransport<R> {
    async fn send_json(
        &self,
        method: WriteMethod,
        path: &str,
        query: &[(&str, String)],
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, TrackerError> {
        let bytes = serde_json::to_vec(body).map_err(|_| TrackerError::Decode)?;
        self.with_retries(|| self.raw.send(method, path, query, &bytes), map_write_response).await
    }
}

/// The production [`RawGet`] and [`RawWrite`]: reqwest with the shared bounded timeouts and
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

impl ReqwestRaw {
    /// Authenticate and send `req`, reading as much of the response as the policy needs.
    async fn execute(&self, req: reqwest::RequestBuilder) -> Result<RawResponse, ()> {
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

#[async_trait::async_trait]
impl RawGet for ReqwestRaw {
    async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<RawResponse, ()> {
        self.execute(self.client.get(format!("{}{}", self.base_url, path)).query(query)).await
    }
}

#[async_trait::async_trait]
impl RawWrite for ReqwestRaw {
    async fn send(
        &self,
        method: WriteMethod,
        path: &str,
        query: &[(&str, String)],
        body: &[u8],
    ) -> Result<RawResponse, ()> {
        let verb = match method {
            WriteMethod::Post => reqwest::Method::POST,
            WriteMethod::Put => reqwest::Method::PUT,
        };
        let req = self
            .client
            .request(verb, format!("{}{}", self.base_url, path))
            .query(query)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_vec());
        self.execute(req).await
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
    /// The same script, as a write backend: records each send's verb, path and body.
    struct ScriptedWrites {
        responses: Mutex<Vec<Result<RawResponse, ()>>>,
        sent: Mutex<Vec<(WriteMethod, String, Vec<u8>)>>,
    }
    impl ScriptedWrites {
        fn new(mut rs: Vec<Result<RawResponse, ()>>) -> Self {
            rs.reverse();
            ScriptedWrites { responses: Mutex::new(rs), sent: Mutex::new(vec![]) }
        }
    }
    #[async_trait::async_trait]
    impl RawWrite for ScriptedWrites {
        async fn send(
            &self,
            method: WriteMethod,
            path: &str,
            _q: &[(&str, String)],
            body: &[u8],
        ) -> Result<RawResponse, ()> {
            self.sent.lock().unwrap().push((method, path.into(), body.to_vec()));
            self.responses.lock().unwrap().pop().expect("a scripted response")
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

    #[tokio::test(start_paused = true)]
    async fn a_write_shares_the_retry_policy_and_sends_its_body() {
        let raw = ScriptedWrites::new((0..4).map(|_| resp(429, Some("2"), "")).collect());
        let t = PolicyTransport::new(raw, 3);
        let started = tokio::time::Instant::now();
        let body = serde_json::json!({"body": "x"});
        assert_eq!(
            t.send_json(WriteMethod::Put, "/c/1", &[], &body).await,
            Err(TrackerError::RateLimited)
        );
        assert_eq!(started.elapsed(), Duration::from_secs(6), "three Retry-After sleeps");
        let sent = t.raw.sent.lock().unwrap().clone();
        assert_eq!(sent.len(), 4, "one try plus three retries");
        assert!(sent.iter().all(|(m, p, b)| *m == WriteMethod::Put
            && p == "/c/1"
            && serde_json::from_slice::<serde_json::Value>(b).unwrap() == body));
    }

    #[test]
    fn an_empty_success_is_null_for_a_write_and_still_a_decode_error_for_a_read() {
        assert_eq!(map_write_response(&resp(204, None, "").unwrap()), Ok(serde_json::Value::Null));
        assert_eq!(map_write_response(&resp(200, None, "").unwrap()), Ok(serde_json::Value::Null));
        assert_eq!(
            map_write_response(&resp(201, None, r#"{"id":"7"}"#).unwrap()),
            Ok(serde_json::json!({"id": "7"}))
        );
        assert_eq!(map_write_response(&resp(404, None, "").unwrap()), Err(TrackerError::NotFound));
        assert_eq!(map_response(&resp(200, None, "").unwrap()), Err(TrackerError::Decode));
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
    async fn a_write_redirect_is_reported_never_followed() {
        let hits = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let h = hits.clone();
        let target = serve(axum::Router::new().route(
            "/stolen",
            axum::routing::any(move || {
                h.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async { "{}" }
            }),
        ))
        .await;
        let location = format!("{target}/stolen");
        let origin = serve(axum::Router::new().route(
            "/x",
            axum::routing::post(move || {
                let location = location.clone();
                async move {
                    (
                        axum::http::StatusCode::TEMPORARY_REDIRECT,
                        [(axum::http::header::LOCATION, location)],
                    )
                }
            }),
        ))
        .await;
        let policy = PolicyTransport::new(ReqwestRaw::new(&origin, basic("u", "t")).unwrap(), 0);
        let body = serde_json::json!({"body": "secret metrics"});
        assert_eq!(
            policy.send_json(WriteMethod::Post, "/x", &[], &body).await,
            Err(TrackerError::Redirect)
        );
        assert_eq!(
            hits.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "a 307 keeps the verb and body: never followed"
        );
    }

    #[tokio::test]
    async fn a_write_puts_its_verb_body_auth_and_query_on_the_wire() {
        let base = serve(axum::Router::new().route(
            "/w",
            axum::routing::any(
                |method: axum::http::Method,
                 headers: axum::http::HeaderMap,
                 q: axum::extract::RawQuery,
                 body: String| async move {
                    axum::Json(serde_json::json!({
                        "method": method.as_str(),
                        "auth": headers.get("authorization").and_then(|v| v.to_str().ok()).is_some(),
                        "type": headers.get("content-type").and_then(|v| v.to_str().ok()),
                        "query": q.0,
                        "body": body,
                    }))
                },
            ),
        ))
        .await;
        let t = PolicyTransport::new(ReqwestRaw::new(&base, basic("u", "t")).unwrap(), 0);
        for (m, name) in [(WriteMethod::Post, "POST"), (WriteMethod::Put, "PUT")] {
            let echo = t
                .send_json(
                    m,
                    "/w",
                    &[("notifyUsers", "false".into())],
                    &serde_json::json!({"a": 1}),
                )
                .await
                .unwrap();
            assert_eq!(echo["method"], serde_json::json!(name));
            assert_eq!(echo["auth"], serde_json::json!(true));
            assert_eq!(echo["type"], serde_json::json!("application/json"));
            assert_eq!(echo["query"], serde_json::json!("notifyUsers=false"));
            assert_eq!(echo["body"], serde_json::json!(r#"{"a":1}"#));
        }
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
