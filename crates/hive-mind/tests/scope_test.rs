//! Verifies the two governance scopes in the single `database/design.yaml`:
//! - `hive` selects the four hive tables + the shared governance closure
//!   (namespaces/scopes/enforcement) and carries NO extensions (its embedded
//!   Postgres lacks pgvector);
//! - `default` (what the daemon applies) excludes the hive schema but keeps the
//!   daemon's own tables and the `vector` extension.

use std::collections::HashSet;
use std::path::PathBuf;

fn database_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database")
}

fn scoped_names(scope_name: Option<&str>) -> HashSet<String> {
    let dir = database_dir();
    let cfg = dir.join("design.yaml");
    let design = dbd_core::Design::from_config_with_dir(&cfg, "prod", Some(&dir))
        .expect("load design.yaml");
    let scope = design.resolve_scope(scope_name, None).expect("resolve scope");
    design
        .scoped_entities(&scope)
        .expect("scoped entities")
        .iter()
        .map(|e| e.name.clone())
        .collect()
}

#[test]
fn hive_scope_selects_hive_plus_closure_and_no_extensions() {
    let names = scoped_names(Some("hive"));
    for n in ["hive.shared_rules", "hive.members", "hive.api_keys", "hive.audit_log"] {
        assert!(names.contains(n), "expected {n} in hive scope; got {names:?}");
    }
    for n in ["sensei.namespaces", "sensei.scopes", "sensei.enforcement"] {
        assert!(names.contains(n), "expected {n} (closure) in hive scope; got {names:?}");
    }
    for n in ["sensei.memories", "sensei.nodes", "sensei.folder_namespaces"] {
        assert!(!names.contains(n), "{n} must NOT be in the hive scope");
    }
    // extensions: [] → the hive scope carries no extensions (embedded PG has no pgvector).
    for ext in ["vector", "extensions.vector", "uuid-ossp", "extensions.uuid-ossp"] {
        assert!(
            !names.contains(ext),
            "hive scope must exclude extension {ext}; got {names:?}"
        );
    }
}

#[test]
fn default_scope_excludes_hive_but_keeps_daemon_tables_and_vector() {
    // resolve_scope(None) returns the `default` scope when one is defined.
    let names = scoped_names(None);
    assert!(
        !names.iter().any(|n| n.starts_with("hive.")),
        "daemon default scope must exclude the hive schema; got {names:?}"
    );
    assert!(names.contains("sensei.memories"), "daemon keeps its own tables");
    // No extensions allowlist on `default` → all extensions apply (vector for embeddings).
    assert!(
        names.contains("vector") || names.contains("extensions.vector"),
        "daemon default scope must keep the vector extension; got {names:?}"
    );
}
