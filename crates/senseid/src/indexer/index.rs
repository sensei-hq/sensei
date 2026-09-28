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
//! - **Where a type lives.** `store.upsert()` mints a MEMBER identity whose
//!   module segment is the one the TYPE is declared in — not the one the call
//!   is written in. Rust puts `impl` blocks anywhere; this repo has 24 for
//!   `PgStore` alone. Spelled in prose rather than as an example fqn on
//!   purpose: this file is read by
//!   `fqn::tests::no_fqn_is_built_by_string_formatting_outside_this_file`,
//!   which forbids the separator character anywhere outside `fqn.rs` —
//!   including a doc comment, because a hand-written fqn is exactly how the
//!   definition and reference sides come to disagree.
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

use super::facts::{FileFacts, Language};
use super::lang::{self, Source, TypeHomes};
use super::resolve::{self, World, member_names_of, members_declared_by, returns_declared_by};

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
/// absent from the result rather than represented by an empty one. An empty
/// [`FileFacts`] reaching the writer is a file reporting zero declarations and
/// zero uses, which is indistinguishable from one genuinely empty — and a
/// [`FileFacts`] has nowhere to put the difference.
///
/// [`index_file`] is the shape that does: it answers with a REASON, so empty
/// and absent stop being the same word. This function keeps the drop because
/// its return type is still a bare `Vec<FileFacts>`; stage 12 is where the
/// callers move across.
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
    let returns = returns_declared_by(&anchored);
    // EMPTY, deliberately. `World::scanned` means "files read so far", and
    // reading absence as externality is the order dependence spec §2 exists to
    // prevent — externality comes from the import, never from absence.
    let scanned = BTreeSet::new();
    let world = World {
        first_party,
        first_party_members: &first_party_members,
        declared_members: &declared_members,
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
///
/// A file [`read_one`] refuses is ABSENT from the result, never present with
/// empty vectors: an empty `FileFacts` is byte-identical to a file that
/// genuinely declares nothing, and reconcile prunes that file's whole graph on
/// the shape (R4). What is dropped HERE is a named [`Skipped`] and not a
/// `Result` from a read — the refusal has already been given a reason by the
/// time it reaches this line. The caller reads the count as the length
/// difference, which is the contract [`index_repo`]'s own doc states.
fn read_all(files: &[Placed<'_>], types: &TypeHomes) -> Vec<FileFacts> {
    files.iter().filter_map(|placed| read_one(placed, types).ok()).collect()
}

/// One file. `Err` when no adapter claims the extension or the grammar rejects
/// the text — a REASON the caller acts on, never a blank stand-in.
///
/// A `Result` and not an `Option` because the adapter's own `read` returns one.
/// Collapsing that `Err` to a `None` at the call site is the swallow
/// `reconcile::tests::nothing_but_a_language_read_can_produce_file_facts`
/// forbids: an `Err` from `read` means the file keeps the graph it had, and a
/// `None` cannot say that. [`Skipped::Rejected`] carries the `ReadError`
/// verbatim, so nothing about the refusal is invented and nothing is lost.
///
/// [`Skipped`] rather than a second enum saying the same thing — it is already
/// this module's word for "a file the scan found produced nothing to index",
/// and [`load_repo`] fills the other three variants.
fn read_one(placed: &Placed<'_>, types: &TypeHomes) -> Result<FileFacts, Skipped> {
    let adapter = adapter_for_path(placed.path).ok_or(Skipped::Unclaimed)?;
    let source = Source {
        package: placed.package,
        module: placed.module,
        path: placed.path,
        text: placed.text,
    };
    adapter.read(&source, types).map_err(Skipped::Rejected)
}

/// WHO READS THIS PATH, decided from its extension and from nothing else.
///
/// One place, shared by [`read_one`] and [`index_file`], because the answer
/// decides two different things — whether there is anything to read, and which
/// LANGUAGE the result is stamped with — and two copies is how those two come
/// to disagree about a file.
///
/// Text work: it opens nothing. That is what lets a [`Mode::Delete`] still be
/// stamped with a language without reading the file being deleted.
fn adapter_for_path(path: &str) -> Option<&'static dyn lang::LanguageAdapter> {
    let (_, ext) = path.rsplit_once('.')?;
    lang::adapter_for_ext(&format!(".{ext}"))
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
///
/// NOT `Copy`, and not ordered, because [`Skipped::Rejected`] carries the
/// adapter's own [`lang::ReadError`]. Dropping that payload to keep the derives
/// would throw away the only description of WHY the grammar refused the file,
/// which is the part a reader of the skip list needs.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// An adapter claims it and the grammar refused the text. The `Err` is
    /// carried rather than dropped at the read: a file that failed to parse
    /// keeps the graph it had, and a caller that could not tell it from a file
    /// with no declarations would prune that graph (R4).
    Rejected(lang::ReadError),
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

// ── stage 11: ONE FILE IN, NODES AND EDGES OUT ───────────────────────────────

/// What [`index_file`] is being asked to do about one file.
///
/// ONE code path for an initial scan and an incremental one; only this differs.
/// An initial scan is every file at [`Mode::New`], in whatever order the walk
/// yields them.
///
/// **THE FOURTH VOCABULARY, AND WHAT IT MAPS ONTO.** Three others already
/// describe change in this indexer, and leaving them unrelated is how two of
/// them come to disagree about what a removal is:
///
/// | [`super::structure::ChangeKind`] | [`super::incremental::Retrigger`] | this | [`super::reconcile::Stated`] |
/// |---|---|---|---|
/// | `Added` | `Parse` | [`Mode::New`] | `Parsed` |
/// | `ContentChanged` | `Parse` | [`Mode::Update`] | `Parsed` |
/// | `TouchedOnly`, `Unchanged` | — (`needs_parse()` is false) | — not called | — |
/// | `StructurePlan::removed`, a PATH | — | [`Mode::Delete`] | `Gone(Located)` |
///
/// The bottom row is the one worth reading twice. `ChangeKind` deliberately has
/// NO `Removed` variant — it classifies an observed `(old, new)` pair, and a
/// removed file has no `new` to classify, so the variant had no constructor and
/// was deleted. `Mode` may have `Delete` because it is not a classification: it
/// is an INSTRUCTION, and `StructurePlan::removed` is the list of paths a caller
/// issues it for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Not seen before. Read it; stage 12 inserts.
    New,
    /// Seen, and its content changed. Read it; stage 12 deletes this file's rows
    /// whose fqn is absent from the result, then inserts.
    Update,
    /// Gone. NOTHING is read, and the result is empty — see [`index_file`].
    Delete,
}

/// One file, and the two things it cannot know about itself, plus what to do.
///
/// The sibling of [`Placed`], one field wider. Carries TEXT rather than a path
/// to open, for the reason [`Source`] does: it keeps every decision here
/// testable on string literals, and `lang::tests::no_adapter_reads_the_filesystem`
/// is what keeps it that way.
///
/// `language` is NOT here. The driver resolves it from the extension and reports
/// what it used — a caller that could name a language would be a caller that
/// could name the wrong one (S1).
#[derive(Debug, Clone, Copy)]
pub struct FileInput<'a> {
    /// The repository this file belongs to, as the graph records it.
    pub repo: &'a str,
    /// Path as the graph records it.
    pub path: &'a str,
    pub mode: Mode,
    pub package: &'a str,
    /// Package-relative module path, from [`super::placement::placement_of`].
    pub module: &'a str,
    pub text: &'a str,
    /// **WHAT THE SCAN KNOWS, HANDED IN.** The ladder cannot run without it and
    /// one file cannot derive it: four of [`World`]'s five fields are repo-wide
    /// artifacts of a completed pass.
    ///
    /// TOLD, exactly as `package` and `module` are told, and for the same
    /// reason — the file scan never climbs and never reaches. The repo scan
    /// computes this once and hands it down.
    ///
    /// **WITHOUT THE LADDER THE REFERENCES ARE NOT MERELY UNRESOLVED, THEY ARE
    /// `Reason::Unplaced`** — the one reason documented as "must be EMPTY once
    /// resolution is done", which `index::corpus` asserts on. An `Unplaced`
    /// reference carries the identities the walk MINTED as evidence and no
    /// `Resolved` fqn, so persistence has nothing to point an edge at:
    /// `TargetKey::Named` becomes `TargetRef::Unresolvable`, and the edge is
    /// written with a name and no target. That is the whole reason a file
    /// indexed alone produced an unlinked graph — not a missing relink in the
    /// writer, which already stubs a `Proven` target and reuses its id
    /// (`OnMiss::CreateStub`).
    pub world: &'a World<'a>,
}

/// What one file's read produced — or what it produced instead.
///
/// SCAFFOLD, and the next edit is what finishes it: the empty variant carries no
/// reason yet, which is precisely the property I8 falsifies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Indexed {
    /// One parse (R1), and everything that parse saw.
    Read(FileFacts),
    /// No nodes and no edges, and WHY. Never an absence, and never an error.
    Empty(Empty),
}

/// Why one call to [`index_file`] has no nodes and no edges (S2, §7).
///
/// **A REASON, AND NOT A SECOND VOCABULARY.** [`Skipped`] already says why a
/// file the scan found produced nothing to index, and [`load_repo`] already
/// emits all three of its variants, so it is carried through verbatim rather
/// than restated. A caller counting empties counts ONE set of codes.
///
/// [`index_file`] can itself only ever mint [`Skipped::Unclaimed`] — placement
/// and byte-reading are the IO half's, and that is exactly the point: the
/// vocabulary is shared BECAUSE the two halves produce different parts of it.
///
/// The two reasons below it are genuinely not skips, and collapsing them into
/// one "nothing here" would be a live defect rather than a tidiness question:
/// [`super::reconcile`] applies a removal as `reconcile(F, ∅)`, which DEMOTES
/// every node the file claimed, and a caller that could not tell
/// [`Empty::Deleted`] from [`Empty::Unread`] would apply that to a file whose
/// parse merely failed. `reconcile`'s own docs say so: on `Err` the caller does
/// not call it at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Empty {
    /// [`Mode::Delete`]: the text was NOT read. Persistence's job for this file
    /// is to remove rows, and a node set read on the way to removing them is
    /// work whose only possible use is to disagree with them.
    Deleted,
    /// The scan's own reason, unchanged — including
    /// [`Skipped::Rejected`], which CARRIES the `ReadError` rather than
    /// swallowing it (§7): the caller COUNTS a grammar rejection, and it is the
    /// one empty a caller must not hand to reconcile.
    ///
    /// ONE variant, not two. A second variant holding a `ReadError` would be a
    /// second way to say one thing, and whichever of the two a reader checks
    /// would decide whether they see rejections at all — the defect
    /// `structure::ChangeKind` records having removed.
    Skipped(Skipped),
}

impl Empty {
    /// The stable label this reason is written and read under — one labeling,
    /// for the reason [`super::facts::Rung::as_label`] gives.
    pub fn as_label(&self) -> &'static str {
        match self {
            Self::Deleted => "deleted",
            Self::Skipped(Skipped::Unclaimed) => "unclaimed",
            Self::Skipped(Skipped::Unplaced) => "unplaced",
            Self::Skipped(Skipped::Unreadable) => "unreadable",
            Self::Skipped(Skipped::Rejected(_)) => "rejected",
        }
    }
}

/// One file, indexed: what the driver was told, what it resolved, and what the
/// file states.
///
/// The same shape for all three modes, so persistence has one input and no
/// special case.
///
/// It carries [`FileFacts`] rather than the flat `nodes[]`/`edges[]` of
/// `docs/spec/indexer/11-file-index.md` §4 ON PURPOSE. One spec `Edge` is three
/// current types — [`super::facts::Relation`], [`super::facts::Reference`] and
/// [`super::facts::Import`] — and `persist::edge_rows_of` is already the
/// flattening between them, guarded by
/// `every_conversion_between_a_fact_and_a_row_names_every_field`. Reshaping here
/// would rewrite that flattening before anything in this stage has pinned the
/// new shape. Stage 12 owns persistence, so stage 12 owns the shape persistence
/// wants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileIndex {
    pub mode: Mode,
    pub repo: String,
    pub file: String,
    /// What the driver RESOLVED from the extension, never what a caller named.
    /// `None` where no adapter claims it.
    pub language: Option<Language>,
    pub package: String,
    pub module: String,
    pub indexed: Indexed,
}

impl FileIndex {
    /// What the file states, when it was read at all.
    pub fn facts(&self) -> Option<&FileFacts> {
        match &self.indexed {
            Indexed::Read(facts) => Some(facts),
            Indexed::Empty(_) => None,
        }
    }

    /// Why this file has no nodes and no edges, or `None` where it has some.
    ///
    /// The whole reason, [`ReadError`] payload included, because a caller that
    /// COUNTS a rejected grammar (§7) needs to say which rejection.
    pub fn empty(&self) -> Option<&Empty> {
        match &self.indexed {
            Indexed::Read(_) => None,
            Indexed::Empty(why) => Some(why),
        }
    }

    /// The stable label of [`FileIndex::empty`].
    ///
    /// A LABEL, the way [`super::facts::Rung::as_label`] is one: it is what a
    /// report prints, and it is what an assertion can read before a variant
    /// exists.
    pub fn why_empty(&self) -> Option<&'static str> {
        self.empty().map(Empty::as_label)
    }
}

/// **INDEX ONE FILE, WITH NO KNOWLEDGE OF ANY OTHER FILE.** PURE.
///
/// The driver, and it adds no facts of its own (S3): everything in the result
/// comes from the adapter, from the file's own path, or from the `package` and
/// `module` it was handed.
///
/// It NEVER matches on a language (S1). It resolves an adapter from the
/// EXTENSION, calls it, and stamps the result with what it used — a caller that
/// could name a language would be a caller that could name the wrong one.
///
/// # All three modes return the same shape
///
/// An extension no adapter claims, a grammar rejection and a deletion are three
/// EMPTY results with three reasons — never an error, and never absence. Most
/// of a repository is not source (942 of 2,309 files here), so a driver that
/// errored on each of them would make the common case a failure path, and one
/// that returned nothing at all could not say which of the three it meant.
///
/// # `TypeHomes::unknown()`, and why that is the whole point
///
/// One file has no repo-wide table and must not be able to reach for one, so
/// `unknown()` is the only table it can honestly be handed. That the walk still
/// TAKES the parameter is what S5 removes next; passing `unknown()` here is the
/// demonstration that it answers nothing.
pub fn index_file(input: FileInput<'_>) -> FileIndex {
    let FileInput { repo, path, mode, package, module, text, world } = input;
    let stamp = |language: Option<Language>, indexed: Indexed| FileIndex {
        mode,
        repo: repo.to_string(),
        file: path.to_string(),
        language,
        package: package.to_string(),
        module: module.to_string(),
        indexed,
    };

    // WHO READS THIS FILE is the first question, and it is asked BEFORE the
    // mode. A deleted `README.md` never had a node to remove, so `Unclaimed` is
    // the truer answer than `Deleted` — and asking it first keeps "who reads
    // this" the driver's first question in every mode.
    let Some(adapter) = adapter_for_path(path) else {
        return stamp(None, Indexed::Empty(Empty::Skipped(Skipped::Unclaimed)));
    };
    let language = Some(adapter.language());

    if matches!(mode, Mode::Delete) {
        // THE TEXT IS NOT READ. Stage 12 deletes every row whose file is this
        // file, and a node set read on the way there has no use but to disagree
        // with them. The LANGUAGE is still stamped, off the extension, because
        // it cost nothing to know and a caller reconciling a removed file has
        // no facts to take one off.
        return stamp(language, Indexed::Empty(Empty::Deleted));
    }

    let source = Source { package, module, path, text };
    // NOT `.ok()`. §7 says a grammar rejection is a `ReadError` the caller
    // COUNTS, and it is also the one empty result a caller must NOT hand to
    // reconcile — `reconcile(F, nothing)` demotes every node the file claimed,
    // and applying that to a failed parse is the defect R10.3 exists for.
    match adapter.read(&source, &TypeHomes::unknown()) {
        // **THE LADDER RUNS HERE, AND IT DID NOT BEFORE.** The walk emits every
        // reference `Unplaced` with the identities it minted as evidence;
        // placing them is what turns an `Observation::Named(fqn)` into a
        // `Resolution::Resolved { fqn }` that persistence can point an edge at
        // (S7, R7).
        //
        // `TypeHomes::unknown()` above STAYS: the WALK may not reach for a
        // repo-wide table (S5). The LADDER may, because that is its job and the
        // world is handed to it rather than fetched.
        Ok(facts) => {
            let placed = super::resolve::resolve(facts, adapter.grammar(), world);
            stamp(language, Indexed::Read(placed))
        }
        Err(rejected) => {
            stamp(language, Indexed::Empty(Empty::Skipped(Skipped::Rejected(rejected))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::indexer::facts::{RefKind, Resolution};

    /// The sets a single-file index is TOLD, owned so a `World` can borrow them.
    ///
    /// Only `first_party` carries anything: the other four fields are repo-wide
    /// artifacts of a completed pass, and a single-file test has none. Empty is
    /// the honest value, NOT a stand-in — and what it costs is visible rather
    /// than hidden, which is the point of making the world an argument.
    #[derive(Default)]
    struct Told {
        first_party: BTreeSet<String>,
        members: BTreeSet<String>,
        declared: BTreeSet<crate::indexer::facts::Fqn>,
        returns: std::collections::BTreeMap<crate::indexer::facts::Fqn, String>,
        scanned: BTreeSet<crate::indexer::facts::Fqn>,
    }

    impl Told {
        fn of(package: &str) -> Self {
            Self { first_party: [package.to_string()].into_iter().collect(), ..Default::default() }
        }
        fn world(&self) -> World<'_> {
            World {
                first_party: &self.first_party,
                first_party_members: &self.members,
                declared_members: &self.declared,
                returns: &self.returns,
                scanned: &self.scanned,
            }
        }
    }

    /// **AN EXTENSION NO ADAPTER CLAIMS YIELDS EMPTY WITH A STATED REASON**
    /// (stage 11, S2 — §6 step 7).
    ///
    /// This REPLACES
    /// `a_file_no_adapter_claims_produces_no_facts_rather_than_empty_ones`,
    /// and it keeps that test's claim as its second half rather than dropping
    /// it. The old claim was "absent, not empty", and the reason it had to be
    /// absent is that an empty `FileFacts` reaching the writer reports zero
    /// declarations and zero uses — indistinguishable from a file that
    /// genuinely has none. A REASON is what makes them distinguishable, so
    /// `index_file` may now answer empty where `index_repo` still has to drop
    /// the file: 942 of this repo's 2,309 files are not source, and a driver
    /// that errored on each of them would make "most of a repo" a failure
    /// path.
    ///
    /// The reason is `Skipped::Unclaimed` — the vocabulary `load_repo` already
    /// emits — and NOT a second enum saying the same thing.
    ///
    /// Asserted on the LABEL rather than the variant, the idiom
    /// `one_file_reaches_a_member_of_a_type_it_imported` already uses: the
    /// assertion is readable before the variant exists, so the RED is a failure
    /// with a message rather than a compile error.
    ///
    /// The mutation that must break it: return `Empty::Deleted` from the
    /// unclaimed arm.
    #[test]
    fn an_unclaimed_extension_yields_empty_with_a_reason() {
        let told = Told::of("p");
        let out = index_file(FileInput {
            repo: "/w/demo",
            path: "README.cobol",
            mode: Mode::New,
            package: "p",
            module: "",
            text: "IDENTIFICATION DIVISION.\n",
            world: &told.world(),
        });

        assert!(
            out.facts().is_none(),
            "no adapter claims `.cobol`, so there is nothing read to report"
        );
        assert_eq!(
            out.why_empty(),
            Some("unclaimed"),
            "empty is not absence: the driver says WHY, in the vocabulary `load_repo` \
             already emits, so a repository that produced an empty graph can say which \
             reason it produced it for"
        );
        assert_eq!(
            out.language, None,
            "`language` is what the driver RESOLVED, and it resolved nothing — a stamped \
             language here would be the driver naming one (S1)"
        );
        assert_eq!(out.file, "README.cobol", "the file is stamped whatever the outcome");
        assert_eq!(out.repo, "/w/demo");

        // THE HALF THE DELETED TEST PROTECTED, kept. `index_repo` still yields
        // `FileFacts` and a `FileFacts` cannot carry a reason, so an unclaimed
        // file must still be ABSENT from its result — an empty one reaching the
        // writer is a file reporting zero declarations and zero uses.
        let unclaimed =
            Placed { path: "README.cobol", package: "p", module: "", text: "IDENTIFICATION" };
        let rust = Placed {
            path: "src/a.rs",
            package: "p",
            module: "a",
            text: "pub fn f() -> u32 { 1 }\n",
        };
        let indexed = index_repo(&[unclaimed, rust], &packages(&["p"]));
        assert_eq!(indexed.len(), 1, "only the file an adapter claims");
        assert_eq!(indexed[0].path, "src/a.rs");
    }

    /// **DELETE MODE YIELDS NO NODES AND NO EDGES** (stage 11, §2.1 — §6
    /// step 8).
    ///
    /// `Delete` returns an EMPTY result rather than skipping the call, so
    /// persistence has one input shape and no special case: every mode answers
    /// with the same struct, and what differs is what is in it.
    ///
    /// The text is DELIBERATELY NOT EMPTY, and that is the whole test. A
    /// fixture with nothing in it would pass with the mode ignored entirely —
    /// so the same input is indexed twice, once at `New` to prove there is
    /// something to lose, and once at `Delete` to prove it was not read. An
    /// empty answer over text that declares four things is evidence the parser
    /// never ran.
    ///
    /// `language` is still stamped, because the EXTENSION answers it and
    /// reading an extension opens nothing. Stage 12 needs it: a removed file
    /// arrives at reconcile as `Stated::Gone(Located { language, .. })`, and
    /// `Located` has no facts to take a language off.
    ///
    /// The mutation that must break it: `matches!(mode, Mode::Delete)` →
    /// `matches!(mode, Mode::New)`, which reads the file anyway.
    #[test]
    fn delete_mode_yields_no_nodes_and_no_edges() {
        let text = "pub struct Gone { pub count: u32 }\n\
                    impl Gone { pub fn total(&self) -> u32 { self.count } }\n";
        let told = Told::of("demo");
        let world = told.world();
        let at = |mode| {
            index_file(FileInput {
                repo: "/w/demo",
                path: "src/gone.rs",
                mode,
                package: "demo",
                module: "gone",
                text,
                world: &world,
            })
        };
        // Counted off the facts, so the assertion is on what stage 12 would
        // WRITE rather than on which variant happens to be in the field.
        let counted = |indexed: &FileIndex| {
            indexed.facts().map_or((0usize, 0usize), |facts| {
                (facts.symbols.len(), facts.references.len() + facts.relations.len())
            })
        };

        // ANTI-VACUITY. Without this, `(0, 0)` below would also hold for a
        // fixture that declares nothing, and the test would prove only that
        // empty text stays empty.
        let (nodes, edges) = counted(&at(Mode::New));
        assert!(
            nodes > 0 && edges > 0,
            "the fixture must have something to lose, or the delete assertion is vacuous: \
             {nodes} node(s), {edges} edge(s) at `New`"
        );

        let deleted = at(Mode::Delete);
        assert_eq!(
            counted(&deleted),
            (0, 0),
            "`Delete` does not read the text — persistence's job for this file is to remove \
             rows, and a node set read on the way to deleting them is work whose only \
             possible use is to disagree with them"
        );
        assert_eq!(
            deleted.why_empty(),
            Some("deleted"),
            "empty is STATED here too, so a counter can tell a deletion from a file no \
             adapter claimed and from one the grammar rejected"
        );
        assert_eq!(
            deleted.language,
            Some(Language::Rust),
            "the EXTENSION answers this and opens nothing; stage 12 needs it, because a \
             removed file reaches reconcile as `Stated::Gone(Located)` and `Located` has \
             no facts to take a language off"
        );
        assert_eq!(deleted.mode, Mode::Delete, "the mode is carried through, not consumed");
    }

    /// **THE SOURCE GUARDS READ THIS DRIVER** (stage 11, S1).
    ///
    /// Spec §3 S1 says the driver never matches on a language and names
    /// `lang::tests::nothing_outside_this_module_dispatches_on_a_language_by_hand`
    /// as the guard that covers it. That guard iterates
    /// `indexer::guard_sources()`, whose `OWNED` list named six files and
    /// `lang/` and NOT `index.rs` — so it read every file except the one the
    /// requirement is about, and was green about a module it never opened.
    ///
    /// Six more guards read the same list: the fqn-separator guard, the
    /// `Option`-for-a-resolution guard, the defaulted-value guard, the
    /// collapsed-spelling guard, the failed-read guard and the
    /// filesystem guard. Membership is the WHOLE property — once `index.rs`
    /// is in the list every one of them reads it, with no per-guard opt-in
    /// and no way to add a rule that quietly skips the driver.
    ///
    /// The body check is anti-vacuity. A rename, a move, or a guard scoped to
    /// a file that no longer holds the driver would satisfy membership while
    /// guarding nothing, which is the shape `outside_tests` was written for
    /// after a doc comment cut two thirds of `lang/rust.rs` out of every
    /// guard.
    #[test]
    fn the_source_guards_read_this_driver() {
        let sources = crate::indexer::guard_sources();
        let guarded = sources
            .iter()
            .find(|(path, _)| path == "index.rs")
            .map(|(_, body)| crate::indexer::outside_tests(body).to_string());
        let Some(guarded) = guarded else {
            panic!(
                "`guard_sources()` does not read `index.rs`, so every guard that iterates it \
                 passes over the driver S1 is about. It read: {:?}",
                sources.iter().map(|(p, _)| p.as_str()).collect::<Vec<_>>()
            )
        };
        for needed in ["fn index_repo", "fn read_all", "fn read_one", "fn load_repo"] {
            assert!(
                guarded.contains(needed),
                "the guarded half of `index.rs` does not contain `{needed}`, so the guards are \
                 reading something other than this driver's production code"
            );
        }
    }

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
            if let crate::indexer::facts::Resolution::Resolved { fqn, .. } = &r.target
                && !facts.symbols.iter().any(|s| s.fqn == *fqn)
            {
                println!("║  {:<42} stub", fqn);
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
                    fqn,
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

    /// **ONE FILE, ON ITS OWN, REACHES A MEMBER OF A TYPE IT IMPORTED**
    /// (stage 11, S7 — §6 step 3).
    ///
    /// This is the property that removes the barrier, and this test used to
    /// assert only HALF of it. `use crate::a::Widget` states where `Widget`
    /// lives, so a call on a `Widget` can be named `a·Widget·wide` from this
    /// file alone. The walk already minted that string; the LADDER refused it,
    /// because `declared_by_its_type` required a repo-wide set built at a
    /// barrier to confirm the declaration exists.
    ///
    /// Existence is not this stage's question. Under stub-and-heal it belongs
    /// to persistence, which mints a node on first mention and promotes it when
    /// its own file arrives (`resolved = nodes.resolved OR EXCLUDED.resolved`).
    /// What this stage owes is an IDENTITY both sides can mint, and the file's
    /// own import is proof enough of that.
    ///
    /// So the evidence is graded. `Named` says THIS FILE'S TEXT establishes the
    /// identity, and becomes an edge unconditionally. `Candidate` is a name
    /// match and still cannot, which is what
    /// `a_bare_name_matching_another_files_declaration_is_not_proof_of_anything`
    /// protects. Relaxing the gate blanket-style was measured at +12,360 and
    /// broke 13 tests; the grading is what makes the relaxation safe.
    ///
    /// The file already proved it had the information: the TYPE reference on
    /// the parameter resolves `ThroughAnImport` on the line above. Only the
    /// CALL failed.
    ///
    /// The mutation that must break it: `name_member`'s `Home::Stated` arm
    /// calling `considered(..)` instead of `named(..)`.
    #[test]
    fn one_file_reaches_a_member_of_a_type_it_imported() {
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
        let Resolution::Resolved { fqn, via } = &call.target else {
            panic!("the call did not reach the member its own import names: {:?}", call.target)
        };
        assert_eq!(
            fqn.to_string(),
            "rust·demo·a·Widget·wide·item",
            "the member is filed under the module the IMPORT says its type lives in"
        );
        // The LABEL rather than the variant, so this assertion is readable
        // before the variant exists and the RED is a failure rather than a
        // compile error.
        assert_eq!(
            via.as_label(),
            "named_by_this_file",
            "the proof is this file's own text, which is a different and stronger claim than \
             `declared_by_its_type` — that rung says the declaration was seen in ANOTHER file, \
             and it is the one this stage stops needing"
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
        // **READ BOTH COLUMNS AND THEIR TOTAL, NEVER `resolved` ALONE.**
        //
        // The corpus IS this repository, so a commit that deletes production
        // code deletes the references in it, and `resolved` falls without the
        // resolver having changed at all. MEASURED, and it cost an hour before
        // it was written down: removing the collapsed-spelling table took
        // resolved 96,420 -> 96,336 and looked like a regression. Unresolved
        // fell too, 96,926 -> 96,879, and the TOTAL fell by 131 — which is the
        // signature of a smaller corpus. A resolver regression holds the total
        // still and moves references from one column to the other.
        //
        // The deleted rung, checked separately, had placed ZERO references.
        println!(
            "  references: {resolved} resolved, {unresolved} unresolved, {} total",
            resolved + unresolved
        );

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
    use crate::indexer::facts::{
        Binding, Evidence, FileFacts, Observation, Resolution, SymbolKind,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    /// Load, place and walk THIS repository, once.
    ///
    /// Shared by both decompositions rather than copied into each, because they
    /// ask one question of the two sides of a single split: a walk that placed
    /// the reference and a walk that did not must be the SAME walk, or the two
    /// tables are not comparable and the subtraction between them means nothing.
    fn walk_this_repository() -> (Vec<Loaded>, Vec<FileFacts>) {
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
        let indexed = {
            let placed: Vec<Placed<'_>> = loaded.iter().map(Loaded::placed).collect();
            index_repo(&placed, &first_party)
        };
        (loaded, indexed)
    }

    /// Every type name the corpus declares anywhere, so a member fqn can be
    /// decomposed without guessing which segment is the type.
    fn type_names(indexed: &[FileFacts]) -> BTreeSet<&str> {
        let mut names = BTreeSet::new();
        for facts in indexed {
            for symbol in &facts.symbols {
                if matches!(
                    symbol.kind,
                    SymbolKind::Struct
                        | SymbolKind::Enum
                        | SymbolKind::Trait
                        | SymbolKind::Class
                        | SymbolKind::Interface
                ) {
                    names.insert(symbol.name.as_str());
                }
            }
        }
        names
    }

    /// What ONE FILE states about names, on its own — the whole of what stage
    /// 11 permits a walk to read (S5).
    struct FileStates<'a> {
        declares: BTreeSet<&'a str>,
        binds: BTreeSet<&'a str>,
        has_glob: bool,
        text: &'a str,
    }

    impl<'a> FileStates<'a> {
        fn of(facts: &'a FileFacts, text: &'a str) -> Self {
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
            Self { declares, binds, has_glob, text }
        }

        /// Where this file could have learned `ty`'s home on its own.
        ///
        /// The ORDER is the grading: a declaration and a by-name import are the
        /// file's own word (`Home::Stated`), a wildcard only says the name
        /// COULD arrive that way, and `spelled` is the weakest — the name
        /// appears in the text somewhere, which an inline qualified path
        /// satisfies and so does a comment.
        fn bucket_for(&self, ty: &str) -> &'static str {
            if self.declares.contains(ty) {
                "local"
            } else if self.binds.contains(ty) {
                "imported"
            } else if self.has_glob {
                "glob"
            } else if self.text.contains(ty) {
                "spelled"
            } else {
                "gap: never named in the file"
            }
        }
    }

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
    ///   through it; can only be confirmed with the target module
    /// - `gap`      — none of the above. ONLY a global table could have
    ///   supplied this, and it is the number that decides whether the barrier
    ///   can be deleted.
    ///
    ///     cargo test -p senseid --bin senseid -- --ignored --nocapture barrier_necessity
    #[test]
    #[ignore = "walks this repository"]
    fn every_resolved_member_is_traced_to_what_its_own_file_states() {
        let (loaded, indexed) = walk_this_repository();
        let type_names = type_names(&indexed);

        // The file's TEXT, because a file can name a type without importing it:
        // `crate::db::pg_store::PgStore::connect(..)` states the module inline
        // and needs no import at all. Counting only imports read that as a gap.
        let text_of: BTreeMap<&str, &str> =
            loaded.iter().map(|l| (l.path.as_str(), l.text.as_str())).collect();
        let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
        let mut gaps: BTreeMap<String, usize> = BTreeMap::new();
        let mut per_language: BTreeMap<(String, &str), usize> = BTreeMap::new();
        for facts in &indexed {
            let states =
                FileStates::of(facts, text_of.get(facts.path.as_str()).copied().unwrap_or(""));

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
                let bucket = states.bucket_for(ty);
                if bucket.starts_with("gap") {
                    *gaps.entry(format!("{ty} in {}", facts.path)).or_default() += 1;
                }
                *tally.entry(bucket).or_default() += 1;
                *per_language.entry((format!("{:?}", facts.language), bucket)).or_default() += 1;
            }
        }

        let total: usize = tally.values().sum();
        println!("\n── resolved member references, by what the FILE states ──");
        for (bucket, count) in &tally {
            println!("  {count:>7}  {bucket}  ({:.1}%)", *count as f64 * 100.0 / total as f64);
        }
        println!("  {total:>7}  total");

        // **PER LANGUAGE, AND THIS IS THE NUMBER §11 IS SEQUENCED ON.** An
        // adapter may drop its table when the FILE already states what the
        // table was supplying, so the `gap` row is that language's risk and
        // the rest is what a file-only rung could recover. Rust's share says
        // nothing about TypeScript's — the migration is one adapter at a time.
        println!("\n── by LANGUAGE ──");
        for ((language, bucket), count) in &per_language {
            println!("  {count:>7}  {language:<12} {bucket}");
        }
        let mut worst: Vec<(&String, &usize)> = gaps.iter().collect();
        worst.sort_by(|a, b| b.1.cmp(a.1));
        println!("  distinct gap sites: {}", gaps.len());
        for (what, count) in worst.iter().take(200) {
            println!("    {count:>5}  {what}");
        }
        assert!(total > 0, "no member references classified, so this proved nothing");
    }

    /// The type an UNRESOLVED reference concerns, when one can be named at all,
    /// and WHICH of the three sources named it.
    ///
    /// The resolved side reads the type out of the fqn the ladder produced. A
    /// miss has no fqn, so the literal instruction "point the classifier at
    /// `Unresolved`" does not typecheck, and the adaptation is this function:
    /// three sources, strongest first, none of them an inference.
    ///
    /// 1. `minted` — the walk named an identity (`Candidate` or `Named`) and
    ///    the LADDER declined it. Closest to resolvable of the three.
    /// 2. `unplaced-type` — `Observation::UnplacedType`: the walk read a type
    ///    and had no home for it. This is what `Home::Unstated` leaves behind.
    /// 3. `declined-path` — the walk minted NOTHING and the use site is a
    ///    qualified path. `Walk::considered_path` returns an empty candidate
    ///    list for any path of three segments or more, so the module the source
    ///    spelled out in full is the one thing the walk refuses to read.
    ///
    /// Returns the type OWNED, because source 1's segment borrows the encoded
    /// fqn, which is a temporary of this call.
    fn type_at_issue(
        evidence: &Evidence,
        types: &BTreeSet<&str>,
    ) -> Option<(String, &'static str)> {
        for fqn in evidence.identities() {
            let encoded = fqn.to_string();
            let Ok(parsed) = crate::indexer::fqn::parse(&encoded) else { continue };
            if parsed.tail.len() < 2 {
                continue;
            }
            let ty = parsed.tail[parsed.tail.len() - 2];
            if types.contains(ty) {
                return Some((ty.to_string(), "minted"));
            }
        }
        for observation in &evidence.saw {
            if let Observation::UnplacedType(ty) = observation
                && types.contains(ty.as_str())
            {
                return Some((ty.clone(), "unplaced-type"));
            }
        }
        // A qualified path at the use site. The segment before the last is the
        // type, exactly as on the resolved side.
        let segments: Vec<&str> = evidence
            .name
            .split("::")
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.starts_with('<'))
            .collect();
        if segments.len() >= 2 {
            let ty = segments[segments.len() - 2];
            if types.contains(ty) {
                return Some((ty.to_string(), "declined-path"));
            }
        }
        None
    }

    /// **WHERE THE MISSING REFERENCES ARE — the other half of the split.**
    ///
    /// Its sibling above classifies what DID resolve and reports that the file
    /// states 97.9% of it. That number is the stage-11 premise confirmed, and
    /// it CANNOT locate a missing reference: it is a decomposition of the
    /// successes. Reading it as coverage was the specific error this test
    /// exists to close.
    ///
    /// So: the same classifier, over the MISSES. The bucket that matters is not
    /// `gap` this time but its opposite — a reference whose type the file
    /// DECLARES or IMPORTS BY NAME is one the file states the home of, and the
    /// walk still did not place it. Those are recoverable without a table, so
    /// each one is a rung that is missing rather than a cost of deleting the
    /// barrier. `gap` here is the honest residue: only a repo-wide table could
    /// ever have placed it.
    ///
    ///     cargo test -p senseid --bin senseid -- --ignored --nocapture barrier_necessity
    #[test]
    #[ignore = "walks this repository"]
    fn every_unresolved_member_is_traced_to_what_its_own_file_states() {
        let (loaded, indexed) = walk_this_repository();
        let type_names = type_names(&indexed);
        let text_of: BTreeMap<&str, &str> =
            loaded.iter().map(|l| (l.path.as_str(), l.text.as_str())).collect();

        // bucket -> source -> count, because "the file declares this type" and
        // "the walk minted an identity for it" are independent facts and the
        // pair is what names the defect. A single axis would average them.
        let mut tally: BTreeMap<(&str, &str), usize> = BTreeMap::new();
        let mut by_reason: BTreeMap<(&str, String), usize> = BTreeMap::new();
        let mut stated_sites: BTreeMap<String, usize> = BTreeMap::new();
        let mut per_language: BTreeMap<(String, &str), usize> = BTreeMap::new();
        let mut unnameable = 0usize;
        // Hypothesis 2, counted directly rather than inferred from the buckets.
        let mut long_paths: BTreeMap<String, usize> = BTreeMap::new();

        for facts in &indexed {
            let states =
                FileStates::of(facts, text_of.get(facts.path.as_str()).copied().unwrap_or(""));

            for reference in &facts.references {
                let Resolution::Unresolved { reason, evidence } = &reference.target else {
                    continue;
                };
                // PLUMBING AND THE EXTERNAL BOUNDARY ARE VERDICTS, NOT MISSES.
                // `Reason::casts_doubt` already draws this line and is read
                // here rather than restated: `.collect()` is not a reference
                // anybody lost, and mixing the two makes the residue look
                // enormous and unactionable.
                if !reason.casts_doubt() {
                    continue;
                }
                if evidence.name.matches("::").count() >= 2 {
                    *long_paths.entry(evidence.name.clone()).or_default() += 1;
                }
                let Some((ty, source)) = type_at_issue(evidence, &type_names) else {
                    unnameable += 1;
                    continue;
                };
                let bucket = states.bucket_for(&ty);
                *tally.entry((bucket, source)).or_default() += 1;
                *by_reason.entry((bucket, format!("{reason:?}"))).or_default() += 1;
                // SPLIT BY LANGUAGE, because the adapters are migrated off the
                // table ONE AT A TIME (§11) and a total mixes a language that
                // has lost its table with four that have not. Rust reaching
                // zero in a bucket says nothing about TypeScript's share of it.
                *per_language.entry((format!("{:?}", facts.language), bucket)).or_default() += 1;
                // The file's own word on where the type lives — `Home::Stated`
                // in the walk's own grading. These are the recoverable ones.
                if bucket == "local" || bucket == "imported" {
                    *stated_sites.entry(format!("{ty} in {}", facts.path)).or_default() += 1;
                }
            }
        }

        let total: usize = tally.values().sum();
        println!("\n── UNRESOLVED references that name a first-party type ──");
        println!("  (`Plumbing` and `ExternalBoundary` excluded: they are verdicts)");
        let mut per_bucket: BTreeMap<&str, usize> = BTreeMap::new();
        for ((bucket, source), count) in &tally {
            *per_bucket.entry(bucket).or_default() += count;
            println!("  {count:>7}  {bucket:<28} via {source}");
        }
        println!("  ── by bucket ──");
        for (bucket, count) in &per_bucket {
            println!("  {count:>7}  {bucket}  ({:.1}%)", *count as f64 * 100.0 / total as f64);
        }
        println!("  {total:>7}  total classified");
        println!("  {unnameable:>7}  name no first-party type at all (receiver, bare name)");

        println!("\n── by LANGUAGE, because the table is dropped one adapter at a time ──");
        for ((language, bucket), count) in &per_language {
            println!("  {count:>7}  {language:<12} {bucket}");
        }

        let recoverable: usize = per_bucket.get("local").copied().unwrap_or_default()
            + per_bucket.get("imported").copied().unwrap_or_default();
        println!(
            "\n  ** {recoverable} unresolved references whose type THIS FILE STATES **\n  \
             (declared here or imported by name — placeable with no table at all)"
        );

        println!("\n── reason, within each bucket ──");
        let mut reasons: Vec<((&str, String), usize)> = by_reason.into_iter().collect();
        reasons.sort_by_key(|entry| std::cmp::Reverse(entry.1));
        for ((bucket, reason), count) in reasons.iter().take(20) {
            println!("  {count:>7}  {bucket:<28} {reason}");
        }

        println!("\n── worst STATED sites (a rung is missing, not a table) ──");
        let mut worst: Vec<(&String, &usize)> = stated_sites.iter().collect();
        worst.sort_by(|a, b| b.1.cmp(a.1));
        println!("  distinct sites: {}", stated_sites.len());
        for (what, count) in worst.iter().take(40) {
            println!("    {count:>5}  {what}");
        }

        println!("\n── hypothesis 2: qualified paths of 3+ segments the walk declines ──");
        let mut long: Vec<(&String, &usize)> = long_paths.iter().collect();
        long.sort_by(|a, b| b.1.cmp(a.1));
        let long_total: usize = long_paths.values().sum();
        println!("  {long_total:>7}  total, over {} distinct paths", long_paths.len());
        for (what, count) in long.iter().take(25) {
            println!("    {count:>5}  {what}");
        }

        assert!(total > 0, "no unresolved member references classified, so this proved nothing");
    }
}
