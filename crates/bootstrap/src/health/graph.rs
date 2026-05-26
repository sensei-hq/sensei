//! Universal dependency spec — same on every platform.
//! Specs are pure data: id, label, note, topological edges. Platform-specific
//! checkers are bound at iteration time by PlatformProvider::checker_for(id)
//! in Section C; the orchestrator iterates these specs and asks the platform
//! for the right checker per id.

use super::types::ComponentId;

#[derive(Debug, Clone, Copy)]
pub struct DependencySpec {
    pub id:         ComponentId,
    pub label:      &'static str,
    pub note:       Option<&'static str>,
    pub depends_on: &'static [ComponentId],
    /// Verb shown when the component is in `ComponentStatus::Installing`.
    /// Service-style deps say "starting" (brew services), database says
    /// "setting up" (dbd schema), pure brew installs say "installing".
    /// Lives here so CLI doctor and the Svelte ledger share one source of
    /// truth via the wire shape.
    pub installing_verb: &'static str,
}

const GRAPH: [DependencySpec; 5] = [
    DependencySpec { id: ComponentId::Postgres, label: "PostgreSQL",        note: None,
                     depends_on: &[], installing_verb: "starting" },
    DependencySpec { id: ComponentId::Ollama,   label: "Ollama",            note: None,
                     depends_on: &[], installing_verb: "starting" },
    DependencySpec { id: ComponentId::Sensei,   label: "Sensei components", note: Some("cli · mcp · daemon"),
                     depends_on: &[], installing_verb: "installing" },
    DependencySpec { id: ComponentId::Database, label: "Database & schema", note: Some("pgvector · sensei tables"),
                     depends_on: &[ComponentId::Postgres], installing_verb: "setting up" },
    DependencySpec { id: ComponentId::Daemon,   label: "Background daemon", note: None,
                     depends_on: &[ComponentId::Database, ComponentId::Sensei], installing_verb: "starting" },
];

pub fn dependency_specs() -> &'static [DependencySpec] { &GRAPH }

pub fn spec_for(id: ComponentId) -> &'static DependencySpec {
    GRAPH.iter().find(|d| d.id == id).expect("ComponentId must be in graph")
}

/// Verb for `ComponentStatus::Installing` keyed by wire id string.
/// CLI doctor uses this for live HealthEvent::Component patches (which
/// carry only `id: String`). The frontend reads `Component.installingVerb`
/// directly off the wire. Both ultimately resolve to the same field on
/// `DependencySpec`.
///
/// Unknown ids (notably the package manager row "homebrew"/"winget", which
/// is not in `dependency_specs`) fall back to "installing".
pub fn installing_verb_for(id: &str) -> &'static str {
    super::ids::parse_component_id(id)
        .map(|cid| spec_for(cid).installing_verb)
        .unwrap_or("installing")
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::COMPONENT_ORDER;

    #[test]
    fn graph_has_exactly_five_specs_in_canonical_order() {
        let g = dependency_specs();
        assert_eq!(g.len(), 5);
        for (i, d) in g.iter().enumerate() {
            assert_eq!(d.id, COMPONENT_ORDER[i], "graph[{i}] must be {:?}", COMPONENT_ORDER[i]);
        }
    }

    #[test]
    fn spec_labels_match_ts_defaults() {
        assert_eq!(spec_for(ComponentId::Postgres).label, "PostgreSQL");
        assert_eq!(spec_for(ComponentId::Ollama).label,   "Ollama");
        assert_eq!(spec_for(ComponentId::Sensei).label,   "Sensei components");
        assert_eq!(spec_for(ComponentId::Sensei).note,    Some("cli · mcp · daemon"));
        assert_eq!(spec_for(ComponentId::Database).label, "Database & schema");
        assert_eq!(spec_for(ComponentId::Database).note,  Some("pgvector · sensei tables"));
        assert_eq!(spec_for(ComponentId::Daemon).label,   "Background daemon");
    }

    #[test]
    fn postgres_ollama_daemon_have_no_note() {
        assert_eq!(spec_for(ComponentId::Postgres).note, None);
        assert_eq!(spec_for(ComponentId::Ollama).note,   None);
        assert_eq!(spec_for(ComponentId::Daemon).note,   None);
    }

    #[test]
    fn database_depends_on_postgres() {
        assert_eq!(spec_for(ComponentId::Database).depends_on, &[ComponentId::Postgres]);
    }

    #[test]
    fn daemon_depends_on_database_and_sensei() {
        assert_eq!(spec_for(ComponentId::Daemon).depends_on,
                   &[ComponentId::Database, ComponentId::Sensei]);
    }

    #[test]
    fn leaf_specs_have_no_dependencies() {
        for id in [ComponentId::Postgres, ComponentId::Ollama, ComponentId::Sensei] {
            assert!(spec_for(id).depends_on.is_empty(), "{:?} should have no deps", id);
        }
    }

    // Per-component verb for `ComponentStatus::Installing`. Service-style
    // deps (postgres/ollama/daemon) are started via brew services, not
    // installed; the database is set up via dbd; only sensei is a plain
    // brew install. The verb is presentation but it's tied 1:1 to the
    // resolver action — keeping it on the spec means CLI + frontend share
    // one source of truth.
    #[test]
    fn service_style_specs_say_starting() {
        assert_eq!(spec_for(ComponentId::Postgres).installing_verb, "starting");
        assert_eq!(spec_for(ComponentId::Ollama).installing_verb,   "starting");
        assert_eq!(spec_for(ComponentId::Daemon).installing_verb,   "starting");
    }

    #[test]
    fn database_spec_says_setting_up() {
        assert_eq!(spec_for(ComponentId::Database).installing_verb, "setting up");
    }

    #[test]
    fn sensei_spec_says_installing() {
        assert_eq!(spec_for(ComponentId::Sensei).installing_verb, "installing");
    }
}
