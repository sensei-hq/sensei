//! Compose a project's resolved constitution for dōjō federation.
//!
//! The daemon owns the authority resolution (dedup + mandatory locks + the
//! discards the ladder makes); the dōjō only DISPLAYS it (resolved design Q2/Q3).
//! This maps a folder's resolved [`RawRule`]s — already ordered strongest-first
//! by [`crate::db::pg_store::PgStore::resolve_rules_raw`] — onto the wire
//! [`RelayConstitution`] the dōjō's project-detail preview renders: the effective
//! rules (carried in the daemon's own `scope_key`/`enforcement` vocabulary so the
//! dōjō reuses its existing scope→rung display map), the discards (a weaker
//! duplicate that lost to a higher-authority scope), and the ★-lock count. Pure —
//! unit-tested without a database.

use crate::governance::RawRule;
use dojo_protocol::relay::{RelayConstitution, RelayConstitutionConflict, RelayConstitutionRule};

/// Compose the effective constitution from already-ordered (strongest-first) raw
/// rules. Dedup keeps the highest-authority instance of identical content (the
/// first occurrence wins) and records each dropped weaker duplicate as a
/// discarded conflict (loser scope → winner scope). Rules are emitted in the
/// daemon's resolution vocabulary (`scope_key`/`title`/`enforcement`/`namespace`);
/// the dōjō maps `scope_key` to a display rung. Pure.
pub fn compose_constitution(rows: Vec<RawRule>) -> RelayConstitution {
    // Dedup by normalized content, keeping the first (strongest) occurrence. Track
    // the winner's scope + mandatory flag per content so a discard names what beat
    // it. `order` preserves the strongest-first order for the effective set.
    let mut winner: std::collections::HashMap<String, (String, bool)> =
        std::collections::HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut kept: std::collections::HashMap<String, RawRule> = std::collections::HashMap::new();
    let mut conflicts: Vec<RelayConstitutionConflict> = Vec::new();

    for r in rows {
        let key = r.content.trim().to_lowercase();
        if key.is_empty() {
            continue;
        }
        if let Some((winner_scope, winner_hard)) = winner.get(&key) {
            conflicts.push(RelayConstitutionConflict {
                topic: r.content.trim().to_string(),
                loser_scope: r.scope.clone(),
                winner_scope: winner_scope.clone(),
                why: "a higher-authority scope already states this rule".to_string(),
                locked: *winner_hard,
            });
        } else {
            winner.insert(key.clone(), (r.scope.clone(), r.enforcement == "mandatory"));
            order.push(key.clone());
            kept.insert(key, r);
        }
    }

    let rules: Vec<RelayConstitutionRule> = order
        .iter()
        .map(|k| {
            let r = &kept[k];
            RelayConstitutionRule {
                scope_key: r.scope.clone(),
                namespace: r.namespace.clone(),
                title: r.title.trim().to_string(),
                enforcement: r.enforcement.clone(),
            }
        })
        .collect();
    let locks = rules.iter().filter(|r| r.enforcement == "mandatory").count() as u32;

    RelayConstitution { rules, conflicts, locks }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(content: &str, enforcement: &str, scope: &str) -> RawRule {
        RawRule {
            id: format!("id-{content}-{scope}"),
            title: content.to_string(),
            content: content.to_string(),
            impact: None,
            enforcement: enforcement.to_string(),
            scope: scope.to_string(),
            namespace: Some(format!("ns-{scope}")),
        }
    }

    #[test]
    fn composes_effective_rules_in_daemon_scope_vocabulary_strongest_first() {
        let out = compose_constitution(vec![
            raw("never log secrets", "mandatory", "organization"),
            raw("prefer early returns", "recommended", "project"),
            raw("tabs over spaces", "advisory", "technology"),
        ]);
        assert_eq!(out.rules.len(), 3);
        assert_eq!(out.rules[0].scope_key, "organization");
        assert_eq!(out.rules[0].title, "never log secrets");
        assert_eq!(out.rules[0].enforcement, "mandatory");
        assert_eq!(out.rules[0].namespace.as_deref(), Some("ns-organization"));
        assert_eq!(out.rules[1].scope_key, "project");
        assert_eq!(out.rules[2].scope_key, "technology");
        assert_eq!(out.locks, 1, "one mandatory rule locked");
        assert!(out.conflicts.is_empty(), "no duplicates → no discards");
    }

    #[test]
    fn dedup_records_the_discarded_weaker_duplicate_as_a_conflict() {
        // Same content at two scopes; ordered strongest-first (organization
        // mandatory precedes the repository advisory dup).
        let out = compose_constitution(vec![
            raw("never log secrets", "mandatory", "organization"),
            raw("never log secrets", "advisory", "repository"),
            raw("prefer early returns", "recommended", "project"),
        ]);
        assert_eq!(out.rules.len(), 2, "the weaker duplicate is not an effective rule");
        assert_eq!(out.conflicts.len(), 1);
        let c = &out.conflicts[0];
        assert_eq!(c.topic, "never log secrets");
        assert_eq!(c.winner_scope, "organization", "kept the higher-authority instance");
        assert_eq!(c.loser_scope, "repository", "discarded the weaker duplicate");
        assert!(c.locked, "the winner is mandatory");
    }

    #[test]
    fn skips_empty_content() {
        let out = compose_constitution(vec![raw("   ", "required", "project")]);
        assert!(out.rules.is_empty());
        assert!(out.conflicts.is_empty());
        assert_eq!(out.locks, 0);
    }
}
