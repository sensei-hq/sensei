//! Parser invariants checked against this repository's own source.
//!
//! The per-language unit tests pin behaviour on hand-written snippets. Those
//! snippets are written by whoever wrote the parser, so they encode the same
//! blind spots — the `"::{"`-then-`','` import splitter passed every one of its
//! tests for as long as it existed, because none of them contained a nested or
//! multi-line group. This module instead runs the parsers over every source file
//! in the working tree and asserts properties that must hold for all of them.
//!
//! The properties are deliberately negative: "no produced name may contain a
//! brace", "no module path may retain a navigation segment". A parser that
//! silently mangles an import satisfies every positive assertion — it still
//! emits *a* name — and fails these.
//!
//! ## Why these do not assert a resolution rate
//!
//! Fixing import handling makes the resolved fraction FALL: an edge that used to
//! point confidently at a fabricated node becomes an honest unresolved edge.
//! Rate is the wrong dial. These assert the shape of what IS produced, which
//! only improves.

use super::{adapter_for_filename, language_for_ext_slug};
use std::path::{Path, PathBuf};

/// Directories that hold build output, dependencies, or vendored source.
const SKIP: &[&str] = &["target", "node_modules", ".svelte-kit", "build", "dist", "examples"];

fn repo_root() -> PathBuf {
    // crates/senseid → crates → repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().to_path_buf()
}

/// Every file in the working tree with one of `exts`.
fn corpus(exts: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![repo_root()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if !SKIP.contains(&name.as_str()) && !name.starts_with('.') {
                    stack.push(path);
                }
            } else if path.extension().and_then(|e| e.to_str()).is_some_and(|e| exts.contains(&e)) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn read(path: &Path) -> Option<(Box<dyn super::LanguageAdapter>, String)> {
    let name = path.file_name()?.to_str()?;
    let adapter = adapter_for_filename(name)?;
    let content = std::fs::read_to_string(path).ok()?;
    Some((adapter, content))
}

/// Every import name is a name a later reference gets looked up under. A brace or
/// a comma in one means the group reader emitted a fragment of syntax instead of
/// a binding, and no lookup can ever match it again.
///
/// Before the grammar-driven reader, this repo's 95 multi-line and 28 nested
/// group `use` declarations each produced names like `extract::{Path` and
/// `State}`.
#[test]
fn no_import_name_contains_syntax() {
    let mut bad = Vec::new();
    let mut files = 0usize;
    for path in corpus(&["rs", "ts", "js", "svelte", "py", "java"]) {
        let Some((adapter, content)) = read(&path) else { continue };
        files += 1;
        let pf = adapter.parse(&content, &path.to_string_lossy());
        for imp in &pf.imports {
            for name in &imp.names {
                if name.contains(['{', '}', ',', ';', '\n']) {
                    bad.push(format!("{}: name {name:?}", path.display()));
                }
            }
            if imp.target_path.contains(['{', '}', ';', '\n']) {
                bad.push(format!("{}: target_path {:?}", path.display(), imp.target_path));
            }
        }
    }
    assert!(files > 200, "corpus too small to be meaningful: {files} files");
    assert!(
        bad.is_empty(),
        "{} mangled import names across {files} files:\n{}",
        bad.len(),
        bad.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
    );
}

/// An import path is a path, not the declaration's prose.
/// `trim_start_matches("use ")` never matched `pub use`, so all 60 re-exports in
/// this repo carried the keywords into the path they were supposed to name; the
/// same reader left `as` aliases as `Error as IoError`.
#[test]
fn no_rust_import_path_carries_keywords() {
    let mut bad = Vec::new();
    for path in corpus(&["rs"]) {
        let Some((adapter, content)) = read(&path) else { continue };
        let pf = adapter.parse(&content, &path.to_string_lossy());
        for imp in &pf.imports {
            let p = &imp.target_path;
            if p.starts_with("pub ") || p.starts_with("use ") || p.contains(" as ") {
                bad.push(format!("{}: {p:?}", path.display()));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} import paths carrying keywords:\n{}",
        bad.len(),
        bad.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
    );
}

/// `super`, `self` and `crate` are navigation instructions, not module names. An
/// FQN retaining one names a module that does not exist — the shape of the
/// highest-degree stub in the live graph,
/// `rust·senseid·tasks::handlers::super::executor·TaskContext·pg`, produced
/// because only the first of a run of `super::` was ever consumed.
///
/// Checked over produced FQNs rather than imports, because the defect was in
/// path classification rather than in collection.
#[test]
fn no_produced_fqn_retains_a_navigation_segment() {
    let mut bad = Vec::new();
    let mut files = 0usize;
    for path in corpus(&["rs"]) {
        let Some((adapter, content)) = read(&path) else { continue };
        let Some(out) = adapter.fqn_output(&path.to_string_lossy(), &content) else { continue };
        files += 1;
        let produced = out
            .defs
            .iter()
            .map(|d| d.fqn.as_str())
            .chain(out.refs.iter().filter_map(|r| r.target_fqn.as_deref()));
        for fqn in produced {
            let navigational = fqn
                .split('·')
                .any(|seg| seg.split("::").any(|m| matches!(m, "super" | "self" | "crate")));
            if navigational || fqn.contains(['{', '}']) {
                bad.push(format!("{}: {fqn}", path.display()));
            }
        }
    }
    assert!(files > 100, "corpus too small to be meaningful: {files} files produced FQNs");
    assert!(
        bad.is_empty(),
        "{} FQNs retaining a navigation segment across {files} files:\n{}",
        bad.len(),
        bad.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
    );
}

/// The docs corpus is real markdown, and `.txt` abstains by design — the suffix
/// cannot decide between an llms corpus and a licence, so content does.
#[test]
fn docs_corpus_maps_to_markdown_and_txt_abstains() {
    let docs = corpus(&["md"]);
    assert!(docs.len() > 50, "expected a real docs corpus, found {}", docs.len());
    for ext in ["md", "markdown", "mdx"] {
        assert_eq!(language_for_ext_slug(ext), Some("markdown"), "{ext}");
    }
    assert_eq!(language_for_ext_slug("txt"), None, "`.txt` is decided by content");
}

/// A runtime global must never be attributed to the module that called it.
///
/// The unresolved arm used to mint `<pkg>·<caller module>·String` per call site,
/// so one built-in became hundreds of distinct fabricated nodes. Checked over
/// this repo's own TypeScript and Svelte, which call these constantly.
#[test]
fn no_runtime_global_is_attributed_to_the_calling_module() {
    const GLOBALS: &[&str] = &[
        "String",
        "Number",
        "Boolean",
        "Object",
        "Array",
        "JSON",
        "Math",
        "Promise",
        "fetch",
        "setTimeout",
        "clearTimeout",
        "parseInt",
        "encodeURIComponent",
        "console",
    ];
    let mut bad = Vec::new();
    let mut files = 0usize;
    let mut hits = 0usize;
    for path in corpus(&["ts", "svelte"]) {
        let Some((adapter, content)) = read(&path) else { continue };
        let Some(out) = adapter.fqn_output(&path.to_string_lossy(), &content) else { continue };
        files += 1;
        for r in &out.refs {
            let Some(fqn) = r.target_fqn.as_deref() else { continue };
            if !GLOBALS.contains(&r.target_name.as_str()) {
                continue;
            }
            hits += 1;
            // Resolved to the runtime => `lib·…`. Anything else claims the global
            // is defined in project code.
            if !fqn.starts_with("lib·") {
                bad.push(format!("{}: {fqn}", path.display()));
            }
        }
    }
    assert!(files > 100, "corpus too small: {files} files produced FQNs");
    assert!(hits > 50, "expected real global usage, saw {hits} references");
    assert!(
        bad.is_empty(),
        "{} runtime globals attributed to project code across {files} files \
         ({hits} global references seen):\n{}",
        bad.len(),
        bad.iter().take(20).cloned().collect::<Vec<_>>().join("\n")
    );
}
