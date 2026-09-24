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
fn process_fixture_subtree(
    rel: &str,
    subtree: &str,
) -> crate::tasks::processors::FileProcessResult {
    let root = fixtures();
    let subtree_root = root.join(subtree);
    let abs = root.join(rel);
    assert!(abs.exists(), "Fixture not found: {}", abs.display());
    process_file(
        &abs.to_string_lossy(),
        &subtree_root.to_string_lossy(),
        &format!("test:{}", subtree),
    )
    .unwrap()
}

// ═══ Code files (from crate source — these exist in this repo) ═══

#[test]
fn rust_adapter_svelte_rs() {
    let root = workspace_root();
    let abs = root.join("crates/senseid/src/indexer/lang/svelte.rs");
    assert!(abs.exists(), "Source file not found: {}", abs.display());
    let r = process_file(&abs.to_string_lossy(), &root.to_string_lossy(), "sensei").unwrap();
    assert_eq!(r.kind, "file");
    assert_eq!(r.tags, "src");
    assert_eq!(r.language.as_deref(), Some("rust"));
    assert!(!r.symbols.is_empty());
    let names: Vec<&str> = r.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"SvelteAdapter"), "should find SvelteAdapter struct");
}

// ═══ Code fixtures ═══════════════════════════════════════════════// ═══ Doc fixtures ════════════════════════════════════════════════

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

/// **A UTF-16 SOURCE FILE PRODUCES SYMBOLS**, end to end through the router.
///
/// SSMS exports UTF-16LE with a BOM, and `read_to_string` refuses it — so this
/// router returned `Failed to read` for a file the scan gate had already
/// classified as text. MEASURED before the fix: all 754 UTF-16 `.sql` files in
/// the watched roots carried `skip_reason = binary_content`, and none of them
/// reached a parser.
///
/// MUTATION: put `read_to_string` back — the read fails and the file yields
/// nothing, while `classify_unscannable` still says it is indexable. The two
/// disagreeing is the shape that made this invisible.
#[test]
fn a_utf16_file_is_read_rather_than_refused() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("Issues.sql");
    // `CREATE TABLE`, terminated, and unbracketed — because what is under test
    // is the DECODE reaching a parser, not how much of T-SQL v1's line-based
    // `sql.rs` understands. It reads no `CREATE PROCEDURE` and needs the `;`,
    // which is two of the reasons `indexer::lang::sql::tsql` exists.
    //
    // The UTF-8 comparison below is what keeps this honest: if both sides
    // returned nothing the assertion would hold vacuously, so the symbol is
    // asserted by name as well.
    let mut bytes = vec![0xFFu8, 0xFE];
    for c in "CREATE TABLE Issues (Id int);\r\n".chars() {
        bytes.extend_from_slice(&(c as u16).to_le_bytes());
    }
    std::fs::write(&file, &bytes).unwrap();

    let r = process_file(&file.to_string_lossy(), &dir.path().to_string_lossy(), "sensei")
        .expect("a UTF-16 file is text");
    assert_eq!(r.language.as_deref(), Some("sql"));

    // THE SAME CONTENT AS UTF-8, so a difference isolates the decode from what
    // the parser does or does not understand.
    let plain = dir.path().join("Issues8.sql");
    std::fs::write(&plain, "CREATE TABLE Issues (Id int);\r\n").unwrap();
    let p8 = process_file(&plain.to_string_lossy(), &dir.path().to_string_lossy(), "sensei")
        .expect("utf-8 reads");
    assert_eq!(
        r.symbols.iter().map(|s| &s.name).collect::<Vec<_>>(),
        p8.symbols.iter().map(|s| &s.name).collect::<Vec<_>>(),
        "UTF-16 and UTF-8 of the same text must yield the same symbols"
    );
    assert!(
        r.symbols.iter().any(|s| s.name.contains("Issues")),
        "the table is found, so the decoded text reached the parser: {:?}",
        r.symbols.iter().map(|s| &s.name).collect::<Vec<_>>()
    );
}

/// A genuine binary is still refused, BOM check or not.
#[test]
fn a_pg_dump_archive_is_still_not_text() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("dump.sql");
    // The real head of a `pg_dump -Fc` archive, which is what one of the
    // skipped files in the corpus actually is.
    std::fs::write(&file, b"PGDMP\x01\x10\x00\x04\x08\x01\x01\x00\x10").unwrap();
    assert!(
        process_file(&file.to_string_lossy(), &dir.path().to_string_lossy(), "sensei").is_err(),
        "null bytes with no BOM are unexplained"
    );
}

/// **A C++ FILE GETS A FILE NODE AND NO SYMBOLS.**
///
/// `c_parser_large_file` stood here and routed a `.c` file through this
/// router, which now reaches a `DetectionOnly` whose `parse` is
/// `unreachable!()` — C is produced by `crate::indexer::lang::c` and
/// `process_file` routes it there first.
///
/// What replaces it is the case this router DOES still own: `.cpp` is claimed
/// by no adapter in either registry, so `code::process` returns `None` at its
/// `adapter_for_ext(..)?` and the file lands on the "unknown file type" branch.
/// That is the skip that makes deleting v1's C parser safe, and it is worth a
/// test because the alternative — a `DetectionOnly` entry for `.cpp` — is a
/// PANIC rather than a skip.
#[test]
fn a_cpp_file_is_a_file_node_with_no_symbols() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("widget.cpp");
    std::fs::write(&file, "namespace app {\nclass Widget { public: int wide(); };\n}\n").unwrap();
    let r = process_file(&file.to_string_lossy(), &dir.path().to_string_lossy(), "sensei").unwrap();
    assert_eq!(r.kind, "file");
    assert!(r.symbols.is_empty(), "nothing here parses C++, so it declares nothing");
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

// The Svelte processor tests STOOD HERE — `svelte_component`,
// `svelte_page_route`, `svelte_ts_appstate`, `page_svelte_route`,
// `svelte_component_step_header`. They exercised v1's Svelte parser, which was
// deleted when TypeScript cut over; `.svelte` is read by
// `crate::indexer::lang::svelte` now, whose own tests cover the script block,
// the dialect, markup interpolations and block tags in more detail than these
// did.
