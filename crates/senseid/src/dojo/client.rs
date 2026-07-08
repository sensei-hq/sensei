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

/// A resolved endpoint for talking to one Dōjō membership: its base URL plus the
/// bounded HTTP client. The auth token is fetched on demand from the Keychain
/// (never held in memory longer than a request), so C6/C7 attach it per call via
/// [`DojoClient::bearer`].
///
/// Not yet exercised beyond construction/auth resolution in C4 — the request
/// methods land with C6/C7. Kept minimal so those chunks can build on it.
#[allow(dead_code)]
pub struct DojoClient {
    base_url: String,
    credential_ref: String,
    http: reqwest::Client,
}

#[allow(dead_code)]
impl DojoClient {
    /// Build a client for a membership, pointing at its `dojo_url`.
    pub fn for_membership(m: &DojoMembership) -> Self {
        Self {
            base_url: m.dojo_url.trim_end_matches('/').to_string(),
            credential_ref: m.credential_ref.clone(),
            http: crate::federation::http_client(),
        }
    }

    /// The base URL calls are made against (no trailing slash).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The bounded HTTP client shared with federation.
    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// Resolve the membership's Bearer token from the OS Keychain.
    ///
    /// # Blocking
    ///
    /// Shells out to `/usr/bin/security`; callers in an async context must wrap
    /// this in `tokio::task::spawn_blocking`.
    pub fn bearer(&self) -> Result<String, crate::gateway_keys::KeychainError> {
        crate::gateway_keys::get_key(&self.credential_ref)
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
    #[cfg_attr(not(target_os = "macos"), ignore)]
    fn bearer_resolves_from_keychain_roundtrip() {
        let cref = format!("dojo-test-{}", uuid::Uuid::new_v4());
        crate::gateway_keys::set_key(&cref, "device-token-abc").unwrap();
        let c = DojoClient::for_membership(&membership("http://localhost:8787/github/acme", &cref));
        assert_eq!(c.bearer().unwrap(), "device-token-abc");
        crate::gateway_keys::delete_key(&cref).unwrap();
    }
}
