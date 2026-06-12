//! Federation: push promoted rules to a hive-mind and poll-pull applicable rules
//! back as memories(origin='federated'). The ACP never talks to a hive; senseid
//! owns all outbound calls (spec §4).

use crate::db::pg_store::MemoryPushPayload;
use hive_protocol::{content_hash, PublishedRule};

/// Build the wire payload for a memory being published. `published_by`/`published_at`
/// are stamped server-side by the hive (the hive overrides them from the API key's
/// member + now()), so we send harmless best-effort placeholders.
pub fn build_published_rule(p: &MemoryPushPayload, origin_repo: Option<&str>) -> PublishedRule {
    PublishedRule {
        content_hash: content_hash(&p.content),
        scope_key: p.scope_key.clone(),
        namespace_slug: p.slug.clone(),
        namespace_name: p.name.clone(),
        rule_type: p.rule_type.clone(),
        title: p.title.clone(),
        content: p.content.clone(),
        impact: p.impact.clone(),
        enforcement: p.enforcement.clone(),
        origin_repo: origin_repo.map(|s| s.to_string()),
        published_by: "senseid".to_string(),       // hive overrides from the API key's member
        published_at: "1970-01-01T00:00:00Z".to_string(), // hive overrides with now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pg_store::MemoryPushPayload;

    #[test]
    fn builds_published_rule_with_content_hash_and_namespace_identity() {
        let p = MemoryPushPayload {
            title: "TDD".into(), content: "  Always use TDD  ".into(), impact: None,
            enforcement: "mandatory".into(), rule_type: "convention".into(), origin: "promoted".into(),
            scope_key: "organization".into(), slug: "sensei-hq".into(), name: "Sensei HQ".into(),
        };
        let pr = build_published_rule(&p, Some("sensei/daemon"));
        // content_hash normalizes (trim+lowercase) — matches hive-protocol.
        assert_eq!(pr.content_hash, hive_protocol::content_hash("Always use TDD"));
        assert_eq!(pr.scope_key, "organization");
        assert_eq!(pr.namespace_slug, "sensei-hq");
        assert_eq!(pr.namespace_name, "Sensei HQ");
        assert_eq!(pr.enforcement, "mandatory");
        assert_eq!(pr.rule_type, "convention");
        assert_eq!(pr.origin_repo.as_deref(), Some("sensei/daemon"));
    }
}
