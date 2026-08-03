//! Compose a project's resolved constitution for dōjō federation.
//!
//! The daemon owns the authority resolution (dedup + mandatory locks + the
//! discards the ladder makes); the dōjō only DISPLAYS it (resolved design Q2/Q3).
//! This maps a folder's resolved [`RawRule`]s — already ordered strongest-first
//! by [`crate::db::pg_store::PgStore::resolve_rules_raw`] — onto the wire
//! [`RelayConstitution`] the dōjō's project-detail preview renders: the effective
//! rules tagged with a ladder level, the discards (a weaker duplicate that lost
//! to a higher-authority scope), and the ★-lock count. Pure — unit-tested
//! without a database.

use crate::governance::RawRule;
use dojo_protocol::relay::{RelayConstitution, RelayConstitutionConflict, RelayConstitutionRule};

/// Map a daemon rule `scope` to the dōjō ladder level (`company | client |
/// personal | project | stack`). Unknown scopes pass through unchanged — honest,
/// never silently coerced onto a wrong rung.
pub fn scope_to_level(scope: &str) -> &str {
    match scope {
        "organization" => "company",
        "client" => "client",
        "user" | "general" => "personal",
        "technology" => "stack",
        "project" | "repository" => "project",
        other => other,
    }
}

/// Compose the effective constitution from already-ordered (strongest-first) raw
/// rules. Dedup keeps the highest-authority instance of identical content (the
/// first occurrence wins) and records each dropped weaker duplicate as a
/// discarded conflict (loser scope → winner scope). Pure.
pub fn compose_constitution(rows: Vec<RawRule>) -> RelayConstitution {
    // Dedup by normalized content, keeping the first (strongest) occurrence — the
    // input is pre-ordered strongest-first, so the first is the highest authority.
    // A later duplicate is the loser the ladder discards; record it as a conflict.
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut rules: Vec<RelayConstitutionRule> = Vec::new();
    // Track the winning level per content so a discard names the scope that beat it.
    let mut winner: std::collections::HashMap<String, (String, bool)> = std::collections::HashMap::new();
    let mut conflicts: Vec<RelayConstitutionConflict> = Vec::new();

    for r in rows {
        let key = r.content.trim().to_lowercase();
        if key.is_empty() {
            continue;
        }
        let level = scope_to_level(&r.scope).to_string();
        let hard = r.enforcement == "mandatory";
        if seen.insert(key.clone()) {
            winner.insert(key, (level.clone(), hard));
            rules.push(RelayConstitutionRule { level, text: r.content.trim().to_string(), hard });
        } else {
            let (winner_level, winner_hard) = winner.get(&key).cloned().unwrap_or_default();
            conflicts.push(RelayConstitutionConflict {
                topic: r.content.trim().to_string(),
                loser_level: level,
                winner_level,
                why: "a higher-authority scope already states this rule".to_string(),
                locked: winner_hard,
            });
        }
    }

    let locks = rules.iter().filter(|r| r.hard).count() as u32;
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
            namespace: Some(scope.to_string()),
        }
    }

    #[test]
    fn maps_scopes_to_ladder_levels() {
        assert_eq!(scope_to_level("organization"), "company");
        assert_eq!(scope_to_level("client"), "client");
        assert_eq!(scope_to_level("user"), "personal");
        assert_eq!(scope_to_level("general"), "personal");
        assert_eq!(scope_to_level("technology"), "stack");
        assert_eq!(scope_to_level("project"), "project");
        assert_eq!(scope_to_level("repository"), "project");
        assert_eq!(scope_to_level("weird"), "weird", "unknown passes through, not coerced");
    }

    #[test]
    fn composes_effective_rules_tagged_by_level_strongest_first() {
        let out = compose_constitution(vec![
            raw("never log secrets", "mandatory", "organization"),
            raw("prefer early returns", "recommended", "project"),
            raw("tabs over spaces", "advisory", "technology"),
        ]);
        assert_eq!(out.rules.len(), 3);
        assert_eq!(out.rules[0].level, "company");
        assert_eq!(out.rules[0].text, "never log secrets");
        assert!(out.rules[0].hard, "mandatory → ★ hard lock");
        assert_eq!(out.rules[1].level, "project");
        assert!(!out.rules[1].hard);
        assert_eq!(out.rules[2].level, "stack");
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
        // The duplicate is dropped from the effective set…
        assert_eq!(out.rules.len(), 2, "the weaker duplicate is not an effective rule");
        // …and surfaced as a discard the ladder made.
        assert_eq!(out.conflicts.len(), 1);
        let c = &out.conflicts[0];
        assert_eq!(c.topic, "never log secrets");
        assert_eq!(c.winner_level, "company", "kept the organization instance");
        assert_eq!(c.loser_level, "project", "discarded the repository instance");
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
