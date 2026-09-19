pub mod community;
pub mod cross_repo;
pub mod doc_indexer;
pub mod index;
pub mod lib_indexer;
pub mod llms_indexer;
pub mod placement;

// ── the indexer (docs/design/indexer.md, docs/plans/indexer-sequence.md) ─────
// Deliberately without a caller. The LEGACY code-graph indexer under
// `crate::languages` keeps producing the graph until the rust cutover at stage
// 10; wiring these in before then would put two producers with different rules
// on the same tables.
/// The acceptance harness — §6's A1–A9 and R8, measured over the real corpus.
///
/// `#[cfg(test)]` because it IS a test-runner tool. What it is NOT is a
/// comparison: the question "does this agree with the producer being replaced"
/// is a transition question, and answering it was setting the agenda for work
/// whose actual goal is an accurate call and reference graph. Thresholds, not
/// comparisons (§6's first line) — a graph is judged against what a reader
/// needs from it, and the legacy producer is not that reader.
#[cfg(test)]
pub mod acceptance;
/// The two coverage barriers, over any corpus — see the module docs for why it
/// is not simply a test in `acceptance`.
#[cfg(test)]
pub mod barrier;
pub mod facts;
pub mod fqn;
pub mod impact;
pub mod incremental;
pub mod lang;
pub mod persist;
pub mod pipeline;
pub mod reconcile;
pub mod resolve;
pub mod scan_repo;
pub mod scan_root;
pub mod structure;

/// Every guarded source file, as `(path relative to `src/indexer/`, body)`.
///
/// Several of this indexer's rules are properties of the CODE rather than of
/// any value it computes — "no fqn is built by string formatting outside
/// `fqn.rs`", "no `Option` stands in for a resolution". Those are only checkable
/// by reading the source, and more than one test file checks one, so the reader
/// lives here instead of being copied into each.
///
/// Scoped to the files listed below rather than to everything under
/// `src/indexer/`, because the LEGACY modules that share this directory are not
/// this indexer's to constrain: `doc_indexer.rs` and its neighbours parse
/// markdown, where the fqn separator is a plausible bullet character, and an
/// edit there must not be able to turn a guard red.
#[cfg(test)]
fn guard_sources() -> Vec<(String, String)> {
    /// Files, and directories whose whole contents this indexer owns. Adding a
    /// module outside these is a deliberate act that has to be recorded here.
    const OWNED: &[&str] =
        &["facts.rs", "fqn.rs", "impact.rs", "persist.rs", "reconcile.rs", "resolve.rs", "lang"];

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/indexer");
    let mut out = Vec::new();
    for entry in OWNED.iter().flat_map(|name| walkdir::WalkDir::new(root.join(name))) {
        let entry = entry.expect("the guarded source tree must be readable");
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
    for required in [
        "facts.rs",
        "fqn.rs",
        "impact.rs",
        "persist.rs",
        "reconcile.rs",
        "resolve.rs",
        "lang/mod.rs",
        "lang/common.rs",
        "lang/rust/mod.rs",
        "lang/rust/walk.rs",
        "lang/rust/types.rs",
        "lang/javascript.rs",
        "lang/svelte.rs",
    ] {
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

/// Every JavaScript, TypeScript and Svelte source file of this repo's own front
/// ends, as `(path relative to the workspace root, body)`.
///
/// The sibling of [`corpus_rust_sources`], and it exists for the same reason: a
/// fixture proves the walk handles what the fixture's author thought of, and
/// the corpus proves it handles what is there. `app/`, `dojo/` and `website/`
/// are three real SvelteKit applications, which is a harder corpus than
/// anything a test would write.
///
/// `node_modules` and build output are excluded, and so is everything
/// generated: `.svelte-kit` holds code nobody wrote, so a defect found in it is
/// a defect in a generator rather than in this reader.
#[cfg(test)]
fn corpus_web_sources() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("the crate sits two levels below the workspace root");
    web_sources_under(root, &["app", "dojo", "website"], 200)
}

/// The same reader, pointed anywhere.
///
/// Split out so a check can run over a codebase NOBODY HERE WROTE. Our own
/// front ends were written alongside this indexer, so agreement with them is
/// weaker evidence than it looks — a grammar shape none of our authors happen
/// to use is a shape the reader has never been asked about. `SENSEI_CORPUS`
/// points this at somebody else's code.
#[cfg(test)]
fn web_sources_under(root: &std::path::Path, tops: &[&str], least: usize) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for top in tops {
        // The SAME walker the scan and the fs-watcher use. A hand-written skip
        // list here was not merely duplication: it let 52 gitignored files —
        // an i18n compiler's generated messages — into the corpus, so every
        // number measured over it was a property of the developer's disk
        // rather than of the source tree.
        for entry in crate::tasks::handlers::helpers::build_walker(&root.join(top)).build() {
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            let claimed = path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                matches!(e, "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "svelte")
            });
            if !claimed {
                continue;
            }
            // A file that will not read as UTF-8 is not source this walk can be
            // handed, and skipping it here is not a fallback: the caller of the
            // real pipeline never gets one either.
            let Ok(body) = std::fs::read_to_string(path) else { continue };
            let relative = path
                .strip_prefix(root)
                .expect("walkdir yields paths under the root it was given")
                .display()
                .to_string();
            out.push((relative, body));
        }
    }
    assert!(out.len() >= least, "expected at least {least} files, found {}", out.len());
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

/// A corpus path made relative to the workspace root — the grain a real walk
/// produces and the grain `sensei.files` is keyed by.
///
/// [`corpus_rust_sources`] yields ABSOLUTE paths because [`package_of`] has to
/// find the nearest `Cargo.toml` on disk. A fixture that then hands those same
/// absolute strings to the persist path is not reproducing a scan: the walk
/// records a file relative to its folder, and a lookup resolves a leading `/`
/// against `folders.abs_path` instead.
#[cfg(test)]
pub(crate) fn workspace_relative(path: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("the crate sits two levels below the workspace root");
    std::path::Path::new(path)
        .strip_prefix(root)
        .map(|p| p.display().to_string())
        // Not a silent fallback: a path outside the workspace is not a corpus
        // path, and this is test-only code whose one caller reads the corpus.
        .unwrap_or_else(|_| panic!("{path} does not sit under {}", root.display()))
}

/// A file's package-relative module path, for a corpus path whose crate root
/// is not separately known.
///
/// The RULE lives in [`lang::rust::module_path`] and this only supplies the
/// missing argument: the crate root, recovered by splitting at `/src/`. That
/// split is a GUESS and is why this stays test-only — the corpus walks real
/// paths and the guess holds for them, while a production caller knows the
/// crate root for real (it found the manifest) and must pass it.
///
/// It used to reimplement the rule instead of delegating, which made two
/// spellings of a string that is a SEGMENT OF EVERY FQN a file declares.
#[cfg(test)]
pub(crate) fn module_of(path: &str) -> String {
    // The crate root is found the way `package_of` finds it — by walking up for
    // the manifest — and NOT by splitting on `/src/`.
    //
    // The split looked equivalent and was not, for every file a package holds
    // OUTSIDE `src/`: `build.rs`, `tests/*.rs`, `examples/*.rs` are each their
    // own crate root, and the split returned an EMPTY module for all of them.
    // An empty module is the LIBRARY crate root's, so every declaration in
    // `tests/e2e_index.rs` minted the identity a declaration of the same name
    // in `src/lib.rs` mints — two files claiming one symbol (spec §2), which is
    // the break this string exists to prevent.
    //
    // MEASURED over this repository: 6 of 390 rust files, caught by the test
    // that pins this helper against `indexer::placement`, the production seam.
    let mut dir = std::path::Path::new(path);
    while let Some(parent) = dir.parent() {
        if parent.join("Cargo.toml").exists() {
            return lang::rust::module_path(path, &parent.to_string_lossy());
        }
        dir = parent;
    }
    String::new()
}

/// One Rust source, walked and then placed against the shared ladder — the two
/// steps the processor will run at cutover, so what a test is handed here is
/// what its subject will be handed then.
///
/// Here rather than in each test module because three of them need it, and the
/// two that had it had it character-for-character twice. A per-module copy is
/// how the walk step and the resolve step drift apart between subjects, and the
/// drift reads as a difference in the SUBJECT rather than in the fixture.
#[cfg(test)]
pub(crate) fn walked_rust(module: &str, path: &str, text: &str) -> facts::FileFacts {
    use std::collections::BTreeSet;

    use lang::{Source, TypeHomes, rust};
    use resolve::{World, resolve};

    let facts =
        rust::read(&Source { package: "senseid", module, path, text }, &TypeHomes::unknown())
            .expect("the fixture parses");
    let first_party: BTreeSet<String> = ["senseid".to_string()].into_iter().collect();
    resolve(
        facts,
        &rust::GRAMMAR,
        &World {
            first_party: &first_party,
            first_party_members: &BTreeSet::new(),
            declared_members: &BTreeSet::new(),
            supplied_members: &resolve::SuppliedMembers::unknown(),
            returns: &std::collections::BTreeMap::new(),
            scanned: &BTreeSet::new(),
        },
    )
}

/// The part of a guarded source file that is NOT its test module.
///
/// A test may legitimately write an expected fqn as a string literal — that is
/// what an assertion IS — while production code may never build one. Splitting
/// here is what lets the guards below be strict about the second without
/// forbidding the first.
///
/// The split is on the ATTRIBUTE — the marker at the start of a line — and not
/// on the phrase anywhere in the file. MEASURED: `lang/rust.rs` carried
/// `#[cfg(test)]` inside a doc comment on line 304, so every guard cut the file
/// there and two thirds of the walk was never read. A vacuous guard is green,
/// which is why this is a rule and not a convenience.
#[cfg(test)]
pub(crate) fn outside_tests(body: &str) -> &str {
    let mut offset = 0;
    for line in body.split_inclusive('\n') {
        if line.trim_start().starts_with("#[cfg(test)]") {
            return &body[..offset];
        }
        offset += line.len();
    }
    body
}
