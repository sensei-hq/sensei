//! Stage 4's missing half: WHERE a file sits, in the terms an identity needs.
//!
//! A [`super::lang::Source`] carries four things — package, module, path, text.
//! Three are trivial. The PACKAGE is not: it is written in a manifest that sits
//! in some ancestor directory, and finding it is filesystem work that
//! [`super::lang::LanguageAdapter::read`] deliberately cannot do. `read` takes
//! source TEXT and no path to open, which is what lets every language test run
//! on string literals with no fixture tree.
//!
//! So the split is: this module answers "which package is this file in, and
//! what is its module path within it", and the adapter answers "what do the
//! contents mean". The acceptance harness has been answering the first question
//! with a per-corpus constant (`package = "web"` for three front ends, a path
//! regex for rust); that is a test tool and never was a rule.
//!
//! NOTHING HERE READS A MANIFEST FORMAT. `adapters::manifest` already owns that
//! across ten ecosystems and returns a [`ParsedManifest`] with the name in it; a
//! second reader would be a second answer to "what is this package called".
//
// No caller until cutover — see the note in `indexer/mod.rs`.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use super::facts::Language;
use crate::adapters::manifest::manifest_adapter_for_filename;

/// Where a file sits: the package that owns it and its module path inside that
/// package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    pub package: String,
    pub module: String,
}

/// The manifest that OWNS a file: the nearest one at or above it, stopping at
/// `repo_root`.
///
/// The same resolution [`super::scan_repo::nearest_lockfile`] uses for "which
/// lockfile serves this manifest", and deliberately so — a repository nests
/// packages (`app/src-tauri/Cargo.toml` under the workspace root), so "the
/// repo's manifest" and "the one in this folder" are both wrong for some file
/// in every such tree.
///
/// PURE: `manifests` is the set already discovered by the stage-2 walk, so this
/// does no IO and is testable on literals.
pub fn owning_manifest(file: &Path, repo_root: &Path, manifests: &[PathBuf]) -> Option<PathBuf> {
    let mut dir = file.parent()?;
    loop {
        // SEVERAL MANIFESTS IN ONE DIRECTORY IS REAL, so the tie is broken by
        // name rather than by the order the caller's walk happened to yield.
        // MEASURED: a checkout with `pyproject.toml` beside `setup.py` at its
        // root placed all 94 of its files or none of them depending on which
        // the directory walk returned first — `setup.py` has no registered
        // adapter, so it names nothing and every file under it goes unplaced.
        // Whichever answer is right, it must not change between two runs over
        // an unchanged tree (R6).
        if let Some(here) = manifests.iter().filter(|m| m.parent() == Some(dir)).min() {
            return Some(here.clone());
        }
        if dir == repo_root {
            return None;
        }
        match dir.parent() {
            // Never climb out of the repo: a sibling checkout's manifest names
            // a package this file is not in.
            Some(p) if p.starts_with(repo_root) || p == repo_root => dir = p,
            _ => return None,
        }
    }
}

/// The package name a manifest states, read by the adapter that owns its
/// format.
///
/// `None` when the manifest states none. A virtual Cargo workspace root and a
/// private `package.json` both do this legitimately, and naming the package
/// after its directory instead would mint an identity no dependency edge ever
/// spells (R4) — so the caller climbs to the next manifest rather than guessing.
pub fn package_named_by(manifest: &Path, text: &str) -> Option<String> {
    let filename = manifest.file_name()?.to_str()?;
    let named = manifest_adapter_for_filename(filename)?.parse_manifest(text).name?;
    match named.trim().is_empty() {
        true => None,
        false => Some(named),
    }
}

/// Where a file sits, given the package that owns it.
///
/// The module path is the ADAPTER's rule — rust drops a trailing `mod`/`lib`,
/// javascript keeps a trailing `index`, java has none at all — so it is asked
/// rather than re-derived here. That keeps one owner for a segment of every
/// identity the file mints.
pub fn placement_of(
    file: &Path,
    package: &str,
    package_root: &Path,
    language: Language,
) -> Placement {
    let module = super::lang::adapter_for(language)
        .module_path(&file.to_string_lossy(), &package_root.to_string_lossy());
    Placement { package: package.to_string(), module }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    /// **THE NEAREST MANIFEST WINS, NOT THE REPO'S.**
    ///
    /// The shape that makes this non-obvious is in this very repository: a
    /// workspace `Cargo.toml` at the root AND a nested one under
    /// `app/src-tauri/`. A "use the repo root" rule puts every Tauri file in
    /// the wrong package; a "same folder only" rule finds nothing for any file
    /// that is not beside a manifest, which is nearly all of them.
    #[test]
    fn the_nearest_manifest_at_or_above_a_file_owns_it() {
        let root = p("/r");
        let manifests =
            vec![p("/r/Cargo.toml"), p("/r/app/src-tauri/Cargo.toml"), p("/r/app/package.json")];

        assert_eq!(
            owning_manifest(&p("/r/crates/senseid/src/main.rs"), &root, &manifests),
            Some(p("/r/Cargo.toml")),
            "nothing nearer, so the workspace root owns it"
        );
        assert_eq!(
            owning_manifest(&p("/r/app/src-tauri/src/lib.rs"), &root, &manifests),
            Some(p("/r/app/src-tauri/Cargo.toml")),
            "the NESTED manifest wins over the root one"
        );
        assert_eq!(
            owning_manifest(&p("/r/app/src/routes/+page.svelte"), &root, &manifests),
            Some(p("/r/app/package.json")),
            "and a different ecosystem's manifest owns the same subtree's web files"
        );
    }

    /// A directory holding SEVERAL manifests resolves to the same one every
    /// run, whatever order the caller's walk yielded them in.
    ///
    /// The mutation that must break this: restore `.find()` in place of
    /// `.min()`. The answer then tracks the argument order, and a file's
    /// package — the second segment of every identity it declares — changes
    /// between two runs over an unchanged tree (R6).
    #[test]
    fn a_directory_with_two_manifests_resolves_the_same_way_every_run() {
        let root = p("/r");
        let file = p("/r/src/a.py");
        let one = vec![p("/r/pyproject.toml"), p("/r/setup.py")];
        let other = vec![p("/r/setup.py"), p("/r/pyproject.toml")];
        assert_eq!(
            owning_manifest(&file, &root, &one),
            owning_manifest(&file, &root, &other),
            "the answer must not depend on the order the walk yielded"
        );
        assert_eq!(owning_manifest(&file, &root, &one), Some(p("/r/pyproject.toml")));
    }

    /// The walk stops at the repo root rather than following the filesystem
    /// out of it. A sibling checkout's manifest names a package this file is
    /// not in, and borrowing it would file every unowned file under somebody
    /// else's package.
    #[test]
    fn the_search_never_climbs_out_of_the_repository() {
        let manifests = vec![p("/elsewhere/Cargo.toml")];
        assert_eq!(owning_manifest(&p("/r/src/main.rs"), &p("/r"), &manifests), None);
    }

    /// A manifest that states NO name yields none, rather than a name derived
    /// from its directory.
    ///
    /// Both shapes are legitimate and both are in this repository's ecosystem:
    /// a virtual Cargo workspace root has `[workspace]` and no `[package]`, and
    /// a private `package.json` may omit `name`. Naming it after the folder
    /// would mint a package no dependency edge ever spells.
    #[test]
    fn a_manifest_that_states_no_name_yields_none() {
        assert_eq!(
            package_named_by(&p("/r/Cargo.toml"), "[package]\nname = \"senseid\"\n").as_deref(),
            Some("senseid")
        );
        assert_eq!(package_named_by(&p("/r/Cargo.toml"), "[workspace]\nmembers = []\n"), None);
        assert_eq!(package_named_by(&p("/r/package.json"), "{\"private\": true}"), None);
        assert_eq!(
            package_named_by(&p("/r/Makefile"), "all:\n"),
            None,
            "a file no manifest adapter claims names no package"
        );
    }

    /// The module path is the ADAPTER's rule, asked rather than re-derived.
    ///
    /// One fixture per language, because the three rules genuinely differ and a
    /// single-language test would pass while the seam handed the wrong string
    /// to the other two.
    #[test]
    fn the_module_path_comes_from_the_language_that_owns_the_rule() {
        let rust = placement_of(
            &p("/r/crates/senseid/src/indexer/facts.rs"),
            "senseid",
            &p("/r/crates/senseid"),
            Language::Rust,
        );
        assert_eq!(
            (rust.package.as_str(), rust.module.as_str()),
            ("senseid", "indexer::facts"),
            "rust counts from the crate's src and joins with ::"
        );

        let web = placement_of(
            &p("/r/app/src/lib/scan-state.svelte.ts"),
            "app",
            &p("/r/app"),
            Language::TypeScript,
        );
        assert_eq!(
            (web.package.as_str(), web.module.as_str()),
            ("app", "lib/scan-state.svelte"),
            "javascript keeps the non-omittable extension and joins with /"
        );

        let java = placement_of(
            &p("/r/server/src/main/java/com/x/Svc.java"),
            "server",
            &p("/r/server"),
            Language::Java,
        );
        assert_eq!(
            (java.package.as_str(), java.module.as_str()),
            ("server", ""),
            "java states its package in the source, so the path contributes no module segment"
        );
    }
}

#[cfg(test)]
mod corpus {
    use super::*;

    /// **THE SEAM AGREES WITH THE HARNESS, OVER THIS REPOSITORY'S OWN RUST.**
    ///
    /// The acceptance harness has been answering "which package is this file
    /// in" with `indexer::package_of`, which walks up for a `Cargo.toml` and
    /// greps `name = ` out of it — a hand-rolled manifest reader that exists
    /// only because there was no production seam to ask. This module is that
    /// seam, and it delegates to `adapters::manifest` instead.
    ///
    /// Two readers of one fact drift, and this fact is a SEGMENT OF EVERY FQN
    /// every file declares — so the drift would not be a wrong count, it would
    /// be a graph where half the identities no longer meet. The test exists to
    /// make that impossible to do quietly.
    ///
    ///     cargo test -p senseid --bin senseid -- --ignored placement::corpus
    #[test]
    #[ignore = "walks this repository"]
    fn the_seam_and_the_harness_name_the_same_package_for_every_rust_file() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("the workspace root")
            .to_path_buf();

        let manifests: Vec<PathBuf> = ignore::WalkBuilder::new(root.join("crates"))
            .build()
            .filter_map(Result::ok)
            .map(|e| e.into_path())
            .filter(|p| p.file_name().is_some_and(|n| n == "Cargo.toml"))
            .chain(std::iter::once(root.join("Cargo.toml")))
            .collect();
        assert!(manifests.len() > 3, "the walk found almost no manifests: {}", manifests.len());

        let mut checked = 0usize;
        let mut disagreements: Vec<String> = Vec::new();
        for (path, _) in crate::indexer::corpus_rust_sources() {
            let file = PathBuf::from(&path);
            let Some(manifest) = owning_manifest(&file, &root, &manifests) else {
                disagreements.push(format!("{path}: the seam found no owning manifest"));
                continue;
            };
            let text = std::fs::read_to_string(&manifest).expect("a manifest is readable");
            let Some(package) = package_named_by(&manifest, &text) else {
                // A virtual workspace root names nothing; the harness would
                // have climbed past it. Not a disagreement, a different
                // question — recorded so the count below stays honest.
                continue;
            };
            let root_of = manifest.parent().expect("a manifest has a directory");
            let placed = placement_of(&file, &package, root_of, Language::Rust);

            let harness = (crate::indexer::package_of(&path), crate::indexer::module_of(&path));
            if (placed.package.clone(), placed.module.clone()) != harness {
                disagreements.push(format!(
                    "{path}: seam {:?} vs harness {:?}",
                    (placed.package, placed.module),
                    harness
                ));
            }
            checked += 1;
        }

        assert!(checked > 300, "only {checked} files compared, so this proved little");
        assert!(
            disagreements.is_empty(),
            "{} of {checked} files disagree:\n{}",
            disagreements.len(),
            disagreements.iter().take(10).cloned().collect::<Vec<_>>().join("\n")
        );
    }
}
