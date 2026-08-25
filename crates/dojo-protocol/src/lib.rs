//! Shared wire contract for sensei Dōjō federation.
//!
//! This crate carries two wire contracts over the Dōjō federation transport,
//! sharing one hashing/normalization discipline (DRY):
//!
//! 1. The **rules** contract — [`PublishedRule`] / [`PublishResponse`] /
//!    [`PulledRule`] / [`PullResponse`] plus the [`content_hash`] dedup key,
//!    pushed/pulled by the daemon's governance federation. `content_hash` MUST
//!    stay in lockstep with the daemon's dedup normalization in
//!    `senseid/src/governance.rs` (trim + lowercase).
//! 2. The **artifact** contract — the six artifact primitives that ride the
//!    contribute → triage → approve → distribute loop (see
//!    `docs/spec/pipeline/dojo-lifecycle.md`): the pure wire types the
//!    daemon (C6 publish) and consumers (C7 pull) exchange with a Dōjō server.
//!    Serializes to/from the `dojo.artifacts` DDL shape: a `kind` enum, a jsonb
//!    `payload` keyed by kind, plus scope / signature / attribution /
//!    dereferenced / status fields.
//!
//! No daemon or DB dependencies — pure serde wire types. The artifact
//! `signature` reuses the same [`content_hash`] normalization as the rules
//! contract (trim + lowercase, stable, order-insensitive).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The **relay** wire contract — filtered status + segment/gate/reply shapes the
/// daemon publishes to the Dōjō Worker `/v1/relay/*` (relay-engine.md §4/§6).
pub mod relay;

// ---------------------------------------------------------------------------
// Rules wire contract + shared hashing discipline.
//
// `content_hash` MUST stay in lockstep with the daemon's dedup normalization in
// `senseid/src/governance.rs` (trim + lowercase). The artifact `signature`
// below reuses it so both federation contracts share one discipline.
// ---------------------------------------------------------------------------

/// Normalize rule content for dedup/identity: trim + lowercase.
/// Mirrors `governance::structure_ruleset`'s dedup key.
pub fn normalize_content(content: &str) -> String {
    content.trim().to_lowercase()
}

/// Stable dedup key for a rule's content (sha256 hex of the normalized form).
pub fn content_hash(content: &str) -> String {
    let mut h = Sha256::new();
    h.update(normalize_content(content).as_bytes());
    format!("{:x}", h.finalize())
}

/// A rule published to a Dōjō — a flattened snapshot (no memory graph).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedRule {
    pub content_hash: String,
    pub scope_key: String,
    pub namespace_slug: String,
    pub namespace_name: String,
    pub rule_type: String,
    pub title: String,
    pub content: String,
    pub impact: Option<String>,
    pub enforcement: String,
    pub origin_repo: Option<String>,
    pub published_by: String,
    pub published_at: String,
}

/// Response to a publish: the canonical identity assigned by the Dōjō.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResponse {
    pub id: String,
    pub version: i32,
    pub seq: i64,
}

/// A rule as returned by a pull (snapshot + Dōjō identity + lifecycle).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulledRule {
    pub id: String,
    pub seq: i64,
    pub status: String, // "active" | "tombstoned"
    pub version: i32,
    #[serde(flatten)]
    pub rule: PublishedRule,
}

/// Response to a pull: deltas + the new cursor to persist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullResponse {
    pub rules: Vec<PulledRule>,
    pub cursor: i64,
}

/// The six primitives that ride the Dōjō loop. Matches the `dojo.artifact_kind`
/// enum exactly (`principle | pattern | prompt | guard | skill | agent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// 理 — durable engineering value.
    Principle,
    /// 紋 — constructive shape promoted to a rule (or an anti-pattern).
    Pattern,
    /// 問 — vetted prompt template or persona.
    Prompt,
    /// 守 — lint / check / safety rail.
    Guard,
    /// 技 — packaged capability an assistant can pick up.
    Skill,
    /// 使 — configured agent with tools and scope.
    Agent,
}

impl ArtifactKind {
    /// Every kind, in DDL declaration order.
    pub const ALL: [ArtifactKind; 6] = [
        ArtifactKind::Principle,
        ArtifactKind::Pattern,
        ArtifactKind::Prompt,
        ArtifactKind::Guard,
        ArtifactKind::Skill,
        ArtifactKind::Agent,
    ];

    /// The `dojo.artifact_kind` enum string. MUST match `artifact_kind.ddl`.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            ArtifactKind::Principle => "principle",
            ArtifactKind::Pattern => "pattern",
            ArtifactKind::Prompt => "prompt",
            ArtifactKind::Guard => "guard",
            ArtifactKind::Skill => "skill",
            ArtifactKind::Agent => "agent",
        }
    }

    /// Parse a `dojo.artifact_kind` enum string.
    pub fn from_db_str(s: &str) -> Option<Self> {
        ArtifactKind::ALL.into_iter().find(|k| k.as_db_str() == s)
    }
}

/// Publication state of an artifact. Matches the `dojo.artifact_status` enum
/// (`submitted | published | archived`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactStatus {
    /// Contributed, pending triage.
    Submitted,
    /// Approved and live — distributes downstream to matching scopes.
    Published,
    /// Declined or retired — retained for history, not distributed.
    Archived,
}

impl ArtifactStatus {
    pub const ALL: [ArtifactStatus; 3] =
        [ArtifactStatus::Submitted, ArtifactStatus::Published, ArtifactStatus::Archived];

    /// The `dojo.artifact_status` enum string. MUST match `artifact_status.ddl`.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            ArtifactStatus::Submitted => "submitted",
            ArtifactStatus::Published => "published",
            ArtifactStatus::Archived => "archived",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        ArtifactStatus::ALL.into_iter().find(|s2| s2.as_db_str() == s)
    }
}

/// How a contribution is credited when it leaves the machine. Matches the
/// `dojo.attribution_mode` enum (`named | anonymous`). Source-dereference is
/// NOT a mode — it is an always-on transform on the publish path (every
/// artifact is stripped regardless of credit). This enum is credit only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributionMode {
    /// Public or org-internal credit to the author.
    Named,
    /// A stable rotated anonymous id (the collective default). Client work is
    /// anonymous with no rotating id (no personal credit at all).
    Anonymous,
}

impl AttributionMode {
    pub const ALL: [AttributionMode; 2] = [AttributionMode::Named, AttributionMode::Anonymous];

    /// The `dojo.attribution_mode` enum string. MUST match `attribution_mode.ddl`.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            AttributionMode::Named => "named",
            AttributionMode::Anonymous => "anonymous",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        AttributionMode::ALL.into_iter().find(|m| m.as_db_str() == s)
    }
}

/// Credit metadata for an artifact — the `dojo.artifacts.attribution` jsonb
/// (`{author, org, anonymous_id}`) plus the resolved [`AttributionMode`].
/// Governed by the contributing membership's rules. Source-dereference is a
/// separate always-on invariant on the publish path, not stored here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attribution {
    /// named | anonymous.
    pub mode: AttributionMode,
    /// Author display name — present for `named`, dropped otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Contributing org.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub org: Option<String>,
    /// Stable rotated anonymous id — present for `anonymous`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anonymous_id: Option<String>,
}

/// Scope tag deciding who receives an artifact downstream — the
/// `dojo.artifacts.scope` jsonb (`{company | team | project | stack}`). A
/// consumer receives an artifact when its membership scope matches these tags.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ArtifactScope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}

/// Mode of a [`PatternPayload`] — mirrors the `inference.detected_patterns`
/// `family` enum (`design | anti | opt`). "Constructive shape" is `design`;
/// anti-patterns are `anti`; optimization opportunities are `opt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternFamily {
    Design,
    Anti,
    Opt,
}

/// Sub-kind of a [`PromptPayload`] — the spec distinguishes a vetted prompt
/// "template or persona".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptKind {
    Template,
    Persona,
}

/// `principle` (理) payload — a durable engineering value. The value statement
/// is the artifact `body`; the optional `rationale` carries the "why".
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PrinciplePayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

/// `pattern` (紋) payload — a constructive shape (or anti-pattern) promoted to
/// a rule downstream. Fields mirror `inference.detected_patterns`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternPayload {
    /// design | anti | opt.
    pub family: PatternFamily,
    /// Stable pattern id (e.g. `codebase.adapter`, `anti.duplication`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern_id: Option<String>,
    /// Observed first-try-resolution delta — the collective contribution bar
    /// is `>= +0.05` (collective-intelligence.md).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ftr_delta_observed: Option<f64>,
}

/// `prompt` (問) payload — a vetted prompt template or persona. The prompt text
/// itself is the artifact `body`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptPayload {
    /// template | persona.
    pub prompt_kind: PromptKind,
}

/// `guard` (守) payload — a lint / check / safety rail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardPayload {
    /// The check definition (the guard's check).
    pub check: String,
    /// The lint / check tool the guard runs under.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
}

/// `skill` (技) payload — a packaged capability an assistant can pick up. The
/// DDL keys this as "a skill's manifest".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillPayload {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The packaged skill manifest (free-form).
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub manifest: serde_json::Value,
}

/// `agent` (使) payload — a configured agent with tools and scope. The DDL keys
/// this as "an agent's tools/scope".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentPayload {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tools the agent is configured with.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    /// The agent's operating scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_scope: Option<String>,
}

/// Type-specific artifact payload — the `dojo.artifacts.payload` jsonb, keyed
/// by kind. Internally tagged on `kind` so the jsonb is self-describing (the
/// tag mirrors the envelope's `kind` column).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ArtifactPayload {
    Principle(PrinciplePayload),
    Pattern(PatternPayload),
    Prompt(PromptPayload),
    Guard(GuardPayload),
    Skill(SkillPayload),
    Agent(AgentPayload),
}

impl ArtifactPayload {
    /// The [`ArtifactKind`] this payload carries.
    pub fn kind(&self) -> ArtifactKind {
        match self {
            ArtifactPayload::Principle(_) => ArtifactKind::Principle,
            ArtifactPayload::Pattern(_) => ArtifactKind::Pattern,
            ArtifactPayload::Prompt(_) => ArtifactKind::Prompt,
            ArtifactPayload::Guard(_) => ArtifactKind::Guard,
            ArtifactPayload::Skill(_) => ArtifactKind::Skill,
            ArtifactPayload::Agent(_) => ArtifactKind::Agent,
        }
    }
}

/// Recursively serialize a JSON value with object keys sorted, so the canonical
/// form is independent of key insertion order (and of serde_json's
/// `preserve_order` feature). Used to make the artifact signature
/// order-insensitive over the payload.
fn canonical_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let inner: Vec<String> =
                keys.into_iter().map(|k| format!("{}:{}", k, canonical_json(&map[k]))).collect();
            format!("{{{}}}", inner.join(","))
        }
        serde_json::Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(canonical_json).collect();
            format!("[{}]", inner.join(","))
        }
        other => other.to_string(),
    }
}

/// Stable content signature for an artifact — the `dojo.artifacts.signature`
/// used for dedup / clustering (near-identical contributions share a signature
/// and are merged in triage).
///
/// Reuses the rules contract's [`normalize_content`] / [`content_hash`]
/// discipline (DRY): each textual
/// component is passed through [`normalize_content`] (trim + lowercase) and the
/// payload is canonicalized with sorted keys, so the signature is stable and
/// insensitive to surrounding whitespace, case, and payload key order — exactly
/// mirroring the rules contract's hash tests. The final digest is
/// [`content_hash`] over the joined canonical form.
pub fn artifact_signature(
    kind: ArtifactKind,
    title: &str,
    body: &str,
    payload: &ArtifactPayload,
) -> String {
    let payload_value = serde_json::to_value(payload).unwrap_or(serde_json::Value::Null);
    // Unit separator keeps components from bleeding into one another.
    let joined = [
        normalize_content(kind.as_db_str()),
        normalize_content(title),
        normalize_content(body),
        normalize_content(&canonical_json(&payload_value)),
    ]
    .join("\u{1f}");
    content_hash(&joined)
}

/// An artifact contributed / published to a Dōjō — a flattened snapshot
/// carrying its type-specific payload. The artifact analogue of
/// [`PublishedRule`]. Server-assigned identity (`id`, `seq`) and
/// downstream telemetry are not part of the contribution snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PublishedArtifact {
    /// Content signature (see [`artifact_signature`]).
    pub signature: String,
    /// Discovery-path key of the destination tenant Dōjō
    /// (`"<origin>/<org>[/<dojo>]"`).
    pub tenant_key: String,
    /// Set when the artifact came from client work under an engagement (such
    /// artifacts must be `dereferenced`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engagement_id: Option<String>,
    pub kind: ArtifactKind,
    pub title: String,
    pub body: String,
    pub payload: ArtifactPayload,
    #[serde(default)]
    pub scope: ArtifactScope,
    pub attribution: Attribution,
    /// Contributor user id, or `None` when anonymised.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contributed_by: Option<String>,
    /// RFC 3339 timestamp, set once published.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
}

impl PublishedArtifact {
    /// Compute the signature from the current title / body / payload.
    pub fn compute_signature(&self) -> String {
        artifact_signature(self.kind, &self.title, &self.body, &self.payload)
    }

    /// Whether the declared `kind` agrees with the payload's tag.
    pub fn kind_matches_payload(&self) -> bool {
        self.kind == self.payload.kind()
    }
}

/// Canonical identity a Dōjō assigns on publish. The artifact analogue of
/// [`PublishResponse`] (artifacts have no version column, so no
/// `version` field). `seq` is the federation cursor value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishArtifactResponse {
    pub id: String,
    pub seq: i64,
}

/// An artifact as returned by a pull — the snapshot plus Dōjō identity and
/// lifecycle state. The artifact analogue of [`PulledRule`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PulledArtifact {
    pub id: String,
    pub seq: i64,
    pub status: ArtifactStatus,
    #[serde(flatten)]
    pub artifact: PublishedArtifact,
}

/// Response to a pull: the delta artifacts plus the new cursor to persist. The
/// artifact analogue of [`PullResponse`] — `seq`-cursor pagination so C7
/// resumes gap-free from the last seen `seq`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactPullResponse {
    pub artifacts: Vec<PulledArtifact>,
    pub cursor: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_payload(kind: ArtifactKind) -> ArtifactPayload {
        match kind {
            ArtifactKind::Principle => ArtifactPayload::Principle(PrinciplePayload {
                rationale: Some("keeps the system honest".into()),
            }),
            ArtifactKind::Pattern => ArtifactPayload::Pattern(PatternPayload {
                family: PatternFamily::Design,
                pattern_id: Some("codebase.adapter".into()),
                ftr_delta_observed: Some(0.07),
            }),
            ArtifactKind::Prompt => {
                ArtifactPayload::Prompt(PromptPayload { prompt_kind: PromptKind::Persona })
            }
            ArtifactKind::Guard => ArtifactPayload::Guard(GuardPayload {
                check: "no unwrap in library code".into(),
                tool: Some("clippy".into()),
            }),
            ArtifactKind::Skill => ArtifactPayload::Skill(SkillPayload {
                name: "extract-docs".into(),
                description: Some("generate docstrings".into()),
                manifest: json!({ "steps": ["map", "write"] }),
            }),
            ArtifactKind::Agent => ArtifactPayload::Agent(AgentPayload {
                name: "sensei-analyst".into(),
                description: Some("problem analysis".into()),
                tools: vec!["Read".into(), "Grep".into()],
                agent_scope: Some("analysis".into()),
            }),
        }
    }

    fn sample_artifact(kind: ArtifactKind) -> PublishedArtifact {
        let payload = sample_payload(kind);
        let title = "title".to_string();
        let body = "body".to_string();
        PublishedArtifact {
            signature: artifact_signature(kind, &title, &body, &payload),
            tenant_key: "github/sensei-hq".into(),
            engagement_id: None,
            kind,
            title,
            body,
            payload,
            scope: ArtifactScope { stack: Some("rust".into()), ..Default::default() },
            attribution: Attribution {
                mode: AttributionMode::Named,
                author: Some("jerry".into()),
                org: Some("sensei-hq".into()),
                anonymous_id: None,
            },
            contributed_by: Some("jerry".into()),
            published_at: Some("2026-07-08T00:00:00Z".into()),
        }
    }

    // ---- kind <-> DB enum string mapping -------------------------------

    #[test]
    fn artifact_kind_db_strings_match_ddl() {
        // MUST match enum/dojo/artifact_kind.ddl exactly.
        let expected = ["principle", "pattern", "prompt", "guard", "skill", "agent"];
        let got: Vec<&str> = ArtifactKind::ALL.iter().map(|k| k.as_db_str()).collect();
        assert_eq!(got, expected);
    }

    #[test]
    fn artifact_kind_db_str_round_trips() {
        for k in ArtifactKind::ALL {
            assert_eq!(ArtifactKind::from_db_str(k.as_db_str()), Some(k));
        }
        assert_eq!(ArtifactKind::from_db_str("nope"), None);
    }

    #[test]
    fn artifact_kind_serde_matches_db_str() {
        for k in ArtifactKind::ALL {
            let json = serde_json::to_string(&k).unwrap();
            assert_eq!(json, format!("\"{}\"", k.as_db_str()));
        }
    }

    #[test]
    fn artifact_status_db_strings_match_ddl() {
        let expected = ["submitted", "published", "archived"];
        let got: Vec<&str> = ArtifactStatus::ALL.iter().map(|s| s.as_db_str()).collect();
        assert_eq!(got, expected);
        for s in ArtifactStatus::ALL {
            assert_eq!(ArtifactStatus::from_db_str(s.as_db_str()), Some(s));
        }
    }

    #[test]
    fn attribution_mode_db_strings_match_ddl() {
        let expected = ["named", "anonymous"];
        let got: Vec<&str> = AttributionMode::ALL.iter().map(|m| m.as_db_str()).collect();
        assert_eq!(got, expected);
        for m in AttributionMode::ALL {
            assert_eq!(AttributionMode::from_db_str(m.as_db_str()), Some(m));
        }
    }

    // ---- payload serde round-trips (one per kind) ----------------------

    #[test]
    fn payload_round_trips_for_every_kind() {
        for kind in ArtifactKind::ALL {
            let payload = sample_payload(kind);
            assert_eq!(payload.kind(), kind);
            let json = serde_json::to_string(&payload).unwrap();
            let back: ArtifactPayload = serde_json::from_str(&json).unwrap();
            assert_eq!(back, payload, "round-trip mismatch for {kind:?}");
            // Internally tagged on `kind` matching the DB enum string.
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(value["kind"], json!(kind.as_db_str()));
        }
    }

    #[test]
    fn published_artifact_round_trips_for_every_kind() {
        for kind in ArtifactKind::ALL {
            let artifact = sample_artifact(kind);
            assert!(artifact.kind_matches_payload());
            let json = serde_json::to_string(&artifact).unwrap();
            let back: PublishedArtifact = serde_json::from_str(&json).unwrap();
            assert_eq!(back, artifact, "envelope round-trip mismatch for {kind:?}");
        }
    }

    #[test]
    fn pull_response_round_trips_with_cursor() {
        let artifact = sample_artifact(ArtifactKind::Guard);
        let resp = ArtifactPullResponse {
            artifacts: vec![PulledArtifact {
                id: "11111111-1111-1111-1111-111111111111".into(),
                seq: 42,
                status: ArtifactStatus::Published,
                artifact: artifact.clone(),
            }],
            cursor: 42,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: ArtifactPullResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back, resp);
        assert_eq!(back.cursor, 42);
        // The pulled row flattens the envelope (id/seq/status alongside fields).
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["artifacts"][0]["seq"], json!(42));
        assert_eq!(value["artifacts"][0]["status"], json!("published"));
        assert_eq!(value["artifacts"][0]["kind"], json!("guard"));
    }

    #[test]
    fn publish_response_round_trips() {
        let resp =
            PublishArtifactResponse { id: "22222222-2222-2222-2222-222222222222".into(), seq: 7 };
        let json = serde_json::to_string(&resp).unwrap();
        let back: PublishArtifactResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back, resp);
    }

    // ---- signature determinism + insensitivity (mirrors the rules content_hash) --

    #[test]
    fn signature_is_deterministic_and_sha256_hex() {
        let payload = sample_payload(ArtifactKind::Principle);
        let a = artifact_signature(ArtifactKind::Principle, "Title", "Body", &payload);
        let b = artifact_signature(ArtifactKind::Principle, "Title", "Body", &payload);
        assert_eq!(a, b);
        assert_eq!(a.len(), 64); // sha256 hex, same as the rules content_hash
    }

    #[test]
    fn signature_is_whitespace_and_case_insensitive() {
        // Mirrors the rules content_hash test: differing only by
        // surrounding whitespace / case yields the same signature.
        let payload = sample_payload(ArtifactKind::Guard);
        let a = artifact_signature(ArtifactKind::Guard, "  Use TDD  ", "  Always.  ", &payload);
        let b = artifact_signature(ArtifactKind::Guard, "use tdd", "always.", &payload);
        assert_eq!(a, b);
    }

    #[test]
    fn signature_is_payload_key_order_insensitive() {
        // Two skill payloads whose manifest objects differ only in key order
        // must share a signature (canonical_json sorts keys).
        let p1 = ArtifactPayload::Skill(SkillPayload {
            name: "s".into(),
            description: None,
            manifest: json!({ "a": 1, "b": 2 }),
        });
        let p2 = ArtifactPayload::Skill(SkillPayload {
            name: "s".into(),
            description: None,
            manifest: json!({ "b": 2, "a": 1 }),
        });
        let a = artifact_signature(ArtifactKind::Skill, "t", "b", &p1);
        let b = artifact_signature(ArtifactKind::Skill, "t", "b", &p2);
        assert_eq!(a, b);
    }

    #[test]
    fn signature_differs_for_different_content() {
        let payload = sample_payload(ArtifactKind::Prompt);
        let a = artifact_signature(ArtifactKind::Prompt, "one", "body", &payload);
        let b = artifact_signature(ArtifactKind::Prompt, "two", "body", &payload);
        assert_ne!(a, b);
    }

    #[test]
    fn signature_differs_by_kind() {
        // Same title/body but different kind → different signature (kind is
        // part of the canonical form, and each kind carries a distinct payload).
        let a = artifact_signature(
            ArtifactKind::Principle,
            "t",
            "b",
            &sample_payload(ArtifactKind::Principle),
        );
        let b = artifact_signature(
            ArtifactKind::Pattern,
            "t",
            "b",
            &sample_payload(ArtifactKind::Pattern),
        );
        assert_ne!(a, b);
    }

    #[test]
    fn compute_signature_matches_free_function() {
        let artifact = sample_artifact(ArtifactKind::Agent);
        assert_eq!(
            artifact.compute_signature(),
            artifact_signature(artifact.kind, &artifact.title, &artifact.body, &artifact.payload)
        );
    }

    // ---- rules wire contract (folded in from the former protocol crate) --

    #[test]
    fn content_hash_matches_governance_normalization() {
        // governance.rs dedups on `content.trim().to_lowercase()`.
        // Same logical content (differing only by surrounding ws / case) → same hash.
        let a = content_hash("  Use TDD always.  ");
        let b = content_hash("use tdd always.");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64); // sha256 hex
    }

    #[test]
    fn content_hash_differs_for_different_content() {
        assert_ne!(content_hash("rule one"), content_hash("rule two"));
    }

    #[test]
    fn published_rule_round_trips() {
        let r = PublishedRule {
            content_hash: content_hash("x"),
            scope_key: "organization".into(),
            namespace_slug: "sensei-hq".into(),
            namespace_name: "Sensei HQ".into(),
            rule_type: "convention".into(),
            title: "t".into(),
            content: "x".into(),
            impact: None,
            enforcement: "mandatory".into(),
            origin_repo: Some("sensei/daemon".into()),
            published_by: "jerry".into(),
            published_at: "2026-06-11T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: PublishedRule = serde_json::from_str(&json).unwrap();
        assert_eq!(back.namespace_slug, "sensei-hq");
        assert_eq!(back.enforcement, "mandatory");
    }
}
