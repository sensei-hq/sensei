//! Stage 4's missing half: every file of ONE REPOSITORY, walked and resolved.
//!
//! This is the composition the whole indexer was missing. `pipeline.rs` ends at
//! the structure barrier — it writes repositories, folders and files, and says
//! so: "writes NO nodes and NO edges". Everything past that point existed as
//! pure functions with no caller: `placement`, the five adapters, the type
//! barrier, the ladder, the writer. Ten modules carried `#![allow(dead_code)]`
//! because nothing joined them up.
//!
//! # The repository is the unit, and that is what makes a file independent
//!
//! The task hierarchy is scan root → scan repo → find files → index. A language
//! adapter indexes ONE file and is TOLD where it sits, so it never scans a
//! sibling and never climbs a directory. But two facts genuinely span files and
//! cannot be read out of any one of them:
//!
//! - **Where a type lives.** `store.upsert()` mints
//!   `<module of PgStore>·PgStore·upsert·item`, and the module is the one the
//!   TYPE is declared in — not the one the call is written in. Rust puts `impl`
//!   blocks anywhere; this repo has 24 for `PgStore` alone.
//! - **What the scan declares at all.** A member nothing first-party declares is
//!   the language's or a library's, which is a BOUNDARY and not a miss.
//!
//! So they are built once per repository, between two passes, and handed to the
//! files — which is exactly what "lookups scoped to the repo" means. Within a
//! pass no file depends on any other, so a pass is parallelisable as-is.
//!
//! # Why two passes and not stub-then-promote
//!
//! A reference to a symbol in an unread file IS handled by stubbing: the writer
//! mints the node on first mention and promotes it when its own file arrives,
//! and `resolved = nodes.resolved OR EXCLUDED.resolved` makes that monotone. So
//! node EXISTENCE needs no barrier.
//!
//! Identity does. A stub is only promotable if the two sides spell the same
//! string, and the member spelling depends on the type's home — so a first pass
//! that guessed the home would mint a stub the declaration never meets, and
//! which guess it made would depend on file order (R6). The barrier is the
//! cheapest thing that removes the order dependence: walk, learn, walk again.

// No caller until the task handler lands — see the note in `indexer/mod.rs`.
#![allow(dead_code)]

use std::collections::BTreeSet;

use super::facts::FileFacts;
use super::lang::{self, Source, TypeHomes};
use super::resolve::{
    self, SuppliedMembers, World, member_names_of, members_declared_by, returns_declared_by,
};

/// One file of a repository: where the scan decided it sits, and its text.
///
/// Carries TEXT rather than a path to open, for the same reason [`Source`]
/// does — it keeps every decision here testable on string literals, and it is
/// the reader's caller that owns the IO.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed<'a> {
    /// Path as the graph records it, which is what the identity of a
    /// package-root file is minted from.
    pub path: &'a str,
    pub package: &'a str,
    /// Package-relative module path, from [`super::placement::placement_of`].
    pub module: &'a str,
    pub text: &'a str,
}

/// Every file of one repository, indexed: two passes around the type barrier,
/// then resolved against the repo-scoped tables.
///
/// `first_party` is every package this repository owns the source of, read from
/// the MANIFESTS rather than from what happened to parse — a package whose files
/// all failed to read is still ours, and deciding otherwise would make an import
/// of it a library (R5).
///
/// A file whose extension no adapter claims, or that the grammar rejects, is
/// absent from the result rather than represented by an empty one. The caller
/// can see which by comparing lengths; inventing facts for it would put a file
/// with no declarations and no uses into the graph as though that were measured.
pub fn index_repo<'a>(files: &[Placed<'a>], first_party: &BTreeSet<String>) -> Vec<FileFacts> {
    // PASS ONE, with no type table: every file read for the DECLARATIONS it
    // makes. Nothing here is anchored and nothing is kept but the symbols.
    let t0 = std::time::Instant::now();
    let first = read_all(files, &TypeHomes::unknown());
    let t_pass1 = t0.elapsed();

    // THE BARRIER. Where each type lives, across the whole repository.
    let homes = TypeHomes::of(
        files.iter().zip(&first).flat_map(|(p, f)| f.symbols.iter().map(move |s| (p.package, s))),
    );

    // PASS TWO, complete, before anything is placed. Every table below is taken
    // off THIS pass and not off `first`: a member's identity carries the module
    // its type lives in, so the pre-barrier pass spells those members
    // differently and a set taken from it would match nothing.
    let t1 = std::time::Instant::now();
    let anchored = read_all(files, &homes);
    let t_pass2 = t1.elapsed();
    let t2 = std::time::Instant::now();

    let first_party_members = member_names_of(&anchored);
    let declared_members = members_declared_by(&anchored);
    let supplied_members = SuppliedMembers::of(&anchored);
    let returns = returns_declared_by(&anchored);
    // EMPTY, deliberately. `World::scanned` means "files read so far", and
    // reading absence as externality is the order dependence spec §2 exists to
    // prevent — externality comes from the import, never from absence.
    let scanned = BTreeSet::new();
    let world = World {
        first_party,
        first_party_members: &first_party_members,
        declared_members: &declared_members,
        supplied_members: &supplied_members,
        returns: &returns,
        scanned: &scanned,
    };

    let out: Vec<FileFacts> = anchored
        .into_iter()
        .map(|facts| {
            let grammar = lang::adapter_for(facts.language).grammar();
            resolve::resolve(facts, grammar, &world)
        })
        .collect();
    if std::env::var("SENSEI_TIME_STAGES").is_ok() {
        eprintln!("  pass1 {:?}  pass2 {:?}  barrier+resolve {:?}", t_pass1, t_pass2, t2.elapsed());
    }
    out
}

/// One pass: every file read once, through the adapter its extension dispatches
/// to. Order-independent by construction — no file is handed anything derived
/// from another file in the same pass.
fn read_all(files: &[Placed<'_>], types: &TypeHomes) -> Vec<FileFacts> {
    files.iter().filter_map(|placed| read_one(placed, types)).collect()
}

/// One file. `None` when no adapter claims the extension or the grammar
/// rejects the text — a fact the caller acts on, never a blank stand-in.
fn read_one(placed: &Placed<'_>, types: &TypeHomes) -> Option<FileFacts> {
    let ext = placed.path.rsplit_once('.').map(|(_, e)| format!(".{e}"))?;
    let adapter = lang::adapter_for_ext(&ext)?;
    let source = Source {
        package: placed.package,
        module: placed.module,
        path: placed.path,
        text: placed.text,
    };
    adapter.read(&source, types).ok()
}

/// One file, read off disk and placed: everything [`index_repo`] needs, owned.
///
/// Separate from [`Placed`], which borrows, because the IO half has to own the
/// text it read before it can lend it out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    pub path: String,
    pub package: String,
    pub module: String,
    pub text: String,
}

impl Loaded {
    pub fn placed(&self) -> Placed<'_> {
        Placed { path: &self.path, package: &self.package, module: &self.module, text: &self.text }
    }
}

/// Why a file the scan found produced nothing to index.
///
/// Counted rather than discarded: a repository where every file is `Unclaimed`
/// has produced an empty graph for a reason, and a summary that reported only
/// "0 files indexed" could not say which reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Skipped {
    /// No adapter claims the extension. The common case by count — a repo is
    /// mostly not source.
    Unclaimed,
    /// An adapter claims it, but no manifest at or above it names a package.
    /// The file has no identity to declare anything under, and inventing one
    /// would mint a package no dependency edge ever spells.
    Unplaced,
    /// The bytes would not read as text.
    Unreadable,
}

/// The files of one repository, loaded and placed, plus what was skipped and
/// why.
///
/// THE IO HALF, and the only part of stage 4 that touches a disk. It reads;
/// every decision it makes is delegated — the adapter registry says who claims
/// an extension, `placement` says which package and module, and `index_repo`
/// says what the text means.
pub fn load_repo(
    repo_root: &std::path::Path,
    files: &[std::path::PathBuf],
    manifests: &[std::path::PathBuf],
) -> (Vec<Loaded>, BTreeSet<String>, Vec<(String, Skipped)>) {
    let mut loaded = Vec::new();
    let mut first_party = BTreeSet::new();
    let mut skipped = Vec::new();
    // One read per manifest, not one per file: a repository has thousands of
    // files and tens of manifests, and the nearest-manifest walk asks about the
    // same few over and over.
    let mut named: std::collections::HashMap<std::path::PathBuf, Option<String>> =
        std::collections::HashMap::new();

    for file in files {
        let shown = file.to_string_lossy().to_string();
        let Some(ext) = file.extension().and_then(|e| e.to_str()) else {
            skipped.push((shown, Skipped::Unclaimed));
            continue;
        };
        let Some(adapter) = lang::adapter_for_ext(&format!(".{ext}")) else {
            skipped.push((shown, Skipped::Unclaimed));
            continue;
        };
        let Some(manifest) = super::placement::owning_manifest(file, repo_root, manifests) else {
            skipped.push((shown, Skipped::Unplaced));
            continue;
        };
        let package = named
            .entry(manifest.clone())
            .or_insert_with(|| {
                std::fs::read_to_string(&manifest)
                    .ok()
                    .and_then(|text| super::placement::package_named_by(&manifest, &text))
            })
            .clone();
        let Some(package) = package else {
            skipped.push((shown, Skipped::Unplaced));
            continue;
        };
        let Ok(text) = std::fs::read_to_string(file) else {
            skipped.push((shown, Skipped::Unreadable));
            continue;
        };
        let root = manifest.parent().unwrap_or(repo_root);
        let placement = super::placement::placement_of(file, &package, root, adapter.language());
        // FROM THE MANIFEST, not from what parsed: a package whose files all
        // fail to read is still ours, and deciding otherwise would make an
        // import of it a library (R5).
        first_party.insert(placement.package.clone());
        loaded.push(Loaded {
            path: shown,
            package: placement.package,
            module: placement.module,
            text,
        });
    }
    (loaded, first_party, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::indexer::facts::{Observation, RefKind, Resolution};

    /// **THE WHOLE STAGE, OVER A REAL TREE ON DISK.**
    ///
    /// Every other test here runs on string literals, which proves the
    /// composition does what I meant and nothing about whether the pieces meet:
    /// the manifest reader, the nearest-manifest walk, the module rule and the
    /// adapter registry each have their own tests and had never been asked to
    /// agree on one directory. This is that question.
    #[test]
    fn a_repository_on_disk_loads_places_and_resolves_across_its_files() {
        let tmp = tempfile::tempdir().expect("a temp dir");
        let root = tmp.path();
        let write = |rel: &str, text: &str| {
            let at = root.join(rel);
            std::fs::create_dir_all(at.parent().expect("a parent")).expect("mkdir");
            std::fs::write(&at, text).expect("write");
            at
        };

        let manifest = write("Cargo.toml", "[package]\nname = \"demo\"\n");
        let a = write(
            "src/a.rs",
            "pub struct Widget { pub w: u32 }\n             impl Widget { pub fn wide(&self) -> u32 { self.w } }\n",
        );
        let b =
            write("src/b.rs", "use crate::a::Widget;\npub fn go(x: &Widget) -> u32 { x.wide() }\n");
        // Claimed by no adapter, and a file with no manifest above it inside
        // the repo — the two skip reasons, present on purpose.
        let readme = write("README.md", "# demo\n");

        let (loaded, first_party, skipped) =
            load_repo(root, &[a, b, readme.clone(), manifest], &[root.join("Cargo.toml")]);

        assert_eq!(loaded.len(), 2, "the two rust files");
        assert_eq!(
            first_party,
            packages(&["demo"]),
            "the package comes from the manifest the scan found, not from a path"
        );
        assert_eq!(
            loaded.iter().map(|l| l.module.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"],
            "the module rule is the language's, applied against the manifest's directory"
        );
        assert!(
            skipped.iter().any(|(p, why)| p.ends_with("README.md") && *why == Skipped::Unclaimed),
            "the markdown file is skipped as unclaimed: {skipped:?}"
        );

        let placed: Vec<Placed<'_>> = loaded.iter().map(Loaded::placed).collect();
        let indexed = index_repo(&placed, &first_party);

        let resolved: BTreeSet<String> = indexed
            .iter()
            .flat_map(|f| f.references.iter())
            .filter_map(|r| match &r.target {
                Resolution::Resolved { fqn, .. } => Some(fqn.to_string()),
                _ => None,
            })
            .collect();
        assert!(
            resolved.contains("rust·demo·a·Widget·wide·item"),
            "b.rs's call reaches the method a.rs declares, with the package the \
             manifest names: {resolved:?}"
        );
    }

    /// What ONE file mints, on its own, with no knowledge of any other file.
    #[test]
    #[ignore = "a printout, not an assertion"]
    fn show_the_minted_nodes_and_edges() {
        let b = Placed {
            path: "src/b.rs",
            package: "demo",
            module: "b",
            text: "use crate::a::Widget;\n\
                   pub struct Report { pub total: u32 }\n\
                   impl Report {\n\
                     pub fn build(w: &Widget) -> u32 { w.wide() }\n\
                   }\n",
        };
        let out = index_repo(&[b], &["demo".to_string()].into_iter().collect());
        let facts = &out[0];

        println!(
            "\n╔═ FILE {} ═ package={} module={} lang={}",
            facts.path,
            facts.package,
            facts.module,
            facts.language.as_str()
        );
        println!("╠═ NODES (declared by this file) ─────────────────────────────");
        for s in &facts.symbols {
            println!("║  {:<42} {:?}", s.fqn.to_string(), s.kind);
        }
        println!("╠═ NODES (referenced, not declared here → STUB) ──────────────");
        for r in &facts.references {
            if let crate::indexer::facts::Resolution::Resolved { fqn, .. } = &r.target {
                if !facts.symbols.iter().any(|s| s.fqn == *fqn) {
                    println!("║  {:<42} stub", fqn.to_string());
                }
            }
        }
        println!("╠═ EDGES ─────────────────────────────────────────────────────");
        for rel in &facts.relations {
            let parent = match &rel.parent {
                crate::indexer::facts::Resolution::Resolved { fqn, .. } => fqn.to_string(),
                _ => "?".into(),
            };
            println!("║  {:?}  {} → {}", rel.kind, parent, rel.child.as_str());
        }
        for r in &facts.references {
            match &r.target {
                crate::indexer::facts::Resolution::Resolved { fqn, via } => println!(
                    "║  {:?}  {} → {}   [resolved via {:?}]",
                    r.kind,
                    r.from.as_str(),
                    fqn.to_string(),
                    via
                ),
                crate::indexer::facts::Resolution::Unresolved { reason, evidence } => println!(
                    "║  {:?}  {} → ??   [HANGING: {:?}, saw {:?}]",
                    r.kind,
                    r.from.as_str(),
                    reason,
                    evidence.name
                ),
            }
        }
        println!("╚═════════════════════════════════════════════════════════════");
    }

    /// **ONE FILE, ON ITS OWN, MINTS THE STUB FOR AN IMPORTED TYPE'S MEMBER.**
    ///
    /// This is the property that removes the barrier. `use crate::a::Widget`
    /// states where `Widget` lives, so a call on a `Widget` can be named
    /// `a·Widget·wide` from this file alone — and when `a.rs` is indexed it
    /// declares that same identity and the stub is promoted.
    ///
    /// The file already proves it has the information: the TYPE reference on
    /// the parameter resolves `ThroughAnImport` on the line above. Only the
    /// CALL failed, because `home_of` consulted the file's declarations and
    /// then a global table, never the imports it had just used.
    #[test]
    fn one_file_names_the_member_of_a_type_it_imported() {
        let b = Placed {
            path: "src/b.rs",
            package: "demo",
            module: "b",
            text: "use crate::a::Widget;\n\
                   pub fn build(w: &Widget) -> u32 { w.wide() }\n",
        };
        // NO other file, so no barrier could possibly help.
        let out = index_repo(&[b], &packages(&["demo"]));
        let call = out[0]
            .references
            .iter()
            .find(|r| r.kind == RefKind::Calls)
            .expect("the body calls something");
        // NAMED, not yet proven. The walk now mints the identity from the
        // import alone — which is the half that removes the barrier. Promoting
        // that name to an EDGE is the ladder's second gate
        // (`declared_by_its_type`), which today requires the scan to have
        // already seen the declaration; under the stub-and-heal model that
        // existence question belongs to the persistence layer. Relaxing it
        // blanket-style was MEASURED at +12,360 resolved references over this
        // repository and broke 13 tests, `a_bare_name_matching_another_files_
        // declaration_is_not_proof_of_anything` among them — so the relaxation
        // has to distinguish "named from this file's own text" from "guessed",
        // and that is the next increment, not this one.
        let Resolution::Unresolved { evidence, .. } = &call.target else {
            panic!("expected a named-but-unproven target, got {:?}", call.target)
        };
        assert!(
            evidence.saw.iter().any(|o| matches!(
                o,
                Observation::Candidate(f) if f.to_string() == "rust·demo·a·Widget·wide·item"
            )),
            "the member was not named from the import alone: {:?}",
            evidence.saw
        );
    }

    fn packages(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    /// **THE PROPERTY THE BARRIER EXISTS FOR, AND THE REASON THIS MODULE IS NOT
    /// A LOOP OVER `read`.**
    ///
    /// `b.rs` calls a method on a type `a.rs` declares. The member identity
    /// carries the module the TYPE lives in, so `b` has to know `Widget` is in
    /// `a` — which is not in `b`'s text and not derivable from it. Without the
    /// barrier `b` mints `rust·p·b·Widget·wide·item` and `a` declares
    /// `rust·p·a·Widget·wide·item`: two identities, no edge, and no error
    /// anywhere.
    ///
    /// The mutation that must break this: hand pass two `TypeHomes::unknown()`.
    #[test]
    fn a_call_reaches_a_type_declared_in_another_file() {
        let a = Placed {
            path: "src/a.rs",
            package: "p",
            module: "a",
            text: "pub struct Widget { pub w: u32 }\n\
                   impl Widget { pub fn wide(&self) -> u32 { self.w } }\n",
        };
        let b = Placed {
            path: "src/b.rs",
            package: "p",
            module: "b",
            text: "use crate::a::Widget;\n\
                   pub fn go(x: &Widget) -> u32 { x.wide() }\n",
        };

        let indexed = index_repo(&[a, b], &packages(&["p"]));
        assert_eq!(indexed.len(), 2, "both files read");

        let call = indexed[1]
            .references
            .iter()
            .find(|r| r.kind == RefKind::Calls)
            .expect("b.rs calls something");
        match &call.target {
            Resolution::Resolved { fqn, .. } => assert_eq!(
                fqn.to_string(),
                "rust·p·a·Widget·wide·item",
                "the member is filed under the module its TYPE lives in, not the caller's"
            ),
            other => panic!("the cross-file call did not resolve: {other:?}"),
        }
    }

    /// The same two files in the OPPOSITE order produce the same facts (R6).
    ///
    /// This is what the barrier buys and it cannot be checked on one file: if
    /// pass two learned homes incrementally, indexing `b` before `a` would leave
    /// `b`'s call unresolved and the graph would depend on directory order.
    #[test]
    fn the_order_the_files_arrive_in_changes_nothing() {
        let a = Placed {
            path: "src/a.rs",
            package: "p",
            module: "a",
            text: "pub struct Widget { pub w: u32 }\n\
                   impl Widget { pub fn wide(&self) -> u32 { self.w } }\n",
        };
        let b = Placed {
            path: "src/b.rs",
            package: "p",
            module: "b",
            text: "use crate::a::Widget;\n\
                   pub fn go(x: &Widget) -> u32 { x.wide() }\n",
        };

        let forward = index_repo(&[a, b], &packages(&["p"]));
        let backward = index_repo(&[b, a], &packages(&["p"]));

        let targets = |facts: &[FileFacts]| -> BTreeSet<String> {
            facts
                .iter()
                .flat_map(|f| f.references.iter())
                .filter_map(|r| match &r.target {
                    Resolution::Resolved { fqn, .. } => Some(fqn.to_string()),
                    _ => None,
                })
                .collect()
        };
        let resolved = targets(&forward);
        // NOT VACUOUS: two empty sets are equal, so the equality below proves
        // nothing unless something actually resolved. It is the cross-file
        // member that matters here — the one the barrier exists for.
        assert!(
            resolved.contains("rust·p·a·Widget·wide·item"),
            "nothing cross-file resolved, so the comparison would hold on two \
             empty sets: {resolved:?}"
        );
        assert_eq!(
            resolved,
            targets(&backward),
            "the resolved set must not depend on which file was handed over first"
        );
    }

    /// A file no adapter claims is ABSENT, not an empty entry. An empty
    /// `FileFacts` reaching the writer is a file that reports zero declarations
    /// and zero uses, which is indistinguishable from one genuinely empty.
    #[test]
    fn a_file_no_adapter_claims_produces_no_facts_rather_than_empty_ones() {
        let unreadable =
            Placed { path: "README.cobol", package: "p", module: "", text: "IDENTIFICATION" };
        let rust = Placed {
            path: "src/a.rs",
            package: "p",
            module: "a",
            text: "pub fn f() -> u32 { 1 }\n",
        };
        let indexed = index_repo(&[unreadable, rust], &packages(&["p"]));
        assert_eq!(indexed.len(), 1, "only the file an adapter claims");
        assert_eq!(indexed[0].path, "src/a.rs");
    }

    /// Several languages in one repository are indexed together and keep their
    /// own grammars — the ladder is asked for the adapter of each file's OWN
    /// language, not of the first one seen.
    ///
    /// The mutation that must break this: resolve every file with
    /// `adapter_for(Language::Rust).grammar()`.
    #[test]
    fn one_repository_may_hold_several_languages() {
        let rs = Placed {
            path: "src/a.rs",
            package: "p",
            module: "a",
            text: "pub fn f() -> u32 { 1 }\n",
        };
        let py = Placed {
            path: "pkg/b.py",
            package: "p",
            module: "pkg.b",
            text: "def g():\n    return 1\n",
        };
        let ts = Placed {
            path: "src/c.ts",
            package: "p",
            module: "c",
            text: "export function h(): number { return 1; }\n",
        };
        let indexed = index_repo(&[rs, py, ts], &packages(&["p"]));
        assert_eq!(indexed.len(), 3);

        let languages: Vec<&str> = indexed.iter().map(|f| f.language.as_str()).collect();
        assert_eq!(languages, vec!["rust", "python", "typescript"]);
    }

    /// **EACH FILE IS RESOLVED WITH ITS OWN LANGUAGE'S GRAMMAR.**
    ///
    /// Separate from the test above, which proved nothing about this: a file's
    /// `language` is stamped by the ADAPTER during the read, so it stays right
    /// even if every file is then handed the wrong grammar. I wrote that test
    /// claiming this mutation would break it, probed it, and it survived.
    ///
    /// The discriminator is a PATH ROOT, which the two grammars spell
    /// differently and neither shares: python roots a relative import at `.`,
    /// rust at `self`/`super`/`crate`. `from . import a` inside `pkg/b.py`
    /// names the sibling module `pkg.a` — and under rust's grammar `.` roots
    /// nothing, so the import cannot be placed at all.
    ///
    /// The mutation that must break this: resolve every file with
    /// `adapter_for(Language::Rust).grammar()`.
    #[test]
    fn a_file_is_resolved_with_the_grammar_of_its_own_language() {
        let a = Placed {
            path: "pkg/a.py",
            package: "p",
            module: "pkg.a",
            text: "def thing():\n    return 1\n",
        };
        let b =
            Placed { path: "pkg/b.py", package: "p", module: "pkg.b", text: "from . import a\n" };
        let indexed = index_repo(&[a, b], &packages(&["p"]));

        let entered = indexed[1]
            .references
            .iter()
            .find(|r| r.kind == RefKind::Imports)
            .expect("b.py imports a sibling");
        match &entered.target {
            Resolution::Resolved { fqn, .. } => assert_eq!(
                fqn.to_string(),
                "python·p·pkg·mod",
                "`.` names the CONTAINING PACKAGE — the module the specifier \
                 reduces to. The member `a` is a separate fact, exactly as `C` \
                 is in `from a.b import C`, whose edge names `a.b`."
            ),
            other => panic!("the relative import did not resolve: {other:?}"),
        }
    }

    /// Every file that went in comes out — the same "total in, total out"
    /// contract `resolve` keeps for references, one level up. A file silently
    /// dropped between the two passes is a file whose declarations never reach
    /// the graph, and nothing downstream would report it.
    #[test]
    fn every_readable_file_survives_both_passes() {
        let files: Vec<Placed<'_>> = vec![
            Placed { path: "src/a.rs", package: "p", module: "a", text: "pub fn a() {}\n" },
            Placed { path: "src/b.rs", package: "p", module: "b", text: "pub fn b() {}\n" },
            Placed { path: "src/c.rs", package: "p", module: "c", text: "pub fn c() {}\n" },
        ];
        let indexed = index_repo(&files, &packages(&["p"]));
        let paths: Vec<&str> = indexed.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, vec!["src/a.rs", "src/b.rs", "src/c.rs"]);
    }
}

#[cfg(test)]
mod corpus {
    use super::*;
    use crate::indexer::facts::{Reason, Resolution};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    /// **THE WHOLE OF STAGE 4, OVER THIS REPOSITORY.**
    ///
    /// The last thing that can be checked before a database is involved: load,
    /// place, walk twice, resolve — at corpus scale, on a tree nobody wrote as
    /// a fixture. What it is looking for is the class of defect a fixture
    /// cannot have: a package the manifests name that no file lands in, a
    /// module two files claim, a resolved edge pointing at the file it came
    /// from.
    ///
    ///     cargo test -p senseid --bin senseid -- --ignored --nocapture index::corpus
    #[test]
    #[ignore = "walks this repository"]
    fn this_repository_loads_places_and_resolves() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("the workspace root")
            .to_path_buf();

        let mut files = Vec::new();
        let mut manifests = Vec::new();
        for entry in ignore::WalkBuilder::new(&root).build().filter_map(Result::ok) {
            let path = entry.into_path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
            if crate::adapters::manifest::manifest_adapter_for_filename(name).is_some() {
                manifests.push(path);
            } else if path.is_file() {
                files.push(path);
            }
        }
        assert!(manifests.len() > 3, "found {} manifests", manifests.len());

        let (loaded, first_party, skipped) = load_repo(&root, &files, &manifests);
        let placed: Vec<Placed<'_>> = loaded.iter().map(Loaded::placed).collect();
        let indexed = index_repo(&placed, &first_party);

        let mut why: BTreeMap<String, usize> = BTreeMap::new();
        for (_, reason) in &skipped {
            *why.entry(format!("{reason:?}")).or_default() += 1;
        }
        let (mut resolved, mut unresolved) = (0usize, 0usize);
        let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
        let mut self_edges: Vec<String> = Vec::new();
        for facts in &indexed {
            let own = super::super::lang::adapter_for(facts.language)
                .file_fqn(&facts.package, &facts.module, &facts.path)
                .map(|f| f.to_string());
            for reference in &facts.references {
                match &reference.target {
                    Resolution::Resolved { fqn, .. } => {
                        resolved += 1;
                        // A FILE IMPORTING ITSELF is the shape that exposed
                        // python's relative-import defect, and it is invisible
                        // in any count of misses: it resolves.
                        if reference.kind == crate::indexer::facts::RefKind::Imports
                            && own.as_deref() == Ok(fqn.to_string().as_str())
                        {
                            let spec = facts
                                .imports
                                .iter()
                                .find(|i| i.at == reference.at)
                                .map(|i| format!("{:?} binds {:?}", i.path, i.binds))
                                .unwrap_or_default();
                            self_edges.push(format!("{}: {spec}", facts.path));
                        }
                    }
                    Resolution::Unresolved { reason, .. } => {
                        unresolved += 1;
                        *reasons.entry(format!("{reason:?}")).or_default() += 1;
                        assert_ne!(
                            *reason,
                            Reason::Unplaced,
                            "{}: `Unplaced` means the ladder never ran on this reference",
                            facts.path
                        );
                    }
                }
            }
        }

        let mut top: Vec<(&String, &usize)> = reasons.iter().collect();
        top.sort_by(|a, b| b.1.cmp(a.1));
        println!("\n── {} ──", root.display());
        println!("  files {}  loaded {}  skipped {why:?}", files.len(), loaded.len());
        println!("  packages {}  indexed {}", first_party.len(), indexed.len());
        println!("  references: {resolved} resolved, {unresolved} unresolved");

        for (reason, count) in top.iter().take(6) {
            println!("  {count:>7}  {reason}");
        }

        // **A SELF-EDGE IS EXPECTED IN EXACTLY ONE SHAPE, AND SUSPECT IN ANY
        // OTHER.**
        //
        // `use super::*` inside an inline `#[cfg(test)] mod tests` names the
        // enclosing file's own module, so the edge is RIGHT — 309 of them here,
        // every one that shape. It is still a self-loop, which carries no
        // information for a traversal; whether the writer should drop one is a
        // separate question and is recorded rather than decided here.
        //
        // The assertion is on the SHAPE, not the count, because this detector
        // is what caught python's relative imports resolving to the importing
        // file — a wrong edge that is invisible to every count of misses, since
        // it resolves. Asserting `is_empty()` would have meant deleting the
        // detector; asserting a count would rot on the next test module added.
        let unexplained: Vec<&String> =
            self_edges.iter().filter(|e| !e.contains("\"super::*\" binds Glob")).collect();
        println!("  self-edges {} (all `use super::*`)", self_edges.len());
        assert!(
            unexplained.is_empty(),
            "{} import(s) resolved to the importing file for a reason other than an \
             inline test module's `use super::*`:\n  {}",
            unexplained.len(),
            unexplained.iter().take(5).map(|s| s.as_str()).collect::<Vec<_>>().join("\n  ")
        );
        // A MODULE IS ONE FILE'S. Two files minting one means every declaration
        // in the second overwrites the first's, and which wins is scan order.
        let mut claimed: BTreeMap<(String, String), Vec<&str>> = BTreeMap::new();
        for facts in &indexed {
            claimed
                .entry((facts.package.clone(), facts.module.clone()))
                .or_default()
                .push(facts.path.as_str());
        }
        let collided: Vec<_> =
            claimed.iter().filter(|((_, m), f)| f.len() > 1 && !m.is_empty()).collect();
        assert!(
            collided.is_empty(),
            "{} module path(s) claimed by more than one file: {:?}",
            collided.len(),
            collided.iter().take(4).collect::<Vec<_>>()
        );
        assert!(resolved > 0, "nothing resolved at all, so this proved nothing");
    }
}

#[cfg(test)]
mod barrier_necessity {
    use super::*;
    use crate::indexer::facts::{Binding, Resolution, SymbolKind};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    /// **IS THE BARRIER SUPPLYING ANYTHING THE FILE DOES NOT ALREADY STATE?**
    ///
    /// The type barrier exists so a member reference can carry the module its
    /// TYPE lives in. The claim under test is that it is redundant: every file
    /// that uses a type either declares it, imports it, or writes its path
    /// inline, so the file is self-describing and a global table is supplying
    /// an answer already present in the text.
    ///
    /// MUST run over the whole corpus, not a fixture — the whole question is
    /// about references that cross files, and a single file has none.
    ///
    /// Classifies every RESOLVED member reference by where the file could have
    /// learned the type's home on its own:
    ///
    /// - `local`    — the type is declared in this same file
    /// - `imported` — an import in this file binds that exact name
    /// - `glob`     — the file has a wildcard import, so the name MIGHT come
    ///                through it; can only be confirmed with the target module
    /// - `gap`      — none of the above. ONLY a global table could have
    ///                supplied this, and it is the number that decides whether
    ///                the barrier can be deleted.
    ///
    ///     cargo test -p senseid --bin senseid -- --ignored --nocapture barrier_necessity
    #[test]
    #[ignore = "walks this repository"]
    fn every_resolved_member_is_traced_to_what_its_own_file_states() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("the workspace root")
            .to_path_buf();

        let mut files = Vec::new();
        let mut manifests = Vec::new();
        for entry in ignore::WalkBuilder::new(&root).build().filter_map(Result::ok) {
            let path = entry.into_path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
            if crate::adapters::manifest::manifest_adapter_for_filename(name).is_some() {
                manifests.push(path);
            } else if path.is_file() {
                files.push(path);
            }
        }
        let (loaded, first_party, _) = load_repo(&root, &files, &manifests);
        let placed: Vec<Placed<'_>> = loaded.iter().map(Loaded::placed).collect();
        let indexed = index_repo(&placed, &first_party);

        // Every type name the corpus declares anywhere, so a member fqn can be
        // decomposed without guessing which segment is the type.
        let mut type_names: BTreeSet<&str> = BTreeSet::new();
        for facts in &indexed {
            for symbol in &facts.symbols {
                if matches!(
                    symbol.kind,
                    SymbolKind::Struct
                        | SymbolKind::Enum
                        | SymbolKind::Trait
                        | SymbolKind::Class
                        | SymbolKind::Interface
                ) {
                    type_names.insert(symbol.name.as_str());
                }
            }
        }

        // The file's TEXT, because a file can name a type without importing it:
        // `crate::db::pg_store::PgStore::connect(..)` states the module inline
        // and needs no import at all. Counting only imports read that as a gap.
        let text_of: BTreeMap<&str, &str> =
            loaded.iter().map(|l| (l.path.as_str(), l.text.as_str())).collect();
        let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
        let mut gaps: BTreeMap<String, usize> = BTreeMap::new();
        for facts in &indexed {
            // What this file states about names, on its own.
            let declares: BTreeSet<&str> = facts.symbols.iter().map(|s| s.name.as_str()).collect();
            let mut binds: BTreeSet<&str> = BTreeSet::new();
            let mut has_glob = false;
            for import in &facts.imports {
                match &import.binds {
                    Binding::Name(n) => {
                        binds.insert(n.as_str());
                        // A path names every segment it passes through, so the
                        // LAST is the name it brings in even when the binding
                        // records the head.
                        if let Some(last) = import.path.rsplit(['.', ':', '/']).next() {
                            binds.insert(last);
                        }
                    }
                    Binding::MemberOf { local, member } => {
                        binds.insert(local.as_str());
                        binds.insert(member.as_str());
                    }
                    Binding::Glob => has_glob = true,
                }
            }

            for reference in &facts.references {
                let Resolution::Resolved { fqn, .. } = &reference.target else { continue };
                let encoded = fqn.to_string();
                let Ok(parsed) = crate::indexer::fqn::parse(&encoded) else { continue };
                // A MEMBER form ends in <Type> <member>; the type is the
                // second-from-last tail segment and must be a type the corpus
                // declares, which is what keeps a module segment from being
                // read as one.
                if parsed.tail.len() < 2 {
                    continue;
                }
                let ty = parsed.tail[parsed.tail.len() - 2];
                if !type_names.contains(ty) {
                    continue;
                }
                let bucket = if declares.contains(ty) {
                    "local"
                } else if binds.contains(ty) {
                    "imported"
                } else if has_glob {
                    "glob"
                } else if text_of.get(facts.path.as_str()).is_some_and(|t| t.contains(ty)) {
                    // Written somewhere in the file — an inline qualified path,
                    // a turbofish, an annotation. The file states it.
                    "spelled"
                } else {
                    *gaps.entry(format!("{ty} in {}", facts.path)).or_default() += 1;
                    "gap: never named in the file"
                };
                *tally.entry(bucket).or_default() += 1;
            }
        }

        let total: usize = tally.values().sum();
        println!("\n── resolved member references, by what the FILE states ──");
        for (bucket, count) in &tally {
            println!("  {count:>7}  {bucket}  ({:.1}%)", *count as f64 * 100.0 / total as f64);
        }
        println!("  {total:>7}  total");
        let mut worst: Vec<(&String, &usize)> = gaps.iter().collect();
        worst.sort_by(|a, b| b.1.cmp(a.1));
        println!("  distinct gap sites: {}", gaps.len());
        for (what, count) in worst.iter().take(200) {
            println!("    {count:>5}  {what}");
        }
        assert!(total > 0, "no member references classified, so this proved nothing");
    }
}
