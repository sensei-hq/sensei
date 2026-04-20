//! Pattern detection module — identifies design patterns, naming conventions,
//! file structures, and directory patterns from the graph.
//!
//! Each detector is a pure function: graph connection + project → Vec<DetectedPattern>.
//! Detectors can be composed and run independently (suitable for task workers).

// All items below are test-only until wired into the pipeline (issue #90).
#[cfg(test)]
use rusqlite::{Connection, params};
#[cfg(test)]
use serde::{Deserialize, Serialize};

/// A detected pattern with its instances and metadata.
#[cfg(test)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    pub name: String,
    pub pattern_type: String,
    pub category: PatternCategory,
    pub instance_count: usize,
    pub instances: Vec<PatternInstance>,
    pub description: String,
}

#[cfg(test)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternInstance {
    pub name: String,
    pub file: Option<String>,
    pub kind: String,
}

#[cfg(test)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternCategory {
    DesignPattern,    // adapter, factory, observer, etc.
    NamingConvention, // get_*, parse_*, handle_*, etc.
    FileStructure,    // mod.rs + types.rs + handlers.rs
    DirectoryPattern, // adapters/, handlers/, workers/
}

/// Minimum instances required to consider something a pattern.
#[cfg(test)]
const MIN_INSTANCES: usize = 2;

// ── Design Pattern Detector (type/class level) ──────────────────────────────

#[cfg(test)]
const DESIGN_SUFFIXES: &[(&str, &str, &str)] = &[
    ("Adapter", "adapter", "Wraps external interface behind a common API"),
    ("Factory", "factory", "Creates instances without exposing construction logic"),
    ("Observer", "observer", "Notifies dependents of state changes"),
    ("Builder", "builder", "Constructs complex objects step by step"),
    ("Strategy", "strategy", "Interchangeable algorithms behind a common interface"),
    ("Handler", "handler", "Processes requests or events"),
    ("Middleware", "middleware", "Intercepts and transforms in a pipeline"),
    ("Provider", "provider", "Supplies dependencies or context"),
    ("Decorator", "decorator", "Wraps to add behavior without modifying"),
    ("Worker", "worker", "Processes tasks from a queue"),
    ("Hook", "hook", "Extension point for customization"),
    ("Plugin", "plugin", "Loadable extension module"),
    ("Controller", "controller", "Coordinates request/response flow"),
    ("Service", "service", "Encapsulates business logic"),
    ("Repository", "repository", "Abstracts data access"),
    ("Validator", "validator", "Checks correctness of input"),
    ("Resolver", "resolver", "Resolves references or dependencies"),
    ("Transformer", "transformer", "Converts data between formats"),
    ("Parser", "parser", "Extracts structured data from text"),
    ("Serializer", "serializer", "Converts objects to/from wire format"),
];

#[cfg(test)]
pub fn detect_design_patterns(graph: &Connection, project: &str) -> Vec<DetectedPattern> {
    let mut results = Vec::new();

    for (suffix, pattern_type, description) in DESIGN_SUFFIXES {
        let like_pattern = format!("%{}", suffix);
        let instances = query_instances(
            graph, project, &like_pattern,
            &["class", "struct", "interface", "type", "component", "enum"],
        );

        if instances.len() >= MIN_INSTANCES {
            results.push(DetectedPattern {
                name: format!("{}-pattern", pattern_type),
                pattern_type: pattern_type.to_string(),
                category: PatternCategory::DesignPattern,
                instance_count: instances.len(),
                instances,
                description: description.to_string(),
            });
        }
    }

    results
}

// ── Naming Convention Detector (function level) ─────────────────────────────

#[cfg(test)]
const NAMING_PREFIXES: &[(&str, &str)] = &[
    ("get_", "Getter functions — read data without side effects"),
    ("set_", "Setter functions — mutate state"),
    ("is_", "Boolean predicate functions"),
    ("has_", "Boolean predicate functions (ownership/containment)"),
    ("create_", "Constructor functions — create new instances"),
    ("update_", "Mutation functions — modify existing data"),
    ("delete_", "Destruction functions — remove data"),
    ("handle_", "Event/request handler functions"),
    ("parse_", "Parser functions — extract structured data from input"),
    ("validate_", "Validation functions — check correctness"),
    ("render_", "Rendering functions — produce output representation"),
    ("test_", "Test functions"),
    ("on_", "Event callback functions"),
    ("to_", "Conversion functions"),
    ("from_", "Constructor-from functions"),
];

#[cfg(test)]
pub fn detect_naming_conventions(graph: &Connection, project: &str) -> Vec<DetectedPattern> {
    let mut results = Vec::new();

    for (prefix, description) in NAMING_PREFIXES {
        let like_pattern = format!("{}%", prefix);
        let mut stmt = match graph.prepare(
            "SELECT name, file, kind FROM hierarchy_nodes WHERE project=?1 AND name LIKE ?2 AND kind IN ('function','method')"
        ) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let instances: Vec<PatternInstance> = stmt.query_map(
            params![project, like_pattern],
            |row| Ok(PatternInstance {
                name: row.get(0)?,
                file: row.get(1)?,
                kind: row.get(2)?,
            })
        ).ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
        .unwrap_or_default();

        if instances.len() >= 3 { // Higher threshold for naming — 3+ to be meaningful
            results.push(DetectedPattern {
                name: format!("{}* convention", prefix),
                pattern_type: format!("naming:{}", prefix.trim_end_matches('_')),
                category: PatternCategory::NamingConvention,
                instance_count: instances.len(),
                instances,
                description: description.to_string(),
            });
        }
    }

    results
}

// ── Directory Pattern Detector ──────────────────────────────────────────────

#[cfg(test)]
const DIRECTORY_PATTERNS: &[(&str, &str)] = &[
    ("adapter", "Adapter modules — wraps external interfaces"),
    ("handler", "Handler modules — processes requests/events"),
    ("worker", "Worker modules — processes background tasks"),
    ("middleware", "Middleware modules — pipeline interceptors"),
    ("hook", "Hook modules — extension points"),
    ("plugin", "Plugin modules — loadable extensions"),
    ("test", "Test modules"),
    ("util", "Utility modules — shared helpers"),
    ("service", "Service modules — business logic"),
    ("model", "Model modules — data structures"),
    ("api", "API modules — HTTP/RPC endpoints"),
    ("config", "Configuration modules"),
    ("db", "Database modules — data access"),
];

#[cfg(test)]
pub fn detect_directory_patterns(graph: &Connection, project: &str) -> Vec<DetectedPattern> {
    let mut results = Vec::new();

    // Get all file paths for this project
    let mut stmt = match graph.prepare(
        "SELECT DISTINCT file FROM hierarchy_nodes WHERE project=?1 AND file IS NOT NULL"
    ) {
        Ok(s) => s,
        Err(_) => return results,
    };

    let files: Vec<String> = stmt.query_map(params![project], |row| row.get(0))
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
        .unwrap_or_default();

    for (dir_pattern, description) in DIRECTORY_PATTERNS {
        let matching: Vec<PatternInstance> = files.iter()
            .filter(|f| {
                let lower = f.to_lowercase();
                // Match directory names like adapters/, adapter/, or files in those dirs
                lower.contains(&format!("/{}/", dir_pattern))
                    || lower.contains(&format!("/{}s/", dir_pattern))
                    || lower.contains(&format!("/{}_", dir_pattern))
            })
            .map(|f| PatternInstance {
                name: f.split('/').last().unwrap_or("").to_string(),
                file: Some(f.clone()),
                kind: "file".to_string(),
            })
            .collect();

        if matching.len() >= MIN_INSTANCES {
            results.push(DetectedPattern {
                name: format!("{}-directory", dir_pattern),
                pattern_type: format!("directory:{}", dir_pattern),
                category: PatternCategory::DirectoryPattern,
                instance_count: matching.len(),
                instances: matching,
                description: description.to_string(),
            });
        }
    }

    results
}

// ── Composite Detector ──────────────────────────────────────────────────────

/// Run all detectors and return combined results.
#[cfg(test)]
pub fn detect_all_patterns(graph: &Connection, project: &str) -> Vec<DetectedPattern> {
    let mut all = Vec::new();
    all.extend(detect_design_patterns(graph, project));
    all.extend(detect_naming_conventions(graph, project));
    all.extend(detect_directory_patterns(graph, project));
    all
}

// ── Helper ──────────────────────────────────────────────────────────────────

#[cfg(test)]
fn query_instances(graph: &Connection, project: &str, like_pattern: &str, kinds: &[&str]) -> Vec<PatternInstance> {
    let kinds_sql = kinds.iter().map(|k| format!("'{}'", k)).collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT name, file, kind FROM hierarchy_nodes WHERE project=?1 AND name LIKE ?2 AND kind IN ({})",
        kinds_sql
    );
    let mut stmt = match graph.prepare(&sql) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    stmt.query_map(params![project, like_pattern], |row| {
        Ok(PatternInstance {
            name: row.get(0)?,
            file: row.get(1)?,
            kind: row.get(2)?,
        })
    }).ok()
    .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_graph() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("
            CREATE TABLE hierarchy_nodes(
                id TEXT PRIMARY KEY, name TEXT, kind TEXT, level TEXT,
                parent_id TEXT, file TEXT, line INTEGER, project TEXT,
                sig TEXT, body TEXT, docstring TEXT, complexity INTEGER,
                tags TEXT, doc_type TEXT, doc_category TEXT
            );
        ").unwrap();
        conn
    }

    fn insert_node(conn: &Connection, name: &str, kind: &str, file: &str, project: &str) {
        conn.execute(
            "INSERT INTO hierarchy_nodes(id, name, kind, file, project) VALUES(?1,?2,?3,?4,?5)",
            params![format!("{}:{}:{}", kind, file, name), name, kind, file, project],
        ).unwrap();
    }

    #[test]
    fn detects_adapter_pattern() {
        let graph = test_graph();
        insert_node(&graph, "TypeScriptAdapter", "class", "ts.rs", "test");
        insert_node(&graph, "PythonAdapter", "class", "py.rs", "test");
        insert_node(&graph, "RustAdapter", "class", "rust.rs", "test");

        let patterns = detect_design_patterns(&graph, "test");
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, "adapter");
        assert_eq!(patterns[0].instance_count, 3);
    }

    #[test]
    fn design_pattern_requires_two_instances() {
        let graph = test_graph();
        insert_node(&graph, "SqlAdapter", "class", "sql.rs", "test");

        let patterns = detect_design_patterns(&graph, "test");
        assert!(patterns.is_empty());
    }

    #[test]
    fn detects_multiple_design_patterns() {
        let graph = test_graph();
        insert_node(&graph, "FooHandler", "class", "foo.rs", "test");
        insert_node(&graph, "BarHandler", "class", "bar.rs", "test");
        insert_node(&graph, "FooWorker", "class", "foo_w.rs", "test");
        insert_node(&graph, "BarWorker", "class", "bar_w.rs", "test");

        let patterns = detect_design_patterns(&graph, "test");
        assert_eq!(patterns.len(), 2);
        let types: Vec<&str> = patterns.iter().map(|p| p.pattern_type.as_str()).collect();
        assert!(types.contains(&"handler"));
        assert!(types.contains(&"worker"));
    }

    #[test]
    fn detects_naming_conventions() {
        let graph = test_graph();
        for name in ["get_user", "get_project", "get_session", "get_config"] {
            insert_node(&graph, name, "function", "api.rs", "test");
        }
        for name in ["parse_file", "parse_config", "parse_args"] {
            insert_node(&graph, name, "function", "parser.rs", "test");
        }
        insert_node(&graph, "helper", "function", "utils.rs", "test"); // no prefix pattern

        let conventions = detect_naming_conventions(&graph, "test");
        assert!(conventions.len() >= 2); // get_ and parse_
        let types: Vec<&str> = conventions.iter().map(|p| p.pattern_type.as_str()).collect();
        assert!(types.contains(&"naming:get"));
        assert!(types.contains(&"naming:parse"));
    }

    #[test]
    fn naming_convention_requires_three() {
        let graph = test_graph();
        insert_node(&graph, "get_foo", "function", "a.rs", "test");
        insert_node(&graph, "get_bar", "function", "b.rs", "test");
        // Only 2 — below threshold

        let conventions = detect_naming_conventions(&graph, "test");
        assert!(conventions.iter().all(|c| c.pattern_type != "naming:get"));
    }

    #[test]
    fn detects_directory_patterns() {
        let graph = test_graph();
        insert_node(&graph, "ts_adapter", "function", "src/adapters/ts.rs", "test");
        insert_node(&graph, "py_adapter", "function", "src/adapters/py.rs", "test");
        insert_node(&graph, "sql_adapter", "function", "src/adapters/sql.rs", "test");
        insert_node(&graph, "main", "function", "src/main.rs", "test");

        let dirs = detect_directory_patterns(&graph, "test");
        assert!(dirs.iter().any(|d| d.pattern_type == "directory:adapter"));
    }

    #[test]
    fn detect_all_combines_results() {
        let graph = test_graph();
        // Design pattern
        insert_node(&graph, "FooAdapter", "class", "src/adapters/foo.rs", "test");
        insert_node(&graph, "BarAdapter", "class", "src/adapters/bar.rs", "test");
        // Naming convention
        for name in ["get_a", "get_b", "get_c"] {
            insert_node(&graph, name, "function", "src/api.rs", "test");
        }

        let all = detect_all_patterns(&graph, "test");
        assert!(all.len() >= 2); // at least adapter pattern + get_ naming + maybe directory
        let categories: Vec<&PatternCategory> = all.iter().map(|p| &p.category).collect();
        assert!(categories.iter().any(|c| matches!(c, PatternCategory::DesignPattern)));
        assert!(categories.iter().any(|c| matches!(c, PatternCategory::NamingConvention)));
    }

    #[test]
    fn isolates_projects() {
        let graph = test_graph();
        insert_node(&graph, "FooAdapter", "class", "a.rs", "proj-a");
        insert_node(&graph, "BarAdapter", "class", "b.rs", "proj-a");
        insert_node(&graph, "BazAdapter", "class", "c.rs", "proj-b");

        let a = detect_design_patterns(&graph, "proj-a");
        let b = detect_design_patterns(&graph, "proj-b");
        assert_eq!(a.len(), 1); // proj-a has 2 adapters
        assert!(b.is_empty()); // proj-b has only 1
    }
}
