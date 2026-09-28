//! Global-dojo anonymisation — READABILITY on top of the deterministic strip.
//!
//! [`anonymize_for_global`] is the C6-facing entry point. It NEVER trusts the
//! model to protect confidentiality:
//!
//! 1. Run the deterministic [`dereference`] and gate on it via [`Dereferenced`]
//!    — if residual risk survives, return [`ResidualRisk`] and publish nothing.
//! 2. Optionally route the *already-dereferenced* text through the `reasoning`
//!    chain (reusing the shipped generalise pattern) to make it a portable,
//!    stack-agnostic principle.
//! 3. Re-run the deterministic check on the model output. If the model
//!    reintroduced ANY identifier (the strip would find something to remove) or
//!    tripped residual risk, discard the model output and keep the deterministic
//!    text. The safe text always wins.
//!
//! Global-dojo adds a project-*shape* descriptor (never the name) and a
//! rotating, opaque anonymous contributor id (never the real user/project id).
//! Attribution is [`AttributionMode::Anonymous`] with the source dereferenced.
//!
//! Forward seam: C6 wires the publish path and supplies a live [`Generalizer`]
//! (the [`GatewayGeneralizer`] below is the ready adapter). Until then the
//! surface is `dead_code`-allowed.
#![allow(dead_code)]

use crate::api::handlers::knowledge::Generalisation;
use crate::dojo::attribution::{
    Dereferenced, ProjectIdentifiers, Redaction, ResidualRisk, dereference,
};
use dojo_protocol::{Attribution, AttributionMode};
use sha2::{Digest, Sha256};
use std::future::Future;

/// The LLM prose-generalisation seam. Kept abstract so the confidentiality
/// pipeline is unit-testable without a live gateway (tests pass a mock; the
/// deterministic guarantee is exercised regardless of what the model returns).
///
/// Returns `Some(rewrite)` when the model produced a usable rule (with or
/// without its synthetic example), `None` when it was unavailable / timed out /
/// returned nothing usable (the caller then keeps the deterministic text — still
/// safe, just less polished).
pub trait Generalizer {
    fn generalize(&self, text: &str) -> impl Future<Output = Option<Generalisation>> + Send;
}

/// Size bucket for a [`ProjectShape`] — a coarse descriptor, never a real
/// metric that could fingerprint a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SizeBucket {
    Small,
    Medium,
    Large,
}

/// A project-shape descriptor: what a receiving user filters on for relevance
/// WITHOUT ever seeing the project name (`{stack: [rust], size: medium, kind:
/// web-service}`). Carries only coarse, non-identifying buckets.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ProjectShape {
    /// Coarse stack tags (`["rust"]`) — languages/frameworks, never a repo name.
    #[serde(default)]
    pub stack: Vec<String>,
    /// Size bucket.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<SizeBucket>,
    /// Kind bucket (`"web-service"`, `"cli"`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// The local contributor identity used to DERIVE (not publish) the rotating
/// anonymous id. `user_key` is a local secret/id that NEVER leaves the machine —
/// only its rotated hash does.
#[derive(Debug, Clone)]
pub struct ContributorIdentity {
    pub user_key: String,
}

/// A contribution ready for the global collective: safe text + shape + opaque
/// anon id + anonymous attribution. Constructed ONLY by [`anonymize_for_global`],
/// which guarantees the deterministic strip ran and passed.
#[derive(Debug, Clone)]
pub struct AnonymizedArtifact {
    /// The publish-safe body (deterministic text, optionally LLM-polished then
    /// re-verified).
    pub text: String,
    /// A synthetic illustration of `text`, when the polish produced one that
    /// survived the same deterministic re-check. `None` whenever there was no
    /// model, no example, or an example that did not verify — this module never
    /// invents one locally.
    pub example: Option<String>,
    /// Project-shape descriptor — never the name.
    pub shape: ProjectShape,
    /// Rotating opaque contributor id — never the real user/project id.
    pub anon_id: String,
    /// Anonymous, source-dereferenced attribution.
    pub attribution: Attribution,
    /// Whether the `reasoning` chain improved the text (vs. the deterministic
    /// fallback). Diagnostic only.
    pub llm_polished: bool,
    /// Categories the deterministic strip removed (no raw values).
    pub removed: Vec<Redaction>,
}

/// Default rotation period for the anonymous id — 30 days. "User identity
/// replaced with a stable anonymous id per user (rotated periodically)."
pub const DEFAULT_ROTATION_PERIOD_DAYS: i64 = 30;

/// The rotation window `unix_secs` falls in, for `period_days`. Pure — the
/// caller feeds a clock so the id derivation stays testable.
pub fn rotation_bucket(unix_secs: i64, period_days: i64) -> i64 {
    let period = period_days.max(1) * 86_400;
    unix_secs.div_euclid(period)
}

/// The current rotation window using the system clock + default period.
pub fn current_rotation_bucket() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    rotation_bucket(secs, DEFAULT_ROTATION_PERIOD_DAYS)
}

/// Derive the opaque anonymous id for a contributor in a given rotation window:
/// `anon-<16 hex>` from `sha256(user_key ':' bucket)`. Stable within a window,
/// rotates across windows, and is irreversible — it is NOT the user_key or any
/// real id.
pub fn rotating_anon_id(user_key: &str, bucket: i64) -> String {
    let mut h = Sha256::new();
    h.update(user_key.as_bytes());
    h.update(b":");
    h.update(bucket.to_le_bytes());
    let digest = h.finalize();
    let mut out = String::from("anon-");
    for b in &digest[..8] {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Anonymise `text` for the global collective. See the module docs for the
/// three-step guarantee. `rotation_bucket` is passed in (use
/// [`current_rotation_bucket`] in production) so the id derivation is testable.
///
/// Fails closed: if the deterministic strip can't guarantee safety, returns
/// [`ResidualRisk`] and produces no artifact — never leak on uncertainty.
pub async fn anonymize_for_global<G: Generalizer>(
    text: &str,
    ctx: &ProjectIdentifiers,
    shape: ProjectShape,
    contributor: &ContributorIdentity,
    rotation_bucket: i64,
    generalizer: &G,
) -> Result<AnonymizedArtifact, ResidualRisk> {
    // 1. Deterministic strip + fail-closed gate. NEVER trust the LLM to strip.
    let base = Dereferenced::new(text, ctx)?;

    // 2. Optional LLM polish over the ALREADY-dereferenced text.
    // A clean rewrite over already-safe text must have NOTHING to strip; if the
    // strip finds anything or trips residual risk, the model reintroduced an
    // identifier and its output is discarded.
    let verified = |candidate: &str| {
        let recheck = dereference(candidate, ctx);
        !recheck.residual_risk && recheck.removed.is_empty()
    };

    let mut best_text = base.text().to_string();
    let mut example = None;
    let mut llm_polished = false;
    if let Some(candidate) = generalizer.generalize(base.text()).await {
        // 3. Re-verify the model output deterministically — the safe text wins.
        if verified(&candidate.generalised) {
            best_text = candidate.generalised;
            llm_polished = true;
            // The example is adopted only alongside an accepted body, and only
            // if it verifies on its own. A model that reintroduced an identifier
            // in the body is not trusted for the illustration either; and an
            // illustration that leaks is simply dropped, since losing it costs
            // an example, not the principle.
            example = candidate.example.filter(|ex| verified(ex));
        }
    }

    let anon_id = rotating_anon_id(&contributor.user_key, rotation_bucket);
    let attribution = Attribution {
        mode: AttributionMode::Anonymous,
        author: None,
        org: None,
        anonymous_id: Some(anon_id.clone()),
    };

    Ok(AnonymizedArtifact {
        text: best_text,
        example,
        shape,
        anon_id,
        attribution,
        llm_polished,
        removed: base.removed().to_vec(),
    })
}

// ── Production gateway adapter (forward seam for C6) ────────────────────────

/// Stage-2 preamble for the global-anonymisation rewrite. Stricter than the
/// stage-1 memory preamble because the input is DIFFERENT: it has already been
/// dereferenced, so the model's only job is to restate it as a portable,
/// stack-agnostic principle and — critically — to NOT reintroduce any identifier
/// (the deterministic post-check enforces this regardless, but the instruction
/// keeps the model on rails).
///
/// The reply FORMAT is not restated here: it comes from the one shared
/// [`GENERALISE_REPLY_CONTRACT`], because both stages are read by the same
/// parser and must agree on the JSON shape and on what makes an example safe.
pub(crate) const GLOBAL_ANONYMISE_PREAMBLE: &str = "You rewrite an already-anonymised engineering lesson into a portable, stack-agnostic principle for a public collective. \
The text you receive has ALREADY had every project name, path, id, and person removed and replaced with placeholders like <project>, <path>, <session>. \
Keep it that way: never invent or reintroduce a concrete name, path, repository, id, or person; leave placeholders as-is or drop them. \
Restate the underlying learning so it applies across projects and stacks. Do not add advice the original did not contain.";

/// Token budget + wall-clock cap for the polish call (mirrors the shipped
/// memory-generalise handler's discipline).
const POLISH_MAX_TOKENS: u32 = 512;
const POLISH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// A [`Generalizer`] backed by the embedded gateway `reasoning` chain — the
/// production polish path C6 uses. Reuses the shipped generalise message builder,
/// reply contract and JSON parser (DRY) so this stage stays in lockstep with
/// `POST /api/knowledge/memories/{id}/generalise`; only the preamble differs,
/// because only the INPUT differs.
pub struct GatewayGeneralizer {
    gateway: std::sync::Arc<gateway::Gateway>,
}

impl GatewayGeneralizer {
    pub fn new(gateway: std::sync::Arc<gateway::Gateway>) -> Self {
        Self { gateway }
    }
}

/// Build the stage-2 polish request. Extracted and pure for the same reason as
/// the stage-1 builder: a test asserts on the prompt actually SENT here, so this
/// stage cannot quietly stop carrying the shared reply contract.
pub(crate) fn build_polish_request(text: &str) -> gateway::types::request::InferenceRequest {
    use gateway::types::capability::Capability;
    use gateway::types::request::{InferenceRequest, Message, MessageRole, Payload};
    // Reuse the shipped generalise message builder + system-prompt composer (DRY).
    use crate::api::handlers::knowledge::{build_generalise_message, generalise_system_prompt};

    InferenceRequest {
        capability: Capability::TextChat,
        model: None,
        router: None,
        // Same seed `reasoning` chain (embedded → ollama → cloud) as the memory
        // generalise handler.
        chain: Some("reasoning".into()),
        payload: Payload::Chat {
            messages: vec![Message::text(
                MessageRole::User,
                build_generalise_message("engineering principle", text),
            )],
            system: Some(generalise_system_prompt(GLOBAL_ANONYMISE_PREAMBLE)),
            max_tokens: Some(POLISH_MAX_TOKENS),
            temperature: None,
            tools: Vec::new(),
        },
        budget: None,
        auth: None,
        panel: None,
        consensus: None,
        allow_fallback: true,
        credentials: std::collections::HashMap::new(),
    }
}

impl Generalizer for GatewayGeneralizer {
    fn generalize(&self, text: &str) -> impl Future<Output = Option<Generalisation>> + Send {
        // Reuse the shipped response parser (DRY) — one parser, both stages.
        use crate::api::handlers::knowledge::parse_generalise_response;

        let gateway = self.gateway.clone();
        let request = build_polish_request(text);
        async move {
            match tokio::time::timeout(POLISH_TIMEOUT, gateway.execute(&request)).await {
                Ok(Ok(resp)) if resp.success => {
                    resp.content.as_deref().and_then(parse_generalise_response)
                }
                Ok(Ok(_)) => None,
                Ok(Err(e)) => {
                    tracing::warn!(error = %e, "global-anonymise: gateway polish failed — keeping deterministic text");
                    None
                }
                Err(_) => {
                    tracing::warn!(
                        "global-anonymise: gateway polish timed out — keeping deterministic text"
                    );
                    None
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A [`Generalizer`] that returns a canned response — the whole point is to
    /// prove the deterministic layer defends against a HOSTILE model.
    ///
    /// `pub(crate)` because `dojo::contribute` needs the same seam to exercise
    /// the model-bearing publish path. One mock, one place: a second copy would
    /// let the two layers' idea of "what the model returns" drift apart.
    pub(crate) struct MockGeneralizer {
        pub(crate) response: Option<Generalisation>,
    }

    impl Generalizer for MockGeneralizer {
        fn generalize(&self, _text: &str) -> impl Future<Output = Option<Generalisation>> + Send {
            let r = self.response.clone();
            async move { r }
        }
    }

    /// A polish reply carrying only the rewritten principle.
    fn polish(generalised: &str) -> Option<Generalisation> {
        Some(Generalisation { generalised: generalised.into(), example: None })
    }

    /// A polish reply carrying the principle AND a synthetic example.
    pub(crate) fn polish_with(generalised: &str, example: &str) -> Option<Generalisation> {
        Some(Generalisation { generalised: generalised.into(), example: Some(example.into()) })
    }

    fn ctx() -> ProjectIdentifiers {
        ProjectIdentifiers {
            project_name: Some("Acme API".into()),
            client_name: Some("Acme Corp".into()),
            repo_names: vec!["acme-api".into()],
            folder_paths: vec!["/Users/dev/work/acme-api".into()],
            session_ids: vec!["s-9931".into()],
            person_names: vec!["Jane Doe".into()],
        }
    }

    fn shape() -> ProjectShape {
        ProjectShape {
            stack: vec!["rust".into()],
            size: Some(SizeBucket::Medium),
            kind: Some("web-service".into()),
        }
    }

    fn contributor() -> ContributorIdentity {
        ContributorIdentity { user_key: "local-user-secret-abc".into() }
    }

    // ── rotating anon id ─────────────────────────────────────────────────────

    #[test]
    fn anon_id_is_stable_within_a_bucket_and_rotates_across() {
        let a = rotating_anon_id("user-x", 100);
        let b = rotating_anon_id("user-x", 100);
        let c = rotating_anon_id("user-x", 101);
        assert_eq!(a, b, "stable within a rotation window");
        assert_ne!(a, c, "rotates across windows");
        assert!(a.starts_with("anon-"));
    }

    #[test]
    fn anon_id_is_not_the_user_key_or_project_id() {
        let id = rotating_anon_id("local-user-secret-abc", 7);
        assert_ne!(id, "local-user-secret-abc");
        assert!(
            !id.contains("local-user-secret-abc"),
            "the user key must not appear in the anon id"
        );
    }

    #[test]
    fn rotation_bucket_groups_by_period() {
        let day = 86_400;
        // Two times 5 days apart, 30-day period → same bucket.
        assert_eq!(rotation_bucket(day * 2, 30), rotation_bucket(day * 7, 30));
        // 40 days apart → different bucket.
        assert_ne!(rotation_bucket(0, 30), rotation_bucket(day * 40, 30));
    }

    // ── the happy path + shape/anon-id shape guarantees ──────────────────────

    #[tokio::test]
    async fn global_output_carries_shape_and_opaque_anon_id_no_names() {
        let g = MockGeneralizer { response: None }; // LLM unavailable → deterministic fallback
        let art = anonymize_for_global(
            "Jane Doe fixed acme-api at /Users/dev/work/acme-api during the Acme Corp engagement",
            &ctx(),
            shape(),
            &contributor(),
            42,
            &g,
        )
        .await
        .expect("clean input must anonymise");

        let low = art.text.to_ascii_lowercase();
        assert!(!low.contains("acme"), "no project/client/repo name may survive: {:?}", art.text);
        assert!(!low.contains("jane"), "no person name may survive: {:?}", art.text);
        assert!(!art.text.contains("/Users/"), "no path may survive: {:?}", art.text);
        assert!(!low.contains("s-9931"), "no session id may survive: {:?}", art.text);

        // Shape carries buckets, never the name.
        assert_eq!(art.shape.stack, vec!["rust".to_string()]);
        assert_eq!(art.shape.size, Some(SizeBucket::Medium));

        // Attribution is anonymous with the opaque id (not the real user key or
        // any real id). Source-dereference is the always-on invariant on the text.
        assert_eq!(art.attribution.mode, AttributionMode::Anonymous);
        assert_eq!(art.attribution.author, None);
        assert_eq!(art.attribution.anonymous_id.as_deref(), Some(art.anon_id.as_str()));
        assert!(art.anon_id.starts_with("anon-"));
        assert_ne!(art.anon_id, contributor().user_key);
        assert!(!art.llm_polished, "no LLM was available");
    }

    // ── LLM post-check: a HOSTILE model output is discarded ──────────────────

    #[tokio::test]
    async fn llm_reintroducing_a_path_is_discarded_and_safe_text_kept() {
        // The model tries to sneak an absolute path back in.
        let g = MockGeneralizer {
            response: polish(
                "Always run migrations first, e.g. under /Users/dev/work/acme-api/db.",
            ),
        };
        let art = anonymize_for_global(
            "run migrations before deploy in acme-api",
            &ctx(),
            shape(),
            &contributor(),
            1,
            &g,
        )
        .await
        .expect("deterministic base is safe");
        assert!(!art.llm_polished, "hostile LLM output must be discarded");
        assert!(
            !art.text.contains("/Users/"),
            "reintroduced path must not survive: {:?}",
            art.text
        );
        assert!(
            !art.text.to_ascii_lowercase().contains("acme"),
            "reintroduced repo must not survive: {:?}",
            art.text
        );
    }

    #[tokio::test]
    async fn llm_reintroducing_a_known_name_is_discarded() {
        // The model reintroduces the client name — the deterministic re-check
        // must catch it and fall back to the safe text.
        let g = MockGeneralizer {
            response: polish("The Acme Corp team learned to write tests first."),
        };
        let art = anonymize_for_global("write tests first", &ctx(), shape(), &contributor(), 1, &g)
            .await
            .expect("base is safe");
        assert!(!art.llm_polished);
        assert!(
            !art.text.to_ascii_lowercase().contains("acme corp"),
            "reintroduced client survived: {:?}",
            art.text
        );
    }

    #[tokio::test]
    async fn clean_llm_rewrite_is_used() {
        let g = MockGeneralizer {
            response: polish(
                "Prefer a dedicated migration tool over hand-rolled SQL when the schema churns.",
            ),
        };
        let art = anonymize_for_global(
            "run migrations before deploy in acme-api",
            &ctx(),
            shape(),
            &contributor(),
            1,
            &g,
        )
        .await
        .expect("base is safe");
        assert!(art.llm_polished, "a clean rewrite should be adopted");
        assert!(art.text.contains("migration tool"));
    }

    // ── the polish's synthetic example ───────────────────────────────────────

    #[tokio::test]
    async fn a_clean_polish_carries_its_synthetic_example() {
        let g = polish_with(
            "Prefer a dedicated migration tool over hand-rolled SQL.",
            "A team hand-rolls one migration, then cannot reproduce the schema on a new laptop.",
        );
        let art = anonymize_for_global(
            "run migrations before deploy in acme-api",
            &ctx(),
            shape(),
            &contributor(),
            1,
            &MockGeneralizer { response: g },
        )
        .await
        .expect("base is safe");
        assert!(art.llm_polished);
        assert_eq!(
            art.example.as_deref(),
            Some(
                "A team hand-rolls one migration, then cannot reproduce the schema on a new laptop."
            ),
        );
    }

    #[tokio::test]
    async fn no_polish_means_no_example_nothing_is_invented_locally() {
        let art = anonymize_for_global(
            "run migrations before deploy in acme-api",
            &ctx(),
            shape(),
            &contributor(),
            1,
            &MockGeneralizer { response: None },
        )
        .await
        .expect("base is safe");
        assert!(!art.llm_polished);
        assert_eq!(art.example, None, "no model, no illustration — never a fabricated one");
    }

    #[tokio::test]
    async fn a_discarded_polish_takes_its_example_down_with_it() {
        // The model reintroduced a path in the BODY. Its example is untrusted for
        // the same reason, even though the example itself looks clean.
        let g = polish_with(
            "Always run migrations first, e.g. under /Users/dev/work/acme-api/db.",
            "A team forgets a migration and the next deploy fails on a missing column.",
        );
        let art = anonymize_for_global(
            "run migrations before deploy in acme-api",
            &ctx(),
            shape(),
            &contributor(),
            1,
            &MockGeneralizer { response: g },
        )
        .await
        .expect("base is safe");
        assert!(!art.llm_polished, "hostile body must be discarded");
        assert_eq!(
            art.example, None,
            "a model that leaked in the body is not trusted for the example either",
        );
    }

    #[tokio::test]
    async fn an_example_that_reintroduces_an_identifier_is_dropped_alone() {
        // Body is clean and survives; the EXAMPLE smuggles the client name back
        // in and must not ship. Dropping it loses an illustration, not the rule.
        let g = polish_with(
            "Prefer a dedicated migration tool over hand-rolled SQL.",
            "Acme Corp hand-rolled a migration and lost a column.",
        );
        let art = anonymize_for_global(
            "run migrations before deploy in acme-api",
            &ctx(),
            shape(),
            &contributor(),
            1,
            &MockGeneralizer { response: g },
        )
        .await
        .expect("base is safe");
        assert!(art.llm_polished, "the clean body is still adopted");
        assert!(art.text.contains("migration tool"));
        assert_eq!(art.example, None, "an example that reintroduces an identifier is dropped");
    }

    // ── fail-closed at the anonymise boundary ────────────────────────────────

    #[tokio::test]
    async fn anonymise_fails_closed_when_deterministic_strip_cant_guarantee_safety() {
        // A SCREAMING_SNAKE secret isn't a known token or a path, so the strip
        // leaves it — residual risk → the whole anonymise must refuse.
        let g = MockGeneralizer { response: polish("safe rewrite") };
        let out = anonymize_for_global(
            "set ACME_PROD_DB_PASSWORD then deploy",
            &ProjectIdentifiers::default(),
            shape(),
            &contributor(),
            1,
            &g,
        )
        .await;
        assert!(out.is_err(), "residual risk must make anonymise fail closed — never publish");
    }
}
