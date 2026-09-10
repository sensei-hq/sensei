pub mod community;
pub mod cross_repo;
pub mod doc_indexer;
pub mod lib_indexer;
pub mod llms_indexer;

// ── indexer v2 (docs/design/indexer-v2.md, docs/plans/indexer-v2-rust.md) ─────
// Deliberately without a caller. The shipped code-graph indexer under
// `crate::languages` keeps producing the graph until the rust cutover in step 9
// of the plan; wiring these in before then would put two producers with
// different rules on the same tables.
pub mod facts;
pub mod fqn;
pub mod lang;
pub mod persist;
pub mod reconcile;
pub mod resolve;
pub mod scan_root;

/// Every v2 source file, as `(path relative to `src/indexer/`, body)`.
///
/// Several v2 rules are properties of the CODE rather than of any value it
/// computes — "no fqn is built by string formatting outside `fqn.rs`", "no
/// `Option` stands in for a resolution". Those are only checkable by reading the
/// source, and more than one test file checks one, so the reader lives here
/// instead of being copied into each.
///
/// Scoped to the v2 files listed below rather than to everything under
/// `src/indexer/`, because the v1 modules that share this directory are not v2's
/// to constrain: `doc_indexer.rs` and its neighbours parse markdown, where the
/// fqn separator is a plausible bullet character, and an edit there must not be
/// able to turn a v2 guard red.
#[cfg(test)]
fn guard_sources() -> Vec<(String, String)> {
    /// Files, and directories whose whole contents are v2's. Adding a v2 module
    /// outside these is a deliberate act that has to be recorded here.
    const V2: &[&str] = &["facts.rs", "fqn.rs", "persist.rs", "reconcile.rs", "resolve.rs", "lang"];

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/indexer");
    let mut out = Vec::new();
    for entry in V2.iter().flat_map(|name| walkdir::WalkDir::new(root.join(name))) {
        let entry = entry.expect("the v2 source tree must be readable");
        if entry.path().extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(&root)
            .expect("walkdir yields paths under the root it was given")
            .display()
            .to_string();
        let body = std::fs::read_to_string(entry.path())
            .unwrap_or_else(|e| panic!("cannot read {relative}: {e}"));
        out.push((relative, body));
    }
    for required in
        ["facts.rs", "fqn.rs", "persist.rs", "reconcile.rs", "resolve.rs", "lang/rust.rs"]
    {
        assert!(
            out.iter().any(|(p, _)| p == required),
            "the guard did not find {required}, so it is not reading what it claims to guard"
        );
    }
    out
}

/// Every source file of this repo's own Rust, as `(absolute path, body)`.
///
/// This is the corpus the counting, histogram and order-independence tests run
/// over. A fixture proves the walk handles what the fixture's author thought of;
/// the corpus proves it handles what is there. It lives here rather than in one
/// language module's tests because both the walk and the ladder are measured
/// against the same files, and two readers would be two corpora.
#[cfg(test)]
fn corpus_rust_sources() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("the crate sits two levels below the workspace root")
        .join("crates");
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(&root).into_iter().filter_entry(|e| {
        e.file_name() != "target" && !e.file_name().to_string_lossy().starts_with('.')
    }) {
        let entry = entry.expect("the workspace source tree must be readable");
        if entry.path().extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let body = std::fs::read_to_string(entry.path())
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", entry.path().display()));
        out.push((entry.path().display().to_string(), body));
    }
    assert!(out.len() > 100, "the corpus is this repo's rust; {} files is not it", out.len());
    // Alphabetical, so a test that reverses it is reversing a known order rather
    // than one the filesystem chose.
    out.sort_by(|(a, _), (b, _)| a.cmp(b));
    out
}

/// The package a crate's manifest declares, which source spells with an
/// underscore where the manifest hyphenates it.
#[cfg(test)]
pub(crate) fn package_of(path: &str) -> String {
    let mut dir = std::path::Path::new(path);
    while let Some(parent) = dir.parent() {
        let manifest = parent.join("Cargo.toml");
        if manifest.exists() {
            let body = std::fs::read_to_string(&manifest).expect("a manifest is readable");
            for line in body.lines() {
                if let Some(name) = line.strip_prefix("name = ") {
                    return name.trim().trim_matches('"').to_string();
                }
            }
        }
        dir = parent;
    }
    panic!("{path} sits under no manifest");
}

/// A file's package-relative module path. A crate root and a `mod.rs` name
/// the directory they sit in, not themselves.
#[cfg(test)]
pub(crate) fn module_of(path: &str) -> String {
    let Some((_, tail)) = path.split_once("/src/") else {
        return String::new();
    };
    let tail = tail.strip_suffix(".rs").unwrap_or(tail);
    let mut segments: Vec<&str> = tail.split('/').collect();
    if matches!(segments.last(), Some(&"lib") | Some(&"main") | Some(&"mod")) {
        segments.pop();
    }
    segments.join("::")
}

/// The part of a v2 source file that is NOT its test module.
///
/// A test may legitimately write an expected fqn as a string literal — that is
/// what an assertion IS — while production code may never build one. Splitting
/// here is what lets the guards below be strict about the second without
/// forbidding the first.
#[cfg(test)]
pub(crate) fn outside_tests(body: &str) -> &str {
    body.split("#[cfg(test)]").next().unwrap_or(body)
}
