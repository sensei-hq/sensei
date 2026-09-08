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
    const V2: &[&str] = &["facts.rs", "fqn.rs", "lang"];

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
    for required in ["facts.rs", "fqn.rs", "lang/rust.rs"] {
        assert!(
            out.iter().any(|(p, _)| p == required),
            "the guard did not find {required}, so it is not reading what it claims to guard"
        );
    }
    out
}

/// The part of a v2 source file that is NOT its test module.
///
/// A test may legitimately write an expected fqn as a string literal — that is
/// what an assertion IS — while production code may never build one. Splitting
/// here is what lets the guards below be strict about the second without
/// forbidding the first.
#[cfg(test)]
fn outside_tests(body: &str) -> &str {
    body.split("#[cfg(test)]").next().unwrap_or(body)
}
