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
    let first = read_all(files, &TypeHomes::unknown());

    // THE BARRIER. Where each type lives, across the whole repository.
    let homes = TypeHomes::of(
        files.iter().zip(&first).flat_map(|(p, f)| f.symbols.iter().map(move |s| (p.package, s))),
    );

    // PASS TWO, complete, before anything is placed. Every table below is taken
    // off THIS pass and not off `first`: a member's identity carries the module
    // its type lives in, so the pre-barrier pass spells those members
    // differently and a set taken from it would match nothing.
    let anchored = read_all(files, &homes);

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

    anchored
        .into_iter()
        .map(|facts| {
            let grammar = lang::adapter_for(facts.language).grammar();
            resolve::resolve(facts, grammar, &world)
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::indexer::facts::{RefKind, Resolution};

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
