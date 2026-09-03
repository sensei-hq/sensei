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

/// The folder-relative path the indexer would store in `nodes.file_path`.
///
/// The corpus walk yields absolute paths, but `fqn_output` takes rel_path as its
/// scope anchor for path-scoped languages — passing the absolute path here would
/// test a shape production never sees.
fn rel_of(path: &Path) -> String {
    path.strip_prefix(repo_root()).unwrap_or(path).to_string_lossy().to_string()
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
        let Some(out) = adapter.fqn_output(&path.to_string_lossy(), &rel_of(&path), &content)
        else {
            continue;
        };
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
        let Some(out) = adapter.fqn_output(&path.to_string_lossy(), &rel_of(&path), &content)
        else {
            continue;
        };
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

/// A prelude item or type must never be attributed to project code. Checked over
/// this repo's own Rust, which uses `Some`/`Ok`/`Err`/`String::new` constantly.
#[test]
fn no_rust_prelude_name_is_attributed_to_project_code() {
    const ITEMS: &[&str] = &["Some", "None", "Ok", "Err"];
    let mut bad = Vec::new();
    let (mut files, mut hits) = (0usize, 0usize);
    for path in corpus(&["rs"]) {
        let Some((adapter, content)) = read(&path) else { continue };
        let Some(out) = adapter.fqn_output(&path.to_string_lossy(), &rel_of(&path), &content)
        else {
            continue;
        };
        files += 1;
        for r in &out.refs {
            let Some(fqn) = r.target_fqn.as_deref() else { continue };
            // A BARE prelude item resolves to `lib·std·prelude·X` (4 segments); a
            // qualified `Type::Ok` on a LOCAL enum legitimately resolves to
            // `rust·pkg·module·Type·Ok` (5). This repo has three such enums —
            // WarmAttempt, BatchOutcome, ProbeOutcome — each with an `Ok` variant,
            // and attributing those to std would be the opposite error. So only
            // count refs whose fqn has no type segment before the name.
            let segments = fqn.split('·').count();
            let bare_prelude_item = ITEMS.contains(&r.target_name.as_str()) && segments == 4;
            let on_prelude_type = fqn.contains("·String·") || fqn.contains("·Vec·");
            if !(bare_prelude_item || on_prelude_type) {
                continue;
            }
            hits += 1;
            if !fqn.starts_with("lib·") {
                bad.push(format!("{}: {fqn}", path.display()));
            }
        }
    }
    assert!(files > 100, "corpus too small: {files} files produced FQNs");
    assert!(hits > 50, "expected real prelude usage, saw {hits} references");
    assert!(
        bad.is_empty(),
        "{} prelude names attributed to project code across {files} files \
         ({hits} prelude references seen):\n{}",
        bad.len(),
        bad.iter().take(20).cloned().collect::<Vec<_>>().join("\n")
    );
}

/// NO produced FQN may embed an absolute filesystem path.
///
/// `fqn_output` receives both `abs_path` (for the manifest walk) and `rel_path`
/// (the scope anchor). Passing the wrong one is a silent, compiler-invisible
/// mistake: the fqns still look well-formed, and every unit test with a tempdir
/// fixture still passes — the tempdir path just leaks into the value. In
/// production it would mean an fqn that can never match across machines, and a
/// home directory written into the graph.
///
/// This ran against the real tree because the defect it guards was found in real
/// tree data, not by reasoning: a C project with parallel `Cpp/` and `Hpp/`
/// trees and no build file, where a stem-only module made a header and its
/// implementation share an fqn. Hand-written fixtures had agreed with the bug.
///
/// WHAT THIS DOES AND DOES NOT COVER, established by probing rather than
/// assumed: this repo has a `Makefile` at its root, so every `.c` file in the
/// tree resolves through `c_fqn`'s BUILD-ROOT branch. Mutating that branch to
/// use `abs_path` fails this test with 10 named offenders. The no-build-root
/// FALLBACK is unreachable from this corpus and is covered by
/// `c_fqn_tests::a_header_impl_pair_in_parallel_trees_does_not_collide_without_a_build_root`
/// instead — a mutation there will NOT fail this test, so do not read a pass
/// here as covering it.
///
/// Breaking mutation: in `c_fqn::c_file_context`, derive the build-root module
/// from `abs_path` instead of `path.strip_prefix(d)`.
#[test]
fn no_produced_fqn_embeds_an_absolute_path() {
    let root = repo_root().to_string_lossy().to_string();
    let mut bad: Vec<String> = Vec::new();
    let mut files = 0usize;

    // Every extension the adapters claim, so a new language is covered the day
    // it registers rather than whenever someone remembers to extend this list.
    let exts: Vec<String> = super::all_adapters()
        .iter()
        .flat_map(|a| a.extensions().iter().map(|e| e.trim_start_matches('.').to_string()))
        .collect();
    let ext_refs: Vec<&str> = exts.iter().map(String::as_str).collect();

    for path in corpus(&ext_refs) {
        let Some((adapter, content)) = read(&path) else { continue };
        let rel = rel_of(&path);
        let Some(out) = adapter.fqn_output(&path.to_string_lossy(), &rel, &content) else {
            continue;
        };
        files += 1;

        // Compared against the checkout root rather than a literal home path, so
        // the assertion holds on any machine and CI.
        for d in &out.defs {
            if d.fqn.contains(&root) || d.fqn.contains("··/") {
                bad.push(format!("{rel}: {}", d.fqn));
            }
        }
        if out.module.starts_with('/') || out.module.contains(&root) {
            bad.push(format!("{rel}: module={}", out.module));
        }
    }

    assert!(files > 0, "corpus walk produced no FQN files — the test would vacuously pass");
    bad.truncate(10);
    assert!(bad.is_empty(), "{} fqns embed an absolute path, e.g. {bad:#?}", bad.len());
}

/// Trait-impl relations must be produced AT SCALE over the real tree, and most
/// must resolve to a parent FQN.
///
/// A unit test with three hand-written impls proves the shape; it cannot prove
/// the producer survives real Rust — generic bounds, `where` clauses, nested
/// modules, macro-adjacent code. In slice 1b a fixture-only producer agreed with
/// a real bug, so the volume and the resolution rate are asserted here against
/// the actual corpus.
///
/// The floor is deliberately loose: this pins "the producer works broadly", not
/// an exact count that churns with every commit to this repo.
#[test]
fn trait_impl_relations_are_produced_and_mostly_resolve_over_the_real_tree() {
    let mut total = 0usize;
    let mut resolved = 0usize;
    let mut lib_parents = 0usize;
    let mut files = 0usize;

    for path in corpus(&["rs"]) {
        let Some((adapter, content)) = read(&path) else { continue };
        let Some(out) = adapter.fqn_output(&path.to_string_lossy(), &rel_of(&path), &content)
        else {
            continue;
        };
        files += 1;
        for r in &out.relations {
            total += 1;
            if r.parent_fqn.is_some() {
                resolved += 1;
            }
            if r.is_lib {
                lib_parents += 1;
            }
            // An unresolved relation must STILL name its parent — that is the
            // difference between an honest unresolved edge and a useless one.
            assert!(
                !r.parent_name.trim().is_empty(),
                "{}: relation with no parent_name: {r:?}",
                rel_of(&path)
            );
            // A parent_fqn must never be a bare name; it is a lookup key.
            if let Some(f) = &r.parent_fqn {
                assert!(
                    f.contains(super::fqn::SEP),
                    "{}: parent_fqn {f:?} is not an encoded FQN",
                    rel_of(&path)
                );
            }
        }
    }

    assert!(files > 100, "corpus walk found only {files} rust files — test would be vacuous");
    // Floor, not a pin. MEASURED at 112 across 386 files when written. A regex
    // for `impl .. for ..` reports 122, and the 10 it adds are impls written
    // inside raw-string TEST FIXTURES — which tree-sitter correctly parses as
    // string literals and does not emit. The parser's number is the accurate
    // one; the regex over-counts.
    assert!(total > 100, "only {total} trait-impl relations across {files} files");
    let rate = resolved as f64 / total as f64;
    assert!(
        rate > 0.90,
        "only {resolved}/{total} ({:.1}%) relations resolved a parent FQN",
        rate * 100.0
    );
    // std/external traits (Debug, Display, From, Default) are pervasive in this
    // tree, so zero lib parents would mean the external arm never fires.
    assert!(lib_parents > 0, "no external trait parents among {total} relations");
}
