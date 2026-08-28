//! The USER-plane dōjō client — "what may I sync, as me?"
//!
//! Distinct from `dojo/client.rs`, which is the TENANT plane: that one carries a
//! per-membership device token and addresses artifacts that genuinely belong to a
//! tenant. This one carries the persona's own Supabase access token and asks two
//! questions that are only answerable for a USER:
//!
//!   POST /v1/you/repositories   IDENTITY.    Which tenant does each repo belong to?
//!   GET  /v1/you/sync/plan      ENTITLEMENT. Of those, which may I sync right now?
//!
//! The split is the dōjō's (`dojo/src/lib/server/repositories.ts`) and the reason
//! is that a repository mapping to NO tenant has no tenant to be denied under —
//! so `unmapped` is a registration outcome and `denied` carries only entitlement
//! reasons.
//!
//! Deserialization is the whole risk surface here, which is why the tests below
//! decode the dōjō's literal response shapes rather than round-tripping this
//! module's own structs: a round-trip agrees with itself no matter how far both
//! ends have drifted from the server.

use serde::Deserialize;

/// A repository the dōjō resolved to a tenant.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct MappedRepo {
    pub repo_key: String,
    /// The tenant's `{origin}/{slug}` discovery key — for logs and display.
    pub tenant: String,
    /// The tenant's uuid. What gets stored in `sensei.repositories.tenant_id`,
    /// because a slug rename changes `tenant` and would strand every stored row.
    pub tenant_id: String,
    pub repo_id: String,
}

/// A repository that could not be attached to a tenant.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct UnmappedRepo {
    pub repo_key: String,
    /// `unknown_host` | `no_connection` | `ambiguous` | `not_a_member`.
    ///
    /// Kept as a String rather than an enum on purpose: an unrecognised reason
    /// from a newer dōjō must reach the log intact. Decoding it into an `Other`
    /// variant or a default would discard the only information the message had.
    pub reason: String,
}

/// A repository that is known but may not sync.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DeniedRepo {
    pub repo_key: String,
    pub tenant: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RegisterResult {
    #[serde(default)]
    pub mapped: Vec<MappedRepo>,
    #[serde(default)]
    pub unmapped: Vec<UnmappedRepo>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SyncPlan {
    #[serde(default)]
    pub allowed: Vec<MappedRepo>,
    /// Always empty in phase 1 — `all_my_repositories` hardcodes
    /// `sync_enabled = true`. Decoded anyway (D6): the shape must not start
    /// crashing the daemon on the day the gate arrives.
    #[serde(default)]
    pub denied: Vec<DeniedRepo>,
}

/// One repository being offered for registration.
#[derive(Debug, serde::Serialize)]
pub struct RepoInput<'a> {
    pub repo_key: &'a str,
    pub remote_url: Option<&'a str>,
    pub name: &'a str,
}

fn endpoint(dojo_url: &str, path: &str) -> String {
    format!("{dojo_url}/v1/you/{path}")
}

/// Send a response body through serde, failing loudly on a shape we do not know.
///
/// The body is included in the error. A bare "missing field `tenant_id`" with no
/// sight of what actually arrived is the least useful failure this client can
/// produce, and the cycle runs unattended — nobody will be watching to re-run it
/// with more logging.
fn decode<T: serde::de::DeserializeOwned>(what: &str, text: &str) -> Result<T, String> {
    serde_json::from_str(text)
        .map_err(|e| format!("dōjō returned an unreadable {what}: {e} — body: {text}"))
}

async fn send(req: reqwest::RequestBuilder, token: &str, what: &str) -> Result<String, String> {
    let r = req
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| format!("could not reach dōjō for the {what}: {e}"))?;
    let status = r.status();
    let text = r.text().await.unwrap_or_default();
    if !status.is_success() {
        // Surfaced, never swallowed into an empty result: a 403 that decoded as
        // "no repositories allowed" would silently stop all syncing and look
        // exactly like having nothing to sync.
        return Err(format!("dōjō returned {} for the {what}: {text}", status.as_u16()));
    }
    Ok(text)
}

/// `POST /v1/you/repositories` — register these repositories and learn their tenants.
pub async fn register_repositories(
    dojo_url: &str,
    token: &str,
    repos: &[RepoInput<'_>],
) -> Result<RegisterResult, String> {
    let req = crate::federation::http_client()
        .post(endpoint(dojo_url, "repositories"))
        .json(&serde_json::json!({ "repos": repos }));
    decode("registration", &send(req, token, "registration").await?)
}

/// `GET /v1/you/sync/plan` — what this persona may sync right now.
///
/// Never cached. The answer changes when a seat is revoked or a subscription
/// lapses, and acting on a stale allow-list is the one failure this endpoint
/// exists to prevent.
pub async fn sync_plan(dojo_url: &str, token: &str) -> Result<SyncPlan, String> {
    let req = crate::federation::http_client().get(endpoint(dojo_url, "sync/plan"));
    decode("sync plan", &send(req, token, "sync plan").await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_endpoints_are_user_scoped_not_tenant_scoped() {
        // §VIII.1 F9 corrected these from `/v1/t/{tenant}/…`. A repo that maps to
        // NO tenant has no tenant to be addressed under, so a tenant-scoped plan
        // could not report `unmapped` at all.
        assert_eq!(
            endpoint("https://dojo.test", "repositories"),
            "https://dojo.test/v1/you/repositories"
        );
        assert_eq!(
            endpoint("https://dojo.test", "sync/plan"),
            "https://dojo.test/v1/you/sync/plan"
        );
    }

    #[test]
    fn an_unreadable_body_names_what_arrived() {
        // Unattended: nobody is watching to re-run it with more logging.
        let e = decode::<SyncPlan>("sync plan", "<html>502 Bad Gateway</html>").unwrap_err();
        assert!(e.contains("502 Bad Gateway"), "the body must be in the error, got {e}");
    }

    /// The literal body `GET /v1/you/sync/plan` returns.
    #[test]
    fn a_plan_decodes_with_the_tenant_uuid_the_daemon_has_to_store() {
        let body = serde_json::json!({
            "allowed": [{
                "repo_key": "github.com/acme/api",
                "tenant": "organization/acme",
                "tenant_id": "1f0c0e3a-0000-4000-8000-000000000001",
                "repo_id": "9a0c0e3a-0000-4000-8000-000000000002"
            }],
            "denied": []
        });
        let plan: SyncPlan = serde_json::from_value(body).expect("the plan decodes");
        assert_eq!(plan.allowed[0].repo_key, "github.com/acme/api");
        assert_eq!(plan.allowed[0].tenant_id, "1f0c0e3a-0000-4000-8000-000000000001");
        assert!(plan.denied.is_empty());
    }

    #[test]
    fn a_non_empty_denied_list_decodes_rather_than_crashing_the_cycle() {
        // D6: phase 1 never populates this, so nothing would notice it being
        // wrong until the entitlement gate ships — and then it would fail on the
        // one release where the daemon is oldest.
        let body = serde_json::json!({
            "allowed": [],
            "denied": [{ "repo_key": "github.com/acme/api",
                         "tenant": "organization/acme",
                         "reason": "no_seat" }]
        });
        let plan: SyncPlan = serde_json::from_value(body).expect("a populated denied[] decodes");
        assert_eq!(plan.denied[0].reason, "no_seat");
    }

    #[test]
    fn an_absent_array_is_empty_rather_than_a_decode_failure() {
        // A dōjō that omits an empty array must not take the cycle down. This is
        // honest-empty: the field genuinely carries nothing.
        let plan: SyncPlan = serde_json::from_value(serde_json::json!({})).expect("decodes");
        assert!(plan.allowed.is_empty() && plan.denied.is_empty());
    }

    #[test]
    fn every_unmapped_reason_the_dojo_defines_survives_the_trip() {
        // Four reasons, four different problems — `unknown_host` is a self-hosted
        // forge, `not_a_member` is an authorization refusal. Collapsing them
        // would make the log useless exactly when someone is debugging why their
        // repository never appeared.
        let body = serde_json::json!({
            "mapped": [],
            "unmapped": [
                { "repo_key": "git.internal/x/y", "reason": "unknown_host" },
                { "repo_key": "github.com/never/seen", "reason": "no_connection" },
                { "repo_key": "github.com/acme/api", "reason": "ambiguous" },
                { "repo_key": "github.com/secret/x", "reason": "not_a_member" }
            ]
        });
        let out: RegisterResult = serde_json::from_value(body).expect("decodes");
        let reasons: Vec<&str> = out.unmapped.iter().map(|u| u.reason.as_str()).collect();
        assert_eq!(reasons, ["unknown_host", "no_connection", "ambiguous", "not_a_member"]);
    }

    #[test]
    fn an_unrecognised_reason_reaches_the_log_intact() {
        // From a newer dōjō. Mapping it to a known variant or a default would
        // report a cause that is not the real one.
        let body = serde_json::json!({
            "mapped": [],
            "unmapped": [{ "repo_key": "github.com/acme/api", "reason": "quota_exceeded" }]
        });
        let out: RegisterResult = serde_json::from_value(body).expect("decodes");
        assert_eq!(out.unmapped[0].reason, "quota_exceeded");
    }
}
