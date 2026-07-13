//! Verifies the folded governance scope in the single `database/design.yaml`:
//! - `dojo` selects BOTH the governance-federation tables (shared_rules/members/
//!   api_keys/audit_log) AND the SaaS-layer artifact
//!   tables, plus the shared governance closure (namespaces/scopes/enforcement),
//!   and carries NO extensions (its embedded Postgres lacks pgvector);
//! - `default` (what the daemon applies) excludes the `dojo` service schema but
//!   keeps the daemon's own tables (including the `sensei.dojo_*` mirror tables)
//!   and the `vector` extension.

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
fn dojo_scope_selects_governance_and_artifact_tables_plus_closure_and_no_extensions() {
    let names = scoped_names(Some("dojo"));
    // Governance-federation tables.
    for n in ["dojo.shared_rules", "dojo.members", "dojo.api_keys", "dojo.audit_log"] {
        assert!(names.contains(n), "expected {n} in dojo scope; got {names:?}");
    }
    // SaaS-layer artifact tables (a representative sample).
    for n in ["dojo.artifacts", "dojo.tenants", "dojo.memberships", "dojo.identities"] {
        assert!(names.contains(n), "expected {n} in dojo scope; got {names:?}");
    }
    // Shared governance closure: deps:include pulls namespaces → scopes; the
    // enforcement enum is a column *type* included explicitly.
    for n in ["sensei.namespaces", "sensei.scopes", "sensei.enforcement"] {
        assert!(names.contains(n), "expected {n} (closure) in dojo scope; got {names:?}");
    }
    // Daemon-only tables must NOT leak into the service scope.
    for n in ["sensei.memories", "sensei.nodes", "sensei.folder_namespaces"] {
        assert!(!names.contains(n), "{n} must NOT be in the dojo scope");
    }
    // extensions: [] → the dojo scope carries no extensions (embedded PG has no pgvector).
    for ext in ["vector", "extensions.vector", "uuid-ossp", "extensions.uuid-ossp"] {
        assert!(
            !names.contains(ext),
            "dojo scope must exclude extension {ext}; got {names:?}"
        );
    }
}

#[test]
fn default_scope_excludes_dojo_but_keeps_daemon_tables_and_vector() {
    // resolve_scope(None) returns the `default` scope when one is defined.
    let names = scoped_names(None);
    assert!(
        !names.iter().any(|n| n.starts_with("dojo.")),
        "daemon default scope must exclude the dojo service schema; got {names:?}"
    );
    assert!(names.contains("sensei.memories"), "daemon keeps its own tables");
    // The daemon-local mirror tables live in the `sensei` schema, not `dojo`, so
    // excluding the `dojo` schema keeps them in the daemon's scope.
    assert!(
        names.contains("sensei.dojo_memberships"),
        "daemon keeps its sensei.dojo_* mirror tables; got {names:?}"
    );
    // No extensions allowlist on `default` → all extensions apply (vector for embeddings).
    assert!(
        names.contains("vector") || names.contains("extensions.vector"),
        "daemon default scope must keep the vector extension; got {names:?}"
    );
}
