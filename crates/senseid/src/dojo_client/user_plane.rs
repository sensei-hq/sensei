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
    /// Catalogue metric KEYS this tenant has switched OFF for this repository.
    ///
    /// The DISABLED set, because absence means enabled: a metric added to the
    /// catalogue later is then automatically on, whereas an "enabled" list would
    /// have it arrive off everywhere no row happens to mention.
    ///
    /// `#[serde(default)]` so a dōjō that predates the field is read as "nothing
    /// disabled" — the whole catalogue, which is the correct default and the only
    /// safe direction: guessing DISABLED would silently stop computing metrics
    /// nobody asked to stop.
    #[serde(default)]
    pub disabled_metrics: Vec<String>,
}

/// Which metrics are still wanted for each repository, across every tenant.
///
/// A UNION, and that is the whole point: a repository can be shared with more
/// than one dōjō, and one tenant switching a metric off must not stop the others
/// from getting it. So a metric is wanted while ANY consuming tenant still wants
/// it, and skippable only when every one of them has turned it off.
///
/// Returns the DISABLED-everywhere set per repo_key, so an empty entry (the
/// common case) means "compute everything" and costs nothing to carry.
pub fn disabled_everywhere(plan: &SyncPlan) -> std::collections::HashMap<String, Vec<String>> {
    use std::collections::{HashMap, HashSet};
    // Per repo: the intersection of the tenants' disabled sets. Intersection,
    // not union — a metric is only skippable when NO tenant wants it.
    let mut acc: HashMap<String, Option<HashSet<String>>> = HashMap::new();
    for r in &plan.allowed {
        let mine: HashSet<String> = r.disabled_metrics.iter().cloned().collect();
        match acc.get_mut(&r.repo_key) {
            // Second and later tenants narrow the set.
            Some(Some(seen)) => *seen = seen.intersection(&mine).cloned().collect(),
            Some(None) => {}
            None => {
                acc.insert(r.repo_key.clone(), Some(mine));
            }
        }
    }
    acc.into_iter()
        .filter_map(|(k, v)| {
            let mut keys: Vec<String> = v?.into_iter().collect();
            if keys.is_empty() {
                return None;
            }
            keys.sort();
            Some((k, keys))
        })
        .collect()
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

/// The three calls the sync cycle makes, behind a trait.
///
/// Not indirection for its own sake. Before this existed the cycle called the free
/// functions below directly, so no test could reach `sync_persona` or
/// `push_allowed` at all: **the entire body of `tick` could be replaced with
/// `Ok(())` and both of its tests still passed.** Push failure, partial
/// acceptance, an unparseable tenant id, an empty allow-list and the per-persona
/// isolation were all unguarded.
///
/// `#[async_trait]` because the cycle holds it as `&dyn UserPlane` — the alternative
/// (generics) would infect every caller's signature for no gain here.
#[async_trait::async_trait]
pub trait UserPlane: Send + Sync {
    async fn register_repositories(
        &self,
        token: &str,
        repos: &[RepoInput<'_>],
    ) -> Result<RegisterResult, String>;

    async fn sync_plan(&self, token: &str) -> Result<SyncPlan, String>;

    /// Hand the dōjō the forge token so it can re-read repository visibility.
    ///
    /// `Err` when the dōjō reports it did NOT provision, even though that answer
    /// arrives as an HTTP 200.
    async fn provision(
        &self,
        token: &str,
        provider_token: &str,
    ) -> Result<ProvisionOutcome, String>;

    async fn push_metrics(
        &self,
        token: &str,
        metrics: &[MetricPush<'_>],
    ) -> Result<IngestResult, String>;
}

/// The real transport: HTTP to a dōjō.
pub struct HttpUserPlane {
    pub dojo_url: String,
}

#[async_trait::async_trait]
impl UserPlane for HttpUserPlane {
    async fn register_repositories(
        &self,
        token: &str,
        repos: &[RepoInput<'_>],
    ) -> Result<RegisterResult, String> {
        register_repositories(&self.dojo_url, token, repos).await
    }
    async fn sync_plan(&self, token: &str) -> Result<SyncPlan, String> {
        sync_plan(&self.dojo_url, token).await
    }
    async fn provision(
        &self,
        token: &str,
        provider_token: &str,
    ) -> Result<ProvisionOutcome, String> {
        provision(&self.dojo_url, token, provider_token).await
    }
    async fn push_metrics(
        &self,
        token: &str,
        metrics: &[MetricPush<'_>],
    ) -> Result<IngestResult, String> {
        push_metrics(&self.dojo_url, token, metrics).await
    }
}

/// One metric row on the wire.
///
/// Borrowed, not owned: a push batch is built from rows the store already holds,
/// and cloning a thousand of them to send them once is waste. `computed_on` is
/// the exception — a date has to be rendered to a string somewhere.
///
/// Field names are snake_case to match `MetricInput` in `metrics-ingest.ts`. A
/// camelCase key would simply be absent on arrival, and an absent `scope` files a
/// per-person row as repository-wide.
#[derive(Debug, serde::Serialize)]
pub struct MetricPush<'a> {
    pub repo_key: &'a str,
    pub metric: &'a str,
    pub scope: &'a str,
    pub grain: &'a str,
    pub computed_on: String,
    pub value: f64,
    pub commit_sha: Option<&'a str>,
    pub props: &'a serde_json::Value,
    pub source: &'a str,
}

/// A row the dōjō refused, and why.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RejectedMetric {
    pub repo_key: String,
    pub metric: String,
    /// `not_permitted` | `unknown_repository` | `unknown_metric` |
    /// `unsupported_scope`. A String for the same reason `UnmappedRepo::reason`
    /// is: a reason from a newer dōjō must reach the log intact.
    pub reason: String,
}

/// What the dōjō did with a batch.
///
/// Partial acceptance is the designed outcome — one bad row must not block a
/// machine's whole history — so both halves matter: `accepted` is what may have
/// its `shared_at` marked, and a rejected row must stay unpushed rather than be
/// recorded as sent.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct IngestResult {
    #[serde(default)]
    pub accepted: u32,
    #[serde(default)]
    pub rejected: Vec<RejectedMetric>,
}

/// `POST /v1/you/metrics` — push metric rows.
pub async fn push_metrics(
    dojo_url: &str,
    token: &str,
    metrics: &[MetricPush<'_>],
) -> Result<IngestResult, String> {
    let req = crate::federation::http_client()
        .post(endpoint(dojo_url, "metrics"))
        .json(&serde_json::json!({ "metrics": metrics }));
    decode("metric push result", &send(req, token, "metric push").await?)
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

/// How many repositories one provisioning pass actually asked the forge about.
///
/// `captured` is the only count that moved a verdict. The others are why a pass
/// can "succeed" and change nothing: `failed` is a forge read that threw,
/// `deferred` is the per-pass cap (40) leaving rows for next time, and
/// `unavailable` is a repository the token cannot see.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
pub struct VisibilityCounts {
    #[serde(default)]
    pub captured: u32,
    #[serde(default)]
    pub unavailable: u32,
    #[serde(default)]
    pub failed: u32,
    #[serde(default)]
    pub deferred: u32,
    #[serde(default)]
    pub unsupported: u32,
}

/// What one `POST /v1/you/provision` pass reports it did.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ProvisionOutcome {
    /// NOT defaulted. The dōjō documents this as never omitted, so an absent
    /// field is a shape we do not understand — and inventing either value would
    /// assert a verdict the dōjō never gave.
    pub synced: bool,
    /// `no_forge_token` | `forge_unreachable` | `forge_token_rejected` |
    /// `no_identity`. A String for the same reason `UnmappedRepo::reason` is: a
    /// reason from a newer dōjō must reach the log intact.
    #[serde(default)]
    pub reason: Option<String>,
    /// Present only when the forge was genuinely read.
    #[serde(default)]
    pub visibility: Option<VisibilityCounts>,
}

/// Read the provisioning result, and treat "I could not do it" as a failure.
///
/// Split from the I/O so the verdict is testable without a dōjō — this is the
/// whole point of the function, and it was previously unreachable by any test
/// because the body never left the HTTP call.
///
/// The dōjō answers **HTTP 200** when it could not read the forge, carrying
/// `{synced:false, reason:…}`. `send` only tests `status.is_success()`, so
/// without this the caller could not tell a real capture from a total no-op.
fn provision_verdict(text: &str) -> Result<ProvisionOutcome, String> {
    let out: ProvisionOutcome = decode("provisioning result", text)?;
    if !out.synced {
        return Err(format!(
            "the dōjō did not provision from the forge token: {}",
            out.reason.as_deref().unwrap_or("no reason given")
        ));
    }
    Ok(out)
}

/// Ask the dōjō to re-read forge visibility, by handing it the forge token.
///
/// `POST /v1/you/provision` is the endpoint that runs `refreshForgeVisibility`,
/// and it already accepts `provider_token` in the body precisely because the
/// daemon's copy outlives the web session's (§IV.8). So this needs no new
/// endpoint — only a caller.
///
/// The tenant list in the response is genuinely not interesting here: whether
/// the VERDICT changed is the sync plan's answer, not this one. But `synced` and
/// `reason` are the dōjō's report on THIS call, and discarding them is what let
/// a dead forge token read as a successful refresh — so they are returned.
pub async fn provision(
    dojo_url: &str,
    token: &str,
    provider_token: &str,
) -> Result<ProvisionOutcome, String> {
    let req = crate::federation::http_client()
        .post(endpoint(dojo_url, "provision"))
        .json(&serde_json::json!({ "provider_token": provider_token }));
    provision_verdict(&send(req, token, "provision").await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ingest_result_decodes_and_names_what_was_refused() {
        // Partial acceptance is the designed outcome: one bad row must not block
        // a machine's whole history. So the daemon has to read BOTH halves —
        // `accepted` drives the shared_at watermark, and a rejected row must stay
        // unpushed rather than be marked sent.
        let body = serde_json::json!({
            "accepted": 2,
            "rejected": [{ "repo_key": "github.com/acme/api",
                           "metric": "commits_per_day",
                           "reason": "unsupported_scope" }]
        });
        let out: IngestResult = serde_json::from_value(body).expect("decodes");
        assert_eq!(out.accepted, 2);
        assert_eq!(out.rejected[0].reason, "unsupported_scope");
    }

    #[test]
    fn a_metric_serializes_to_the_field_names_the_dojo_reads() {
        // snake_case on the wire, matching `MetricInput` in metrics-ingest.ts. A
        // camelCase key would be silently absent — and an absent `scope` files a
        // per-person row as repository-wide.
        let m = MetricPush {
            repo_key: "github.com/acme/api",
            metric: "commits_per_day",
            scope: "repo",
            grain: "daily",
            computed_on: "2026-08-27".to_string(),
            value: 12.0,
            commit_sha: None,
            props: &serde_json::json!({}),
            source: "measured",
        };
        let v = serde_json::to_value(&m).unwrap();
        for k in ["repo_key", "metric", "scope", "grain", "computed_on", "value", "source"] {
            assert!(v.get(k).is_some(), "the dōjō reads `{k}` — it must be on the wire");
        }
        assert!(
            v.get("commit_sha").is_some(),
            "an absent sha travels as null, not as a missing key"
        );
    }

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
    fn a_provision_that_did_not_sync_is_a_failure_not_a_success() {
        // THE defect this decode exists for. `POST /v1/you/provision` answers
        // HTTP 200 carrying `{synced:false, reason:'forge_unreachable'}` — the
        // dōjō reporting that it could not read the forge. The old client tested
        // only `status.is_success()` and threw the body away, so the daemon's
        // self-heal took its SUCCESS branch and logged "refreshed forge
        // visibility and re-read the plan" on a pass that refreshed nothing.
        //
        // Live consequence: a dead GitHub token produced an unbounded 60s loop —
        // forge_visibility_unknown → provision → 200 → identical plan → same
        // denial — announcing success every time.
        let body = serde_json::json!({
            "synced": false, "reason": "forge_unreachable",
            "personal": null, "tenants": []
        });
        let e = provision_verdict(&body.to_string()).unwrap_err();
        assert!(e.contains("forge_unreachable"), "the dōjō's own reason must survive, got {e}");
    }

    #[test]
    fn a_provision_with_no_forge_token_is_also_a_failure() {
        // The daemon calls this ONLY when it holds a forge token, so being told
        // the token was unusable is a real failure here even though the same
        // reason is ordinary for the browser caller.
        let body = serde_json::json!({ "synced": false, "reason": "no_forge_token",
                                       "personal": null, "tenants": [] });
        assert!(provision_verdict(&body.to_string()).unwrap_err().contains("no_forge_token"));
    }

    #[test]
    fn a_successful_provision_carries_what_the_forge_answered_for() {
        // `visibility` is present only when the forge was genuinely read. The
        // counts are what tells an operator that a pass "succeeded" while
        // capturing nothing — `failed` and `deferred` are the starvation signal.
        let body = serde_json::json!({
            "synced": true, "personal": null, "tenants": [],
            "visibility": { "captured": 3, "unavailable": 1, "failed": 2,
                            "deferred": 40, "unsupported": 0 }
        });
        let out = provision_verdict(&body.to_string()).expect("a synced pass is Ok");
        let v = out.visibility.expect("counts present when the forge was read");
        assert_eq!((v.captured, v.failed, v.deferred), (3, 2, 40));
    }

    #[test]
    fn a_body_without_synced_fails_loudly_rather_than_being_assumed_good() {
        // `synced` is documented as never omitted. If that ever stops holding,
        // the daemon must say so with the body in hand — defaulting it either way
        // invents a verdict the dōjō did not give.
        let e = provision_verdict(&serde_json::json!({ "tenants": [] }).to_string()).unwrap_err();
        assert!(e.contains("provisioning result"), "names what failed to decode, got {e}");
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

#[cfg(test)]
mod activation_union {
    use super::*;

    fn repo(key: &str, tenant: &str, disabled: &[&str]) -> MappedRepo {
        MappedRepo {
            repo_key: key.into(),
            tenant: tenant.into(),
            tenant_id: format!("t-{tenant}"),
            repo_id: format!("r-{key}"),
            disabled_metrics: disabled.iter().map(|s| (*s).to_string()).collect(),
        }
    }
    fn plan(allowed: Vec<MappedRepo>) -> SyncPlan {
        SyncPlan { allowed, denied: vec![] }
    }

    #[test]
    fn one_tenant_disabling_does_not_stop_another_from_getting_it() {
        // Jerry's rule, verbatim: "for a user who has two tenants and one
        // disables ftr, other does not then computation needs to be done for the
        // one that has not disabled."
        let p = plan(vec![
            repo("github.com/a/b", "acme", &["ftr"]),
            repo("github.com/a/b", "personal", &[]),
        ]);
        assert!(
            !disabled_everywhere(&p).contains_key("github.com/a/b"),
            "one tenant still wants ftr, so nothing is skippable"
        );
    }

    #[test]
    fn a_metric_every_tenant_turned_off_is_skippable() {
        let p = plan(vec![
            repo("github.com/a/b", "acme", &["ftr", "churn_rate"]),
            repo("github.com/a/b", "personal", &["ftr"]),
        ]);
        // Only `ftr` is off for BOTH. `churn_rate` still has a consumer.
        assert_eq!(disabled_everywhere(&p).get("github.com/a/b"), Some(&vec!["ftr".to_string()]));
    }

    #[test]
    fn deactivation_is_per_repository_not_per_tenant() {
        // A tenant may want churn on the service it operates and not on a
        // vendored mirror. Nothing about repo b may leak onto repo c.
        let p = plan(vec![
            repo("github.com/a/b", "acme", &["ftr"]),
            repo("github.com/a/c", "acme", &[]),
        ]);
        let out = disabled_everywhere(&p);
        assert_eq!(out.get("github.com/a/b"), Some(&vec!["ftr".to_string()]));
        assert!(!out.contains_key("github.com/a/c"), "repo c was never touched");
    }

    #[test]
    fn a_repository_nobody_disabled_anything_for_is_absent_not_empty() {
        // The common case, and it must cost nothing: an absent entry means
        // "compute everything", so the map stays empty on a normal install.
        let p = plan(vec![repo("github.com/a/b", "acme", &[])]);
        assert!(disabled_everywhere(&p).is_empty());
    }

    #[test]
    fn an_older_dojo_that_sends_no_field_disables_nothing() {
        // `#[serde(default)]`. Guessing DISABLED from a missing field would stop
        // computing metrics nobody asked to stop — the unsafe direction.
        let json = r#"{"allowed":[{"repo_key":"github.com/a/b","tenant":"personal/x",
                       "tenant_id":"t1","repo_id":"r1"}],"denied":[]}"#;
        let p: SyncPlan = serde_json::from_str(json).expect("an older plan still parses");
        assert!(p.allowed[0].disabled_metrics.is_empty());
        assert!(disabled_everywhere(&p).is_empty());
    }

    #[test]
    fn the_result_is_sorted_so_logs_and_assertions_are_stable() {
        let p = plan(vec![repo("github.com/a/b", "acme", &["zeta", "alpha"])]);
        assert_eq!(
            disabled_everywhere(&p).get("github.com/a/b"),
            Some(&vec!["alpha".to_string(), "zeta".to_string()])
        );
    }
}

/// The config key holding the last plan's disabled-everywhere map.
///
/// ## Why this is cached when the entitlement ruling deliberately is not
///
/// `dojo_sync`'s module doc is emphatic: *"The daemon ASKS; it never
/// remembers"*, because a cached `may_share` would be a second source of truth
/// for CONSENT — a revoked seat has to bite on the next cycle, and a TTL whose
/// only job is to bound how wrong the cache can be is not a design.
///
/// Activation is a different kind of fact and the asymmetry is the reason:
/// staleness here costs one cycle of wasted computation, or one cycle of delay
/// before a re-enabled metric returns. Neither ships data without consent.
/// Meanwhile the metric tasks run on their own schedule and must not each open a
/// dōjō round trip to ask — that WOULD make the cost lever cost something.
///
/// So: cached, keyed on nothing but the persona, and overwritten whole on every
/// successful plan pull. A repository absent from the map has nothing disabled.
pub const DISABLED_METRICS_KEY: &str = "dojo.disabled_metrics";
