//! Dōjō HTTP client seam.
//!
//! C4 establishes *where* a Dōjō connection points and *how* it authenticates;
//! it does NOT make live service calls — the artifact publish (C6) and pull (C7)
//! paths build on this seam. The client reuses the federation module's bounded
//! HTTP client ([`crate::federation::http_client`], DRY — same connect/total
//! timeout discipline). Its Bearer is the membership's [`TenantCredential`]: the
//! session of the persona the membership is bound to — the only credential the
//! dōjō accepts (#236 step 4).

use crate::db::pg_store::DojoMembership;
use dojo_protocol::relay::{
    RelayInboxAck, RelayInboxItem, RelayInboxPull, RelayInboxStatus, RelaySegment,
    RelaySegmentsPublish, RelaySessionAck, RelaySessionUpdate,
};
use dojo_protocol::{ArtifactPullResponse, PublishArtifactResponse, PublishedArtifact};
use std::time::Duration;

/// A resolved endpoint for talking to one Dōjō membership: the registry base +
/// tenant path plus the bounded HTTP client. The persona's access token is
/// minted on demand (never held in memory longer than a request), so calls
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
    credential: TenantCredential,
    http: reqwest::Client,
}

/// What a membership's tenant calls present: the person's session, the only
/// credential the dōjō accepts (#236 step 4,
/// docs/spec/dojo/access-matrix.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TenantCredential {
    /// The persona's session — minted fresh per call from its Keychain slot.
    Session { slot: String },
    /// Bound to a persona that is signed out. Presents nothing.
    SignedOut { persona_id: uuid::Uuid },
    /// No signed-in persona has claimed this membership. Presents nothing: it
    /// joins a persona when that persona's sync lists it.
    Unbound,
    /// A test's fixed bearer — see [`test_bearers`].
    #[cfg(test)]
    Fixed { bearer: String },
}

impl TenantCredential {
    pub fn of(m: &DojoMembership) -> Self {
        #[cfg(test)]
        if let Some(bearer) = test_bearers::get(&m.id) {
            return TenantCredential::Fixed { bearer };
        }
        match &m.persona {
            Some(p) => match &p.session_slot {
                Some(slot) => TenantCredential::Session { slot: slot.clone() },
                None => TenantCredential::SignedOut { persona_id: p.persona_id },
            },
            None => TenantCredential::Unbound,
        }
    }

    /// The Bearer for one call: the persona's freshly refreshed access token. A
    /// membership with no live session — signed out, or claimed by no persona —
    /// is [`DojoClientError::SignedOut`]; a dōjō that cannot be reached to
    /// refresh the session is a retryable network fault.
    pub async fn bearer(&self) -> Result<String, DojoClientError> {
        match self {
            TenantCredential::Session { slot } => {
                crate::api::handlers::auth::live_access_token(slot).await.map_err(|e| {
                    if e.needs_sign_in() {
                        DojoClientError::SignedOut
                    } else {
                        DojoClientError::Network(format!("session refresh: {e:?}"))
                    }
                })
            }
            TenantCredential::SignedOut { .. } | TenantCredential::Unbound => {
                Err(DojoClientError::SignedOut)
            }
            #[cfg(test)]
            TenantCredential::Fixed { bearer } => Ok(bearer.clone()),
        }
    }
}

/// Fixed bearers for tests, by membership id: [`TenantCredential::of`] presents
/// one when a test registered it, so an HTTP test needs no Keychain session.
#[cfg(test)]
pub(crate) mod test_bearers {
    use std::collections::HashMap;
    use std::sync::{LazyLock, Mutex};

    static BEARERS: LazyLock<Mutex<HashMap<uuid::Uuid, String>>> = LazyLock::new(Default::default);

    pub(crate) fn set(membership: uuid::Uuid, bearer: &str) {
        BEARERS.lock().unwrap().insert(membership, bearer.to_string());
    }

    pub(super) fn get(membership: &uuid::Uuid) -> Option<String> {
        BEARERS.lock().unwrap().get(membership).cloned()
    }
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
    /// The membership has no live session — its persona must sign in (again),
    /// or no signed-in persona has claimed it. Nothing was sent.
    SignedOut,
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
            DojoClientError::Decode(_) | DojoClientError::SignedOut => false,
        }
    }
}

impl std::fmt::Display for DojoClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DojoClientError::Network(m) => write!(f, "dojo network error: {m}"),
            DojoClientError::Status(c) => write!(f, "dojo service returned {c}"),
            DojoClientError::Decode(m) => write!(f, "dojo response decode error: {m}"),
            DojoClientError::SignedOut => {
                write!(f, "no signed-in session for this dōjō membership — sign in")
            }
        }
    }
}

impl std::error::Error for DojoClientError {}

/// Percent-encode one URL path segment (unreserved chars pass through, everything
/// else — `/` included — is encoded). [`encode_tenant_path`] applies it to each
/// segment of a tenant key.
fn encode_path_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Encode a `tenant_key` ("origin/org") as a MULTI-segment URL path: each
/// `/`-separated segment is percent-encoded individually, then rejoined with `/`.
/// The Worker's relay routes are `/v1/t/[origin]/[org]/relay/…` — TWO path segments —
/// so the `/` in the discovery path is a real separator here, NOT an encoded byte.
fn encode_tenant_path(tenant_key: &str) -> String {
    tenant_key.split('/').map(encode_path_segment).collect::<Vec<_>>().join("/")
}

/// The Worker's tenant mount, `{registry}/v1/t/{origin}/{org}` — what relay,
/// rules and a rules knowledge source's `url` all hang off.
pub fn tenant_base_url(registry_url: &str, tenant_key: &str) -> String {
    format!("{}/v1/t/{}", registry_url.trim_end_matches('/'), encode_tenant_path(tenant_key))
}

impl DojoClient {
    /// Build a client for a membership, pointing at its registry + tenant path.
    pub fn for_membership(m: &DojoMembership) -> Self {
        Self {
            base_url: m.dojo_url.trim_end_matches('/').to_string(),
            registry_url: m.registry_url.trim_end_matches('/').to_string(),
            tenant_key: m.tenant_key.clone(),
            credential: TenantCredential::of(m),
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
    /// `{registry_url}/v1/t/{origin}/{org}/artifacts` — the Worker's tenant route.
    fn artifacts_url(&self) -> String {
        format!("{}/artifacts", tenant_base_url(&self.registry_url, &self.tenant_key))
    }

    /// The Bearer for one call — see [`TenantCredential::bearer`].
    async fn bearer_async(&self) -> Result<String, DojoClientError> {
        self.credential.bearer().await
    }

    /// Publish (contribute) one artifact to this membership's tenant Dōjō.
    ///
    /// `POST {registry_url}/v1/t/{tenant_key}/artifacts` with the persona's bearer,
    /// JSON body = the artifact. Reuses federation's HTTP discipline (bounded
    /// connect/total timeouts via [`crate::federation::http_client`]). Errors are
    /// returned, never panicked; the caller records the outcome in the outbox.
    ///
    /// Every text field on the artifact — `title`, `body` AND `example` — MUST
    /// already be the confidentiality-checked text (see
    /// [`crate::dojo::contribute`]); this seam performs no stripping. Adding a
    /// text field to [`PublishedArtifact`] without routing it through that gate
    /// ships it raw.
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
    /// persona's bearer, parse an [`ArtifactPullResponse`] (the delta artifacts +
    /// the new cursor to persist). Mirrors [`Self::publish_artifact`]'s error
    /// discipline exactly: transport faults → [`DojoClientError::Network`],
    /// non-2xx → [`DojoClientError::Status`], an undecodable 2xx →
    /// [`DojoClientError::Decode`]. Errors are returned, never panicked — the
    /// caller (the pull loop) logs and moves on so a downstream pull can never
    /// wedge the rules-federation pull it runs beside.
    pub async fn pull_artifacts(
        &self,
        since: i64,
    ) -> Result<ArtifactPullResponse, DojoClientError> {
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
    /// `{registry_url}/v1/t/{origin}/{org}/relay/{suffix}` — the same
    /// mount shape as [`Self::artifacts_url`].
    fn relay_url(&self, suffix: &str) -> String {
        format!("{}/relay/{suffix}", tenant_base_url(&self.registry_url, &self.tenant_key))
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
        let url =
            format!("{}/rules/resolved", tenant_base_url(&self.registry_url, &self.tenant_key));
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
        let publish =
            RelaySegmentsPublish { run_id: run_id.to_string(), segments: segments.to_vec() };
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
        resp.json::<RelayInboxAck>().await.map_err(|e| DojoClientError::Decode(e.to_string()))
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
        resp.json::<RelayInboxPull>().await.map_err(|e| DojoClientError::Decode(e.to_string()))
    }

    /// Fetch ONE inbox row by id (`GET relay/inbox?id={id}`).
    ///
    /// This is what a waiting gate needs. The daemon raised the gate and holds its
    /// id, so "has it been answered" is a lookup, not a scan — and a lookup has no
    /// watermark to skip past. A cursor scan does: `seq` comes from `nextval`,
    /// which is assigned BEFORE commit, so two concurrent writers can commit out
    /// of order and a poller that advanced past the higher one never sees the
    /// lower. Same reason the artifacts path leans on signature dedup rather than
    /// trusting `last_seq` alone.
    ///
    /// dōjō scopes the lookup to the caller's membership, so an id alone can never
    /// read another tenant's gate.
    pub async fn inbox_item(
        &self,
        inbox_id: &str,
    ) -> Result<Option<RelayInboxItem>, DojoClientError> {
        let token = self.bearer_async().await?;
        let resp = self
            .http
            .get(self.relay_url("inbox"))
            .query(&[("id", inbox_id)])
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| DojoClientError::Network(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(DojoClientError::Status(status.as_u16()));
        }
        let pull: RelayInboxPull =
            resp.json().await.map_err(|e| DojoClientError::Decode(e.to_string()))?;
        // A miss is None, never a fabricated row: the gate genuinely is not there.
        Ok(pull.items.into_iter().find(|it| it.id.as_deref() == Some(inbox_id)))
    }

    /// Block (bounded) until a raised inbox row is answered, then return its
    /// `reply`. Polls [`Self::inbox_item`] every ~1.5s, checking for
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
        timeout: Duration,
    ) -> Result<Option<serde_json::Value>, DojoClientError> {
        const POLL_EVERY: Duration = Duration::from_millis(1500);
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            match self.inbox_item(inbox_id).await {
                Ok(Some(it)) => {
                    if it.status == RelayInboxStatus::Answered
                        && let Some(reply) = it.reply.clone()
                    {
                        return Ok(Some(reply));
                    }
                }
                // The gate is not there. Not an error worth aborting on — the row
                // may not have propagated yet — so keep waiting out the deadline
                // rather than failing open early on a read that may still succeed.
                Ok(None) => {}
                // No session won't recover mid-loop — a sign-in is the person's
                // to do — so surface it at once rather than spinning to the
                // deadline. Transient network/status blips are retried.
                Err(e @ DojoClientError::SignedOut) => return Err(e),
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
    /// The GOVERNANCE scope the pack was ADOPTED at (the adoption namespace's
    /// `scope_key`), NOT the pack's own area/category — so a federated pack rule
    /// groups on the same scope axis as a memory. Empty from a stale Worker →
    /// the daemon falls back to `general` (see `pack_rule_to_raw`). The Worker
    /// still sends `area` (the category) but the daemon no longer reads it —
    /// serde ignores the unknown field.
    #[serde(default)]
    pub scope_key: String,
}

/// The `rules/resolved` response envelope: `{ rules: [...] }`.
#[derive(Debug, serde::Deserialize)]
struct ResolvedRulesResponse {
    rules: Vec<PackRuleWire>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn membership(dojo_url: &str) -> DojoMembership {
        DojoMembership {
            id: uuid::Uuid::new_v4(),
            registry_url: "http://localhost:7755".into(),
            tenant_key: "github/acme".into(),
            dojo_url: dojo_url.into(),
            kind: "client".into(),
            org_slugs: vec![],
            role: "contributor".into(),
            authenticated_via: "github_oauth".into(),
            attribution_default: "anonymous".into(),
            sync_status: "authenticating".into(),
            last_seq: 0,
            last_heartbeat_at: None,
            enabled: true,
            persona: None,
        }
    }

    fn bound(slot: Option<&str>) -> DojoMembership {
        DojoMembership {
            persona: Some(crate::db::pg_store::MembershipPersona {
                persona_id: uuid::Uuid::new_v4(),
                session_slot: slot.map(str::to_string),
            }),
            ..membership("http://127.0.0.1:9/organization/acme")
        }
    }

    // #236 step 4: the membership decides the credential — the person's session
    // when bound to a signed-in persona, nothing otherwise.
    #[test]
    fn a_bound_membership_presents_its_persona_session() {
        assert_eq!(
            TenantCredential::of(&bound(Some("acme-slot"))),
            TenantCredential::Session { slot: "acme-slot".into() }
        );
    }

    #[test]
    fn an_unbound_membership_presents_nothing() {
        assert_eq!(
            TenantCredential::of(&membership("http://127.0.0.1:9/organization/acme")),
            TenantCredential::Unbound
        );
    }

    #[test]
    fn a_bound_membership_whose_persona_signed_out_presents_nothing() {
        assert!(matches!(TenantCredential::of(&bound(None)), TenantCredential::SignedOut { .. }));
    }

    // #236 step 4e: the dōjō takes the person's session only, so a membership
    // no signed-in persona has claimed asks for a sign-in — nothing is sent.
    #[tokio::test]
    async fn an_unbound_membership_asks_for_a_sign_in_before_any_call() {
        let c = DojoClient::for_membership(&membership("http://127.0.0.1:9/organization/acme"));
        let err = c.poll_inbox(0).await.unwrap_err();
        assert!(matches!(err, DojoClientError::SignedOut), "got {err:?}");
    }

    // A sign-in is the person's to do: waiting on a gate does not spin to its
    // deadline asking for one, it says so at once.
    #[tokio::test]
    async fn awaiting_a_reply_stops_at_once_when_the_persona_must_sign_in() {
        let c = DojoClient::for_membership(&bound(None));
        let started = std::time::Instant::now();
        let out = c.await_reply("gate-1", std::time::Duration::from_secs(5)).await;
        assert!(matches!(out, Err(DojoClientError::SignedOut)), "got {out:?}");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(1),
            "spun {:?}",
            started.elapsed()
        );
    }

    #[tokio::test]
    async fn a_signed_out_persona_fails_the_call_before_it_is_sent() {
        // Port 9 (discard) is never answered: a Network error here would mean the
        // call went out without a credential.
        let c = DojoClient::for_membership(&bound(None));
        let err = c.poll_inbox(0).await.unwrap_err();
        assert!(matches!(err, DojoClientError::SignedOut), "got {err:?}");
        assert!(!err.is_retryable(), "signing in again is the user's, not a retry's");
    }

    #[tokio::test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    async fn a_session_with_nothing_stored_is_signed_out_not_a_network_fault() {
        // A retryable fault here would queue the call forever behind a sign-in
        // nobody was asked for.
        let c = DojoClient::for_membership(&bound(Some(&format!(
            "no-such-slot-{}",
            uuid::Uuid::new_v4()
        ))));
        let err = c.bearer_async().await.unwrap_err();
        assert!(matches!(err, DojoClientError::SignedOut), "got {err:?}");
    }

    #[test]
    fn the_tenant_base_url_is_two_real_path_segments_under_the_registry() {
        assert_eq!(
            tenant_base_url("http://localhost:7755/", "organization/acme"),
            "http://localhost:7755/v1/t/organization/acme"
        );
        assert_eq!(tenant_base_url("http://r", "team/a b"), "http://r/v1/t/team/a%20b");
    }

    #[test]
    fn base_url_trims_trailing_slash() {
        let c = DojoClient::for_membership(&membership("http://localhost:7755/github/acme/"));
        assert_eq!(c.base_url(), "http://localhost:7755/github/acme");
    }

    #[test]
    fn artifacts_url_is_the_workers_two_segment_tenant_route() {
        // The Worker mounts /v1/t/[origin]/[org]/artifacts. The single encoded
        // segment (`github%2Facme`) was dojo-mind's mount and 404s on the Worker
        // (checked live 2026-10-12: 404 encoded, 401 two-segment).
        let c = DojoClient::for_membership(&membership("http://localhost:7755/github/acme"));
        assert_eq!(c.artifacts_url(), "http://localhost:7755/v1/t/github/acme/artifacts");
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
        assert!(!DojoClientError::SignedOut.is_retryable());
    }

    #[tokio::test]
    async fn pull_artifacts_forwards_since_and_parses_response() {
        use axum::{Json, Router, extract::Query, routing::get};
        use dojo_protocol::{
            ArtifactKind, ArtifactPayload, ArtifactPullResponse, ArtifactScope, ArtifactStatus,
            Attribution, AttributionMode, PrinciplePayload, PublishedArtifact, PulledArtifact,
            artifact_signature,
        };
        use std::collections::HashMap;

        // A fake Dōjō service: echoes `since` into the cursor and returns one
        // published principle. Echoing `since` proves the client forwarded the
        // cursor; a bad path/query would 404 and fail the parse.
        async fn artifacts(Query(q): Query<HashMap<String, String>>) -> Json<ArtifactPullResponse> {
            let since: i64 = q.get("since").and_then(|s| s.parse().ok()).unwrap_or(-1);
            let payload = ArtifactPayload::Principle(PrinciplePayload { rationale: None });
            let (title, body) =
                ("prefer small functions".to_string(), "keep units testable".to_string());
            let artifact = PublishedArtifact {
                signature: artifact_signature(
                    ArtifactKind::Principle,
                    &title,
                    &body,
                    None,
                    &payload,
                ),
                tenant_key: "github/acme".into(),
                engagement_id: None,
                kind: ArtifactKind::Principle,
                title,
                body,
                example: None,
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
                artifacts: vec![PulledArtifact {
                    id: "art-1".into(),
                    seq: since + 1,
                    status: ArtifactStatus::Published,
                    artifact,
                }],
                cursor: since,
            })
        }

        let app = Router::new().route("/v1/t/{origin}/{org}/artifacts", get(artifacts));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let mut m = membership("http://localhost:7755/github/acme");
        test_bearers::set(m.id, "device-token-pull");
        m.registry_url = format!("http://{addr}");
        let c = DojoClient::for_membership(&m);

        let resp = c.pull_artifacts(7).await.expect("pull succeeds");
        assert_eq!(resp.cursor, 7, "the client forwarded since=7 (echoed as the cursor)");
        assert_eq!(resp.artifacts.len(), 1);
        assert_eq!(resp.artifacts[0].seq, 8);
        assert_eq!(resp.artifacts[0].status, ArtifactStatus::Published);
        assert_eq!(resp.artifacts[0].artifact.kind, ArtifactKind::Principle);
    }

    #[test]
    fn relay_url_encodes_tenant_as_path_segments() {
        // The Worker relay routes are /v1/t/[origin]/[org]/relay/… — the tenant_key's
        // '/' is a real path separator (two segments), NOT an encoded %2F. (Regression
        // guard: the daemon previously sent github%2Facme → the Worker 404'd.)
        let c = DojoClient::for_membership(&membership("http://localhost:7755/github/acme"));
        assert_eq!(c.relay_url("inbox"), "http://localhost:7755/v1/t/github/acme/relay/inbox");
    }

    #[tokio::test]
    async fn relay_post_retries_transient_failures() {
        use axum::{Router, extract::State, routing::post};
        use dojo_protocol::relay::{RelayRunStatus, RelaySessionUpdate};
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        // Fake Worker that 400s the first two hits (mimicking the vite dev Worker's
        // cold-route body-drop) then 200s — proving post_retry retries a transient
        // non-2xx and eventually succeeds.
        async fn session(State(hits): State<Arc<AtomicUsize>>) -> axum::http::StatusCode {
            let n = hits.fetch_add(1, Ordering::SeqCst);
            if n < 2 { axum::http::StatusCode::BAD_REQUEST } else { axum::http::StatusCode::OK }
        }
        let hits = Arc::new(AtomicUsize::new(0));
        let app = Router::new()
            .route("/v1/t/{origin}/{org}/relay/session", post(session))
            .with_state(hits.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let mut m = membership("http://localhost:7755/github/acme");
        test_bearers::set(m.id, "device-token-relay");
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
            workspace_slug: None,
            workspace: None,
        };
        c.publish_session_update(&update)
            .await
            .expect("retries past the two transient 400s and succeeds");
        assert_eq!(hits.load(Ordering::SeqCst), 3, "two 400s + one 200 = 3 attempts");
    }

    #[tokio::test]
    async fn raise_inbox_item_does_not_retry_ambiguous_failures() {
        use axum::{Router, extract::State, routing::post};
        use dojo_protocol::relay::{
            RelayInboxItem, RelayInboxKind, RelayInboxStatus, RelayMessageDirection,
        };
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

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
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let mut m = membership("http://localhost:7755/github/acme");
        test_bearers::set(m.id, "device-token-relay");
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
    }

    #[tokio::test]
    async fn poll_inbox_forwards_since_and_parses_response() {
        use axum::{Json, Router, extract::Query, routing::get};
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
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let mut m = membership("http://localhost:7755/github/acme");
        test_bearers::set(m.id, "device-token-relay");
        m.registry_url = format!("http://{addr}");
        let c = DojoClient::for_membership(&m);

        let resp = c.poll_inbox(5).await.expect("poll succeeds");
        assert_eq!(resp.cursor, 5, "the client forwarded since=5 (echoed as the cursor)");
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].kind, RelayInboxKind::Approval);
        assert_eq!(resp.items[0].status, RelayInboxStatus::Answered);
        assert_eq!(resp.items[0].reply.as_ref().unwrap()["verdict"], "approve");
    }

    #[tokio::test]
    async fn relay_publishes_serialize_and_succeed() {
        use axum::{Json, Router, routing::post};
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
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let mut m = membership("http://localhost:7755/github/acme");
        test_bearers::set(m.id, "device-token-relay");
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
            workspace_slug: None,
            workspace: None,
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
    }

    #[tokio::test]
    async fn publish_session_update_returning_id_decodes_the_cloud_session_id() {
        use axum::{Json, Router, routing::post};
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
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let mut m = membership("http://localhost:7755/github/acme");
        test_bearers::set(m.id, "device-token-relay");
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
            workspace_slug: None,
            workspace: None,
        };
        let id = c
            .publish_session_update_returning_id(&update)
            .await
            .expect("publish returns the cloud session id");
        assert_eq!(id, "cloud-sess-77");
    }

    #[tokio::test]
    async fn await_reply_returns_reply_once_answered() {
        use axum::{Json, Router, extract::Query, routing::get};
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
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let mut m = membership("http://localhost:7755/github/acme");
        test_bearers::set(m.id, "device-token-await");
        m.registry_url = format!("http://{addr}");
        let c = DojoClient::for_membership(&m);

        // Generous timeout — the row answers on the 2nd poll (~1.5s later).
        let reply = c
            .await_reply("gate-1", std::time::Duration::from_secs(10))
            .await
            .expect("await_reply ok");
        assert_eq!(reply, Some(serde_json::json!({"verdict": "deny"})));
        assert!(CALLS.load(Ordering::SeqCst) >= 2, "polled past the pending row");
    }

    #[tokio::test]
    async fn await_reply_asks_for_its_own_gate_by_id_not_a_cursor() {
        use axum::{Json, Router, routing::get};
        use dojo_protocol::relay::{
            RelayInboxItem, RelayInboxKind, RelayInboxPull, RelayInboxStatus, RelayMessageDirection,
        };

        // The shape of the request IS the fix. Asking `?since=<ack.seq>` only ever
        // worked because an in-DB trigger bumped the row's seq on the answering
        // UPDATE, lifting it back above a watermark the daemon had already passed.
        // That also inherits the nextval-before-commit gap: a lower seq can commit
        // after a higher one and be skipped forever. A lookup by id has no
        // watermark. Without this test both shapes pass the mocks here, which
        // ignore the query string.
        static SEEN: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

        async fn inbox(uri: axum::http::Uri) -> Json<RelayInboxPull> {
            SEEN.lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(uri.query().unwrap_or_default().to_string());
            Json(RelayInboxPull {
                items: vec![RelayInboxItem {
                    id: Some("gate-7".into()),
                    run_id: "run-1".into(),
                    segment_id: None,
                    kind: RelayInboxKind::Approval,
                    direction: RelayMessageDirection::AgentToHuman,
                    status: RelayInboxStatus::Answered,
                    payload: serde_json::json!({}),
                    reply: Some(serde_json::json!({"verdict": "allow"})),
                    created_at: None,
                    answered_at: None,
                }],
                cursor: 0,
            })
        }

        let app = Router::new().route("/v1/t/{origin}/{org}/relay/inbox", get(inbox));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let mut m = membership("http://localhost:7755/github/acme");
        test_bearers::set(m.id, "device-token-byid");
        m.registry_url = format!("http://{addr}");
        let c = DojoClient::for_membership(&m);

        let reply = c.await_reply("gate-7", std::time::Duration::from_secs(5)).await;

        assert_eq!(reply.expect("ok"), Some(serde_json::json!({"verdict": "allow"})));
        let seen = SEEN.lock().unwrap_or_else(|e| e.into_inner()).clone();
        assert!(!seen.is_empty(), "the gate was never fetched");
        assert!(seen.iter().all(|q| q.contains("id=gate-7")), "must ask by gate id: {seen:?}");
        assert!(seen.iter().all(|q| !q.contains("since=")), "must not send a cursor: {seen:?}");
    }

    #[tokio::test]
    async fn await_reply_returns_none_on_timeout() {
        use axum::{Json, Router, extract::Query, routing::get};
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
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let mut m = membership("http://localhost:7755/github/acme");
        test_bearers::set(m.id, "device-token-await");
        m.registry_url = format!("http://{addr}");
        let c = DojoClient::for_membership(&m);

        // 1s timeout < the 1.5s poll interval ⇒ one poll then deadline → None.
        let reply = c
            .await_reply("gate-1", std::time::Duration::from_secs(1))
            .await
            .expect("await_reply ok");
        assert_eq!(reply, None, "unanswered gate times out to None (fail-open)");
    }
}
