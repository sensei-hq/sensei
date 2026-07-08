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
use dojo_protocol::{PublishArtifactResponse, PublishedArtifact};

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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn membership(dojo_url: &str, credential_ref: &str) -> DojoMembership {
        DojoMembership {
            id: uuid::Uuid::new_v4(),
            registry_url: "http://localhost:8787".into(),
            tenant_key: "github/acme".into(),
            dojo_url: dojo_url.into(),
            kind: "client".into(),
            role: "contributor".into(),
            authenticated_via: "device_code".into(),
            attribution_default: "dereferenced".into(),
            credential_ref: credential_ref.into(),
            sync_status: "authenticating".into(),
            last_seq: 0,
            last_heartbeat_at: None,
            enabled: true,
        }
    }

    #[test]
    fn base_url_trims_trailing_slash() {
        let c = DojoClient::for_membership(&membership("http://localhost:8787/github/acme/", "dojo-x"));
        assert_eq!(c.base_url(), "http://localhost:8787/github/acme");
    }

    #[test]
    fn artifacts_url_encodes_tenant_as_single_segment() {
        // tenant_key "github/acme" must ride as ONE segment (`github%2Facme`)
        // under the registry root — NOT under the dojo_url tenant path.
        let c = DojoClient::for_membership(&membership("http://localhost:8787/github/acme", "dojo-x"));
        assert_eq!(c.artifacts_url(), "http://localhost:8787/v1/t/github%2Facme/artifacts");
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

    #[test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    fn bearer_resolves_from_keychain_roundtrip() {
        let cref = format!("dojo-test-{}", uuid::Uuid::new_v4());
        crate::gateway_keys::set_key(&cref, "device-token-abc").unwrap();
        let c = DojoClient::for_membership(&membership("http://localhost:8787/github/acme", &cref));
        assert_eq!(c.bearer().unwrap(), "device-token-abc");
        crate::gateway_keys::delete_key(&cref).unwrap();
    }
}
