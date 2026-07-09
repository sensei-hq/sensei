//! Verifies the `dojo` scope in the single `database/design.yaml` selects the
//! self-contained `dojo.*` closure (tables + the seed procedure) and carries NO
//! extensions (its embedded Postgres, shared with `hive`, lacks pgvector). This
//! is the artifact-federation analogue of `scope_test`'s `hive` assertions.

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
fn dojo_scope_selects_dojo_closure_and_no_extensions() {
    let names = scoped_names(Some("dojo"));
    for n in [
        "dojo.tenants",
        "dojo.memberships",
        "dojo.identities",
        "dojo.artifacts",
    ] {
        assert!(names.contains(n), "expected {n} in dojo scope; got {names:?}");
    }
    // Self-contained: the dojo scope must NOT pull in hive or daemon tables.
    for n in ["hive.shared_rules", "sensei.memories", "sensei.namespaces"] {
        assert!(!names.contains(n), "{n} must NOT be in the dojo scope; got {names:?}");
    }
    // extensions: [] → no pgvector / uuid-ossp in the dojo scope.
    for ext in ["vector", "extensions.vector", "uuid-ossp", "extensions.uuid-ossp"] {
        assert!(
            !names.contains(ext),
            "dojo scope must exclude extension {ext}; got {names:?}"
        );
    }
}

#[test]
fn default_scope_excludes_dojo() {
    // The daemon's `default` scope must not carry the dojo schema.
    let names = scoped_names(None);
    assert!(
        !names.iter().any(|n| n.starts_with("dojo.")),
        "daemon default scope must exclude the dojo schema; got {names:?}"
    );
}
