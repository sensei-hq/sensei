//! Governance plane — deterministic Tier-1 rule resolution.
//!
//! Turns a repo's namespace memberships + the always-on general/user scopes
//! into the ordered, deduplicated, mandatory-aware ruleset a session should
//! obey. The DB query ([`crate::db::pg_store::PgStore::resolve_rules_raw`])
//! does the gather + ordering (enforcement desc → scope level desc); the pure
//! [`structure_ruleset`] below applies dedup + the mandatory-lock flag so the
//! logic is unit-testable without a database.
//!
//! Tier 2 (LLM consolidation merge, #19) layers on top of this and is not part
//! of this module.

/// A raw resolution row as returned by the DB, already ordered strongest-first
/// (enforcement desc, then scope level desc, then strength desc).
#[derive(Debug, Clone, PartialEq)]
pub struct RawRule {
    pub id: String,
    pub title: String,
    pub content: String,
    pub impact: Option<String>,
    /// `advisory` | `recommended` | `required` | `mandatory`.
    pub enforcement: String,
    /// Scope key the rule applies at (`general`…`repository`).
    pub scope: String,
    /// Namespace instance name (e.g. "Sensei HQ", "sensei", "rust"), if scoped.
    pub namespace: Option<String>,
}

/// One resolved rule in a repo's governing ruleset.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ResolvedRule {
    pub id: String,
    pub title: String,
    pub content: String,
    pub impact: Option<String>,
    pub enforcement: String,
    pub scope: String,
    pub namespace: Option<String>,
    /// `enforcement == "mandatory"` — the non-overridable constitution tier.
    pub mandatory: bool,
}

/// A repo's resolved governing ruleset, strongest-first.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ResolvedRuleset {
    pub rules: Vec<ResolvedRule>,
    pub total: usize,
    pub mandatory_count: usize,
}

/// Structure already-ordered raw rules into a resolved ruleset: drop duplicates
/// (same normalized content), keeping the **highest-authority** instance — since
/// the input is pre-ordered strongest-first, the first occurrence wins — and
/// flag the mandatory (non-overridable) ones. Pure.
pub fn structure_ruleset(rows: Vec<RawRule>) -> ResolvedRuleset {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut rules: Vec<ResolvedRule> = Vec::with_capacity(rows.len());
    for r in rows {
        let key = r.content.trim().to_lowercase();
        if key.is_empty() || !seen.insert(key) {
            continue; // empty or a weaker duplicate of a rule already kept
        }
        let mandatory = r.enforcement == "mandatory";
        rules.push(ResolvedRule {
            id: r.id,
            title: r.title,
            content: r.content,
            impact: r.impact,
            enforcement: r.enforcement,
            scope: r.scope,
            namespace: r.namespace,
            mandatory,
        });
    }
    let mandatory_count = rules.iter().filter(|r| r.mandatory).count();
    let total = rules.len();
    ResolvedRuleset { rules, total, mandatory_count }
}

/// Header marking the global rules file as daemon-managed.
const RULES_MD_HEADER: &str = "<!-- Managed by sensei — regenerated from the governance plane on daemon \
start and on demand. Do not edit by hand: add rules via the knowledge plane \
(save_memory / propose_memory) and they appear here. Repository-specific rules \
resolve live — call the `get_rules` MCP tool at the start of a task. -->";

/// Render the global, always-on ruleset (user + general scope) to the Markdown
/// written to `~/.sensei/rules.md`. Rules are grouped by enforcement, strongest
/// first, so the constitution tier reads at the top. Pure.
pub fn render_rules_md(set: &ResolvedRuleset) -> String {
    let mut out = String::new();
    out.push_str(RULES_MD_HEADER);
    out.push_str("\n\n# Sensei Rules\n\n");
    out.push_str(
        "Your always-on rules (user + general scope). Repository-specific rules \
         (organization / project / technology) are **not** listed here — call the \
         `get_rules` MCP tool in a repo to resolve them.\n",
    );

    if set.rules.is_empty() {
        out.push_str("\n_No global rules yet. Save a user- or general-scope rule and it will appear here._\n");
        return out;
    }

    // Group by enforcement, strongest first. Input is already ordered, but
    // section headers make authority explicit.
    for (level, heading) in [
        ("mandatory", "## Mandatory — non-negotiable"),
        ("required", "## Required"),
        ("recommended", "## Recommended"),
        ("advisory", "## Advisory"),
    ] {
        let group: Vec<&ResolvedRule> = set.rules.iter().filter(|r| r.enforcement == level).collect();
        if group.is_empty() {
            continue;
        }
        out.push('\n');
        out.push_str(heading);
        out.push('\n');
        for r in group {
            let scope = if r.scope.is_empty() { "general".to_string() } else { r.scope.clone() };
            out.push_str(&format!("- **{}** — {} _({})_\n", r.title.trim(), r.content.trim(), scope));
            if let Some(impact) = r.impact.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                out.push_str(&format!("  > {impact}\n"));
            }
        }
    }
    out
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
    fn dedup_keeps_highest_authority_first_occurrence() {
        // Input is pre-ordered strongest-first: the mandatory org rule precedes
        // an advisory repo rule with identical content. The mandatory one wins.
        let rows = vec![
            raw("never log secrets", "mandatory", "organization"),
            raw("never log secrets", "advisory", "repository"),
            raw("prefer early returns", "recommended", "project"),
        ];
        let set = structure_ruleset(rows);
        assert_eq!(set.total, 2, "the duplicate is dropped");
        assert_eq!(set.mandatory_count, 1);
        let secrets = set.rules.iter().find(|r| r.content == "never log secrets").unwrap();
        assert_eq!(secrets.enforcement, "mandatory", "kept the mandatory instance, not the advisory dup");
        assert!(secrets.mandatory);
        assert_eq!(set.rules[0].content, "never log secrets", "ordering preserved");
    }

    #[test]
    fn empty_content_skipped_and_empty_input_is_empty() {
        let rows = vec![raw("   ", "required", "project")];
        assert_eq!(structure_ruleset(rows).total, 0);
        assert_eq!(structure_ruleset(vec![]).total, 0);
    }

    #[test]
    fn mandatory_flag_only_for_mandatory() {
        let rows = vec![
            raw("a", "required", "project"),
            raw("b", "mandatory", "organization"),
            raw("c", "advisory", "user"),
        ];
        let set = structure_ruleset(rows);
        assert_eq!(set.mandatory_count, 1);
        assert!(set.rules.iter().find(|r| r.content == "b").unwrap().mandatory);
        assert!(!set.rules.iter().find(|r| r.content == "a").unwrap().mandatory);
    }

    #[test]
    fn render_groups_by_enforcement_strongest_first() {
        let set = structure_ruleset(vec![
            raw("never log secrets", "mandatory", "user"),
            raw("prefer early returns", "recommended", "general"),
            raw("tabs over spaces", "advisory", "user"),
        ]);
        let md = render_rules_md(&set);
        assert!(md.contains("Managed by sensei"), "has managed header");
        assert!(md.contains("get_rules"), "points at the live per-repo tool");
        let mand = md.find("## Mandatory").unwrap();
        let rec = md.find("## Recommended").unwrap();
        let adv = md.find("## Advisory").unwrap();
        assert!(mand < rec && rec < adv, "sections ordered by authority");
        assert!(md.contains("**never log secrets**"));
    }

    #[test]
    fn render_empty_ruleset_has_placeholder_not_sections() {
        let md = render_rules_md(&structure_ruleset(vec![]));
        assert!(md.contains("No global rules yet"));
        assert!(!md.contains("## Mandatory"));
    }
}
