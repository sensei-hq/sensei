//! Verifies the `hive` scope in design.hive.yaml resolves to exactly the
//! intended entity set, and that the daemon's design.yaml skips hive.

use std::path::PathBuf;

fn database_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database")
}

#[test]
fn hive_scope_resolves_to_expected_entities() {
    let dir = database_dir();
    let cfg = dir.join("design.hive.yaml");
    let design = dbd_core::Design::from_config_with_dir(&cfg, "prod", Some(&dir))
        .expect("load design.hive.yaml");
    let scope = design.resolve_scope(Some("hive"), None).expect("resolve hive scope");
    let entities = design.scoped_entities(&scope).expect("scoped entities");
    let names: std::collections::HashSet<String> =
        entities.iter().map(|e| e.name.clone()).collect();

    for n in ["hive.shared_rules", "hive.members", "hive.api_keys", "hive.audit_log"] {
        assert!(names.contains(n), "expected {n} in hive scope; got {names:?}");
    }
    for n in ["sensei.namespaces", "sensei.scopes", "sensei.enforcement"] {
        assert!(names.contains(n), "expected {n} (closure) in hive scope; got {names:?}");
    }
    for n in ["sensei.memories", "sensei.nodes", "sensei.folder_namespaces"] {
        assert!(!names.contains(n), "{n} must NOT be in the hive scope");
    }
}

// ADDITIONAL TEST (required): prove the skip_schemas change keeps hive.* out of the daemon design.
#[test]
fn daemon_design_skips_hive_schema() {
    let dir = database_dir();
    let cfg = dir.join("design.yaml");
    let design = dbd_core::Design::from_config_with_dir(&cfg, "prod", Some(&dir))
        .expect("load design.yaml");
    // The daemon applies the full set (scope=all). No hive.* entity should be present.
    let scope = design.resolve_scope(None, None).expect("resolve default/all scope");
    let entities = design.scoped_entities(&scope).expect("scoped entities");
    let has_hive = entities.iter().any(|e| e.name.starts_with("hive."));
    assert!(!has_hive, "daemon design.yaml must skip the hive schema; found hive.* entities");
    // sanity: daemon DOES still have its own tables
    assert!(entities.iter().any(|e| e.name == "sensei.memories"));
}
