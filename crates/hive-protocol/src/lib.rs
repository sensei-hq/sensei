//! Shared wire contract for sensei hive-mind federation.
//! `content_hash` MUST stay in lockstep with the daemon's dedup
//! normalization in `senseid/src/governance.rs` (trim + lowercase).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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

/// A rule published to the hive — a flattened snapshot (no memory graph).
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

/// Response to a publish: the canonical identity assigned by the hive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResponse {
    pub id: String,
    pub version: i32,
    pub seq: i64,
}

/// A rule as returned by a pull (snapshot + hive identity + lifecycle).
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

#[cfg(test)]
mod tests {
    use super::*;

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
