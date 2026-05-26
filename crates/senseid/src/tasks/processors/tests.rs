//! Tests for file processors using bundled fixture files.
//!
//! Fixtures live in tests/fixtures/ — self-contained, no references to sibling repos.

use crate::tasks::processors::process_file;
use std::path::PathBuf;

/// Root of the senseid crate (CARGO_MANIFEST_DIR).
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Daemon workspace root (one level up from crate root).
fn workspace_root() -> PathBuf {
    crate_root().parent().unwrap().parent().unwrap().to_path_buf()
}

/// Fixtures directory inside the crate.
fn fixtures() -> PathBuf {
    crate_root().join("tests/fixtures")
}

/// Process a fixture file relative to the fixtures directory.
fn process_fixture(rel: &str) -> crate::tasks::processors::FileProcessResult {
    let root = fixtures();
    let abs = root.join(rel);
    assert!(abs.exists(), "Fixture not found: {}", abs.display());
    process_file(&abs.to_string_lossy(), &root.to_string_lossy(), "test").unwrap()
}

/// Process a fixture file with a subtree root for extension detection.
fn process_fixture_subtree(rel: &str, subtree: &str) -> crate::tasks::processors::FileProcessResult {
    let root = fixtures();
    let subtree_root = root.join(subtree);
    let abs = root.join(rel);
    assert!(abs.exists(), "Fixture not found: {}", abs.display());
    process_file(&abs.to_string_lossy(), &subtree_root.to_string_lossy(), &format!("test:{}", subtree)).unwrap()
}

// ═══ Code files (from crate source — these exist in this repo) ═══

#[test]
fn rust_adapter_svelte_rs() {
    let root = workspace_root();
    let abs = root.join("crates/senseid/src/languages/svelte.rs");
    assert!(abs.exists(), "Source file not found: {}", abs.display());
    let r = process_file(&abs.to_string_lossy(), &root.to_string_lossy(), "sensei").unwrap();
    assert_eq!(r.kind, "file");
    assert_eq!(r.tags, "src");
    assert_eq!(r.language.as_deref(), Some("rust"));
    assert!(!r.symbols.is_empty());
    let names: Vec<&str> = r.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"SvelteAdapter"), "should find SvelteAdapter struct");
}

// ═══ Code fixtures ═══════════════════════════════════════════════

#[test]
fn svelte_component() {
    let r = process_fixture("code/StepHeader.svelte");
    assert_eq!(r.kind, "file");
    assert_eq!(r.tags, "src");
    assert_eq!(r.language.as_deref(), Some("svelte"));
    let kinds: Vec<&str> = r.symbols.iter().map(|s| s.kind.as_str()).collect();
    assert!(kinds.contains(&"component"), "should find component");
}

#[test]
fn svelte_ts_appstate() {
    let r = process_fixture("code/appstate.svelte.ts");
    assert_eq!(r.kind, "file");
    assert_eq!(r.tags, "src");
    assert!(!r.symbols.is_empty(), "should extract symbols from .svelte.ts");
}

#[test]
fn svelte_page_route() {
    let r = process_fixture("code/+page.svelte");
    assert_eq!(r.kind, "file");
    assert_eq!(r.tags, "src");
}

// ═══ Doc fixtures ════════════════════════════════════════════════

#[test]
fn design_doc() {
    let r = process_fixture("docs/architecture.md");
    assert_eq!(r.kind, "doc");
    assert_eq!(r.tags, "doc");
    assert_eq!(r.doc_type.as_deref(), Some("design"));
    assert!(r.title.is_some());
}

#[test]
fn feature_doc() {
    let r = process_fixture("docs/workflow-commands.md");
    assert_eq!(r.kind, "doc");
    assert_eq!(r.doc_type.as_deref(), Some("feature"));
}

#[test]
fn idea_doc() {
    let r = process_fixture("docs/idea-workflow.md");
    assert_eq!(r.kind, "doc");
    assert_eq!(r.tags, "doc");
}

#[test]
fn gap_analysis_doc() {
    let r = process_fixture("docs/gap-analysis.md");
    assert_eq!(r.kind, "doc");
    assert_eq!(r.tags, "doc");
}

#[test]
fn root_readme() {
    let root = workspace_root();
    let abs = root.join("README.md");
    assert!(abs.exists(), "README.md not found at workspace root");
    let r = process_file(&abs.to_string_lossy(), &root.to_string_lossy(), "sensei").unwrap();
    assert_eq!(r.kind, "doc");
    assert_eq!(r.doc_type.as_deref(), Some("usage"));
}

#[test]
fn homebrew_readme() {
    let r = process_fixture("docs/homebrew-readme.md");
    assert_eq!(r.kind, "doc");
    assert_eq!(r.doc_type.as_deref(), Some("usage"));
}

// ═══ Extension fixtures ══════════════════════════════════════════

#[test]
fn marketplace_skill_md() {
    let r = process_fixture_subtree("extensions/analyze-skill.md", "extensions");
    assert_eq!(r.kind, "extension");
    assert_eq!(r.tags, "doc");
    assert_eq!(r.doc_type.as_deref(), Some("skill"));
}

// ═══ Config fixtures ═════════════════════════════════════════════

#[test]
fn desktop_package_json() {
    let r = process_fixture("config/package.json");
    assert_eq!(r.kind, "file");
    assert_eq!(r.tags, "config");
}

#[test]
fn sensei_mcp_cargo_toml() {
    let root = workspace_root();
    let abs = root.join("crates/mcp/Cargo.toml");
    assert!(abs.exists(), "sensei-mcp Cargo.toml not found");
    let r = process_file(&abs.to_string_lossy(), &root.to_string_lossy(), "sensei").unwrap();
    assert_eq!(r.kind, "file");
    assert_eq!(r.tags, "config");
}

// ═══ Non-code files (conditional — may not exist) ════════════════

#[test]
fn c_parser_large_file() {
    let root = workspace_root();
    let abs = root.join("crates/senseid/grammars/kotlin/src/parser.c");
    if abs.exists() {
        let r = process_file(&abs.to_string_lossy(), &root.to_string_lossy(), "sensei").unwrap();
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "src");
        assert!(r.symbols.is_empty(), "large generated C file should have no symbols");
    }
}

// ═══ Tag classification ═══════════════════════════════════════════

#[test]
fn test_file_detected() {
    use crate::tasks::processors::types::classify_file_tag;
    assert_eq!(classify_file_tag("src/main.spec.ts", "ts"), "test");
    assert_eq!(classify_file_tag("src/main.test.ts", "ts"), "test");
    assert_eq!(classify_file_tag("tests/unit/foo.rs", "rs"), "test");
    assert_eq!(classify_file_tag("src/__tests__/foo.ts", "ts"), "test");
    assert_eq!(classify_file_tag("test_helper.py", "py"), "test");
}

#[test]
fn e2e_file_detected() {
    use crate::tasks::processors::types::classify_file_tag;
    assert_eq!(classify_file_tag("e2e/login.spec.ts", "ts"), "e2e");
    assert_eq!(classify_file_tag("src/app.e2e.ts", "ts"), "e2e");
}

#[test]
fn config_file_detected() {
    use crate::tasks::processors::types::classify_file_tag;
    assert_eq!(classify_file_tag("package.json", "json"), "config");
    assert_eq!(classify_file_tag("Cargo.toml", "toml"), "config");
    assert_eq!(classify_file_tag("config.yaml", "yaml"), "config");
}

#[test]
fn src_file_default() {
    use crate::tasks::processors::types::classify_file_tag;
    assert_eq!(classify_file_tag("src/main.rs", "rs"), "src");
    assert_eq!(classify_file_tag("lib/utils.ts", "ts"), "src");
}
