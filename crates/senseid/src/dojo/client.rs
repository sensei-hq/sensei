//! Dōjō HTTP client seam.
//!
//! C4 establishes *where* a Dōjō connection points and *how* it authenticates;
//! it does NOT make live service calls — the artifact publish (C6) and pull (C7)
//! paths build on this seam. The client reuses the federation module's bounded
//! HTTP client ([`crate::federation::http_client`], DRY — same connect/total
//! timeout discipline) and resolves the per-membership device token from the OS
//! Keychain (dual-plane auth: the daemon is credential-BEARING, using a
//! Keychain-backed Bearer token — never Supabase).

use crate::db::pg_store::DojoMembership;
use dojo_protocol::relay::{
    RelayInboxAck, RelayInboxItem, RelayInboxPull, RelayInboxStatus, RelaySegment,
    RelaySegmentsPublish, RelaySessionAck, RelaySessionUpdate,
};
use dojo_protocol::{ArtifactPullResponse, PublishArtifactResponse, PublishedArtifact};
use std::time::Duration;

/// A resolved endpoint for talking to one Dōjō membership: the registry base +
/// tenant path plus the bounded HTTP client. The auth token is fetched on demand
/// from the Keychain (never held in memory longer than a request), so calls
/// attach it per request via [`DojoClient::bearer`].
///
/// C4 established construction/auth resolution; C6 adds [`DojoClient::publish_artifact`]
/// (the upstream contribute POST). The pull path (C7) builds on the same seam.
pub struct DojoClient {
    /// The full membership URL (`registry_url` + tenant path), no trailing slash.
    /// Retained for C7's pull path; the artifact endpoint is built from
    /// `registry_url` + `tenant_key` (the service mounts `/v1/t/{tenant_key}/…`
    /// at the registry root, not under the tenant path).
    base_url: String,
    /// The registry base URL (no trailing slash) — the service root.
    registry_url: String,
    /// The `<origin>/<org>[/<dojo>]` discovery path of the destination tenant.
    tenant_key: String,
    credential_ref: String,
    http: reqwest::Client,
}

/// A failed Dōjō service call. Graceful — never panics; the contribute path maps
/// this onto an outbox state (retryable → queued, permanent → error).
#[derive(Debug)]
pub enum DojoClientError {
    /// Transport failure (connect/timeout/reset) — retryable.
    Network(String),
    /// The service returned a non-success status.
    Status(u16),
    /// A 2xx response body could not be decoded.
    Decode(String),
    /// The Keychain bearer token could not be resolved.
    Keychain(String),
    /// The blocking Keychain task failed to join.
    Join(String),
}

impl DojoClientError {
    /// Whether a replay could plausibly succeed. Network faults and 5xx are
    /// transient (→ `queued`); 4xx and undecodable 2xx are permanent (→ `error`).
    /// A `Decode` after a 2xx is treated as permanent so a possibly-succeeded
    /// publish is not blindly re-sent (the Dōjō dedups by signature regardless).
    pub fn is_retryable(&self) -> bool {
        match self {
            DojoClientError::Network(_) => true,
            DojoClientError::Status(code) => (500..=599).contains(code),
            DojoClientError::Decode(_) | DojoClientError::Keychain(_) | DojoClientError::Join(_) => false,
        }
    }
}

impl std::fmt::Display for DojoClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DojoClientError::Network(m) => write!(f, "dojo network error: {m}"),
            DojoClientError::Status(c) => write!(f, "dojo service returned {c}"),
            DojoClientError::Decode(m) => write!(f, "dojo response decode error: {m}"),
            DojoClientError::Keychain(m) => write!(f, "dojo bearer resolution failed: {m}"),
            DojoClientError::Join(m) => write!(f, "dojo keychain task join failed: {m}"),
        }
    }
}

impl std::error::Error for DojoClientError {}

/// Percent-encode one URL path segment (unreserved chars pass through, everything
/// else — notably `/` → `%2F` — is encoded), so the multi-segment tenant key
/// (`github/acme`) rides as a SINGLE path segment the service decodes back. The
/// service mounts `/v1/t/{tenant_key}/artifacts` where `{tenant_key}` is one
/// segment, so the `/`s inside the discovery path must be encoded.
fn encode_path_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Encode a `tenant_key` ("origin/org") as a MULTI-segment URL path: each
/// `/`-separated segment is percent-encoded individually, then rejoined with `/`.
/// The Worker's relay routes are `/v1/t/[origin]/[org]/relay/…` — TWO path segments —
/// so the `/` in the discovery path is a real separator here, NOT an encoded byte.
/// (Contrast [`encode_path_segment`] / [`DojoClient::artifacts_url`], which target the
/// dojo-mind artifacts mount that takes the whole tenant_key as ONE encoded segment.)
fn encode_tenant_path(tenant_key: &str) -> String {
    tenant_key
        .split('/')
        .map(encode_path_segment)
        .collect::<Vec<_>>()
        .join("/")
}

impl DojoClient {
    /// Build a client for a membership, pointing at its registry + tenant path.
    pub fn for_membership(m: &DojoMembership) -> Self {
        Self {
            base_url: m.dojo_url.trim_end_matches('/').to_string(),
            registry_url: m.registry_url.trim_end_matches('/').to_string(),
            tenant_key: m.tenant_key.clone(),
            credential_ref: m.credential_ref.clone(),
            http: crate::federation::http_client(),
        }
    }

    /// The base URL calls are made against (no trailing slash).
    #[allow(dead_code)] // consumed by C7's pull path
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The bounded HTTP client shared with federation.
    #[allow(dead_code)] // consumed by C7's pull path
    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// The artifact publish/pull endpoint for this membership's tenant:
    /// `{registry_url}/v1/t/{url-encoded tenant_key}/artifacts`.
    fn artifacts_url(&self) -> String {
        format!("{}/v1/t/{}/artifacts", self.registry_url, encode_path_segment(&self.tenant_key))
    }

    /// Resolve the membership's Bearer token from the OS Keychain.
    ///
    /// # Blocking
    ///
    /// Shells out to `/usr/bin/security`; callers in an async context must wrap
    /// this in `tokio::task::spawn_blocking` (as [`Self::publish_artifact`] does
    /// via [`Self::bearer_async`]).
    #[allow(dead_code)] // C4 auth seam retained for symmetry + the keychain roundtrip test
    pub fn bearer(&self) -> Result<String, crate::gateway_keys::KeychainError> {
        crate::gateway_keys::get_key(&self.credential_ref)
    }

    /// Resolve the Keychain bearer off the async runtime (blocking shell-out).
    async fn bearer_async(&self) -> Result<String, DojoClientError> {
        let cref = self.credential_ref.clone();
        tokio::task::spawn_blocking(move || crate::gateway_keys::get_key(&cref))
            .await
            .map_err(|e| DojoClientError::Join(e.to_string()))?
            .map_err(|e| DojoClientError::Keychain(e.to_string()))
    }

    /// Publish (contribute) one artifact to this membership's tenant Dōjō.
    ///
    /// `POST {registry_url}/v1/t/{tenant_key}/artifacts` with the Keychain bearer,
    /// JSON body = the artifact. Reuses federation's HTTP discipline (bounded
    /// connect/total timeouts via [`crate::federation::http_client`]). Errors are
    /// returned, never panicked; the caller records the outcome in the outbox.
    ///
    /// The `artifact.body`/`title` MUST already be the confidentiality-checked
    /// text (see [`crate::dojo::contribute`]) — this seam performs no stripping.
    pub async fn publish_artifact(
        &self,
        art: &PublishedArtifact,
    ) -> Result<PublishArtifactResponse, DojoClientError> {
        let token = self.bearer_async().await?;
        let resp = self
            .http
            .post(self.artifacts_url())
            .bearer_auth(token)
            .json(art)
            .send()
            .await
            .map_err(|e| DojoClientError::Network(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(DojoClientError::Status(status.as_u16()));
        }
        resp.json::<PublishArtifactResponse>()
            .await
            .map_err(|e| DojoClientError::Decode(e.to_string()))
    }

    /// Pull (receive) this membership's downstream artifacts since a cursor (C7).
    ///
    /// `GET {registry_url}/v1/t/{tenant_key}/artifacts?since={since}` with the
    /// Keychain bearer, parse an [`ArtifactPullResponse`] (the delta artifacts +
    /// the new cursor to persist). Mirrors [`Self::publish_artifact`]'s error
    /// discipline exactly: transport faults → [`DojoClientError::Network`],
    /// non-2xx → [`DojoClientError::Status`], an undecodable 2xx →
    /// [`DojoClientError::Decode`]. Errors are returned, never panicked — the
    /// caller (the pull loop) logs and moves on so a downstream pull can never
    /// wedge the rules-federation pull it runs beside.
    pub async fn pull_artifacts(&self, since: i64) -> Result<ArtifactPullResponse, DojoClientError> {
        let token = self.bearer_async().await?;
        let resp = self
            .http
            .get(self.artifacts_url())
            .query(&[("since", since.to_string())])
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| DojoClientError::Network(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(DojoClientError::Status(status.as_u16()));
        }
        resp.json::<ArtifactPullResponse>()
            .await
            .map_err(|e| DojoClientError::Decode(e.to_string()))
    }

    // -- Relay (P0/P1) --------------------------------------------------------
    // The bidirectional live channel over the SAME outbound seam: the daemon
    // PUBLISHES filtered status + outline + gates to the Worker `/v1/relay/*`
    // (poll-first, device-token) and POLLS answered inbox rows back. The Worker
    // (SvelteKit + cloud Supabase) is the server — no Rust API server here; the
    // daemon stays a credential-BEARING outbound client. Types are the shared
    // `dojo_protocol::relay` wire contract. Nothing published here carries code
    // or diffs — the caller supplies already-filtered logical status (D10).

    /// The relay endpoint for this membership's tenant:
    /// `{registry_url}/v1/t/{url-encoded tenant_key}/relay/{suffix}` — the same
    /// mount shape as [`Self::artifacts_url`].
    fn relay_url(&self, suffix: &str) -> String {
        format!(
            "{}/v1/t/{}/relay/{}",
            self.registry_url,
            encode_tenant_path(&self.tenant_key),
            suffix
        )
    }

    /// Fetch the rules of every pack ADOPTED at the given `(scope_key, slug)`
    /// namespaces (the P2 full-resolve leg — the daemon can't query the Dōjō DB,
    /// Fork 1). GETs `/v1/t/{tenant}/rules/resolved?ns=organization:acme,…`; the
    /// Worker unions the adopted packs with each rule's tier override applied.
    /// Best-effort at the call site — a transport/parse/status fault surfaces as
    /// an error the caller logs and skips (pack rules are additive).
    pub async fn resolved_pack_rules(
        &self,
        ns_pairs: &[(String, String)],
    ) -> Result<Vec<PackRuleWire>, DojoClientError> {
        if ns_pairs.is_empty() {
            return Ok(Vec::new());
        }
        let token = self.bearer_async().await?;
        let ns = ns_pairs
            .iter()
            .map(|(scope, slug)| format!("{scope}:{slug}"))
            .collect::<Vec<_>>()
            .join(",");
        let url = format!(
            "{}/v1/t/{}/rules/resolved",
            self.registry_url,
            encode_tenant_path(&self.tenant_key)
        );
        let resp = self
            .http
            .get(&url)
            .query(&[("ns", ns.as_str())])
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| DojoClientError::Network(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(DojoClientError::Status(resp.status().as_u16()));
        }
        Ok(resp
            .json::<ResolvedRulesResponse>()
            .await
            .map_err(|e| DojoClientError::Network(e.to_string()))?
            .rules)
    }

    /// Bounded-retry JSON POST for the relay publishes, returning the successful
    /// response. Retries a transient failure — a transport error or ANY non-2xx —
    /// up to a few attempts with a short backoff. Publishes are safe to retry:
    /// session/segments are idempotent upserts; a raise whose request body was
    /// dropped (a 4xx) created no row, and a rare post-success blip at worst dups a
    /// gate the human simply answers once. Mirrors the P1 harness `req` retry against
    /// the vite dev Worker (which drops bodies on cold routes); a precompiled prod
    /// Worker succeeds on the first attempt. Keychain/token faults are NOT retried —
    /// `bearer_async` surfaces them before the loop.
    async fn post_retry<T: serde::Serialize + ?Sized>(
        &self,
        suffix: &str,
        body: &T,
        idempotent: bool,
    ) -> Result<reqwest::Response, DojoClientError> {
        const ATTEMPTS: usize = 4;
        const BACKOFF: Duration = Duration::from_millis(400);
        let token = self.bearer_async().await?;
        let url = self.relay_url(suffix);
        let mut last: Option<DojoClientError> = None;
        for attempt in 0..ATTEMPTS {
            if attempt > 0 {
                tokio::time::sleep(BACKOFF).await;
            }
            match self.http.post(&url).bearer_auth(&token).json(body).send().await {
                Ok(resp) if resp.status().is_success() => return Ok(resp),
                Ok(resp) => {
                    let code = resp.status().as_u16();
                    last = Some(DojoClientError::Status(code));
                    // A non-idempotent POST (raise_inbox_item) is retried ONLY when the
                    // failure is provably before any row is created: 400 (the Worker
                    // validates run_id/kind first — a dropped request body) or 404 (cold
                    // route). A 5xx may have inserted before failing, so retrying would
                    // DUPLICATE the gate → bail. Idempotent publishes (session/segments
                    // upserts) retry any non-2xx.
                    if !idempotent && code != 400 && code != 404 {
                        break;
                    }
                }
                Err(e) => {
                    last = Some(DojoClientError::Network(e.to_string()));
                    // A transport error on a non-idempotent POST is ambiguous (the row
                    // may already exist) → don't retry, to avoid a duplicate gate.
                    if !idempotent {
                        break;
                    }
                }
            }
            tracing::warn!(url = %url, attempt = attempt + 1, idempotent, "relay POST failed — retrying (transient)");
        }
        Err(last.unwrap_or_else(|| DojoClientError::Network("relay POST: no attempts".into())))
    }

    /// Fire-and-check relay publish (status / segments): bearer + JSON body, status-
    /// only result, bounded retry. See [`Self::post_retry`].
    async fn relay_post<T: serde::Serialize + ?Sized>(
        &self,
        suffix: &str,
        body: &T,
    ) -> Result<(), DojoClientError> {
        self.post_retry(suffix, body, true).await.map(|_| ())
    }

    /// Publish the filtered status snapshot for a run (`POST relay/session`).
    pub async fn publish_session_update(
        &self,
        update: &RelaySessionUpdate,
    ) -> Result<(), DojoClientError> {
        self.relay_post("session", update).await
    }

    /// Publish the status snapshot AND decode the Worker's `{ id }` response — the
    /// cloud `dojo.relay_sessions(id)` this run upserted to. Same idempotent POST
    /// as [`Self::publish_session_update`], but the P1 run-bridge needs the id to
    /// persist into `activity.runs.dojo_session_id` (so the local run joins to its
    /// relay session). Mirrors [`Self::publish_artifact`]'s decode discipline: a
    /// non-2xx → [`DojoClientError::Status`], an undecodable 2xx →
    /// [`DojoClientError::Decode`].
    pub async fn publish_session_update_returning_id(
        &self,
        update: &RelaySessionUpdate,
    ) -> Result<String, DojoClientError> {
        let resp = self.post_retry("session", update, true).await?;
        let ack = resp
            .json::<RelaySessionAck>()
            .await
            .map_err(|e| DojoClientError::Decode(e.to_string()))?;
        Ok(ack.id)
    }

    /// Upsert the run's outline segments (`POST relay/segments`). The Worker maps
    /// `run_id` → the cloud session and upserts each segment by (session, seq).
    pub async fn upsert_segments(
        &self,
        run_id: &str,
        segments: &[RelaySegment],
    ) -> Result<(), DojoClientError> {
        let publish = RelaySegmentsPublish {
            run_id: run_id.to_string(),
            segments: segments.to_vec(),
        };
        self.relay_post("segments", &publish).await
    }

    /// Raise an inbox row — a gate / decision / chat / nudge / stall
    /// (`POST relay/inbox`) — and return the Worker's [`RelayInboxAck`]
    /// (`{id, seq}`): the server-assigned inbox id to await a reply against, and
    /// the sequence to poll from. Unlike the fire-and-check publishes this
    /// decodes the response body (mirroring [`Self::publish_artifact`]'s error
    /// discipline: transport → `Network`, non-2xx → `Status`, undecodable 2xx →
    /// `Decode`). The reply arrives via [`Self::await_reply`] / [`Self::poll_inbox`].
    pub async fn raise_inbox_item(
        &self,
        item: &RelayInboxItem,
    ) -> Result<RelayInboxAck, DojoClientError> {
        // Non-idempotent: retry only on provably-pre-insert failures (see post_retry)
        // so a dropped response can't duplicate the gate.
        let resp = self.post_retry("inbox", item, false).await?;
        resp.json::<RelayInboxAck>()
            .await
            .map_err(|e| DojoClientError::Decode(e.to_string()))
    }

    /// Poll inbox rows since a cursor (`GET relay/inbox?since={cursor}`) — the
    /// answered replies the daemon consumes to continue held runs, plus the new
    /// cursor to persist. Mirrors [`Self::pull_artifacts`] exactly (poll-first;
    /// realtime is a later phone-side add).
    pub async fn poll_inbox(&self, since: i64) -> Result<RelayInboxPull, DojoClientError> {
        let token = self.bearer_async().await?;
        let resp = self
            .http
            .get(self.relay_url("inbox"))
            .query(&[("since", since.to_string())])
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| DojoClientError::Network(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(DojoClientError::Status(status.as_u16()));
        }
        resp.json::<RelayInboxPull>()
            .await
            .map_err(|e| DojoClientError::Decode(e.to_string()))
    }

    /// Block (bounded) until a raised inbox row is answered, then return its
    /// `reply`. Polls [`Self::poll_inbox`] every ~1.5s from `since`, scanning for
    /// the row whose `id == inbox_id` AND `status == Answered`. Returns
    /// `Ok(Some(reply))` on the answer, `Ok(None)` on timeout (the hook-gate leg
    /// treats a timeout as fail-open → allow). A transient poll error does NOT
    /// abort the wait — it is swallowed and retried until the deadline, so a
    /// single blip can't collapse the gate to a spurious timeout; only the
    /// bearer/`Keychain`/`Join` faults (which will never recover mid-loop) and
    /// the deadline end it.
    ///
    /// The whole wait is bounded by `timeout` (the caller passes < Claude's 60s
    /// hook cap). Uses [`tokio::time`] only — never wall-clock `Instant`.
    pub async fn await_reply(
        &self,
        inbox_id: &str,
        since: i64,
        timeout: Duration,
    ) -> Result<Option<serde_json::Value>, DojoClientError> {
        const POLL_EVERY: Duration = Duration::from_millis(1500);
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            match self.poll_inbox(since).await {
                Ok(pull) => {
                    if let Some(reply) = pull.items.iter().find_map(|it| {
                        let matches = it.id.as_deref() == Some(inbox_id)
                            && it.status == RelayInboxStatus::Answered;
                        matches.then(|| it.reply.clone()).flatten()
                    }) {
                        return Ok(Some(reply));
                    }
                }
                // A bearer/keychain fault won't recover mid-loop → surface it so
                // the caller fails open immediately rather than spinning to the
                // deadline. Transient network/status blips are retried.
                Err(e @ (DojoClientError::Keychain(_) | DojoClientError::Join(_))) => {
                    return Err(e);
                }
                Err(e) => {
                    tracing::warn!(inbox_id, error = %e, "await_reply: poll failed, retrying");
                }
            }

            // Stop if the next poll would land past the deadline.
            if tokio::time::Instant::now() + POLL_EVERY >= deadline {
                return Ok(None);
            }
            tokio::time::sleep(POLL_EVERY).await;
        }
    }
}

/// One resolved pack rule from `GET /v1/.../rules/resolved` — the fields the
/// daemon folds into its ruleset. Matches the Worker's `ResolvedPackRule`
/// (`dojo/src/lib/server/rules-data.ts`); extra fields there are ignored.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PackRuleWire {
    pub rule_id: String,
    pub statement: String,
    #[serde(default)]
    pub body: String,
    pub rationale: Option<String>,
    pub enforcement: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub area: String,
}

/// The `rules/resolved` response envelope: `{ rules: [...] }`.
#[derive(Debug, serde::Deserialize)]
struct ResolvedRulesResponse {
    rules: Vec<PackRuleWire>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn membership(dojo_url: &str, credential_ref: &str) -> DojoMembership {
        DojoMembership {
            id: uuid::Uuid::new_v4(),
            registry_url: "http://localhost:7755".into(),
            tenant_key: "github/acme".into(),
            dojo_url: dojo_url.into(),
            kind: "client".into(),
            org_slugs: vec![],
            role: "contributor".into(),
            authenticated_via: "device_code".into(),
            attribution_default: "anonymous".into(),
            credential_ref: credential_ref.into(),
            sync_status: "authenticating".into(),
            last_seq: 0,
            last_heartbeat_at: None,
            enabled: true,
        }
    }

    #[test]
    fn base_url_trims_trailing_slash() {
        let c = DojoClient::for_membership(&membership("http://localhost:7755/github/acme/", "dojo-x"));
        assert_eq!(c.base_url(), "http://localhost:7755/github/acme");
    }

    #[test]
    fn artifacts_url_encodes_tenant_as_single_segment() {
        // tenant_key "github/acme" must ride as ONE segment (`github%2Facme`)
        // under the registry root — NOT under the dojo_url tenant path.
        let c = DojoClient::for_membership(&membership("http://localhost:7755/github/acme", "dojo-x"));
        assert_eq!(c.artifacts_url(), "http://localhost:7755/v1/t/github%2Facme/artifacts");
    }

    #[test]
    fn encode_path_segment_encodes_reserved_but_not_unreserved() {
        assert_eq!(encode_path_segment("github/acme/mobile"), "github%2Facme%2Fmobile");
        assert_eq!(encode_path_segment("org-1.a_b~c"), "org-1.a_b~c");
        assert_eq!(encode_path_segment("a b"), "a%20b");
    }

    #[test]
    fn client_error_retryability() {
        assert!(DojoClientError::Network("reset".into()).is_retryable());
        assert!(DojoClientError::Status(503).is_retryable());
        assert!(!DojoClientError::Status(400).is_retryable());
        assert!(!DojoClientError::Status(403).is_retryable());
        assert!(!DojoClientError::Decode("bad json".into()).is_retryable());
        assert!(!DojoClientError::Keychain("missing".into()).is_retryable());
    }

    #[tokio::test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    async fn pull_artifacts_forwards_since_and_parses_response() {
        use axum::{extract::Query, routing::get, Json, Router};
        use dojo_protocol::{
            artifact_signature, ArtifactKind, ArtifactPayload, ArtifactPullResponse, ArtifactScope,
            ArtifactStatus, Attribution, AttributionMode, PrinciplePayload, PublishedArtifact, PulledArtifact,
        };
        use std::collections::HashMap;

        // A fake Dōjō service: echoes `since` into the cursor and returns one
        // published principle. Echoing `since` proves the client forwarded the
        // cursor; a bad path/query would 404 and fail the parse.
        async fn artifacts(Query(q): Query<HashMap<String, String>>) -> Json<ArtifactPullResponse> {
            let since: i64 = q.get("since").and_then(|s| s.parse().ok()).unwrap_or(-1);
            let payload = ArtifactPayload::Principle(PrinciplePayload { rationale: None });
            let (title, body) = ("prefer small functions".to_string(), "keep units testable".to_string());
            let artifact = PublishedArtifact {
                signature: artifact_signature(ArtifactKind::Principle, &title, &body, &payload),
                tenant_key: "github/acme".into(),
                engagement_id: None,
                kind: ArtifactKind::Principle,
                title,
                body,
                payload,
                scope: ArtifactScope { stack: Some("rust".into()), ..Default::default() },
                attribution: Attribution {
                    mode: AttributionMode::Anonymous,
                    author: None,
                    org: None,
                    anonymous_id: Some("anon-1".into()),
                },
                contributed_by: None,
                published_at: None,
            };
            Json(ArtifactPullResponse {
                artifacts: vec![PulledArtifact { id: "art-1".into(), seq: since + 1, status: ArtifactStatus::Published, artifact }],
                cursor: since,
            })
        }

        let app = Router::new().route("/v1/t/{tenant}/artifacts", get(artifacts));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let cref = format!("dojo-pull-{}", uuid::Uuid::new_v4());
        crate::gateway_keys::set_key(&cref, "device-token-pull").unwrap();
        let mut m = membership("http://localhost:7755/github/acme", &cref);
        m.registry_url = format!("http://{addr}");
        let c = DojoClient::for_membership(&m);

        let resp = c.pull_artifacts(7).await.expect("pull succeeds");
        assert_eq!(resp.cursor, 7, "the client forwarded since=7 (echoed as the cursor)");
        assert_eq!(resp.artifacts.len(), 1);
        assert_eq!(resp.artifacts[0].seq, 8);
        assert_eq!(resp.artifacts[0].status, ArtifactStatus::Published);
        assert_eq!(resp.artifacts[0].artifact.kind, ArtifactKind::Principle);

        crate::gateway_keys::delete_key(&cref).unwrap();
    }

    #[test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    fn bearer_resolves_from_keychain_roundtrip() {
        let cref = format!("dojo-test-{}", uuid::Uuid::new_v4());
        crate::gateway_keys::set_key(&cref, "device-token-abc").unwrap();
        let c = DojoClient::for_membership(&membership("http://localhost:7755/github/acme", &cref));
        assert_eq!(c.bearer().unwrap(), "device-token-abc");
        crate::gateway_keys::delete_key(&cref).unwrap();
    }

    #[test]
    fn relay_url_encodes_tenant_as_path_segments() {
        // The Worker relay routes are /v1/t/[origin]/[org]/relay/… — the tenant_key's
        // '/' is a real path separator (two segments), NOT an encoded %2F. (Regression
        // guard: the daemon previously sent github%2Facme → the Worker 404'd.)
        let c = DojoClient::for_membership(&membership("http://localhost:7755/github/acme", "dojo-x"));
        assert_eq!(
            c.relay_url("inbox"),
            "http://localhost:7755/v1/t/github/acme/relay/inbox"
        );
    }

    #[tokio::test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    async fn relay_post_retries_transient_failures() {
        use axum::{extract::State, routing::post, Router};
        use dojo_protocol::relay::{RelayRunStatus, RelaySessionUpdate};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        // Fake Worker that 400s the first two hits (mimicking the vite dev Worker's
        // cold-route body-drop) then 200s — proving post_retry retries a transient
        // non-2xx and eventually succeeds.
        async fn session(State(hits): State<Arc<AtomicUsize>>) -> axum::http::StatusCode {
            let n = hits.fetch_add(1, Ordering::SeqCst);
            if n < 2 {
                axum::http::StatusCode::BAD_REQUEST
            } else {
                axum::http::StatusCode::OK
            }
        }
        let hits = Arc::new(AtomicUsize::new(0));
        let app = Router::new()
            .route("/v1/t/{origin}/{org}/relay/session", post(session))
            .with_state(hits.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let cref = format!("dojo-relay-retry-{}", uuid::Uuid::new_v4());
        crate::gateway_keys::set_key(&cref, "device-token-relay").unwrap();
        let mut m = membership("http://localhost:7755/github/acme", &cref);
        m.registry_url = format!("http://{addr}");
        let c = DojoClient::for_membership(&m);

        let update = RelaySessionUpdate {
            run_id: "run-1".into(),
            status: RelayRunStatus::Running,
            title: "t".into(),
            goal: None,
            progress_done: 0,
            progress_total: 0,
            current_phase: None,
            current_feature: None,
            last_event_at: None,
            paused_until: None,
            pause_reason: None,
            heartbeat_at: None,
            project_slug: None,
            project: None,
        };
        c.publish_session_update(&update)
            .await
            .expect("retries past the two transient 400s and succeeds");
        assert_eq!(hits.load(Ordering::SeqCst), 3, "two 400s + one 200 = 3 attempts");

        crate::gateway_keys::delete_key(&cref).unwrap();
    }

    #[tokio::test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    async fn raise_inbox_item_does_not_retry_ambiguous_failures() {
        use axum::{extract::State, routing::post, Router};
        use dojo_protocol::relay::{
            RelayInboxItem, RelayInboxKind, RelayInboxStatus, RelayMessageDirection,
        };
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        // A 5xx may have created the row, so a non-idempotent raise must NOT retry —
        // else it duplicates the gate (the exact bug dogfooding hit). Exactly ONE
        // attempt, then the error surfaces (→ hook_gate fails open, no dup).
        async fn inbox(State(hits): State<Arc<AtomicUsize>>) -> axum::http::StatusCode {
            hits.fetch_add(1, Ordering::SeqCst);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
        let hits = Arc::new(AtomicUsize::new(0));
        let app = Router::new()
            .route("/v1/t/{origin}/{org}/relay/inbox", post(inbox))
            .with_state(hits.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let cref = format!("dojo-raise-noretry-{}", uuid::Uuid::new_v4());
        crate::gateway_keys::set_key(&cref, "device-token-relay").unwrap();
        let mut m = membership("http://localhost:7755/github/acme", &cref);
        m.registry_url = format!("http://{addr}");
        let c = DojoClient::for_membership(&m);

        let item = RelayInboxItem {
            id: None,
            run_id: "run-1".into(),
            segment_id: None,
            kind: RelayInboxKind::Approval,
            direction: RelayMessageDirection::AgentToHuman,
            status: RelayInboxStatus::Pending,
            payload: serde_json::json!({"prompt": "Approve Bash?"}),
            reply: None,
            created_at: None,
            answered_at: None,
        };
        let err = c.raise_inbox_item(&item).await.unwrap_err();
        assert!(matches!(err, DojoClientError::Status(500)), "got {err:?}");
        assert_eq!(hits.load(Ordering::SeqCst), 1, "no retry on an ambiguous 5xx (dup-safety)");

        crate::gateway_keys::delete_key(&cref).unwrap();
    }

    #[tokio::test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    async fn poll_inbox_forwards_since_and_parses_response() {
        use axum::{extract::Query, routing::get, Json, Router};
        use dojo_protocol::relay::{
            RelayInboxItem, RelayInboxKind, RelayInboxPull, RelayInboxStatus, RelayMessageDirection,
        };
        use std::collections::HashMap;

        // A fake Worker /v1/relay/inbox: echoes `since` into the cursor and returns
        // one answered approval. Echoing `since` proves the client forwarded the
        // cursor; a bad path/query would 404 and fail the parse.
        async fn inbox(Query(q): Query<HashMap<String, String>>) -> Json<RelayInboxPull> {
            let since: i64 = q.get("since").and_then(|s| s.parse().ok()).unwrap_or(-1);
            Json(RelayInboxPull {
                items: vec![RelayInboxItem {
                    id: Some("inbox-1".into()),
                    run_id: "run-1".into(),
                    segment_id: None,
                    kind: RelayInboxKind::Approval,
                    direction: RelayMessageDirection::AgentToHuman,
                    status: RelayInboxStatus::Answered,
                    payload: serde_json::json!({"prompt": "run cargo test?"}),
                    reply: Some(serde_json::json!({"verdict": "approve"})),
                    created_at: None,
                    answered_at: None,
                }],
                cursor: since,
            })
        }

        let app = Router::new().route("/v1/t/{origin}/{org}/relay/inbox", get(inbox));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let cref = format!("dojo-relay-poll-{}", uuid::Uuid::new_v4());
        crate::gateway_keys::set_key(&cref, "device-token-relay").unwrap();
        let mut m = membership("http://localhost:7755/github/acme", &cref);
        m.registry_url = format!("http://{addr}");
        let c = DojoClient::for_membership(&m);

        let resp = c.poll_inbox(5).await.expect("poll succeeds");
        assert_eq!(resp.cursor, 5, "the client forwarded since=5 (echoed as the cursor)");
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].kind, RelayInboxKind::Approval);
        assert_eq!(resp.items[0].status, RelayInboxStatus::Answered);
        assert_eq!(resp.items[0].reply.as_ref().unwrap()["verdict"], "approve");

        crate::gateway_keys::delete_key(&cref).unwrap();
    }

    #[tokio::test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    async fn relay_publishes_serialize_and_succeed() {
        use axum::{routing::post, Json, Router};
        use dojo_protocol::relay::{
            GateSeverity, RelayInboxItem, RelayInboxKind, RelayMessageDirection, RelayRunStatus,
            RelaySegment, RelaySegmentsPublish, RelaySessionUpdate, SegmentState,
        };

        // Handlers use the typed `Json<T>` extractor: if the client serialized the
        // wrong wire shape, extraction 422s → the client sees Status(422) and the
        // test fails. So a green run proves the daemon↔Worker contract holds.
        async fn session(Json(_): Json<RelaySessionUpdate>) -> axum::http::StatusCode {
            axum::http::StatusCode::OK
        }
        async fn segments(Json(s): Json<RelaySegmentsPublish>) -> axum::http::StatusCode {
            assert_eq!(s.segments.len(), 2);
            assert_eq!(s.run_id, "run-1");
            axum::http::StatusCode::OK
        }
        // The Worker's `POST relay/inbox` returns the ack `{id, seq}` — decoded
        // by `raise_inbox_item`. Typed `Json<RelayInboxItem>` extraction 422s on
        // a wrong wire shape, so a green run also proves the raise serializes.
        async fn inbox(Json(_): Json<RelayInboxItem>) -> Json<dojo_protocol::relay::RelayInboxAck> {
            Json(dojo_protocol::relay::RelayInboxAck { id: "inbox-42".into(), seq: 7 })
        }

        let app = Router::new()
            .route("/v1/t/{origin}/{org}/relay/session", post(session))
            .route("/v1/t/{origin}/{org}/relay/segments", post(segments))
            .route("/v1/t/{origin}/{org}/relay/inbox", post(inbox));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let cref = format!("dojo-relay-pub-{}", uuid::Uuid::new_v4());
        crate::gateway_keys::set_key(&cref, "device-token-relay").unwrap();
        let mut m = membership("http://localhost:7755/github/acme", &cref);
        m.registry_url = format!("http://{addr}");
        let c = DojoClient::for_membership(&m);

        let update = RelaySessionUpdate {
            run_id: "run-1".into(),
            status: RelayRunStatus::Running,
            title: "Relay engine".into(),
            goal: None,
            progress_done: 1,
            progress_total: 5,
            current_phase: Some("P1".into()),
            current_feature: None,
            last_event_at: None,
            paused_until: None,
            pause_reason: None,
            heartbeat_at: None,
            project_slug: None,
            project: None,
        };
        c.publish_session_update(&update).await.expect("session publish ok");

        let segs = vec![
            RelaySegment {
                id: None,
                parent_id: None,
                parent_seq: None,
                seq: 0,
                title: "Phase 1".into(),
                summary: Some("vertical slice".into()),
                detail: None,
                state: SegmentState::Active,
                is_gate: false,
                gate_severity: None,
                response_verdict: None,
                response_note: None,
                agent: None,
                model: None,
                spec_ref: None,
            },
            RelaySegment {
                id: None,
                parent_id: None,
                parent_seq: None,
                seq: 1,
                title: "Gate".into(),
                summary: None,
                detail: None,
                state: SegmentState::Blocked,
                is_gate: true,
                gate_severity: Some(GateSeverity::Blocking),
                response_verdict: None,
                response_note: None,
                agent: None,
                model: None,
                spec_ref: None,
            },
        ];
        c.upsert_segments("run-1", &segs).await.expect("segments publish ok");

        let item = RelayInboxItem {
            id: None,
            run_id: "run-1".into(),
            segment_id: None,
            kind: RelayInboxKind::Approval,
            direction: RelayMessageDirection::AgentToHuman,
            status: dojo_protocol::relay::RelayInboxStatus::Pending,
            payload: serde_json::json!({"prompt": "approve?"}),
            reply: None,
            created_at: None,
            answered_at: None,
        };
        let ack = c.raise_inbox_item(&item).await.expect("inbox publish ok");
        assert_eq!(ack.id, "inbox-42", "raise decodes the Worker ack id");
        assert_eq!(ack.seq, 7, "raise decodes the Worker ack seq");

        crate::gateway_keys::delete_key(&cref).unwrap();
    }

    #[tokio::test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    async fn publish_session_update_returning_id_decodes_the_cloud_session_id() {
        use axum::{routing::post, Json, Router};
        use dojo_protocol::relay::{RelayRunStatus, RelaySessionUpdate};

        // The Worker's `POST relay/session` returns `{ id }` — the upserted cloud
        // session id. Typed `Json<RelaySessionUpdate>` extraction 422s on a wrong
        // wire shape (so a green run also proves the run→session update serializes,
        // including the new heartbeat_at field), and the ack `{id}` is decoded.
        async fn session(
            Json(u): Json<RelaySessionUpdate>,
        ) -> Json<dojo_protocol::relay::RelaySessionAck> {
            // The bridge's status + heartbeat crossed the wire intact.
            assert_eq!(u.status, RelayRunStatus::Stalled);
            assert_eq!(u.heartbeat_at.as_deref(), Some("2026-07-24T10:05:00Z"));
            Json(dojo_protocol::relay::RelaySessionAck { id: "cloud-sess-77".into() })
        }
        let app = Router::new().route("/v1/t/{origin}/{org}/relay/session", post(session));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let cref = format!("dojo-sess-id-{}", uuid::Uuid::new_v4());
        crate::gateway_keys::set_key(&cref, "device-token-relay").unwrap();
        let mut m = membership("http://localhost:7755/github/acme", &cref);
        m.registry_url = format!("http://{addr}");
        let c = DojoClient::for_membership(&m);

        let update = RelaySessionUpdate {
            run_id: "run-1".into(),
            status: RelayRunStatus::Stalled,
            title: "sensei".into(),
            goal: None,
            progress_done: 2,
            progress_total: 5,
            current_phase: Some("P1".into()),
            current_feature: None,
            last_event_at: Some("2026-07-24T10:04:30Z".into()),
            paused_until: None,
            pause_reason: None,
            heartbeat_at: Some("2026-07-24T10:05:00Z".into()),
            project_slug: None,
            project: None,
        };
        let id = c
            .publish_session_update_returning_id(&update)
            .await
            .expect("publish returns the cloud session id");
        assert_eq!(id, "cloud-sess-77");

        crate::gateway_keys::delete_key(&cref).unwrap();
    }

    #[tokio::test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    async fn await_reply_returns_reply_once_answered() {
        use axum::{extract::Query, routing::get, Json, Router};
        use dojo_protocol::relay::{
            RelayInboxItem, RelayInboxKind, RelayInboxPull, RelayInboxStatus, RelayMessageDirection,
        };
        use std::collections::HashMap;
        use std::sync::atomic::{AtomicUsize, Ordering};

        // A fake Worker inbox that flips the row from Pending → Answered on the
        // SECOND poll: first call returns the still-pending gate (no reply yet),
        // second returns it answered with a deny verdict. Proves await_reply
        // keeps polling until the row is Answered and then returns its reply.
        static CALLS: AtomicUsize = AtomicUsize::new(0);
        async fn inbox(Query(_q): Query<HashMap<String, String>>) -> Json<RelayInboxPull> {
            let n = CALLS.fetch_add(1, Ordering::SeqCst);
            let (status, reply) = if n == 0 {
                (RelayInboxStatus::Pending, None)
            } else {
                (RelayInboxStatus::Answered, Some(serde_json::json!({"verdict": "deny"})))
            };
            Json(RelayInboxPull {
                items: vec![RelayInboxItem {
                    id: Some("gate-1".into()),
                    run_id: "run-1".into(),
                    segment_id: None,
                    kind: RelayInboxKind::Approval,
                    direction: RelayMessageDirection::AgentToHuman,
                    status,
                    payload: serde_json::json!({"prompt": "Approve Bash?"}),
                    reply,
                    created_at: None,
                    answered_at: None,
                }],
                cursor: 1,
            })
        }

        let app = Router::new().route("/v1/t/{origin}/{org}/relay/inbox", get(inbox));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let cref = format!("dojo-await-{}", uuid::Uuid::new_v4());
        crate::gateway_keys::set_key(&cref, "device-token-await").unwrap();
        let mut m = membership("http://localhost:7755/github/acme", &cref);
        m.registry_url = format!("http://{addr}");
        let c = DojoClient::for_membership(&m);

        // Generous timeout — the row answers on the 2nd poll (~1.5s later).
        let reply = c
            .await_reply("gate-1", 0, std::time::Duration::from_secs(10))
            .await
            .expect("await_reply ok");
        assert_eq!(reply, Some(serde_json::json!({"verdict": "deny"})));
        assert!(CALLS.load(Ordering::SeqCst) >= 2, "polled past the pending row");

        crate::gateway_keys::delete_key(&cref).unwrap();
    }

    #[tokio::test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    async fn await_reply_returns_none_on_timeout() {
        use axum::{extract::Query, routing::get, Json, Router};
        use dojo_protocol::relay::{
            RelayInboxItem, RelayInboxKind, RelayInboxPull, RelayInboxStatus, RelayMessageDirection,
        };
        use std::collections::HashMap;

        // A fake Worker that NEVER answers the gate — the row stays Pending. With
        // a sub-poll-interval timeout, await_reply must give up and return None
        // (the fail-open → allow path).
        async fn inbox(Query(_q): Query<HashMap<String, String>>) -> Json<RelayInboxPull> {
            Json(RelayInboxPull {
                items: vec![RelayInboxItem {
                    id: Some("gate-1".into()),
                    run_id: "run-1".into(),
                    segment_id: None,
                    kind: RelayInboxKind::Approval,
                    direction: RelayMessageDirection::AgentToHuman,
                    status: RelayInboxStatus::Pending,
                    payload: serde_json::json!({"prompt": "Approve Bash?"}),
                    reply: None,
                    created_at: None,
                    answered_at: None,
                }],
                cursor: 1,
            })
        }

        let app = Router::new().route("/v1/t/{origin}/{org}/relay/inbox", get(inbox));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let cref = format!("dojo-await-to-{}", uuid::Uuid::new_v4());
        crate::gateway_keys::set_key(&cref, "device-token-await").unwrap();
        let mut m = membership("http://localhost:7755/github/acme", &cref);
        m.registry_url = format!("http://{addr}");
        let c = DojoClient::for_membership(&m);

        // 1s timeout < the 1.5s poll interval ⇒ one poll then deadline → None.
        let reply = c
            .await_reply("gate-1", 0, std::time::Duration::from_secs(1))
            .await
            .expect("await_reply ok");
        assert_eq!(reply, None, "unanswered gate times out to None (fail-open)");

        crate::gateway_keys::delete_key(&cref).unwrap();
    }
}
