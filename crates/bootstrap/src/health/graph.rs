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
}

const GRAPH: [DependencySpec; 5] = [
    DependencySpec { id: ComponentId::Postgres, label: "PostgreSQL",        note: None,
                     depends_on: &[] },
    DependencySpec { id: ComponentId::Ollama,   label: "Ollama",            note: None,
                     depends_on: &[] },
    DependencySpec { id: ComponentId::Sensei,   label: "Sensei components", note: Some("cli · mcp · daemon"),
                     depends_on: &[] },
    DependencySpec { id: ComponentId::Database, label: "Database & schema", note: Some("pgvector · sensei tables"),
                     depends_on: &[ComponentId::Postgres] },
    DependencySpec { id: ComponentId::Daemon,   label: "Background daemon", note: None,
                     depends_on: &[ComponentId::Database, ComponentId::Sensei] },
];

pub fn dependency_specs() -> &'static [DependencySpec] { &GRAPH }

pub fn spec_for(id: ComponentId) -> &'static DependencySpec {
    GRAPH.iter().find(|d| d.id == id).expect("ComponentId must be in graph")
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
}
