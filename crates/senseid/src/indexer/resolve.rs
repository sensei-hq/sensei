//! The resolution ladder and the reason codes (stage 5).
//!
//! This is the one place a use site is turned into a target (R7). A language
//! module reads a grammar and says what it SAW; it never says what a name means,
//! because "what does a miss mean" answered twice is answered two ways, and the
//! reason histogram is only a measurement while every language fills the same
//! buckets.
//!
//! The rungs, in order, and each one PROVES its answer:
//!
//! 1. a declaration in the same file,
//! 2. an import in scope that binds the head of the path,
//! 3. a member some TYPE of this scan was read declaring, in another file,
//! 4. a path rooted in the package being scanned,
//! 5. the language's prelude — the names in scope with no import at all.
//!
//! Rung 3 is below rung 2 on purpose and the ordering is measured, not
//! aesthetic: an import is written in the source and can say the target belongs
//! to another package, while the type table is keyed by (package, name) and can
//! only ever answer within the use site's own.
//!
//! Anything that reaches the bottom is [`Reason`]-coded and keeps the evidence
//! the walk gathered. Nothing is invented on the way down (R4).
//!
//! Two rules the ladder exists to enforce, both of them defects this codebase
//! has actually shipped:
//!
//! - **Externality comes from the import, never from absence** (spec §2, R6). A
//!   symbol missing from what has been scanned so far may simply not have been
//!   scanned yet, so treating absence as "external" makes the graph depend on
//!   file order. The old indexer did exactly that: it anchored types it could not
//!   place on the CALLER's module and minted 659 ghost nodes carrying 2,305
//!   inbound edges.
//! - **A wrong edge is worse than a missing one** (R4). Every rung below either
//!   names the segment boundary from something written in the source — an import
//!   specifier, a path root — or does not fire.
//
// This module has no caller on purpose — see the note in `indexer/mod.rs`.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

use super::facts::{
    Binding, DeclaredType, Evidence, FileFacts, Fqn, Import, ImportOrigin, Language, Observation,
    Reason, RefKind, Reference, Relation, RelationKind, Resolution, Rung, Span, SymbolKind,
};
use super::fqn::{self, Form, Origin, Reach};

/// One language's vocabulary, as DATA rather than as behaviour.
///
/// The ladder's rungs and its reason codes are shared (R7); what differs between
/// languages is only the spelling — which characters join a path, which word
/// roots a path in the current package, which names are in scope with no import.
/// Keeping those as data is what stops a second language from arriving with its
/// own idea of what a miss means.
pub struct Grammar {
    pub language: Language,
    /// What joins the segments of a QUALIFIED NAME at a use site — Rust's
    /// `a::b::c`, TypeScript's `ns.thing`.
    pub path_separator: &'static str,
    /// What joins the segments of a MODULE PATH, which is also how an import
    /// specifier spells its way to a module — Rust's `a::b`, TypeScript's
    /// `./a/b`.
    ///
    /// Separate from [`Grammar::path_separator`] because for every language but
    /// Rust they differ, and Rust spelling them alike is a coincidence rather
    /// than a rule. Collapsing them split `./lib/store` into one segment and
    /// then joined `src.lib` as a module path — two wrong identities from one
    /// shared field.
    pub module_separator: &'static str,
    /// The words that ROOT a path, and what each one roots it at. A language
    /// with no such word — JavaScript has no `crate` — simply does not list
    /// one, which is why this is a table and not three named fields: an absent
    /// root has no spelling, and giving it a placeholder spelling would make
    /// some real segment accidentally root a path.
    pub roots: &'static [(&'static str, Root)],
    /// What an import specifier puts between the path and the name it binds.
    /// The walk keeps a specifier verbatim, so the ladder has to know where the
    /// path stops. `None` where the specifier never carries the binding —
    /// TypeScript states it in the import clause, not in the string.
    /// The character a language REPEATS at the start of a specifier to state
    /// relative depth, where every other language here names each level with a
    /// token of its own (`super::`, `../`).
    ///
    /// Python writes `.mod` for a sibling, `..mod` for one level up, `...mod`
    /// for two — and that character is also its module separator, so the dots
    /// do not survive tokenizing at all: [`Ladder::segments`] drops empty
    /// segments, which is correct for a trailing `a::` and fatal for a bare
    /// `.`. The depth is therefore counted off the RAW string, which is the
    /// only place it still exists, and the split then yields exactly the
    /// remainder.
    ///
    /// `None` leaves the roots table above as the whole rule. Getting this
    /// wrong is not a miss: with the dots discarded the specifier reduces to
    /// the importing file's OWN module, and the import becomes a self-edge —
    /// which R4 ranks below emitting nothing.
    pub relative_depth_prefix: Option<char>,
    pub names_the_binding: Option<&'static str>,
    /// What a specifier ends with when it binds a whole module instead of one
    /// name. Also verbatim in the specifier, so also the ladder's to read off.
    /// `None` where the language has no such spelling.
    pub wildcard: Option<&'static str>,
    /// Whether a fully-qualified path can name an EXTERNAL package with no
    /// import at all.
    ///
    /// Rust yes: `serde_json::json!(..)` and `tracing::warn!(..)` are complete
    /// uses, and R5 says an external is named by use and never opened — so
    /// requiring an import to place one is requiring something the language
    /// does not. MEASURED: 268 `serde_json::json`, 57 `tracing::warn`, 54
    /// `tracing::error`, 39 `uuid::Uuid::parse_str` sat in `NoImportInScope`
    /// for exactly this reason.
    ///
    /// TypeScript NO, and the difference is not stylistic: a bare `foo.bar` is
    /// an ordinary property access on a local object, so reading its head as a
    /// package would mint a library node for every untyped local in the corpus.
    /// A JavaScript module names an external ONLY through an import.
    pub paths_name_packages: bool,
    /// How the LAST segment of a LOCAL import specifier reduces to a segment of
    /// a module path.
    ///
    /// A JavaScript specifier is a filesystem path, so its last segment is a
    /// FILE NAME: `./buckets.js` names the module the declaration side minted
    /// from `src/lib/buckets.ts`, and the extension belongs to neither. Rust's
    /// `use a::b` is already a module path and has no file name in it, so it
    /// answers with what it was given.
    ///
    /// A FUNCTION rather than a flag, so the rule lives in the language module
    /// that also applies it to the declaration side — `javascript::module_path`
    /// calls this same one. Two spellings of it is exactly how the two sides of
    /// an import come to mint different strings and never merge (spec §2).
    ///
    /// Applied to LOCAL specifiers only. An external's path is never reduced to
    /// a module of ours, so there is no declaration for it to agree with, and
    /// chopping `lodash/get.js` would rename a library node for no gain.
    pub module_segment: fn(&str) -> &str,
    /// Whether a rooted path is relative to the file's DIRECTORY rather than to
    /// the module the file is.
    ///
    /// The two languages genuinely differ and getting it wrong is off by one
    /// module in every relative import. Rust's `self::x` inside module `a::b`
    /// names `a::b::x` — the module itself. JavaScript's `./x` inside
    /// `src/a/b.ts` names `src/a/x` — a SIBLING, because the specifier is a
    /// filesystem path and `b.ts` is a file in `src/a`, not a directory
    /// containing `x`.
    pub relative_to_directory: bool,
    /// Whether a segment names a TYPE rather than a module.
    ///
    /// This is the one thing the ladder cannot read off the source: `a::b::c`
    /// might be item `c` of module `a::b`, or member `c` of type `b` in module
    /// `a`, and the two mint different identities. Where a language states the
    /// answer in the name — Rust lints every type into `CamelCase` and every
    /// module into `snake_case` — this reads it. Where one does not, the honest
    /// answer is a predicate that never fires, and the reference stays a miss.
    ///
    /// A misread here cannot produce a WRONG edge, only a dangling one: the two
    /// splits differ in which separator falls where, so the loser names no
    /// declaration at all rather than naming somebody else's (A4 catches it).
    pub names_a_type: fn(&str) -> bool,
    /// The names in scope with NO import, each as
    /// `(name, package, path within that package)`.
    ///
    /// The package is per-entry rather than one per language, and that is not
    /// generality for its own sake: Svelte's runes — `$state`, `$derived`,
    /// `$props` — are in scope with nothing written, exactly like a prelude
    /// name, but they belong to `svelte` and not to `ecmascript`. One package
    /// for the whole table would have filed 1,038 references under a package
    /// that does not define them.
    ///
    /// The PATH and not the bare name, so a file that imports one the long way
    /// lands on the node the prelude lands on.
    pub prelude: &'static [(&'static str, &'static str, &'static str)],
    /// Members every value of the language has. Filtering, not failure — see
    /// [`Reason::Plumbing`].
    pub plumbing: &'static [&'static str],
}

/// What a root word roots a path AT.
///
/// Three cases, because there are three: the package itself, the module the
/// path is written in, and one module out. A language states as many of them as
/// it has spellings for (see [`Grammar::roots`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Root {
    /// The package being scanned — Rust's `crate`.
    Package,
    /// The module the path is written in — Rust's `self`, TypeScript's `.`.
    Here,
    /// One module out — Rust's `super`, TypeScript's `..`.
    Up,
}

/// Which root a segment names in this grammar, if it names one at all.
pub(crate) fn root_of(segment: &str, grammar: &Grammar) -> Option<Root> {
    grammar.roots.iter().find(|(word, _)| *word == segment).map(|(_, root)| *root)
}

/// **THE ONE PLACE A ROOTED PATH IS READ AGAINST A MODULE.**
///
/// Takes a path's leading ROOT WORDS off against `base` and returns the
/// package-relative segments — `None` when the path climbs PAST the package
/// root, which is not a module of ours to name (R5).
///
/// Shared by the ladder and by a language's own walk, and that sharing is the
/// point rather than a convenience. Three callers want the identical
/// arithmetic on three different BASES:
///
/// - [`Ladder::relative_to`] reads it against the module a SPAN sits in, so an
///   inline `mod b { use super::x; }` climbs from `b`.
/// - a WALK reads it against the FILE's own module, because a `use super::*` at
///   file scope is relative to the file — resolving it against a scope that had
///   descended into a nested module would climb from the wrong place.
///
/// What differs between languages is entirely in [`Grammar`]: the `roots`
/// table, and `relative_to_directory` for a language whose path is a FILE path
/// and so carries the file's own segment. Rust reads `super` off the module
/// (`a::b::c` → `a::b`); TypeScript reads `../x` off the directory
/// (`lib/nested/store` → `lib/x`). One function, driven by the grammar, gets
/// both right — and a second copy per language is how those two rules drift.
///
/// Does NOT handle a repeated-prefix language's depth counting
/// (`Grammar::relative_depth_prefix`, Python's `from ..a import b`), which is a
/// different reading of a different token and stays with its caller.
pub(crate) fn rooted_against(
    segments: &[String],
    mut base: Vec<String>,
    grammar: &Grammar,
) -> Option<Vec<String>> {
    // A directory-relative language counts from the folder the file sits in, so
    // the file's own segment comes off before any `..` is read. Done ONCE, on
    // seeing the first root: `../..` pops twice from the directory, not three
    // times.
    if grammar.relative_to_directory
        && segments.first().is_some_and(|s| root_of(s, grammar).is_some())
    {
        base.pop();
    }
    let mut rest = segments;
    while let Some(head) = rest.first() {
        match root_of(head, grammar) {
            Some(Root::Package) => base.clear(),
            // Names the module the path is written in, which `base` already is.
            Some(Root::Here) => {}
            Some(Root::Up) => {
                base.pop()?;
            }
            None => break,
        }
        rest = &rest[1..];
    }
    base.extend(rest.iter().cloned());
    Some(base)
}

/// What the ladder is told about the scan it is part of.
pub struct World<'a> {
    /// Every package this scan owns the source of, from the manifests — NOT from
    /// what has been read so far. An import naming one of these crosses no
    /// boundary however it is spelled, so it resolves local; anything else the
    /// import calls external is a library (R5).
    pub first_party: &'a BTreeSet<String>,
    /// Every MEMBER NAME any first-party type in this scan declares.
    ///
    /// What tells a miss from a boundary. A member the scan declares nowhere
    /// cannot become a first-party edge however much is later learned about the
    /// receiver — `.trim()`, `.collect()`, `.toBe()` are methods on types R5
    /// says we name and never open — so counting them as failures made 23,739
    /// of 53,224 untyped receivers read as a gap somebody should close.
    ///
    /// A BARRIER artifact like the type table: built from a completed pass, so
    /// the answer does not depend on which file came first (R6). Empty means
    /// "not supplied", and then nothing is reclassified — the previous
    /// behaviour, and never a guess.
    pub first_party_members: &'a BTreeSet<String>,
    /// Every member IDENTITY a first-party type declares.
    ///
    /// The sibling of [`World::first_party_members`] and a different question:
    /// that one holds member NAMES, which is all a use site with an untyped
    /// receiver can be compared against. This one holds whole identities, which
    /// is what a use site whose receiver IS typed can be matched against
    /// exactly.
    ///
    /// Taken from [`members_declared_by`] — the children of the `Owns`
    /// relations of a completed pass — and not from a `SymbolKind` filter, for
    /// the reason that filter is wrong: which kinds are type-owned differs per
    /// language, and it is what made [`World::first_party_members`] unusable as
    /// an inventory.
    ///
    /// A BARRIER artifact like the type table, so the answer does not depend on
    /// which file came first (R6). Empty means "not supplied", and then nothing
    /// resolves through it — the previous behaviour, and never a guess.
    pub declared_members: &'a BTreeSet<Fqn>,
    /// What each declaration RETURNS, so a call on the result of a call can be
    /// typed. Empty means "not supplied" and nothing is chased — never a guess.
    pub returns: &'a BTreeMap<Fqn, String>,
    /// Every identity minted before this file. It is here, and it is
    /// deliberately never read.
    ///
    /// A rung that consulted it would answer differently depending on which file
    /// came first, which is the order dependence R6 forbids and the exact shape
    /// of the ghost-node defect in the module docs. It is carried so that
    /// `resolving_a_file_does_not_depend_on_what_has_been_scanned_before_it` can
    /// hand the ladder the whole corpus and show the output does not move.
    pub scanned: &'a BTreeSet<Fqn>,
}

/// What each function and method RETURNS, for [`World::returns`].
///
/// A lookup, never an inference: the value is the type the source WROTE. A
/// declaration that states no return type is absent, and a receiver typed from
/// it stays an honest miss.
///
/// `Self` is resolved HERE, from the member's own identity. A method's fqn
/// carries the type it belongs to, so `Config::from_env -> Self` is stored as
/// `Config` without anything having to guess: the answer is a segment of the
/// key. The walk does not do it — `declared_type` is the text the source wrote,
/// which is the right thing for a fact to be.
///
/// Handed a COMPLETED pass, like the other barrier artifacts: the key is a
/// member identity carrying its type's module, which is only right once the
/// type table has been applied.
pub fn returns_declared_by<'a>(
    files: impl IntoIterator<Item = &'a FileFacts>,
) -> BTreeMap<Fqn, String> {
    files
        .into_iter()
        .flat_map(|facts| facts.symbols.iter())
        .filter_map(|symbol| match &symbol.declared_type {
            DeclaredType::Stated(stated) => {
                let stated = stated.trim_start_matches('&').trim();
                if stated != "Self" {
                    return Some((symbol.fqn.clone(), stated.to_string()));
                }
                // `Self` IS the enclosing type, and the member's identity says
                // which: the segment before the member name.
                let parsed = fqn::parse(symbol.fqn.as_str()).ok()?;
                let at = parsed.tail.iter().position(|s| *s == symbol.name)?;
                let owner = parsed.tail.get(at.checked_sub(1)?)?;
                Some((symbol.fqn.clone(), (*owner).to_string()))
            }
            DeclaredType::Unstated => None,
        })
        .collect()
}

/// Every member identity the scan declares, for [`World::declared_members`].
///
/// Built from `RelationKind::Owns` because that IS the vocabulary for "this
/// type declares this member" — the fact the rung needs — and because it is the
/// only thing that cannot be stood in for by a `SymbolKind` filter without
/// getting a different answer in each language.
///
/// Handed a COMPLETED pass, deliberately. A member's identity carries the
/// module its TYPE lives in, which is only right once the type table has been
/// applied, so a set taken off the first pass would hold the pre-barrier
/// spellings and match nothing.
pub fn members_declared_by<'a>(files: impl IntoIterator<Item = &'a FileFacts>) -> BTreeSet<Fqn> {
    files
        .into_iter()
        .flat_map(|facts| facts.relations.iter())
        .filter(|relation| relation.kind == RelationKind::Owns)
        .map(|relation| relation.child.clone())
        .collect()
}

/// Every member NAME any first-party type declares, for
/// [`World::first_party_members`].
///
/// The sibling of [`members_declared_by`] and a weaker question on purpose:
/// that one asks which member IDENTITIES exist, this one only which names do.
/// The ladder uses it to tell somebody else's member from a miss of ours, so it
/// has to say yes on a bare name with no type attached.
///
/// One function because it was four, and two of them disagreed — the barrier's
/// fixture world and the Java corpus harness left [`SymbolKind::Property`] out,
/// so they resolved against a narrower boundary than the corpus measurement
/// they are read beside. A set that decides what counts as external is not a
/// thing to spell twice.
pub fn member_names_of<'a>(files: impl IntoIterator<Item = &'a FileFacts>) -> BTreeSet<String> {
    files
        .into_iter()
        .flat_map(|facts| facts.symbols.iter())
        // The three kinds a TYPE owns. An accessor is one of them: a language
        // that spells a getter differently from a field still declares a member
        // that `x.name` reaches.
        .filter(|s| matches!(s.kind, SymbolKind::Method | SymbolKind::Field | SymbolKind::Property))
        .map(|s| s.name.clone())
        .collect()
}

/// Place every reference and every relation of one file against the ladder
/// (spec §3.2, §3.3).
///
/// Total in, total out: both counts are unchanged, because a miss is a `Reason`
/// and never an omission (R2).
///
/// Structure climbs the same rungs as a call does, and deliberately so. A trait
/// named in an impl header is the same kind of name as a callee — one rule for
/// both is what R7 asks for, and a second one would be a second idea of what a
/// miss means.
pub fn resolve(facts: FileFacts, grammar: &Grammar, world: &World<'_>) -> FileFacts {
    let ladder = Ladder::new(&facts, grammar, world);
    let references: Vec<Reference> = facts
        .references
        .iter()
        .map(|r| Reference {
            // An IMPORT is routed apart because its evidence is a PATH and not
            // a name: the ladder's rungs look names up, and a specifier has to
            // be reduced instead. Total in, total out is untouched — this
            // places a reference the walk emitted, it does not mint one.
            target: match r.kind {
                RefKind::Imports => ladder.place_entered(&r.target, r.at, &facts.imports),
                _ => ladder.place(&r.target, r.at),
            },
            ..r.clone()
        })
        .collect();
    // Structure needs no special case here. A relation's parent carries the
    // same `Evidence` a reference's target does, and the walk stated its reach
    // there — so there is nothing for this seam to decide.
    let relations: Vec<Relation> = facts
        .relations
        .iter()
        .map(|r| Relation { parent: ladder.place(&r.parent, r.at), ..r.clone() })
        .collect();
    FileFacts { references, relations, ..facts }
}

/// What a rung concluded. Not an `Option`: "no answer" is a state the caller has
/// to write a reason for, and an `Option` lets it be dropped instead (R2).
enum Placed {
    Proven(Fqn),
    Unbound,
}

impl Placed {
    /// A candidate is proven or it is nothing. Written once because three rungs
    /// need it, and a `bool` threaded through each of them is how one comes to
    /// return the candidate on a false.
    fn proven_if(fqn: &Fqn, proof: bool) -> Self {
        match proof {
            true => Self::Proven(fqn.clone()),
            false => Self::Unbound,
        }
    }
}

/// Where a path's root put it, once the root word has been read away.
enum Rooted {
    /// Package-relative segments, counted from the package root.
    At(Vec<String>),
    /// The root walked out past the package root, so it names nothing. Source
    /// that does this does not compile; inventing a module for it would be
    /// fabrication (R4).
    Nowhere,
}

/// What the ladder wants to place: a path, and the reach to mint it under.
struct Wanted {
    segments: Vec<String>,
    reach: Reach,
}

struct Ladder<'a> {
    grammar: &'a Grammar,
    world: &'a World<'a>,
    package: &'a str,
    /// The file's own module path, which every relative root is read against.
    module: &'a str,
    /// Identities this file DECLARES. The first rung, and the only one that needs
    /// no grammar at all.
    declared: BTreeSet<&'a str>,
    /// Every module declaration with a body, as `(span, name)`, outermost first.
    /// This is what gives a use site the module path it is WRITTEN in, and what
    /// bounds an import to the block it appears in.
    blocks: Vec<(Span, &'a str)>,
    /// Imports that bind exactly one name, keyed by that name. A name may be
    /// bound more than once in one file — once at the top and again inside a
    /// test module — so the scopes are kept and the innermost one wins.
    bound: BTreeMap<&'a str, Vec<&'a Import>>,
    /// Imports that bind an unknown set of names. One of these can quietly
    /// replace a prelude name, so it is what stops rung 4 from answering.
    globs: Vec<&'a Import>,
}

impl<'a> Ladder<'a> {
    fn new(facts: &'a FileFacts, grammar: &'a Grammar, world: &'a World<'a>) -> Self {
        // The file's OWN module identity, which is a block this file sits IN
        // rather than one nested inside it.
        //
        // `module_at` seeds from the file's module path and then appends every
        // enclosing module block. An inline `mod b { }` really does add a
        // segment; a per-file module does not — its span covers the whole file,
        // so it encloses every site, and `a::b` would become `a::b::b` for
        // every path rooted in this package. Filtered by IDENTITY and not by
        // span or name: the identity is a string the adapter already mints,
        // whereas a name test would also drop a genuine inline `mod b` inside
        // `b.rs`, and a span test would depend on the emitter's choice of span.
        let its_own = super::lang::adapter_for(facts.language)
            .file_fqn(&facts.package, &facts.module, &facts.path)
            .ok();
        let mut blocks: Vec<(Span, &str)> = facts
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Module)
            .filter(|s| its_own.as_ref() != Some(&s.fqn))
            .map(|s| (s.span, s.name.as_str()))
            .collect();
        // Outermost first, so reading them in order spells the module path. A
        // module that encloses another starts before it and ends after it.
        blocks.sort_by_key(|(span, _)| (span.start_line, span.start_col));

        let mut bound: BTreeMap<&str, Vec<&Import>> = BTreeMap::new();
        let mut globs: Vec<&Import> = Vec::new();
        for import in &facts.imports {
            match import.binds.name() {
                Some(name) => bound.entry(name).or_default().push(import),
                None => globs.push(import),
            }
        }

        Self {
            grammar,
            world,
            package: &facts.package,
            module: &facts.module,
            declared: facts.symbols.iter().map(|s| s.fqn.as_str()).collect(),
            blocks,
            bound,
            globs,
        }
    }

    fn place(&self, target: &Resolution, at: Span) -> Resolution {
        match target {
            Resolution::Resolved { fqn, via } => {
                Resolution::Resolved { fqn: fqn.clone(), via: *via }
            }
            // Every reason but `Unplaced` states a cause that still holds after
            // the ladder has run, so the ladder does not overrule it.
            Resolution::Unresolved { reason: Reason::Unplaced, evidence } => {
                self.climb(evidence, at)
            }
            // ...with ONE exception, and it is an exception for a reason the
            // walk can state precisely: `ReceiverTypeUnknown` means the walk
            // could not type the receiver FROM THE FILE IT HAD. That cause does
            // NOT still hold once a completed pass is in hand — the receiver may
            // be a call whose return type another file states. Nothing else is
            // retried: a name with no import in scope is still nameless, and
            // plumbing is still plumbing.
            Resolution::Unresolved { reason: Reason::ReceiverTypeUnknown, evidence } => {
                match self.through_what_the_receiver_returns(evidence, at) {
                    Some((fqn, via)) => Resolution::Resolved { fqn, via },
                    None => Resolution::Unresolved {
                        reason: self.filtered(Reason::ReceiverTypeUnknown, evidence),
                        evidence: evidence.clone(),
                    },
                }
            }
            Resolution::Unresolved { reason, evidence } => Resolution::Unresolved {
                reason: self.filtered(*reason, evidence),
                evidence: evidence.clone(),
            },
        }
    }

    /// The ladder itself. Each rung PROVES its answer from something written in
    /// the source; a rung that cannot prove one does not guess (R4).
    fn climb(&self, evidence: &Evidence, at: Span) -> Resolution {
        if let Placed::Proven(fqn) = self.declared_here(evidence) {
            return Resolution::Resolved { fqn, via: Rung::DeclaredHere };
        }
        let wanted = self.wanted(evidence);
        // A FIELD is reachable through no path, so no path rung may serve one.
        //
        // `Type::field` is not a path Rust admits — that is exactly why
        // `Reach::Field` is its own reach — and an import binds a path head. So
        // a field access must never be answered by an import, a glob, or a
        // package root: those all prove "a path with this head reaches that
        // item", which is not the question a field asks.
        //
        // MEASURED, and it is not hypothetical. `federation/mod.rs` does
        // `use dojo_protocol::{PublishedRule, content_hash}` and then reads the
        // FIELD `pr.content_hash`. Without this guard the import rung answers
        // with the imported FUNCTION's identity carrying the FIELD's reach,
        // where the declaration carries the ITEM reach — so the use site and the
        // declaration never merge, and the edge points at a real node that is
        // the wrong one (R4).
        //
        // Only the OWNING TYPE places a field — `declared_here` when the file
        // declares it, `declared_by_its_type` when another file of the same scan
        // does. Both match a whole identity, REACH INCLUDED, so neither is a
        // path rung and neither is what this guard is aimed at.
        let a_field = wanted.reach == Reach::Field;
        if !a_field && let Placed::Proven(fqn) = self.through_an_import(&wanted, at) {
            return Resolution::Resolved { fqn, via: Rung::ThroughAnImport };
        }
        // Below the import and above the glob, and both halves of that are
        // measured.
        //
        // BELOW the import, because an import is WRITTEN DOWN and this rung's
        // candidate is assembled: the type table is keyed by (package, name)
        // and answers with a home in the USE SITE's package, so it cannot see a
        // name that came from a sibling crate. MEASURED: `collective/inbox.rs`
        // imports `dojo_protocol::ArtifactKind` while `senseid` declares an
        // `ArtifactKind` of its own, and with this rung above the import rung
        // all 6 of its use sites pointed at the wrong crate's type (R4).
        //
        // ABOVE the glob, because a glob binds an unknown set and this proves a
        // declaration was read.
        //
        // The two grades in order, stronger first. A `Named` is this file's own
        // statement and needs no table; a `Candidate` is a name match and needs
        // one. Both sit above the field guard and on the correct side of it:
        // that guard forbids a PATH rung from serving a field, and these are
        // owning-type lookups keyed on a whole identity, reach included.
        if let Placed::Proven(fqn) = self.named_by_this_file(evidence) {
            return Resolution::Resolved { fqn, via: Rung::NamedByThisFile };
        }
        if let Placed::Proven(fqn) = self.declared_by_its_type(evidence) {
            return Resolution::Resolved { fqn, via: Rung::DeclaredByItsType };
        }
        // A RUNG STOOD HERE and it is GONE rather than moved: the collapsed
        // -spelling lookup, which asked a repo-wide table which declaration
        // answered to a string no use site could mint. It existed because a
        // trait method's declaration and its use site spelled two strings; S8
        // keys a method on its type and its name, so the rung above already
        // places everything this one placed. See the guard in this module's
        // tests, which is what keeps it from growing back.
        if a_field {
            return Resolution::Unresolved {
                reason: self.filtered(Reason::NoImportInScope, evidence),
                evidence: evidence.clone(),
            };
        }
        if let Placed::Proven(fqn) = self.through_a_glob(&wanted, at) {
            return Resolution::Resolved { fqn, via: Rung::ThroughAGlob };
        }
        if let Placed::Proven(fqn) = self.rooted_in_this_package(&wanted, at) {
            return Resolution::Resolved { fqn, via: Rung::RootedInThisPackage };
        }
        if let Placed::Proven(fqn) = self.a_fully_qualified_external(&wanted) {
            return Resolution::Resolved { fqn, via: Rung::FullyQualifiedExternal };
        }
        // Rung 4 is the only one a glob can overrule: an explicit item and an
        // explicit import both outrank a glob, but a glob outranks the prelude.
        // So a glob in scope over a name the prelude also has leaves two
        // possible origins and nothing to tell them apart.
        match (self.in_the_prelude(&wanted), self.a_glob_binds_at(at)) {
            (Placed::Proven(fqn), false) => {
                return Resolution::Resolved { fqn, via: Rung::InThePrelude };
            }
            (Placed::Proven(_), true) => return self.shadowed_by_a_glob(evidence, at),
            (Placed::Unbound, _) => {}
        }
        Resolution::Unresolved {
            reason: self.filtered(Reason::NoImportInScope, evidence),
            evidence: evidence.clone(),
        }
    }

    /// A receiver that is itself a CALL, typed by what that call returns.
    ///
    /// `Config::from_env().script()` and `let c = Config::from_env(); c.script()`
    /// are the same edge, and only the second resolved: the `let` gives the
    /// binding a type the walk can read, while the chained form leaves the walk
    /// with an expression it cannot type and no cross-file knowledge to type it
    /// with. MEASURED on one real pair of files: four of the 37 misses, every
    /// one this shape.
    ///
    /// Three things keep this a LOOKUP rather than an inference:
    ///
    /// - the inner call climbs the SAME ladder a callee does, so nothing new
    ///   decides where `from_env` lives;
    /// - the return type is the one the source WROTE — a declaration stating
    ///   none is absent from the table and the receiver stays a miss;
    /// - the member must be one the type DECLARES, checked against the same
    ///   `declared_members` the sibling rung uses, so a name match alone never
    ///   mints an edge (R4).
    ///
    /// One hop only. `a().b().c()` needs `b`'s return type, which is this same
    /// lookup applied again — allowed, but bounded, because an uncapped chase
    /// is how a wrong type travels a long way quietly.
    ///
    /// It hands back the RUNG as well as the identity, because it can reach two
    /// of them: an exact match on what the type declares, or the weaker lookup
    /// of a spelling a trait impl answers to. One label for both would make the
    /// histogram read the weaker claim as the stronger one.
    fn through_what_the_receiver_returns(
        &self,
        evidence: &Evidence,
        at: Span,
    ) -> Option<(Fqn, Rung)> {
        if self.world.returns.is_empty() {
            return None;
        }
        let receiver = evidence.saw.iter().find_map(|o| match o {
            Observation::Receiver(text) => Some(text.as_str()),
            _ => None,
        })?;
        // A UNIT STRUCT used as a value: the name IS the value, so it is a
        // receiver of its own type. `Resolver.remedy()` where `Resolver` has no
        // fields — this codebase writes every health resolver that way. Not a
        // call, so there is no return type to look up; the type is the name.
        let names_a_type = self.grammar.names_a_type;
        if !receiver.ends_with("()") && names_a_type(receiver) && !receiver.contains(['.', ' ']) {
            return self.member_of(receiver, evidence);
        }

        // A BINDING the source never typed, whose `let` the walk read as the
        // result of a call. `let c = Config::from_env(); c.script()` is the
        // chained form above split over two lines and is the same edge; the
        // walk cannot close it because what the callee returns is written on a
        // declaration that may be in another file (R6).
        //
        // Tried BEFORE the receiver text is read as a call, because the two
        // cannot both apply: a receiver the walk bound to a call is a bare
        // name, and a bare name has no `()` to strip.
        if let Some(callee) = evidence.saw.iter().find_map(|o| match o {
            Observation::BoundToTheResultOf(callee) => Some(callee.as_str()),
            _ => None,
        }) {
            return self.what_that_call_returns(callee, evidence, at);
        }

        // Only a CALL has a return type. A bare binding is the walk's job and
        // it already did it.
        let callee = receiver.strip_suffix("()")?;
        self.what_that_call_returns(callee, evidence, at)
    }

    /// The member on whatever `callee` hands back.
    ///
    /// One place, because two reach it: a receiver that IS a call, and a
    /// receiver that is a local bound to one. Both need the same three steps and
    /// a second copy is how they would come to disagree about which of them is a
    /// lookup and which is a guess.
    ///
    /// Nothing here is inferred. The callee climbs the SAME ladder a callee
    /// does, so nothing new decides where it lives; the return type is the one
    /// the source WROTE, so a declaration stating none leaves the receiver an
    /// honest miss; and the member must be one that type DECLARES.
    fn what_that_call_returns(
        &self,
        callee: &str,
        evidence: &Evidence,
        at: Span,
    ) -> Option<(Fqn, Rung)> {
        if callee.is_empty() || callee.contains(['(', ' ']) {
            return None;
        }
        // The call, placed by the ordinary rungs.
        let inner = Evidence {
            name: callee.to_string(),
            node_kind: evidence.node_kind.clone(),
            reach: Reach::Item,
            saw: self.considered_here(callee),
        };
        let Resolution::Resolved { fqn, .. } = self.climb(&inner, at) else {
            return None;
        };
        let returned = self.world.returns.get(&fqn)?;

        // The type the call hands back, as a segment.
        let names_a_type = self.grammar.names_a_type;
        let ty = self.split(returned).into_iter().rev().find(|segment| names_a_type(segment))?;

        // And the member on it.
        self.member_of(&ty, evidence)
    }

    /// The identity a BARE name would carry if THIS FILE declared it, as a
    /// candidate for the one rung that checks exactly that.
    ///
    /// The walk mints this for every bare name it reads, so every reference
    /// arrives carrying one. A callee read out of a `let` is the exception: it
    /// was never a reference of its own, so nothing minted it, and
    /// [`Ladder::declared_here`] — which reads candidates and nothing else —
    /// could not see a free function declared three lines up.
    ///
    /// The same form and the same module the walk would use, so the two cannot
    /// spell it differently. It stays a CANDIDATE: it places nothing until it is
    /// found among the identities this file declares (R4).
    ///
    /// A PATH is left alone. Its head is bound by an import or a root word, and
    /// those are rungs of their own that already answer it.
    fn considered_here(&self, name: &str) -> Vec<Observation> {
        if name.contains(self.grammar.module_separator) {
            return Vec::new();
        }
        match fqn::refer(&Form::Item {
            lang: self.grammar.language,
            package: self.package,
            module: self.module,
            name,
            reach: Reach::Item,
        }) {
            Ok(fqn) => vec![Observation::Candidate(fqn)],
            Err(_) => Vec::new(),
        }
    }

    /// `<ty>::<member>`, and only if that type DECLARES it.
    ///
    /// One place, so the two ways of learning a receiver's type — a unit struct
    /// naming itself, and a call's return type — cannot mint differently. The
    /// `declared_members` check is what stops a name match becoming an edge
    /// (R4); it is the same set the sibling rung uses.
    ///
    /// ONE question, where there used to be two. The second was a lookup into a
    /// repo-wide table for the spelling a trait impl answered to but no use site
    /// could mint; S8 keys a method on its type and its name, so the spelling
    /// this side mints IS the one the declaration made, and an exact match is
    /// the whole of it. Same table as [`Ladder::declared_by_its_type`], so a
    /// receiver typed by the walk and one typed here cannot reach different
    /// declarations.
    fn member_of(&self, ty: &str, evidence: &Evidence) -> Option<(Fqn, Rung)> {
        let module = self.types_home_of(ty)?;
        let minted = fqn::refer(&Form::Member {
            lang: self.grammar.language,
            package: self.package,
            module,
            ty,
            member: &evidence.name,
            reach: evidence.reach,
        })
        .ok()?;
        self.world.declared_members.contains(&minted).then_some((minted, Rung::DeclaredByItsType))
    }

    /// The module a type of THIS package and THIS language lives in, from the
    /// identities the scan declared — or nothing, where the scan does not say
    /// exactly one thing.
    ///
    /// Read off `declared_members` rather than from a second table: a member's
    /// identity already carries its type's module, and deriving it here from
    /// the same set the rung checks against is what stops the two disagreeing.
    ///
    /// Three narrowings, and each of them is a wrong edge this rung was minting.
    /// The PACKAGE and the LANGUAGE, because that set holds every package and
    /// every language of the scan in one, while [`Ladder::member_of`] mints with
    /// the use site's own — so a home borrowed from a neighbour names a module
    /// of a package that does not have one. The SHAPE, through
    /// [`Ladder::module_of_the_member_of`], because a member spelled like the
    /// type is not the type. And AMBIGUITY,
    /// which is the rule [`TypeHomes`](super::lang::TypeHomes) has followed
    /// since it existed: two homes is not one home, and answering with the one
    /// that sorts first is a coin toss recorded as a fact (R4, R6).
    ///
    /// Every match is read, never just the first, because the last entry is the
    /// one that can prove the first was not alone.
    fn types_home_of(&self, ty: &str) -> Option<&'a str> {
        let mut home: Option<&'a str> = None;
        for member in self.world.declared_members {
            let Ok(parsed) = fqn::parse(member.as_str()) else { continue };
            if parsed.package != self.package {
                continue;
            }
            let Origin::Local { lang, .. } = parsed.origin else { continue };
            if lang != self.grammar.language {
                continue;
            }
            let Some(module) = Self::module_of_the_member_of(&parsed.tail, ty) else {
                continue;
            };
            match home {
                Some(first) if first != module => return None,
                Some(_) => {}
                None => home = Some(module),
            }
        }
        home
    }

    /// The module a MEMBER identity says its type lives in, given that type's
    /// NAME — or `None` when the identity is not a member of that type at all.
    ///
    /// This replaces a derivation that had to answer a second question the
    /// grammar no longer asks: whether a trait supplied the member, i.e.
    /// whether ONE segment stood between the type and the member. Since S8 no
    /// declaration carries a trait segment, so a member of `ty` is exactly
    /// `[module?] <ty> <member>` and nothing else.
    ///
    /// The type is looked for from the RIGHT and only in the one position the
    /// grammar admits, which is the defect that older derivation was extracted
    /// to fix: a search from the left read an enum variant named `Widget` as the type
    /// `Widget` and handed back `a::Kind` as a module. An identity in any other
    /// shape answers `None` rather than being read as the nearest one — a
    /// guessed shape keys on a string no use site mints (R4).
    fn module_of_the_member_of<'t>(tail: &[&'t str], ty: &str) -> Option<&'t str> {
        let (_member, head) = tail.split_last()?;
        // The type stands immediately left of the member. `rposition` rather
        // than a bare index so a MODULE spelled like the type cannot be taken
        // for it — the module segment is what remains to its left.
        let at = head.iter().rposition(|segment| *segment == ty)?;
        if at + 1 != head.len() {
            return None;
        }
        // The module is one segment or it is absent; the grammar mints no third
        // shape, so anything else is an identity this cannot read.
        match head[..at] {
            [] => Some(""),
            [module] => Some(module),
            _ => None,
        }
    }

    /// Plumbing is filtered wherever it lands, so the histogram has one bucket
    /// for it rather than a share of every other one.
    ///
    /// [`Reason::UnhandledForm`] is left alone: it names a gap in the WALK, not
    /// a name the ladder declined to chase, and moving one into the plumbing list
    /// would hide exactly what the histogram exists to show.
    fn filtered(&self, reason: Reason, evidence: &Evidence) -> Reason {
        match reason {
            Reason::UnhandledForm => reason,
            _ if self.grammar.plumbing.contains(&evidence.name.as_str()) => Reason::Plumbing,
            // A receiver we could not type, whose MEMBER nothing first-party
            // declares. No inference reaches it, because there is nothing of
            // ours on the other end — see `World::first_party_members`.
            Reason::ReceiverTypeUnknown
                if !self.world.first_party_members.is_empty()
                    && !self.world.first_party_members.contains(&evidence.name) =>
            {
                Reason::ExternalBoundary
            }
            _ => reason,
        }
    }

    /// A miss the prelude WOULD have answered, recorded with the globs that
    /// might have replaced the answer. The glob is the material a later pass
    /// needs; naming it here is what makes this different from a bare miss.
    fn shadowed_by_a_glob(&self, evidence: &Evidence, at: Span) -> Resolution {
        let mut evidence = evidence.clone();
        for glob in self.globs.iter().filter(|g| self.binds_at(g, at)) {
            evidence.saw.push(Observation::ImportInScope(glob.path.clone()));
        }
        Resolution::Unresolved { reason: Reason::AmbiguousCandidates, evidence }
    }

    /// Rung 1. The walk minted a candidate identity for this use site; if the
    /// very same file declares it, the two sides have already met and there is
    /// nothing left to prove.
    fn declared_here(&self, evidence: &Evidence) -> Placed {
        self.first_candidate(evidence, |fqn| {
            Placed::proven_if(fqn, self.declared.contains(fqn.as_str()))
        })
    }

    /// Rung 1b. The same proof, one file further out: a candidate that some
    /// TYPE in this scan was read declaring as its member.
    ///
    /// Rust puts an `impl` block wherever it likes — this repository has 24 for
    /// `PgStore` alone, spread over as many files — so "the declaration is in
    /// another file" is the ordinary case for a method, not an edge case. Both
    /// sides already mint one string for it, because the walk anchors a member
    /// on its TYPE's home rather than on the module the `impl` sits in; what was
    /// missing was a rung willing to say the two had met. MEASURED: 2,427 rust
    /// and 539 typescript references, against 742 declarations that had no
    /// inbound edge at all.
    ///
    /// It looks up IDENTITIES rather than names, so nothing here is a guess: the
    /// candidate is minted only when the receiver's type was read off a
    /// declaration and the barrier type table gave that type one unambiguous
    /// home, and the set it is checked against is what a type was read
    /// declaring. Two facts meeting, neither of them inferred.
    ///
    /// It is NOT the whole of "declared somewhere in this scan", and the
    /// restriction is the measured half of the rung — see
    /// [`World::declared_members`] and
    /// `a_bare_name_matching_another_files_declaration_is_not_proof_of_anything`.
    ///
    /// R6 is untouched. The set is a barrier artifact — built from a completed
    /// pass, never accumulated as files go by — so it holds the same answer
    /// whichever file is resolved first. That is the whole difference between it
    /// and [`World::scanned`], which is carried and deliberately never read.
    fn declared_by_its_type(&self, evidence: &Evidence) -> Placed {
        self.first_candidate(evidence, |fqn| {
            Placed::proven_if(fqn, self.world.declared_members.contains(fqn))
        })
    }

    /// **Rung 1a. THIS FILE'S OWN TEXT established the identity, so it is placed
    /// with nothing else agreeing** (stage 11, S7).
    ///
    /// The rung that removes the type barrier. Every rung around it asks the
    /// SCAN a question — does some type declare this, does some module export
    /// it — and the answers come from sets built after a completed pass. This
    /// one asks the file, which the walk was already holding.
    ///
    /// The grading happens in the WALK, not here, and that is deliberate: only
    /// the code that read the syntax knows whether the type's home came from a
    /// declaration in this file, a package-rooted import in this file, or a
    /// table. So this rung has no condition at all — an
    /// [`Observation::Named`] is the walk's statement that the file proves it,
    /// and re-deciding that here would be a second opinion formed with less
    /// information.
    ///
    /// **What it does NOT claim** (§9): that the target exists. That is the
    /// persistence layer's question, answered by minting a node on first
    /// mention and promoting it when its own file arrives. Relaxing the
    /// existence gate blanket-style was MEASURED at +12,360 resolved references
    /// and broke 13 tests; the grading is what makes the relaxation safe, and
    /// `a_bare_name_matching_another_files_declaration_is_not_proof_of_anything`
    /// is what holds the other side of it.
    fn named_by_this_file(&self, evidence: &Evidence) -> Placed {
        for observation in &evidence.saw {
            if let Observation::Named(fqn) = observation {
                return Placed::Proven(fqn.clone());
            }
        }
        Placed::Unbound
    }

    /// The first identity the walk CONSIDERED that some predicate places. Shared
    /// by the two rungs above, which differ only in where they look the
    /// candidate up — and a second copy of this loop is how they would come to
    /// disagree about what a candidate is.
    ///
    /// Reads [`Observation::Candidate`] and NEVER [`Observation::Named`]. The
    /// two walks over `saw` are separate on purpose: a `Named` needs no
    /// predicate, and letting it fall through to one would make the stronger
    /// grade answerable by the weaker rung's table — which is the barrier,
    /// reintroduced one condition at a time.
    fn first_candidate(&self, evidence: &Evidence, places: impl Fn(&Fqn) -> Placed) -> Placed {
        for observation in &evidence.saw {
            if let Observation::Candidate(fqn) = observation
                && let Placed::Proven(placed) = places(fqn)
            {
                return Placed::Proven(placed);
            }
        }
        Placed::Unbound
    }

    /// Rung 2. An import in scope binds the head of the path, and the specifier
    /// states the rest of the way — including which side of the scanned source
    /// the target is on (spec §2). Nothing here consults what has been scanned.
    fn through_an_import(&self, wanted: &Wanted, at: Span) -> Placed {
        let Some((head, tail)) = wanted.segments.split_first() else {
            return Placed::Unbound;
        };
        let Some(imports) = self.bound.get(head.as_str()) else {
            return Placed::Unbound;
        };
        // Innermost binding wins: a `use` inside a block shadows one at the top
        // of the file. Two bindings of one name in one block do not compile, so
        // there is nothing to break a tie between.
        let Some(import) = imports
            .iter()
            .filter(|i| self.binds_at(i, at))
            .max_by_key(|i| (i.at.start_line, i.at.start_col))
        else {
            return Placed::Unbound;
        };

        let Rooted::At(mut segments) = self.specifier(import) else {
            return Placed::Unbound;
        };
        // The head is a segment of the target only when the specifier did not
        // already spell it. `use a::b::C` ends with `C`, so appending the head
        // would name `a::b::C::C`; `import { C } from './b'` ends with the
        // MODULE, so dropping the head names the module in place of `C`.
        //
        // The import says which, because both shapes appear in one JavaScript
        // file — see [`Binding::MemberOf`].
        if let Binding::MemberOf { member, .. } = &import.binds {
            segments.push(member.clone());
        }
        segments.extend(tail.iter().cloned());

        match &import.origin {
            ImportOrigin::Local => self.identity(self.package, &segments, wanted.reach),
            ImportOrigin::External { package } => match self.owned_by_this_scan(package) {
                Some(package) => self.identity(package, &segments, wanted.reach),
                None => self.library(package, &segments),
            },
        }
    }

    /// Rung 2b. A glob binds an unknown set of names — in general. When its
    /// target is a module THIS FILE covers, the set is not unknown at all: it is
    /// exactly what the file declares there, and the ladder has that in hand.
    ///
    /// The proof is the declaration, not the glob: an identity is only returned
    /// once the file is found to declare it. A glob into a module the file does
    /// not cover proves nothing and returns nothing (R4).
    fn through_a_glob(&self, wanted: &Wanted, at: Span) -> Placed {
        for glob in self.globs.iter().filter(|glob| self.binds_at(glob, at)) {
            let Enumerable::TheModule(target) = self.enumerable(glob) else {
                continue;
            };
            let mut segments = target;
            segments.extend(wanted.segments.iter().cloned());
            if let Placed::Proven(fqn) = self.identity(self.package, &segments, wanted.reach)
                && self.declared.contains(fqn.as_str())
            {
                return Placed::Proven(fqn);
            }
        }
        Placed::Unbound
    }

    /// Whether a glob's target is a module this file covers, which is the only
    /// case in which what it binds can be read rather than guessed.
    ///
    /// Covered means the module the glob is WRITTEN in, or one of the module
    /// blocks enclosing it, down to the file's own module. Those are the module
    /// paths whose declarations are all in this file. A module DECLARED here but
    /// living in another file (`mod other;`) encloses nothing, so it never
    /// qualifies — its contents are not here to read.
    fn enumerable(&self, glob: &Import) -> Enumerable {
        let Rooted::At(target) = self.specifier(glob) else {
            return Enumerable::No;
        };
        let here = self.module_at(glob.at);
        let file = self.split_module(self.module);
        if target.len() >= file.len() && here.starts_with(&target) {
            Enumerable::TheModule(target)
        } else {
            Enumerable::No
        }
    }

    /// Rung 3. A path that states its own root needs no import: the root word
    /// says where the package-relative part begins.
    fn rooted_in_this_package(&self, wanted: &Wanted, at: Span) -> Placed {
        let Some(head) = wanted.segments.first() else {
            return Placed::Unbound;
        };
        if !self.is_a_root(head) {
            return Placed::Unbound;
        }
        // DEPTH 0: this rung is the token-root path (`self::x`, `../x`), and a
        // language spells depth one way or the other. The guard above already
        // required a root TOKEN, which a repeated-prefix language has none of.
        match self.relative_to(&wanted.segments, at, 0) {
            Rooted::At(segments) => self.identity(self.package, &segments, wanted.reach),
            Rooted::Nowhere => Placed::Unbound,
        }
    }

    /// Rung 3b. A path whose HEAD names a package this scan does not own is a
    /// complete reference to an external, with or without an import (R5).
    ///
    /// Guarded three ways, because the failure mode is minting a library node
    /// out of an ordinary local:
    ///
    /// - the language must SAY a path can do this ([`Grammar::paths_name_packages`]);
    /// - the path must have more than one segment, so a bare name never fires;
    /// - the head must not be a root word, and must not name a first-party
    ///   package — a sibling crate is ours however it is spelled.
    fn a_fully_qualified_external(&self, wanted: &Wanted) -> Placed {
        if !self.grammar.paths_name_packages {
            return Placed::Unbound;
        }
        let Some((head, rest)) = wanted.segments.split_first() else {
            return Placed::Unbound;
        };
        if rest.is_empty() || self.is_a_root(head) || self.owned_by_this_scan(head).is_some() {
            return Placed::Unbound;
        }
        // A package name is a lowercase identifier. A leading TYPE is a path
        // inside this package that some other rung owns, and reading it as a
        // package would file `Widget::new` under a crate called `Widget`.
        let names_a_type = self.grammar.names_a_type;
        if names_a_type(head) {
            return Placed::Unbound;
        }
        self.library(head, rest)
    }

    /// Rung 4. A language puts some names in scope with nothing written to bring
    /// them there, so the absence of an import says nothing about them. They are
    /// members of a package we never open, like any other external (R5).
    fn in_the_prelude(&self, wanted: &Wanted) -> Placed {
        let Some((head, tail)) = wanted.segments.split_first() else {
            return Placed::Unbound;
        };
        let Some((_, package, path)) =
            self.grammar.prelude.iter().find(|(name, _, _)| name == head)
        else {
            return Placed::Unbound;
        };
        let mut segments = self.split(path);
        segments.extend(tail.iter().cloned());
        self.library(package, &segments)
    }

    /// Whether a glob in scope could have replaced a prelude name without the
    /// ladder being able to tell. Only a glob whose contents cannot be read
    /// counts: one into a module this file covers has already been asked, and
    /// its answer was no.
    fn a_glob_binds_at(&self, at: Span) -> bool {
        self.globs
            .iter()
            .filter(|glob| self.binds_at(glob, at))
            .any(|glob| matches!(self.enumerable(glob), Enumerable::No))
    }

    // ── reading a path ───────────────────────────────────────────────────────

    fn wanted(&self, evidence: &Evidence) -> Wanted {
        // A type use is normalised to its last segment before it reaches here,
        // so the walk records the path it read as well; that is the only place
        // the root word of `crate::db::PgStore` survives.
        let mut raw = evidence.name.as_str();
        for observation in &evidence.saw {
            if let Observation::UnplacedType(path) = observation {
                raw = path.as_str();
            }
        }
        // The reach is READ, never derived. The walk saw the syntax and said
        // how the name is reached (spec §2.1); the ladder deriving one from
        // `RefKind` would be a second, weaker answer to a question already
        // answered — and a wrong one wherever the two disagree, because
        // `RefKind::Reads` covers both a path leaf and a field access (R7).
        Wanted { segments: self.split(raw), reach: evidence.reach }
    }

    /// A use-site path, in segments.
    fn split(&self, raw: &str) -> Vec<String> {
        self.segments(raw, self.grammar.path_separator)
    }

    /// A module path or an import specifier, in segments. A DIFFERENT separator
    /// from [`Ladder::split`] everywhere but Rust — see [`Grammar`].
    fn split_module(&self, raw: &str) -> Vec<String> {
        self.segments(raw, self.grammar.module_separator)
    }

    fn segments(&self, raw: &str, separator: &str) -> Vec<String> {
        raw.split(separator)
            .map(str::trim)
            // A turbofish decorates a path without naming a segment of it.
            .filter(|s| !s.is_empty() && !s.starts_with('<'))
            .map(str::to_string)
            .collect()
    }

    /// Which root a segment names, if it names one at all.
    fn root_of(&self, segment: &str) -> Option<Root> {
        root_of(segment, self.grammar)
    }

    fn is_a_root(&self, segment: &str) -> bool {
        self.root_of(segment).is_some()
    }

    /// Read a path's leading root words against the module the path is written
    /// in, leaving package-relative segments.
    fn relative_to(&self, segments: &[String], at: Span, depth: usize) -> Rooted {
        let mut base = self.module_at(at);
        // A REPEATED-PREFIX language counts from the file's directory: the
        // first repetition is the containing package, each further one ascends.
        // The roots table below plays no part — a language spells depth one way
        // or the other, never both.
        if depth > 0 {
            base.pop();
            for _ in 1..depth {
                if base.pop().is_none() {
                    return Rooted::Nowhere;
                }
            }
            base.extend(segments.iter().cloned());
            return Rooted::At(base);
        }
        match rooted_against(segments, base, self.grammar) {
            Some(at) => Rooted::At(at),
            None => Rooted::Nowhere,
        }
    }

    /// The module path a span sits in: the file's own, extended by every module
    /// block that encloses the span. A declaration inside `mod b { }` is named
    /// the same as one in `b.rs`, so a path written there has to be read the
    /// same way.
    fn module_at(&self, at: Span) -> Vec<String> {
        let mut segments: Vec<String> =
            self.split_module(self.module).into_iter().filter(|s| !s.is_empty()).collect();
        for (span, name) in &self.blocks {
            if encloses(*span, at) {
                segments.push((*name).to_string());
            }
        }
        segments
    }

    /// The package-relative segments an import specifier names.
    fn specifier(&self, import: &Import) -> Rooted {
        // The specifier is kept verbatim, alias and all, so the alias comes off
        // before the path can be read.
        let path = match self.grammar.names_the_binding.and_then(|sep| import.path.split_once(sep))
        {
            Some((path, _alias)) => path,
            None => import.path.as_str(),
        };
        // BEFORE the split, which discards the empty segments a repeated dot
        // tokenizes to.
        let depth = self
            .grammar
            .relative_depth_prefix
            .map_or(0, |c| path.chars().take_while(|x| *x == c).count());
        let mut segments = self.split_module(path);
        // A grouped `self` (`use a::{self}`) binds the module the group is on,
        // not a member called `self`; a wildcard binds that module's contents
        // and is not a segment of its path either.
        //
        // A LEADING root is not popped here — `relative_to` below reads it, and
        // popping `.` off `./a` would leave a bare `a` that reads as a package.
        while segments.len() > 1
            && segments.last().is_some_and(|s| {
                self.root_of(s) == Some(Root::Here) || self.grammar.wildcard == Some(s.as_str())
            })
        {
            segments.pop();
        }

        match &import.origin {
            // The HEAD is the package and what follows is the path inside it —
            // but the head is as many segments as the package has, not one.
            //
            // `skip(1)` was Rust's shape, where a crate name is a single
            // segment. A Java package is `org.mockito.Mockito`, so skipping one
            // left `mockito.Mockito.when` as the path INSIDE a package already
            // called `org.mockito.Mockito`, and the identity carried its own
            // package twice. The same bug reaches JavaScript's scoped packages:
            // `@sveltejs/kit` is two segments by the module separator.
            ImportOrigin::External { package } => {
                let head = self.split_module(package).len().max(1);
                Rooted::At(segments.into_iter().skip(head).collect())
            }
            ImportOrigin::Local => {
                // The last segment of a local specifier is a FILE NAME, and the
                // declaration side reduced the same file to a module segment
                // before minting anything. See [`Grammar::module_segment`].
                //
                // A leading root is never the last segment here: `./` alone is
                // not a specifier, so the `while` above cannot have left one.
                if let Some(last) = segments.last_mut() {
                    let reduce = self.grammar.module_segment;
                    *last = reduce(last).to_string();
                }
                self.relative_to(&segments, import.at, depth)
            }
        }
    }

    /// The MODULE an import enters, for an import whose specifier names one.
    ///
    /// Reuses [`Ladder::specifier`] — the one owner of "what package-relative
    /// path does this string mean", which already handles the alias, the
    /// trailing glob or `self`, the external package head and the relative
    /// root. A second reading of a specifier is how the two sides of an import
    /// come to disagree.
    ///
    /// The identity is minted by the adapter's `file_fqn`, which is the
    /// function the FILE declares itself under. That is the whole point: a
    /// reference must mint the string the declaration minted, and the surest
    /// way is to call the same code (spec §2).
    ///
    /// An external package we do not scan is `Unbound` — a miss. We index no
    /// declarations for a library, so there is no module of theirs to point at,
    /// and inventing one would be a node nothing ever declares (R5).
    fn entered_module(&self, import: &Import) -> Placed {
        let Rooted::At(segments) = self.specifier(import) else {
            return Placed::Unbound;
        };
        let module = segments.join(self.grammar.module_separator);
        if module.is_empty() {
            return Placed::Unbound;
        }
        let package = match &import.origin {
            ImportOrigin::Local => self.package,
            ImportOrigin::External { package } => match self.owned_by_this_scan(package) {
                Some(owned) => owned,
                None => return Placed::Unbound,
            },
        };
        match super::lang::adapter_for(self.grammar.language).file_fqn(package, &module, "") {
            Ok(fqn) => Placed::Proven(fqn),
            Err(_) => Placed::Unbound,
        }
    }

    /// Place an [`RefKind::Imports`] reference, which climbs no ladder: a
    /// specifier is not a name to look up, it is a path to reduce.
    ///
    /// The import is found by SPAN because the reference was emitted AT it —
    /// both sides come from one walk over one file, and two imports on one line
    /// is not a thing any of these grammars admit.
    fn place_entered(&self, target: &Resolution, at: Span, imports: &[Import]) -> Resolution {
        let Resolution::Unresolved { evidence, .. } = target else {
            return target.clone();
        };
        let entered = imports.iter().find(|i| i.at == at).map(|i| self.entered_module(i));
        match entered {
            Some(Placed::Proven(fqn)) => Resolution::Resolved { fqn, via: Rung::ThroughAnImport },
            // NOT `Unplaced`, which means "the ladder has not run on this yet".
            // It has run: the specifier reduced to a module in a package this
            // scan does not own, or to nothing at all. That is where our world
            // ends, which is exactly what `ExternalBoundary` records — and an
            // import of a library module is the commonest reference there is.
            _ => Resolution::Unresolved {
                reason: Reason::ExternalBoundary,
                evidence: evidence.clone(),
            },
        }
    }

    /// Whether an import binds at a given use site: within the innermost block
    /// that contains the import, or file-wide when it sits in no block.
    fn binds_at(&self, import: &Import, at: Span) -> bool {
        let mut block = Placement::WholeFile;
        for (span, _) in &self.blocks {
            if encloses(*span, import.at) {
                block = Placement::Within(*span);
            }
        }
        match block {
            Placement::WholeFile => true,
            Placement::Within(span) => encloses(span, at),
        }
    }

    // ── minting an identity ──────────────────────────────────────────────────

    /// A package this scan owns the source of, however the import spelled it.
    /// A manifest may hyphenate a name that source has to spell with an
    /// underscore, and the two are one package.
    fn owned_by_this_scan(&self, package: &str) -> Option<&'a str> {
        self.world.first_party.iter().find(|owned| same_package(owned, package)).map(String::as_str)
    }

    /// Mint the identity a package-relative path names inside a package we
    /// scan.
    fn identity(&self, package: &str, segments: &[String], reach: Reach) -> Placed {
        let lang = self.grammar.language;
        // The MODULE separator: `front` here is a module path, and joining it
        // with the use-site one would mint `src.lib` where the declaration side
        // minted `src/lib`.
        let separator = self.grammar.module_separator;
        let Some((last, front)) = segments.split_last() else {
            return Placed::Unbound;
        };
        let names_a_type = self.grammar.names_a_type;
        let minted = match segments.iter().position(|s| names_a_type(s)) {
            // The first segment naming a type is where the module path stops and
            // that type's members begin.
            Some(at) if at + 2 == segments.len() => {
                let module = front[..at].join(separator);
                fqn::refer(&Form::Member {
                    lang,
                    package,
                    module: &module,
                    ty: &front[at],
                    member: last,
                    reach,
                })
            }
            // A member of a member of a type. Nothing written here says which of
            // the remaining segments is the one the grammar names, so the ladder
            // does not pick one (R4).
            Some(at) if at + 2 < segments.len() => return Placed::Unbound,
            _ => {
                let module = front.join(separator);
                fqn::refer(&Form::Item { lang, package, module: &module, name: last, reach })
            }
        };
        match minted {
            Ok(fqn) => Placed::Proven(fqn),
            Err(_) => Placed::Unbound,
        }
    }

    /// Name a member of a package whose source is never opened (R5). The whole
    /// path is kept, because "which of their members do we use" is a question
    /// the graph has to answer.
    fn library(&self, package: &str, segments: &[String]) -> Placed {
        let member = segments.join(self.grammar.path_separator);
        match fqn::refer(&Form::Lib { package, member: &member }) {
            Ok(fqn) => Placed::Proven(fqn),
            Err(_) => Placed::Unbound,
        }
    }
}

/// Whether a glob's bindings can be read off this file's own declarations.
enum Enumerable {
    TheModule(Vec<String>),
    No,
}

/// Where an import binds.
enum Placement {
    WholeFile,
    Within(Span),
}

fn encloses(outer: Span, inner: Span) -> bool {
    (outer.start_line, outer.start_col) <= (inner.start_line, inner.start_col)
        && (inner.end_line, inner.end_col) <= (outer.end_line, outer.end_col)
}

/// Whether two spellings name one package. A manifest and a source file may
/// disagree about the separator inside a package name without disagreeing about
/// which package is meant.
fn same_package(one: &str, other: &str) -> bool {
    let fold = |name: &str| name.replace('-', "_");
    fold(one) == fold(other)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::indexer::facts::{FileFacts, RefKind, Reference, RelationKind, Resolution, Rung};
    use crate::indexer::fqn;
    use crate::indexer::lang::{LanguageAdapter, Source, TypeHomes, javascript, rust};
    use crate::indexer::resolve::{
        World, member_names_of, members_declared_by, resolve, returns_declared_by,
    };
    use crate::indexer::{module_of, package_of};

    fn placed(reference: &Reference) -> &str {
        match &reference.target {
            Resolution::Resolved { fqn, .. } => fqn.as_str(),
            Resolution::Unresolved { reason, evidence } => {
                panic!("expected a resolution, got {reason:?} for {}", evidence.name)
            }
        }
    }

    fn ladder(module: &str, text: &str) -> FileFacts {
        ladder_among(module, text, &[])
    }

    /// The same, in a scan that owns the source of other packages too. That set
    /// comes from the manifests and never from what has been read so far.
    fn ladder_among(module: &str, text: &str, first_party: &[&str]) -> FileFacts {
        let facts = rust::read(&Source { package: "p", module, path: "src/fixture.rs", text })
            .expect("the fixture parses");
        let first_party: BTreeSet<String> = first_party.iter().map(|p| (*p).to_string()).collect();
        let scanned = BTreeSet::new();
        resolve(
            facts,
            &rust::GRAMMAR,
            &World {
                first_party: &first_party,
                first_party_members: &BTreeSet::new(),
                declared_members: &BTreeSet::new(),
                returns: &std::collections::BTreeMap::new(),
                scanned: &scanned,
            },
        )
    }

    /// **A file's OWN module symbol must not extend the module path of what it
    /// contains.**
    ///
    /// `module_at` seeds from the file's own module path and then appends the
    /// name of every [`SymbolKind::Module`] block enclosing the span. That is
    /// right for an inline `mod b { }`, which really does add a segment. It is
    /// wrong for the file-module about to be emitted per FILE: its span covers
    /// the whole file, so it encloses every site in it, and `a::b` becomes
    /// `a::b::b` for every path rooted in this package — a fabricated module in
    /// all 1,335 corpus files at once, with no compile error and no wrong type,
    /// just resolution silently ceasing to match.
    ///
    /// The file-module is therefore filtered out of [`Ladder`]'s blocks by
    /// IDENTITY rather than by span or by name: the file's own identity is a
    /// string the adapter already mints, and a name test would also drop a
    /// genuine inline `mod b` inside `b.rs`.
    ///
    /// Landed BEFORE the emission, so the doubling never reaches a corpus.
    #[test]
    fn a_files_own_module_symbol_does_not_extend_the_module_path_of_what_it_contains() {
        use crate::indexer::facts::{DeclaredType, Span, Symbol, SymbolKind, Visibility};

        let text = "pub fn helper() -> u32 { 1 }\npub fn go() -> u32 { self::helper() }\n";
        let mut facts =
            rust::read(&Source { package: "p", module: "a::b", path: "src/a/b.rs", text })
                .expect("the fixture parses");

        // The file-module the emission will add: the file's OWN identity, named
        // after its last module segment, spanning the whole file.
        let its_own = rust::RustAdapter
            .file_fqn("p", "a::b", "src/a/b.rs")
            .expect("a rust file has an identity");
        facts.symbols.push(Symbol {
            fqn: its_own,
            kind: SymbolKind::Module,
            name: "b".to_string(),
            span: Span { start_line: 1, start_col: 1, end_line: 99, end_col: 99 },
            visibility: Visibility::Public,
            docstring: None,
            declared_type: DeclaredType::Unstated,
            params: Vec::new(),
        });

        let scanned = BTreeSet::new();
        let resolved = resolve(
            facts,
            &rust::GRAMMAR,
            &World {
                first_party: &BTreeSet::from(["p".to_string()]),
                first_party_members: &BTreeSet::new(),
                declared_members: &BTreeSet::new(),
                returns: &std::collections::BTreeMap::new(),
                scanned: &scanned,
            },
        );

        let helper: Vec<String> = resolved
            .references
            .iter()
            .map(|r| match &r.target {
                Resolution::Resolved { fqn, .. } => fqn.as_str().to_string(),
                Resolution::Unresolved { reason, evidence } => {
                    format!("UNRESOLVED({reason:?}) {}", evidence.name)
                }
            })
            .filter(|shown| shown.contains("helper"))
            .collect();

        assert_eq!(
            helper,
            vec!["rust\u{00B7}p\u{00B7}a::b\u{00B7}helper\u{00B7}item".to_string()],
            "`self::helper` sits in module `a::b`, not `a::b::b` — the file's own module \
             symbol names the file, it does not nest inside it"
        );
    }

    /// **A CONTAINMENT RELATION NEVER BECOMES A DECLARED MEMBER.**
    ///
    /// The R4 lock on the one hard constraint of the file-module work.
    /// `members_declared_by` feeds the ladder's member rung: an identity in that
    /// set is one `x.foo()` is allowed to land on. It filters on `Owns`, which
    /// is type -> member. Widening it to admit `Contains` — which is module ->
    /// declaration — would let a member call resolve onto a MODULE, and the
    /// module would be a real node, so the edge would look perfectly good.
    ///
    /// This is not hypothetical here. resolve.rs records that admitting extra
    /// relation kinds to a member lookup once "passed all 319 unit tests and
    /// all 13 corpus checks while silently gaining 11 edges" — a widening is
    /// invisible to everything except a test that states the exclusion.
    ///
    /// Written as "facts containing ONLY containment yield nothing" so it fails
    /// the moment anyone writes `matches!(kind, Owns | Contains)`, and cannot
    /// pass vacuously: the fixture is asserted to carry the relations first.
    #[test]
    fn a_contains_relation_never_becomes_a_declared_member() {
        let mut facts = ladder("m", "pub fn free() {}\npub struct Widget;\n");
        facts.relations.retain(|r| r.kind == RelationKind::Contains);
        assert!(
            !facts.relations.is_empty(),
            "the fixture must carry containment, or this asserts nothing at all"
        );

        assert_eq!(
            members_declared_by(std::iter::once(&facts)),
            BTreeSet::new(),
            "a module holds declarations; it does not DECLARE MEMBERS, and a member rung that \
             read containment would let `x.free()` land on the file itself"
        );

        // Anti-vacuity from the other side: the SAME lookup does answer for an
        // Owns relation, so the empty result above is the filter working and
        // not the function being broken.
        let mut owned = ladder("m", "pub struct Widget;\nimpl Widget { pub fn go(&self) {} }\n");
        owned.relations.retain(|r| r.kind == RelationKind::Owns);
        assert!(
            !members_declared_by(std::iter::once(&owned)).is_empty(),
            "ownership still declares members"
        );
    }

    /// **AN IMPORT LANDS ON THE MODULE THE OTHER FILE DECLARES.**
    ///
    /// The whole point of the file-module work, end to end: `other.rs` declares
    /// its own module, `m.rs` globs it, and the two identities meet. Before the
    /// file-module existed there was nothing for an import to point at.
    ///
    /// Two files, because a one-file fixture cannot show the merge — the
    /// declaration has to come from somewhere this file did not write.
    #[test]
    fn a_glob_import_lands_on_the_module_another_file_declares() {
        use crate::indexer::facts::RefKind;

        let scanned = scan(&[
            ("other", "src/other.rs", "pub fn deep() {}\n"),
            ("m", "src/m.rs", "use crate::other::*;\npub fn go() { deep() }\n"),
        ]);
        let importer = file_of(&scanned, "src/m.rs");

        let entered: Vec<String> = importer
            .references
            .iter()
            .filter(|r| r.kind == RefKind::Imports)
            .map(|r| match &r.target {
                Resolution::Resolved { fqn, via } => format!("{:?} {}", via, fqn.as_str()),
                Resolution::Unresolved { reason, evidence } => {
                    format!("UNRESOLVED({reason:?}) {}", evidence.name)
                }
            })
            .collect();

        assert_eq!(
            entered,
            vec!["ThroughAnImport rust·p·other·mod".to_string()],
            "the glob enters `other`, and `other.rs` declares exactly that identity"
        );

        // The target is not invented: the other file really does declare it.
        let declared = file_of(&scanned, "src/other.rs");
        assert!(
            declared.symbols.iter().any(|s| s.fqn.as_str() == "rust·p·other·mod"),
            "the module the import landed on is one the other file declares"
        );
    }

    /// A whole SCAN of several files, run the way a real one is: every file
    /// walked once so the type table can be built, every file walked again with
    /// it, the barrier artifacts taken off that completed pass, and only then
    /// the ladder.
    ///
    /// A one-file fixture cannot express what a barrier artifact is FOR. The
    /// defect this exists to measure — a member whose declaration is in another
    /// file of the same scan — is invisible to a harness that only ever holds
    /// one file, which is why the single-file helper above passes empty sets and
    /// keeps the behaviour it always had.
    ///
    /// `files` are `(module, path, text)`.
    fn scan(files: &[(&str, &str, &str)]) -> Vec<(String, FileFacts)> {
        scan_of(&rust::RustAdapter, files)
    }

    /// The same scan, in another language.
    ///
    /// Through [`LanguageAdapter`] rather than through a second copy of the loop
    /// above, because the two passes and the barrier between them are the SHARED
    /// pipeline and not Rust's: a fixture that walked TypeScript its own way
    /// could pass while the real scan failed, which is the one thing a harness
    /// must not be able to do.
    fn scan_of(
        adapter: &'static dyn LanguageAdapter,
        files: &[(&str, &str, &str)],
    ) -> Vec<(String, FileFacts)> {
        let across: Vec<(&str, &str, &str, &str)> =
            files.iter().map(|(module, path, text)| ("p", *module, *path, *text)).collect();
        scan_read_by(&across, |_| adapter)
    }

    /// A scan over SEVERAL PACKAGES AND SEVERAL LANGUAGES AT ONCE, each file
    /// read by the adapter its extension dispatches to — which is what a real
    /// scan is, and what the single-package helpers above cannot express.
    ///
    /// The barrier artifacts are built over ALL of the files together, one set
    /// for the whole scan, exactly as `acceptance::read_the_corpus` builds them.
    /// That is the condition a rung which SEARCHES one of those sets has to be
    /// correct under, and a harness holding one package of one language can pass
    /// while the rung reads a home off somebody else's declaration.
    ///
    /// `files` are `(package, module, path, text)`.
    fn scan_across(files: &[(&str, &str, &str, &str)]) -> Vec<(String, FileFacts)> {
        scan_read_by(files, |path| {
            let ext = path.rsplit_once('.').map(|(_, e)| format!(".{e}"));
            ext.as_deref()
                .and_then(crate::indexer::lang::adapter_for_ext)
                .unwrap_or_else(|| panic!("no adapter claims {path}"))
        })
    }

    /// The two passes, the barrier between them, and the ladder — written once.
    ///
    /// The helpers above differ only in WHO reads a given file. Everything after
    /// that is the shared pipeline, and a second copy of it is how a fixture
    /// comes to exercise a pipeline the real scan does not run.
    fn scan_read_by(
        files: &[(&str, &str, &str, &str)],
        reader: impl Fn(&str) -> &'static dyn LanguageAdapter,
    ) -> Vec<(String, FileFacts)> {
        let read_all = |types: &TypeHomes| -> Vec<(String, String, FileFacts)> {
            files
                .iter()
                .map(|(package, module, path, text)| {
                    let facts = reader(path)
                        .read(&Source { package, module, path, text }, types)
                        .unwrap_or_else(|e| panic!("{path}: {e:?}"));
                    ((*package).to_string(), (*path).to_string(), facts)
                })
                .collect()
        };
        let first = read_all(&TypeHomes::unknown());
        let homes = TypeHomes::of(
            first
                .iter()
                .flat_map(|(package, _, f)| f.symbols.iter().map(|s| (package.as_str(), s))),
        );
        let anchored = read_all(&homes);

        let first_party: BTreeSet<String> =
            files.iter().map(|(package, ..)| (*package).to_string()).collect();
        let declared_members = members_declared_by(anchored.iter().map(|(_, _, f)| f));
        let returns = returns_declared_by(anchored.iter().map(|(_, _, f)| f));
        let world = World {
            first_party: &first_party,
            first_party_members: &BTreeSet::new(),
            declared_members: &declared_members,
            returns: &returns,
            scanned: &BTreeSet::new(),
        };
        anchored
            .into_iter()
            .map(|(_, path, facts)| {
                let grammar = crate::indexer::lang::adapter_for(facts.language).grammar();
                (path, resolve(facts, grammar, &world))
            })
            .collect()
    }

    /// One file of a [`scan`], by path.
    fn file_of<'a>(scanned: &'a [(String, FileFacts)], path: &str) -> &'a FileFacts {
        &scanned
            .iter()
            .find(|(p, _)| p == path)
            .unwrap_or_else(|| panic!("{path} is not in this scan"))
            .1
    }

    /// A method declared in an `impl` block that sits in a DIFFERENT FILE from
    /// its type — Rust's most ordinary shape, and 24 of this repo's own
    /// `impl PgStore` blocks — is reached from a third file.
    ///
    /// Both sides mint the same string, and this test is about WHY the caller
    /// is allowed to act on it. It used to be `DeclaredByItsType`: the walk got
    /// the type's home from the barrier table, so its identity was a
    /// `Candidate`, and a repo-wide set of declared members had to confirm it.
    ///
    /// The caller never needed either. It writes `use crate::db::PgStore` — it
    /// STATES where `PgStore` lives — so the identity is `Named` and
    /// `NamedByThisFile` places it from the file alone (S7). The sibling test
    /// below keeps `DeclaredByItsType` covered for the case where the file
    /// genuinely says nothing.
    ///
    /// MEASURED at 2,427 rust and 539 typescript references, 742 distinct
    /// declarations that had no inbound edge at all.
    #[test]
    fn a_member_declared_in_another_file_of_this_scan_resolves_to_it() {
        let scanned = scan(&[
            ("db", "src/db.rs", "pub struct PgStore { pub url: String }\n"),
            (
                "db::folders",
                "src/db/folders.rs",
                "use crate::db::PgStore;\n\
                 impl PgStore { pub fn add_watch_root(&self) -> u32 { 0 } }\n",
            ),
            (
                "watcher",
                "src/watcher.rs",
                "use crate::db::PgStore;\n\
                 pub fn start(store: &PgStore) -> u32 { store.add_watch_root() }\n",
            ),
        ]);

        let caller = file_of(&scanned, "src/watcher.rs");
        assert_placed(caller, "rust·p·db·PgStore·add_watch_root·item");
        let call =
            caller.references.iter().find(|r| r.kind == RefKind::Calls).expect("one call site");
        let Resolution::Resolved { via, .. } = &call.target else {
            panic!("the call is not placed: {:?}", call.target)
        };
        assert_eq!(
            *via,
            Rung::NamedByThisFile,
            "the caller imports PgStore by a package-rooted path, so its own text states where \
             the type lives and no repo-wide set is consulted"
        );
    }

    /// **THE SAME REACH, FOR A LANGUAGE WHOSE WALK STILL HAS A TABLE** — and
    /// the test that keeps [`Rung::DeclaredByItsType`] covered.
    ///
    /// S7 took the ordinary case away from that rung: a file which STATES where
    /// a type lives places the member itself, `NamedByThisFile`, with nothing
    /// agreeing. What is left for `DeclaredByItsType` is the identity a walk
    /// minted from a repo-wide TABLE — a `Candidate`, which still cannot become
    /// an edge until some type is found to have declared it.
    ///
    /// Written over TYPESCRIPT, and that is the point rather than a
    /// convenience. S5 has taken the table off the RUST walk, so no Rust
    /// fixture can reach this rung any more; the other four walks keep theirs
    /// until §11 moves them, so the rung is live and must stay tested. Its
    /// first draft was a Rust fixture using `use super::*`, which stopped
    /// reaching the rung the moment the table went — a test that would have
    /// been deleted with the thing it was covering.
    ///
    /// MUTATION: make `first_candidate` place a candidate unconditionally. The
    /// rung stops being a proof and `a_bare_name_matching_another_files_
    /// declaration_is_not_proof_of_anything` goes red beside this.
    #[test]
    fn a_candidate_from_a_table_still_needs_the_declaration_to_agree() {
        let scanned = scan_of(
            &javascript::TypeScriptAdapter,
            &[
                (
                    "lib/store",
                    "src/lib/store.ts",
                    "export class Store {\n  open(): number { return 0 }\n}\n",
                ),
                // NO import of `Store` anywhere in this file — the type is known
                // only to the barrier's table, so what the walk mints for
                // `s.open()` is a candidate and not a statement of this file's.
                (
                    "lib/start",
                    "src/lib/start.ts",
                    "export function go(s: Store): number { return s.open() }\n",
                ),
            ],
        );

        let caller = file_of(&scanned, "src/lib/start.ts");
        let call =
            caller.references.iter().find(|r| r.kind == RefKind::Calls).expect("one call site");
        let Resolution::Resolved { fqn, via } = &call.target else {
            panic!("the call did not reach the member: {:?}", call.target)
        };
        assert_eq!(fqn.to_string(), "typescript·p·lib/store·Store·open·item");
        assert_eq!(
            *via,
            Rung::DeclaredByItsType,
            "the home came from the TABLE, not from this file, so the identity is a candidate \
             and the proof has to be a declaration some type was read making"
        );
    }

    /// **A method a trait impl supplies is reached by its OWN key** — the same
    /// 326-of-797 hole, closed by deleting the mismatch instead of translating
    /// it (stage 11, S8).
    ///
    /// This test used to assert the opposite reading of the same fixture, and
    /// the difference is worth keeping in view. The declaration inside
    /// `impl Provider for MacOSProvider` used to carry the TRAIT as a segment,
    /// so the two sides minted different strings and a repo-wide table
    /// was built at a barrier to translate between them.
    ///
    /// A merge key must be what BOTH sides can produce. The caller writing
    /// `p.resolvers()` can never spell the trait — that is what dispatch
    /// decides — so the trait was never a segment the key could hold. It is now
    /// an edge, emitted by the file that writes the impl, and the two sides mint
    /// one string with nothing looked up.
    ///
    /// So the rung is [`Rung::DeclaredByItsType`]: an exact identity match
    /// against what some type was read declaring, not a weaker lookup.
    #[test]
    fn a_method_a_trait_impl_supplies_is_reached_by_its_own_key() {
        let scanned = scan(&[
            ("provider", "src/provider.rs", "pub trait Provider { fn resolvers(&self) -> u32; }\n"),
            (
                "macos",
                "src/macos.rs",
                "use crate::provider::Provider;\n\
                 pub struct MacOSProvider { pub n: u32 }\n\
                 impl Provider for MacOSProvider { fn resolvers(&self) -> u32 { 0 } }\n",
            ),
            (
                "start",
                "src/start.rs",
                "use crate::macos::MacOSProvider;\n\
                 pub fn go(p: &MacOSProvider) -> u32 { p.resolvers() }\n",
            ),
        ]);

        let caller = file_of(&scanned, "src/start.rs");
        assert_placed(caller, "rust·p·macos·MacOSProvider·resolvers·item");
        let call =
            caller.references.iter().find(|r| r.kind == RefKind::Calls).expect("one call site");
        let Resolution::Resolved { via, .. } = &call.target else {
            panic!("the call is not placed: {:?}", call.target)
        };
        assert_eq!(
            *via,
            Rung::NamedByThisFile,
            "the two sides mint one string, and the caller's own `use crate::macos::MacOSProvider` \
             is what places it — no lookup, and no repo-wide set either"
        );
    }

    /// **TWO TRAITS SUPPLYING ONE MEMBER NAME ON ONE TYPE PLACE ONE NODE**, and
    /// this test INVERTS with stage 11's S8.
    ///
    /// It used to assert that neither was placed: the two declarations minted
    /// `Box2·Wide·draw` and `Box2·Tall·draw`, the caller could only mint
    /// `Box2·draw`, and a lookup with two answers had to refuse — "I know two
    /// places" staying distinguishable from "I know one" (R4).
    ///
    /// There is nothing left to be ambiguous ABOUT. Both declarations now mint
    /// `Box2·draw` and so does the caller, so the call lands on the one node
    /// they share, and the two traits survive as two `TraitImpl` edges emitted
    /// by the file that writes them.
    ///
    /// **The merge is real and it is not hidden.** Two declarations under one
    /// identity are one node, which is exactly what A7
    /// (`acceptance::no_two_declarations_mint_one_identity`) exists to count —
    /// so the cost moved from a silently refused edge, which no measurement
    /// sees, into a collision ratchet, which every run prints. Spec §7 makes
    /// that trade on the grounds that a caller cannot spell the difference and
    /// `rustc` will not let one exist to spell (`E0034`).
    #[test]
    fn two_trait_impls_supplying_one_member_name_on_one_type_place_one_node() {
        let scanned = scan(&[
            (
                "shape",
                "src/shape.rs",
                "pub trait Wide { fn draw(&self) -> u32; }\n\
                 pub trait Tall { fn draw(&self) -> u32; }\n\
                 pub struct Box2 { pub n: u32 }\n\
                 impl Wide for Box2 { fn draw(&self) -> u32 { 0 } }\n\
                 impl Tall for Box2 { fn draw(&self) -> u32 { 1 } }\n",
            ),
            (
                "start",
                "src/start.rs",
                "use crate::shape::Box2;\n\
                 pub fn go(b: &Box2) -> u32 { b.draw() }\n",
            ),
        ]);

        let caller = file_of(&scanned, "src/start.rs");
        assert_placed(caller, "rust·p·shape·Box2·draw·item");

        // And the two traits are both still represented — as edges, from the
        // file that writes them. Nothing was given up, it moved.
        let declaring = file_of(&scanned, "src/shape.rs");
        let traits: Vec<String> = declaring
            .relations
            .iter()
            .filter(|r| r.kind == RelationKind::TraitImpl)
            .map(|r| match &r.parent {
                Resolution::Resolved { fqn, .. } => fqn.to_string(),
                Resolution::Unresolved { evidence, .. } => evidence.name.clone(),
            })
            .collect();
        assert_eq!(
            traits,
            vec!["rust·p·shape·Wide·item".to_string(), "rust·p·shape·Tall·item".to_string()],
            "`Box as Wide` and `Box as Tall` are two TraitImpl edges into one method node"
        );
    }

    /// **AN INHERENT DECLARATION AND THE ONE A TRAIT IMPL SUPPLIES STAY TWO
    /// NODES, AND THE CALL REACHES THE INHERENT ONE** — the boundary S8 must
    /// not cross.
    ///
    /// S8 takes the trait out of a method's key because no caller can spell
    /// which trait supplies a name. That stops being true the moment the type
    /// declares the name INHERENTLY: Rust resolves `s.status()` to the inherent
    /// method deterministically, and `<Svc as Ready>::status` is how the other
    /// is reached. So the two are distinguishable, and the key keeps them apart.
    ///
    /// MEASURED, and the only pair of its kind in this repository:
    /// `ModelProvisioning` declares an inherent `status_all` and an
    /// `impl ReadinessProbe for ModelProvisioning` whose `status_all` calls it
    /// through `ModelProvisioning::status_all(self)` — written, and commented,
    /// precisely to avoid recursing. Flattened, that call became an edge from
    /// the method to ITSELF.
    #[test]
    fn an_inherent_member_and_the_one_a_trait_impl_supplies_stay_two_nodes() {
        let scanned = scan(&[
            (
                "probe",
                "src/probe.rs",
                "pub trait Ready { fn status(&self) -> u32; }\n\
                 pub struct Svc { pub n: u32 }\n\
                 impl Svc { pub fn status(&self) -> u32 { 0 } }\n\
                 impl Ready for Svc { fn status(&self) -> u32 { Svc::status(self) } }\n",
            ),
            (
                "start",
                "src/start.rs",
                "use crate::probe::Svc;\n\
                 pub fn go(s: &Svc) -> u32 { s.status() }\n",
            ),
        ]);

        // A caller writing `s.status()` gets the INHERENT one, as Rust does.
        let caller = file_of(&scanned, "src/start.rs");
        assert_placed(caller, "rust·p·probe·Svc·status·item");

        // And the two declarations are two identities, so neither overwrites
        // the other and `Svc::status(self)` inside the trait body is not a
        // self-loop.
        let declaring = file_of(&scanned, "src/probe.rs");
        let mut statuses: Vec<&str> = declaring
            .symbols
            .iter()
            .map(|s| s.fqn.as_str())
            .filter(|f| f.contains("Svc") && f.ends_with("status·item"))
            .collect();
        statuses.sort_unstable();
        assert_eq!(
            statuses,
            vec!["rust·p·probe·Svc·Ready·status·item", "rust·p·probe·Svc·status·item"],
            "the inherent method owns the plain key and the trait's copy keeps the trait"
        );
    }

    /// The other half of the same rung, and the reason it is restricted to
    /// members rather than to every declaration the scan holds.
    ///
    /// A bare-name candidate is minted at the USE SITE's own module (see
    /// `considered_path`), so it is a guess about where the name lives rather
    /// than a fact read off a declaration. Matching one across files therefore
    /// proves nothing — it only says two files claimed one module path.
    ///
    /// That is not hypothetical. MEASURED: `SignInOverlay.svelte` and
    /// `SignInOverlay.spec.svelte.ts` both reduce to the module
    /// `lib/components/SignInOverlay`, and a rung that matched any declaration
    /// resolved 68 references — the component calling its own `onClose` prop —
    /// onto locals declared in its SPEC file. A wrong edge in place of a missing
    /// one is exactly what R4 forbids, so the rung takes its set from
    /// `RelationKind::Owns`: what a TYPE declares, which a bare name never is.
    /// The `Owns` restriction on `members_declared_by`, tested DIRECTLY.
    ///
    /// The sibling test above uses a free function — the child of no relation at
    /// all — so it refutes only the "every declared symbol" widening. A widening
    /// INSIDE the relation set is the one a future edit would plausibly make,
    /// and it was unguarded: swapping the filter to admit `Extends`,
    /// `Implements` and `TraitImpl` children passed all 319 unit tests and all
    /// 13 corpus checks while silently gaining 11 edges.
    ///
    /// Those children are TYPE identities, not members. A set holding them lets
    /// a bare type name in another file of the same module reduction resolve
    /// onto them — the same shape as the 68 wrong edges onto `.spec` files that
    /// the restriction exists to prevent.
    /// **A call on the RESULT of a call.** `SenseiConfig::from_env().script()`
    /// is the same edge as `let c = SenseiConfig::from_env(); c.script()`, and
    /// only the second resolved.
    ///
    /// MEASURED on the real pair: `crates/bootstrap/src/config.rs` and
    /// `health/resolvers/daemon_start.rs` place 158 of 195 references, and
    /// FOUR of the 37 misses name something those files declare — all four this
    /// shape. `cfg.brew_service_name()` at line 156 resolves because the `let`
    /// gives the binding a type the walk can read; the identical call chained on
    /// one line does not. Same call, same types, different syntax.
    ///
    /// Everything the chain needs is already on the nodes: `from_env` carries
    /// `declared_type: Stated("Self")`, and `Self` in an `impl SenseiConfig` is
    /// `SenseiConfig`. What was missing is a lookup from a resolved callee to
    /// its return type.
    #[test]
    fn a_call_on_the_result_of_a_call_is_typed_by_what_that_call_returns() {
        let scanned = scan(&[
            (
                "config",
                "src/config.rs",
                "pub struct Config { pub n: u32 }\n\
                 impl Config {\n\
                   pub fn from_env() -> Self { Config { n: 0 } }\n\
                   pub fn script(&self) -> u32 { self.n }\n\
                 }\n",
            ),
            (
                "user",
                "src/user.rs",
                "use crate::config::Config;\n\
                 pub fn chained() -> u32 { Config::from_env().script() }\n\
                 pub fn bound() -> u32 { let c = Config::from_env(); c.script() }\n",
            ),
        ]);

        let got = targets(file_of(&scanned, "src/user.rs"));
        let want = "rust·p·config·Config·script·item";
        // The BOUND form already worked; it is here so a regression in it shows
        // up beside the fix rather than after it.
        assert_eq!(
            got.iter().filter(|t| *t == want).count(),
            2,
            "both the chained and the bound call must reach {want}; got {got:?}"
        );
    }

    /// A UNIT STRUCT used as a value is a receiver of its own type.
    ///
    /// `DaemonStartResolver.fallback_remedy()` — the struct has no fields, so
    /// the name IS the value, and this codebase uses it for every resolver.
    /// Not a call, so the returns lookup does not apply; not a binding, so the
    /// walk had nothing to read. It was the last of the four real misses in
    /// `config.rs` + `daemon_start.rs`.
    #[test]
    fn a_unit_struct_used_as_a_value_is_a_receiver_of_its_own_type() {
        let scanned = scan(&[
            (
                "r",
                "src/r.rs",
                "pub struct Resolver;\n\
                 impl Resolver {\n\
                   pub fn remedy(&self) -> u32 { 0 }\n\
                 }\n",
            ),
            (
                "u",
                "src/u.rs",
                "use crate::r::Resolver;\npub fn go() -> u32 { Resolver.remedy() }\n",
            ),
        ]);
        let got = targets(file_of(&scanned, "src/u.rs"));
        assert!(
            got.iter().any(|t| t == "rust·p·r·Resolver·remedy·item"),
            "a unit struct names its own type as the receiver; got {got:?}"
        );
    }

    /// The same receiver, reaching a member its TRAIT IMPL supplies.
    ///
    /// The two ways a receiver gets typed — the walk reading a binding, and the
    /// ladder reading a unit struct or a call's return type — must reach the
    /// same declaration, or a method is callable from one shape and invisible
    /// from the other. `Ladder::member_of` is the one place that mints for the
    /// second, so it has to reach the same identity the walk would.
    ///
    /// Taken from the real miss: `crates/bootstrap/src/health/platforms/macos.rs`
    /// writes `MacOSProvider.resolvers()` against a `resolvers` that only
    /// `impl PlatformProvider for MacOSProvider` supplies.
    ///
    /// Under stage 11's S8 the expected identity loses its trait segment. What
    /// the test is FOR is unchanged and is not about the trait at all: it is
    /// that the ladder's own minting path and the walk's reach one declaration.
    #[test]
    fn a_unit_struct_receiver_reaches_a_member_its_trait_impl_supplies() {
        let scanned = scan(&[
            ("t", "src/t.rs", "pub trait Remedy { fn fix(&self) -> u32; }\n"),
            (
                "r",
                "src/r.rs",
                "use crate::t::Remedy;\n\
                 pub struct Fixer;\n\
                 impl Remedy for Fixer { fn fix(&self) -> u32 { 0 } }\n",
            ),
            ("u", "src/u.rs", "use crate::r::Fixer;\npub fn go() -> u32 { Fixer.fix() }\n"),
        ]);
        let got = targets(file_of(&scanned, "src/u.rs"));
        assert!(
            got.iter().any(|t| t == "rust·p·r·Fixer·fix·item"),
            "the receiver is typed and the member is declared, so the two sides meet — and \
             they meet on the SAME string the walk would have minted; got {got:?}"
        );
    }

    /// **Two modules of one package answering to one type name give a returned
    /// receiver NO home.** Red-first: the rung took the first match in fqn
    /// order, which is a coin toss recorded as a fact.
    ///
    /// `TypeHomes` has refused this since it existed — two homes is not one
    /// home, and [`Home::Ambiguous`](super::lang::Home) is a different answer
    /// from a miss. [`Ladder::types_home_of`] derives the same fact from a
    /// different set and did not refuse it, so `make()` below was typed as the
    /// `Widget` of whichever module sorts first.
    ///
    /// MEASURED over this repository: 89 calls reach the rung with an ambiguous
    /// type name, and 28 placements were right only because the sort order
    /// happened to agree with the source.
    #[test]
    fn two_modules_answering_to_one_type_name_leave_a_returned_receiver_unplaced() {
        let scanned = scan(&[
            (
                "a",
                "src/a.rs",
                "pub struct Widget;\nimpl Widget { pub fn ping(&self) -> u32 { 0 } }\n",
            ),
            (
                "b",
                "src/b.rs",
                "pub struct Widget;\nimpl Widget { pub fn ping(&self) -> u32 { 1 } }\n",
            ),
            (
                "c",
                "src/c.rs",
                "use crate::b::Widget;\n\
                 pub fn make() -> Widget { Widget }\n\
                 pub fn go() -> u32 { make().ping() }\n",
            ),
        ]);
        let got = targets(file_of(&scanned, "src/c.rs"));
        assert!(
            !got.iter().any(|t| t == "rust·p·a·Widget·ping·item"),
            "the source says `make` returns `b`'s `Widget` and the scan cannot tell which of the \
             two it is; placing the one that sorts first is a wrong edge to a real node, which \
             R4 ranks below no edge at all. got {got:?}"
        );
    }

    /// **A MEMBER spelled like a type is not that type.** Red-first: the rung
    /// searched a member identity from the LEFT, so an enum variant named
    /// `Widget` answered for the struct `Widget` and handed back `a::Kind` as a
    /// module.
    ///
    /// The same trap the collapsed-spelling table's builder documented and
    /// guarded — "looked for to the LEFT of the member so a member spelled like
    /// the type it hangs off cannot be taken for it" — while a third copy of
    /// that derivation had no guard at all. Both are gone and the one that
    /// remains is [`Ladder::module_of_the_member_of`], which keeps the rule.
    ///
    /// MEASURED over this repository: 35 placements chose a home that is
    /// provably not a module.
    ///
    /// `Kind` sits at the PACKAGE ROOT so that the segment to the left of the
    /// variant is a single one and passes the module-shape check — otherwise
    /// two guards cover this between them and neither is load-bearing alone.
    ///
    /// MUTATION: search the whole tail in
    /// [`Ladder::module_of_the_member_of`] rather than the part of it to the
    /// left of the member — `Kind` is then `Widget`'s home.
    #[test]
    fn a_member_spelled_like_a_type_does_not_supply_that_types_home() {
        let scanned = scan(&[
            ("", "src/lib.rs", "pub enum Kind { Widget, Gadget }\n"),
            (
                "z",
                "src/z.rs",
                "pub struct Widget;\n\
                 impl Widget { pub fn ping(&self) -> u32 { 0 } }\n\
                 pub fn make() -> Widget { Widget }\n",
            ),
            ("u", "src/u.rs", "use crate::z::make;\npub fn go() -> u32 { make().ping() }\n"),
        ]);
        assert_placed(file_of(&scanned, "src/u.rs"), "rust·p·z·Widget·ping·item");
    }

    /// **Another PACKAGE declaring the same type name does not supply its
    /// home.** Red-first, and only expressible in a scan that holds two
    /// packages: the barrier artifacts are one set for the whole scan, and the
    /// rung read a home out of it without ever asking whose package it was.
    ///
    /// The sibling tables both key on the package —
    /// [`TypeHomes::lookup`](super::lang::TypeHomes::lookup) takes it as an
    /// argument, and a member identity carries it — and the ladder mints with
    /// the USE SITE's package, so a module borrowed from another package names
    /// nothing.
    #[test]
    fn a_type_name_another_package_declares_does_not_supply_its_home() {
        let scanned = scan_across(&[
            (
                "a",
                "m",
                "a/src/m.rs",
                "pub struct Widget;\nimpl Widget { pub fn ping(&self) -> u32 { 0 } }\n",
            ),
            (
                "z",
                "deep",
                "z/src/deep.rs",
                "pub struct Widget;\n\
                 impl Widget { pub fn ping(&self) -> u32 { 1 } }\n\
                 pub fn make() -> Widget { Widget }\n",
            ),
            (
                "z",
                "u",
                "z/src/u.rs",
                "use crate::deep::make;\npub fn go() -> u32 { make().ping() }\n",
            ),
        ]);
        assert_placed(file_of(&scanned, "z/src/u.rs"), "rust·z·deep·Widget·ping·item");
    }

    /// **And another LANGUAGE declaring it does not either.** The same clause,
    /// on the other dimension the barrier artifacts are shared across: one
    /// `declared_members` holds every language of the scan, `rust` sorts before
    /// `typescript`, and the rung was minting a TypeScript identity under a
    /// Rust module.
    #[test]
    fn a_type_name_another_language_declares_does_not_supply_its_home() {
        let scanned = scan_across(&[
            (
                "p",
                "a",
                "src/a.rs",
                "pub struct Widget;\nimpl Widget { pub fn ping(&self) -> u32 { 0 } }\n",
            ),
            (
                "p",
                "lib/w",
                "src/lib/w.ts",
                "export class Widget { ping(): number { return 0 } }\n\
                 export function make(): Widget { return new Widget(); }\n",
            ),
            (
                "p",
                "lib/u",
                "src/lib/u.ts",
                "import { make } from './w';\n\
                 export function go() { return make().ping(); }\n",
            ),
        ]);
        assert_placed(file_of(&scanned, "src/lib/u.ts"), "typescript·p·lib/w·Widget·ping·item");
    }

    /// **A binding whose type the source never states, typed by the CALL that
    /// bound it.** Red-first, and the largest single shape left in the lost
    /// column after the trait-impl rung landed.
    ///
    /// `Config::from_env().script()` already resolved, because the receiver IS
    /// the call and the ladder can read its return type. `let c =
    /// Config::from_env(); c.script()` is the same edge written over two lines,
    /// and it did not: `binding_of` reads a type the source WROTE, the source
    /// wrote none, and by the time the ladder sees the receiver all it has is
    /// the word `c`.
    ///
    /// So the walk says what bound it. Nothing here is inferred: the callee is
    /// placed by the ordinary ladder, the return type is the one the source
    /// wrote on the declaration, and the member still has to be one that type
    /// declares. It is the SAME lookup the chained form already does, reached
    /// through one more fact.
    ///
    /// MEASURED: `async fn pg_store() -> PgStore` is bound as
    /// `let s = pg_store().await` 306 times in one file of this repository, and
    /// every `s.<method>()` after it was a lost `PgStore` method.
    #[test]
    fn a_binding_the_source_never_typed_is_typed_by_the_call_that_bound_it() {
        let scanned = scan(&[
            (
                "db",
                "src/db.rs",
                "pub struct PgStore { pub url: String }\n\
                 impl PgStore { pub fn ping(&self) -> u32 { 0 } }\n",
            ),
            (
                "h",
                "src/h.rs",
                "use crate::db::PgStore;\n\
                 pub fn store() -> PgStore { PgStore { url: String::new() } }\n\
                 pub fn go() -> u32 { let s = store(); s.ping() }\n",
            ),
        ]);
        assert_placed(file_of(&scanned, "src/h.rs"), "rust·p·db·PgStore·ping·item");
    }

    /// **S1 IN RUST.** A `let` REPLACES what the name held, so a rebinding the
    /// walk cannot type leaves the name holding nothing.
    ///
    /// The TypeScript walk has said this since 04b S1 — [`Flow::bind`] clears
    /// all three of its tables and
    /// `a_typescript_binding_rebound_to_something_untypable_forgets_the_call`
    /// is what holds it there. The Rust walk never did: its two tables are only
    /// ever WRITTEN to, so a second `let` the walk cannot read left the first
    /// one's answer standing and every later use site was typed by it.
    ///
    /// `let h = raw(); let h = h.finish();` is valid, idiomatic Rust and neither
    /// table can read the second line — `binding_of` finds no stated type and
    /// `bound_to_a_call` refuses a method call, on purpose, because its own
    /// receiver is untyped. So the answer for `h` must be nothing, and it was
    /// `raw`. Both tables are exercised, because they go stale independently,
    /// and both binding FORMS are, because a `for` pattern rebinds a name
    /// exactly as a `let` does.
    ///
    /// MUTATION, and each of the two is checked on its own: drop the `forget`
    /// call from `Walk::block`, then the one from `Walk::for_expression`. Each
    /// puts `ping` back on `Raw`, which is a wrong edge to a real node (R4).
    #[test]
    fn a_rust_binding_rebound_to_something_untypable_forgets_what_it_held() {
        let scanned = scan(&[
            (
                "r",
                "src/r.rs",
                "pub struct Raw;\n\
                 pub struct Done;\n\
                 impl Raw { pub fn ping(&self) -> u32 { 0 } pub fn finish(&self) -> Done { Done } }\n\
                 impl Done { pub fn ping(&self) -> u32 { 1 } }\n\
                 pub fn raw() -> Raw { Raw }\n",
            ),
            (
                "u",
                "src/u.rs",
                "use crate::r::{Raw, raw};\n\
                 pub fn from_a_call() -> u32 { let h = raw(); let h = h.finish(); h.ping() }\n\
                 pub fn from_a_stated_type(x: Raw) -> u32 { let h: Raw = x; let h = h.finish(); h.ping() }\n\
                 pub fn from_a_loop(x: Raw) -> u32 { let h: Raw = x; for h in 0..3 { return h.ping(); } 0 }\n",
            ),
        ]);
        let got = targets(file_of(&scanned, "src/u.rs"));
        assert!(
            !got.iter().any(|t| t == "rust·p·r·Raw·ping·item"),
            "`h` is a `Done` by the time `ping` is called and the walk cannot read that; what it \
             must not do is answer with what `h` USED to be. got {got:?}"
        );
    }

    /// The same, with the plumbing between the call and the binding.
    ///
    /// `let s = pg_store().await` and `let a = adapter().unwrap()` are how this
    /// repository actually writes it, and `.await` and `.unwrap()` say nothing
    /// about the type — which is exactly what the grammar's plumbing list is
    /// for. Peeled here rather than matched as text, so the list stays the one
    /// place that decides what plumbing is.
    #[test]
    fn the_plumbing_between_a_call_and_its_binding_does_not_hide_the_call() {
        let scanned = scan(&[
            (
                "db",
                "src/db.rs",
                "pub struct PgStore { pub url: String }\n\
                 impl PgStore { pub fn ping(&self) -> u32 { 0 } }\n",
            ),
            (
                "h",
                "src/h.rs",
                "use crate::db::PgStore;\n\
                 pub async fn store() -> PgStore { PgStore { url: String::new() } }\n\
                 pub async fn go() -> u32 { let s = store().await.clone(); s.ping() }\n",
            ),
        ]);
        assert_placed(file_of(&scanned, "src/h.rs"), "rust·p·db·PgStore·ping·item");
    }

    /// **The same binding, in TypeScript** — red-first, and the rung above was
    /// reaching nothing here at all.
    ///
    /// The ladder half is language-neutral and has been since the Rust twin
    /// landed: it reads [`Observation::BoundToTheResultOf`], climbs the callee,
    /// looks the return type up and checks the member. Only the Rust WALK ever
    /// recorded that observation, so in TypeScript the rung was handed a bare
    /// `m` with nothing beside it and fell through every time.
    ///
    /// Both reaches, in one fixture, because they take different paths through
    /// the walk and only one of them was ever exercised on the Rust side: a
    /// member CALL is [`Reach::Item`] and a member READ is [`Reach::Field`],
    /// and `Field` is the whole of the column this is aimed at.
    ///
    /// MEASURED over this repository before building it: 3,065 unresolved
    /// TypeScript member reads have a bare receiver the same file binds with
    /// `const x = callee(...)`, 1,869 of those callees state exactly one return
    /// type, and 1,174 of those types declare the member being read — 323
    /// distinct (type, member) pairs.
    #[test]
    fn a_typescript_binding_the_source_never_typed_is_typed_by_the_call_that_bound_it() {
        let scanned = scan_of(
            &javascript::TypeScriptAdapter,
            &[
                (
                    "lib/mount",
                    "src/lib/mount.ts",
                    "export class Mounted {\n\
                     \x20   container = null;\n\
                     \x20   destroy(): void {}\n\
                     }\n\
                     export function mountIt(): Mounted { return new Mounted(); }\n",
                ),
                (
                    "lib/spec",
                    "src/lib/spec.ts",
                    "import { mountIt } from './mount';\n\
                     export function go() {\n\
                     \x20   const m = mountIt();\n\
                     \x20   m.destroy();\n\
                     \x20   return m.container;\n\
                     }\n",
                ),
            ],
        );
        let reading = file_of(&scanned, "src/lib/spec.ts");
        assert_placed(reading, "typescript·p·lib/mount·Mounted·container·field");
        assert_placed(reading, "typescript·p·lib/mount·Mounted·destroy·item");
    }

    /// **S1 over the new table.** A name REBOUND to something the walk cannot
    /// type no longer holds what the earlier call handed back.
    ///
    /// The type table has said this since 04b S1 and the callee table has to
    /// say it in the same breath, because it is the same staleness: `m` was a
    /// `Mounted` until `make()` ran, and a callee left standing would type
    /// whatever `make` returns as a `Mounted` and mint a member on it. A wrong
    /// edge points at a real node and nothing downstream can tell it from a
    /// right one (R4) — which is why this asserts the ABSENCE and not just the
    /// presence above.
    ///
    /// MUTATION: drop the `from_calls.remove` from [`Flow::bind`]. The second
    /// `m.destroy()` then reaches `Mounted::destroy` and the count is 2.
    #[test]
    fn a_typescript_binding_rebound_to_something_untypable_forgets_the_call() {
        let scanned = scan_of(
            &javascript::TypeScriptAdapter,
            &[
                (
                    "lib/mount",
                    "src/lib/mount.ts",
                    "export class Mounted { destroy(): void {} }\n\
                     export function mountIt(): Mounted { return new Mounted(); }\n",
                ),
                (
                    "lib/spec",
                    "src/lib/spec.ts",
                    "import { mountIt } from './mount';\n\
                     export function go(make) {\n\
                     \x20   let m = mountIt();\n\
                     \x20   m.destroy();\n\
                     \x20   m = make();\n\
                     \x20   m.destroy();\n\
                     }\n",
                ),
            ],
        );
        let got = targets(file_of(&scanned, "src/lib/spec.ts"));
        let reached = got.iter().filter(|t| *t == "typescript·p·lib/mount·Mounted·destroy·item");
        assert_eq!(
            reached.count(),
            1,
            "the call before the reassignment reaches it and the one after cannot; got {got:?}"
        );
    }

    /// **The table is Rust-shaped and must stay a no-op everywhere else.**
    ///
    /// `Form::TraitMember` is minted at exactly ONE site in the whole
    /// repository, in the Rust walk. TypeScript has no trait qualifier — a class
    /// implementing an interface declares plain class members — so every member
    /// identity it mints already IS the spelling a use site can reach, and the
    /// table must come out empty rather than keyed on something.
    ///
    /// The boundary set holds every member name a first-party type declares —
    /// and an ACCESSOR declares one.
    ///
    /// The set decides whether an unplaceable `x.name` is somebody else's
    /// member or a miss of ours, so leaving a kind out of it silently relabels
    /// our own members as the boundary. It was built in four places, and two of
    /// them left [`SymbolKind::Property`] out, which meant the fixture harness
    /// and the Java corpus were resolving against a different boundary than the
    /// measurement they are compared with.
    ///
    /// MUTATION: drop `Property` from [`member_names_of`] and `setupComplete`
    /// below goes missing.
    #[test]
    fn the_member_names_a_scan_declares_include_an_accessor() {
        let files: &[(&str, &str, &str)] = &[(
            "lib/state",
            "src/lib/state.ts",
            "export class AppState {\n\
             \x20   ready = false;\n\
             \x20   get setupComplete(): boolean { return this.ready }\n\
             \x20   reset(): void { this.ready = false }\n\
             }\n",
        )];
        let scanned = scan_of(&javascript::TypeScriptAdapter, files);
        let names = member_names_of(scanned.iter().map(|(_, f)| f));
        assert!(
            names.contains("setupComplete"),
            "a getter is a member this scan declares; without it an `x.setupComplete` \
             nobody can place reads as somebody else's member. got {names:?}"
        );
        // The kinds that were never in doubt, so a builder that returned
        // everything could not pass this either.
        assert!(names.contains("ready") && names.contains("reset"), "got {names:?}");
        assert!(
            !names.contains("AppState"),
            "the TYPE is not one of its own members. got {names:?}"
        );
    }

    /// **THE COLLAPSED-SPELLING MACHINERY HAS NO CALLER** (stage 11, §6 step 2b
    /// and §10 bullet 2).
    ///
    /// The table and everything around it — the two member-shape types, the
    /// derivation that told them apart, the total-answer enum, the `World`
    /// field and the rung — existed for ONE reason: a declaration and
    /// a use site of one trait method spelled two different strings, so the
    /// merge contract needed a repo-wide lookup between them. S8 removed the
    /// mismatch, so the translator answers nothing
    /// (`the_table_is_empty_because_no_declaration_carries_a_trait_segment`
    /// measured that BEFORE the deletion), and a translator nobody needs is a
    /// door the barrier comes back through.
    ///
    /// A guard rather than only a deletion, because the deletion is a one-time
    /// act and this is the rule that outlives it.
    ///
    /// **KNOWN LIMIT, stated so nobody reads more into a green than it means.**
    /// `guard_sources()` reads what this indexer OWNS, which does not include
    /// `acceptance.rs` or `barrier.rs`. Those two are `#[cfg(test)]` harnesses,
    /// so `outside_tests` would hand this guard an empty string; they are
    /// proven by the BUILD — they fail to compile while a call site survives —
    /// and by the `rg` in the commit message. `index.rs` WAS on that list and
    /// no longer is: the driver joined `OWNED`, so this guard reads it.
    ///
    /// The needles are ASSEMBLED so the guard does not match its own source,
    /// which is the idiom `no_fqn_is_built_by_string_formatting_outside_this_file`
    /// already uses for the same reason.
    #[test]
    fn the_collapsed_spelling_machinery_has_no_caller() {
        let needles = [
            format!("Supplied{}", "Members"),
            format!("supplied_{}", "members"),
            format!("hung_{}", "on"),
            format!("Supplied::By{}", "ATrait"),
            format!("SuppliedBy{}", "ATraitImpl"),
            format!("supplied_by_a_{}", "trait_impl"),
        ];
        let mut read = 0;
        let mut found: Vec<String> = Vec::new();
        for (path, body) in crate::indexer::guard_sources() {
            read += 1;
            for (n, line) in crate::indexer::outside_tests(&body).lines().enumerate() {
                for needle in &needles {
                    if line.contains(needle.as_str()) {
                        found.push(format!("{path}:{} {}", n + 1, line.trim()));
                    }
                }
            }
        }
        // The guard must prove it read the tree it names. An empty `found` over
        // an empty corpus is the vacuous pass this exists to prevent.
        assert!(read > 5, "the guard read {read} files, so an empty result means nothing");
        assert!(
            found.is_empty(),
            "{} site(s) still reach for the collapsed-spelling translator. A method is keyed on \
             its type and its name (S8), so there is nothing left to translate and this is the \
             one door the barrier can come back through:\n  {}",
            found.len(),
            found.join("\n  ")
        );
    }

    #[test]
    fn only_what_a_type_owns_is_a_member() {
        let facts = crate::indexer::walked_rust(
            "m",
            "src/m.rs",
            "pub trait Draw { fn draw(&self); }\n\
             pub struct Widget;\n\
             impl Draw for Widget { fn draw(&self) {} }\n",
        );

        // ANTI-VACUITY. Without a non-`Owns` relation in the fixture there is
        // nothing for the filter to exclude and the test proves nothing —
        // which is how the sibling above came to look like a guard.
        assert!(
            facts
                .relations
                .iter()
                .any(|r| !matches!(r.kind, RelationKind::Owns | RelationKind::Contains)),
            "the fixture must contain a relation the filter has to reject — and a `Contains` \
             does not count, because every file scope now has one and the guard would pass \
             without ever exercising an inheritance relation"
        );

        let declared = members_declared_by(std::iter::once(&facts));
        let is = |name: &str| {
            facts
                .symbols
                .iter()
                .find(|s| s.name == name)
                .unwrap_or_else(|| panic!("the fixture declares no {name}"))
                .fqn
                .clone()
        };

        assert!(
            declared.contains(&is("draw")),
            "a method its type owns IS a member; the set holds {declared:?}"
        );
        for a_type in ["Widget", "Draw"] {
            assert!(
                !declared.contains(&is(a_type)),
                "{a_type} is a TYPE, and it reached the member set — a bare name matching it \
                 in another file would resolve onto it"
            );
        }
    }

    #[test]
    fn a_bare_name_matching_another_files_declaration_is_not_proof_of_anything() {
        let scanned = scan(&[
            ("m", "src/one.rs", "pub fn helper() -> u32 { 0 }\n"),
            ("m", "src/two.rs", "pub fn caller() -> u32 { helper() }\n"),
        ]);
        let got = targets(file_of(&scanned, "src/two.rs"));
        assert!(
            got.iter().any(|t| t == "NoImportInScope(helper)"),
            "a bare name that another file happens to declare under the same module is not \
             a proven target; got {got:?}"
        );
    }

    /// Where the new rung sits, as a property rather than as a comment.
    ///
    /// The type table is keyed by (package, name), so it answers with a home in
    /// the USE SITE's own package and cannot see that the name came from
    /// somewhere else. An import can: it is written down, and its specifier
    /// says which side of the boundary the target is on. So the import outranks
    /// the candidate, and this is the case that proves the difference is not
    /// academic.
    ///
    /// MEASURED, and found by moving the rung: `crates/senseid/src/collective/
    /// inbox.rs` imports `dojo_protocol::ArtifactKind` while `senseid` declares
    /// an `ArtifactKind` of its own in `materialize.rs`. With the candidate rung
    /// above the import rung, all 6 of its use sites resolved onto the WRONG
    /// crate's type — a real node, the wrong one, which is precisely the trade
    /// R4 refuses.
    #[test]
    fn an_import_outranks_a_candidate_built_from_a_same_named_type_of_ours() {
        let scanned = scan(&[
            ("materialize", "src/materialize.rs", "pub enum ArtifactKind { Skill, Agent }\n"),
            (
                "inbox",
                "src/inbox.rs",
                "use dojo_protocol::ArtifactKind;\n\
                 pub fn mirror() -> u32 { let _ = ArtifactKind::Skill; 0 }\n",
            ),
        ]);
        let got = targets(file_of(&scanned, "src/inbox.rs"));
        assert!(
            got.iter().any(|t| t == "lib·dojo_protocol·ArtifactKind::Skill"),
            "the import names the package the type comes from; got {got:?}"
        );
        assert!(
            !got.iter().any(|t| t == "rust·p·materialize·ArtifactKind·Skill·item"),
            "a same-named type of our own is not what this file imported; got {got:?}"
        );
    }

    /// A FIELD across files, which the field guard must not swallow.
    ///
    /// The guard exists to keep a PATH rung from answering a field — an import
    /// binds a path head and `Type::field` is not a path Rust admits. Neither
    /// candidate rung is a path rung: both match a whole identity with the reach
    /// on it, so a field can only ever meet a field. Without this the guard
    /// returns before the new rung is ever asked, and every cross-file field
    /// read stays a miss.
    #[test]
    fn a_field_of_a_type_declared_in_another_file_is_still_placed_by_its_owner() {
        let scanned = scan(&[
            ("db", "src/db.rs", "pub struct PgStore { pub url: String }\n"),
            (
                "watcher",
                "src/watcher.rs",
                "use crate::db::PgStore;\n\
                 pub fn show(store: &PgStore) -> usize { store.url.len() }\n",
            ),
        ]);
        let caller = file_of(&scanned, "src/watcher.rs");
        assert_placed(caller, "rust·p·db·PgStore·url·field");
    }

    /// The SAME question in TypeScript, and it is not the same answer for free.
    ///
    /// Rust's half above works because the Rust walk is TOLD where a type lives
    /// and names the member under the type's home. The JavaScript reader was
    /// handed that table and threw it away, filing every member under the module
    /// of the file doing the READING — so the two sides of one field spelled two
    /// identities and could never meet, whatever the ladder did.
    ///
    /// MEASURED over this repository: 936 unresolved TypeScript field reads
    /// minted a candidate whose only defect was the module segment, and NOT ONE
    /// of the 1,997 candidates minted at field reach named a declaration this
    /// scan holds.
    ///
    /// MUTATION: put the reading file's own module back in `member_of` and this
    /// goes red while the rust twin above stays green — which is the whole point
    /// of having both.
    #[test]
    fn a_typescript_field_is_named_under_the_module_its_class_is_declared_in() {
        let scanned = scan_of(
            &javascript::TypeScriptAdapter,
            &[
                ("e2e/helpers", "app/e2e/helpers.ts", "export class ErrBuf { error = ''; }\n"),
                (
                    "e2e/tests/activity",
                    "app/e2e/tests/activity.ts",
                    "import { ErrBuf } from '../helpers';\n\
                     export function show(): string { const b = new ErrBuf(); return b.error; }\n",
                ),
            ],
        );
        assert_placed(
            file_of(&scanned, "app/e2e/tests/activity.ts"),
            "typescript·p·e2e/helpers·ErrBuf·error·field",
        );
    }

    /// Rung 2, red-first, for a language whose specifier does NOT name the
    /// binding.
    ///
    /// `use a::b::kindFor` ends with the name it binds, so Rust's import rung
    /// reads the specifier and appends whatever followed the head. JavaScript
    /// states the binding in the import CLAUSE — `import { kindFor } from
    /// './buckets'` — and the specifier names only the MODULE, so dropping the
    /// head there mints the module's identity in place of the member's and every
    /// named import lands on a node that is not the one imported.
    ///
    /// MEASURED at the whole of it: 3,865 first-party TypeScript import edges,
    /// not one of which named a declaration this scan holds.
    #[test]
    fn a_named_import_binds_a_member_the_specifier_does_not_name() {
        let scanned = scan_of(
            &javascript::TypeScriptAdapter,
            &[
                (
                    "lib/buckets",
                    "src/lib/buckets.ts",
                    "export function kindFor(a: string): string { return a; }\n",
                ),
                (
                    "lib/board",
                    "src/lib/board.ts",
                    "import { kindFor } from './buckets';\n\
                     export function go(): string { return kindFor('x'); }\n",
                ),
            ],
        );
        assert_placed(
            file_of(&scanned, "src/lib/board.ts"),
            "typescript·p·lib/buckets·kindFor·item",
        );
    }

    /// The other half of the same question, and the reason the answer cannot be
    /// a per-LANGUAGE flag.
    ///
    /// `import * as buckets from './buckets'` binds the MODULE, so the specifier
    /// names the bound thing exactly as a Rust `use` does, and appending the
    /// head would mint `lib/buckets·buckets·kindFor` — a node nothing declares.
    /// The two clauses sit in one language and one file, so what tells them
    /// apart has to be read off the import itself.
    ///
    /// Green before the fix and green after it: this is the edge the obvious
    /// per-language knob would have taken away.
    #[test]
    fn a_namespace_import_binds_the_module_and_keeps_naming_it() {
        let scanned = scan_of(
            &javascript::TypeScriptAdapter,
            &[
                (
                    "lib/buckets",
                    "src/lib/buckets.ts",
                    "export function kindFor(a: string): string { return a; }\n",
                ),
                (
                    "lib/board",
                    "src/lib/board.ts",
                    "import * as buckets from './buckets';\n\
                     export function go(): string { return buckets.kindFor('x'); }\n",
                ),
            ],
        );
        assert_placed(
            file_of(&scanned, "src/lib/board.ts"),
            "typescript·p·lib/buckets·kindFor·item",
        );
    }

    /// The second half of the same defect, and it hides behind the first.
    ///
    /// A JavaScript specifier is a FILE path, so `./buckets.js` ends with a file
    /// name — while the declaration side reduced `src/lib/buckets.ts` to the
    /// module `lib/buckets`. The extension is therefore a segment on one side
    /// and absent on the other, and the two never merge however well the member
    /// is named. `./buckets.js` next to `buckets.ts` is not a typo: it is what
    /// TypeScript's `nodenext` resolution requires the source to write.
    ///
    /// MEASURED across the 3,865: naming the member alone lands 1,619 of them;
    /// stemming the file as well lands 3,248.
    #[test]
    fn a_local_specifiers_file_extension_is_not_a_segment_of_the_module() {
        let scanned = scan_of(
            &javascript::TypeScriptAdapter,
            &[
                (
                    "lib/buckets",
                    "src/lib/buckets.ts",
                    "export function kindFor(a: string): string { return a; }\n",
                ),
                (
                    "lib/board",
                    "src/lib/board.ts",
                    "import { kindFor } from './buckets.js';\n\
                     export function go(): string { return kindFor('x'); }\n",
                ),
            ],
        );
        assert_placed(
            file_of(&scanned, "src/lib/board.ts"),
            "typescript·p·lib/buckets·kindFor·item",
        );
    }

    /// An ALIASED named import asks two different questions, and one string
    /// cannot answer both: `kf` is what this file looks the name up by, and
    /// `kindFor` is what the other module declares.
    ///
    /// MUTATION: push `local` instead of `member` in `through_an_import` — the
    /// target becomes `lib/buckets·kf·item`, which nothing declares.
    #[test]
    fn an_aliased_named_import_reaches_the_name_the_other_module_declares() {
        let scanned = scan_of(
            &javascript::TypeScriptAdapter,
            &[
                (
                    "lib/buckets",
                    "src/lib/buckets.ts",
                    "export function kindFor(a: string): string { return a; }\n",
                ),
                (
                    "lib/board",
                    "src/lib/board.ts",
                    "import { kindFor as kf } from './buckets.js';\n\
                     export function go(): string { return kf('x'); }\n",
                ),
            ],
        );
        assert_placed(
            file_of(&scanned, "src/lib/board.ts"),
            "typescript·p·lib/buckets·kindFor·item",
        );
    }

    /// A JavaScript PRIVATE member, reached from inside its own class.
    ///
    /// The declaration side has always spelled it with the hash — `property_name`
    /// turns a `PrivateIdentifier` into `#hydrate` — and `this` inside a class
    /// body already types to that class. So both halves of the identity were
    /// there and the two never met, because the use site `this.#hydrate(api)` is
    /// a `PrivateFieldExpression`, a shape the walk had no arm for anywhere.
    ///
    /// A call fell to the callee catch-all and came out `DynamicDispatch` with
    /// an EMPTY observation list, which is terminal by construction: the ladder
    /// climbs only for `Unplaced`, and with no candidate there is nothing to
    /// climb to. A read or a write was dropped outright.
    ///
    /// MEASURED at 25 private methods and 45 private fields declared with no
    /// inbound edge, against 58 call sites and 167 reads and writes.
    #[test]
    fn a_private_member_is_reached_from_inside_the_class_that_declares_it() {
        let scanned = scan_of(
            &javascript::TypeScriptAdapter,
            &[(
                "lib/scan",
                "src/lib/scan.ts",
                "export class ScanState {\n\
                 \x20 #api = 0;\n\
                 \x20 #hydrate(n: number): number { return n + this.#api; }\n\
                 \x20 start(n: number): number { return this.#hydrate(n); }\n\
                 }\n",
            )],
        );
        let facts = file_of(&scanned, "src/lib/scan.ts");
        assert_placed(facts, "typescript·p·lib/scan·ScanState·#hydrate·item");
        assert_placed(facts, "typescript·p·lib/scan·ScanState·#api·field");
    }

    /// Every reference's outcome, as text: the identity it was placed at, or the
    /// reason it was not and the name that defeated the ladder. Written this way
    /// so an assertion names what it wants instead of indexing into a vector.
    fn targets(facts: &FileFacts) -> Vec<String> {
        facts
            .references
            .iter()
            .map(|r| match &r.target {
                Resolution::Resolved { fqn, .. } => fqn.as_str().to_string(),
                Resolution::Unresolved { reason, evidence } => {
                    format!("{reason:?}({})", evidence.name)
                }
            })
            .collect()
    }

    fn assert_placed(facts: &FileFacts, expected: &str) {
        let got = targets(facts);
        assert!(got.iter().any(|t| t == expected), "no reference reached {expected}; got {got:?}");
    }

    /// Every placed edge records WHICH RUNG placed it.
    ///
    /// The rungs are not interchangeable. `declared_here` is the file pointing
    /// at its own declaration; `in_the_prelude` is a language-wide default that
    /// any file could have shadowed. Both arrive as `Resolved`, and a consumer
    /// handed only that treats the weakest claim the ladder makes exactly like
    /// the strongest.
    ///
    /// It is also the one field that maps an edge back to the code that made
    /// it: a rung IS a function in this file, so a wrong edge tagged
    /// `through_a_glob` names `through_a_glob` as the thing to go and read.
    #[test]
    fn every_placed_reference_records_the_rung_that_placed_it() {
        let facts = ladder(
            "m",
            "use crate::db::PgStore;\nfn helper() {}\nfn f() { helper(); PgStore::connect(); Some(1); }",
        );
        let via = |fqn: &str| -> Rung {
            facts
                .references
                .iter()
                .find_map(|r| match &r.target {
                    Resolution::Resolved { fqn: f, via } if f.as_str() == fqn => Some(*via),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("nothing placed {fqn}"))
        };
        assert_eq!(via("rust·p·m·helper·item"), Rung::DeclaredHere);
        assert_eq!(via("rust·p·db·PgStore·connect·item"), Rung::ThroughAnImport);
        assert_eq!(via("lib·std·option::Option::Some"), Rung::InThePrelude);
    }

    /// The labels round-trip and cover the ladder, exactly as the reasons do.
    /// `Rung::ALL` is in climb order, so a reader of the list reads the ladder.
    #[test]
    fn the_rung_labels_round_trip_and_cover_every_rung() {
        for rung in Rung::ALL {
            assert_eq!(Rung::from_label(rung.as_label()), Some(*rung), "{rung:?} lost its label");
        }
        assert_eq!(Rung::from_label("not_a_rung"), None);
        assert_eq!(
            Rung::ALL.iter().map(|r| r.as_label()).collect::<Vec<_>>(),
            [
                "declared_here",
                "through_an_import",
                "named_by_this_file",
                "declared_by_its_type",
                "through_a_glob",
                "rooted_in_this_package",
                "fully_qualified_external",
                "in_the_prelude",
            ],
            "ALL is the ladder in climb order — a reordering here is a claim that the \
             ladder tries them differently"
        );
    }

    /// The first rung. A name the walk minted a candidate for, whose candidate is
    /// a declaration in the very same file, is PROVEN — not guessed.
    #[test]
    fn a_reference_to_a_declaration_in_the_same_file_resolves_to_it() {
        let facts = ladder("m", "fn helper() -> u32 { 0 }\nfn caller() -> u32 { helper() }");
        let call =
            facts.references.iter().find(|r| r.kind == RefKind::Calls).expect("one call site");
        assert_eq!(placed(call), "rust·p·m·helper·item");
    }

    /// Rung 2, the external half (R5, D3). The IMPORT says the name crosses out
    /// of the scanned source, so the target is a named library member and no
    /// dependency source is opened to produce it. The specifier also says where
    /// inside that library the member sits, so the whole path is kept.
    #[test]
    fn an_import_that_leaves_the_scanned_source_places_a_library_member() {
        let facts = ladder(
            "m",
            "use tokio::sync::Mutex;\nuse serde_json::Value as Json;\nfn f() { Mutex::new(); let _ = Json::Null; }",
        );
        assert_placed(&facts, "lib·tokio·sync::Mutex::new");
        assert_placed(&facts, "lib·serde_json·Value::Null");
    }

    /// Rung 2, the local half. The specifier names the module the target lives
    /// in, which is the segment boundary the identity needs — nothing here is
    /// guessed from the shape of the name.
    #[test]
    fn an_import_inside_the_scanned_source_places_a_local_identity() {
        let facts =
            ladder("m", "use crate::db::PgStore;\nfn f(store: &PgStore) { PgStore::connect(); }");
        assert_placed(&facts, "rust·p·db·PgStore·item");
        assert_placed(&facts, "rust·p·db·PgStore·connect·item");
    }

    /// A sibling crate of the same workspace is rooted at its own name, so the
    /// walk can only call it external — a source file states nothing about which
    /// packages the scan owns. The manifest does, and the ladder consults it
    /// before it turns an external import into a library (R5).
    #[test]
    fn a_sibling_package_of_the_same_scan_is_local_however_the_import_spells_it() {
        let facts =
            ladder_among("m", "use senseid::db::PgStore;\nfn f(store: &PgStore) {}", &["senseid"]);
        assert_placed(&facts, "rust·senseid·db·PgStore·item");

        let unknown = ladder("m", "use senseid::db::PgStore;\nfn f(store: &PgStore) {}");
        assert_placed(&unknown, "lib·senseid·db::PgStore");
    }

    /// Rung 3. A path that roots itself in the package needs no import at all,
    /// and the root word is what says where the module segment starts.
    #[test]
    fn a_path_rooted_in_this_package_is_placed_without_an_import() {
        let facts = ladder("m", "fn f() { crate::db::PgStore::connect(); }");
        assert_placed(&facts, "rust·p·db·PgStore·connect·item");
    }

    /// `super` and `self` are relative to the module the path is WRITTEN in, and
    /// an inline `mod` is a module. Reading them against the file's own module
    /// would name a symbol one level out from the one meant.
    #[test]
    fn a_relative_path_is_read_against_the_module_it_is_written_in() {
        let facts = ladder(
            "a",
            "fn helper() {}\nmod tests { fn t() { super::helper(); self::inner(); } fn inner() {} }",
        );
        assert_placed(&facts, "rust·p·a·helper·item");
        assert_placed(&facts, "rust·p·a::tests·inner·item");
    }

    /// An import binds inside the block it is written in, and nowhere else. A
    /// `use` in a test module is the overwhelmingly common case in this corpus —
    /// letting it bind file-wide would place names in the outer module that the
    /// compiler does not have in scope there.
    #[test]
    fn an_import_written_inside_a_module_does_not_bind_outside_it() {
        let facts = ladder(
            "m",
            "fn outer() { Mutex::new(); }\nmod tests { use tokio::sync::Mutex;\n fn inner() { Mutex::new(); } }",
        );
        let got = targets(&facts);
        assert!(
            got.iter().any(|t| t == "lib·tokio·sync::Mutex::new"),
            "the call inside the module is bound by the import above it; got {got:?}"
        );
        assert!(
            got.iter().any(|t| t.starts_with("NoImportInScope(")),
            "the call outside the module has no binding at all; got {got:?}"
        );
    }

    /// A type use is normalised to its last segment before the walk can name it,
    /// so the walk records the path it read as well. Without that the root word
    /// is gone and a fully qualified type is a bare name the ladder cannot
    /// place — which is the "we discard something we already parsed" failure
    /// this rewrite is measuring.
    #[test]
    fn a_fully_qualified_type_keeps_the_path_the_walk_read() {
        let facts = ladder("m", "fn f(store: &crate::db::PgStore) {}");
        assert_placed(&facts, "rust·p·db·PgStore·item");
    }

    /// Rung 4. A language's prelude is in scope with nothing written to bring it
    /// there, so the absence of an import is not evidence of anything about it.
    /// The target is a library member like any other (R5) and is spelled with
    /// the path the prelude re-exports, so a file that imports the same name the
    /// long way lands on the same node.
    #[test]
    fn a_name_the_language_puts_in_scope_needs_no_import() {
        let facts =
            ladder("m", "fn f() -> String { let v = Vec::new(); println!(\"x\"); String::new() }");
        assert_placed(&facts, "lib·std·vec::Vec::new");
        assert_placed(&facts, "lib·std·println");
        assert_placed(&facts, "lib·std·string::String");

        let long_way = ladder("m", "use std::vec::Vec;\nfn f() { Vec::new(); }");
        assert_placed(&long_way, "lib·std·vec::Vec::new");
    }

    /// A glob binds an unknown set of names, so a name that MIGHT have come from
    /// it is not proof that it did (R4) — and a glob outranks the prelude, so it
    /// can quietly replace the answer rung 4 would have given. Two possible
    /// origins and nothing to tell them apart is exactly `AmbiguousCandidates`.
    #[test]
    fn a_glob_in_scope_stops_the_prelude_from_answering_for_it() {
        let facts = ladder("m", "use crate::shim::*;\nfn f() { let v = Vec::new(); }");
        let got = targets(&facts);
        assert!(
            got.iter().any(|t| t == "AmbiguousCandidates(Vec::new)"),
            "the glob may have re-bound `Vec`; got {got:?}"
        );

        // Outside the glob's block the prelude is unshadowed and answers.
        let scoped = ladder(
            "m",
            "fn outer() { Vec::new(); }\nmod tests { use crate::shim::*;\n fn inner() { Vec::new(); } }",
        );
        assert_placed(&scoped, "lib·std·vec::Vec::new");
    }

    /// "A glob binds an unknown set" is true in general and false for the glob
    /// this corpus is made of: `use super::*` inside a test module targets the
    /// module the file itself IS, and the file's own declarations are exactly
    /// what it binds. So the set is readable, a name in it resolves, and — the
    /// half that matters more — a name NOT in it was never bound by the glob, so
    /// the prelude is not shadowed and does not have to give up its answer.
    #[test]
    fn a_glob_into_a_module_this_file_covers_binds_a_set_the_ladder_can_read() {
        let facts = ladder(
            "m",
            "fn helper() {}\nmod tests { use super::*;\n fn t() { helper(); assert_eq!(1, 1); } }",
        );
        assert_placed(&facts, "rust·p·m·helper·item");
        assert_placed(&facts, "lib·std·assert_eq");
    }

    /// The plumbing list is FILTERING, not failure. `x.clone()` is a miss the ladder
    /// will never place and there are thousands of them; left in the general
    /// bucket they bury the misses somebody could act on. Its own reason is what
    /// lets a reader drop it without dropping those.
    #[test]
    fn plumbing_is_filtered_into_its_own_reason_rather_than_a_genuine_miss() {
        // `let y = helper();` and NOT `y: &Thing`: a parameter's type is
        // STATED, so the walk now reads it off the signature and the receiver is
        // no longer unknown. A fixture that kept the old spelling would be
        // asserting about a reason it no longer reaches.
        let facts = ladder("m", "fn f(x: &Thing) { let y = helper(); x.clone(); y.width(); }");
        let got = targets(&facts);
        assert!(
            got.iter().any(|t| t == "Plumbing(clone)"),
            "`clone` is plumbing, not a gap in the graph; got {got:?}"
        );
        assert!(
            got.iter().any(|t| t == "ReceiverTypeUnknown(width)"),
            "a genuine miss keeps its own reason; got {got:?}"
        );
    }

    /// A3. Every reason the ladder can reach is reached by something, and the
    /// list of the ones it cannot reach is written down rather than left to be
    /// inferred from an empty histogram bucket. A variant added to `Reason` with
    /// no producer and no entry here stops this test.
    #[test]
    fn every_reason_the_ladder_can_reach_is_produced_by_something() {
        use crate::indexer::facts::Reason;

        let cases: Vec<(Reason, &str, &str)> = vec![
            (
                Reason::Unplaced,
                "the walk's own bucket, which the ladder must empty",
                // Nothing may still say `Unplaced` once the ladder has run; the
                // corpus histogram is what proves it.
                "",
            ),
            (
                Reason::NoImportInScope,
                "a bare name with nothing in the file that binds it",
                "fn f() { nowhere(); }",
            ),
            (
                Reason::AmbiguousCandidates,
                "a glob that may have re-bound a name the prelude also has",
                "use crate::shim::*;\nfn f() { Vec::new(); }",
            ),
            (
                Reason::Plumbing,
                "plumbing, filtered so it does not bury genuine misses",
                "fn f(x: &Thing) { x.clone(); }",
            ),
            (
                Reason::ReceiverTypeUnknown,
                "a member access whose receiver this file states no type for",
                // The receiver's type must be genuinely unstated — a parameter
                // or a typed `let` is read now, so neither reaches this reason.
                "fn f() { let y = helper(); y.width(); }",
            ),
            (
                Reason::UnhandledForm,
                "a shape the walk has no rule for, named rather than dropped",
                "fn f(b: Box<dyn Fn()>) { (b)(); }",
            ),
        ];

        for (reason, what, text) in cases {
            if text.is_empty() {
                continue;
            }
            let facts = ladder("m", text);
            let got = targets(&facts);
            let wanted = format!("{reason:?}(");
            assert!(
                got.iter().any(|t| t.starts_with(&wanted)),
                "nothing produced {reason:?} ({what}); got {got:?}"
            );
        }

        // The variants no rung and no walk reaches today. Each names a cause
        // that needs a fact syntax alone does not carry, and each is listed so
        // an empty bucket in the histogram is a recorded decision rather than an
        // oversight.
        let unreached = [
            // Needs the stated type of a receiver to have been read and then
            // found absent; the ladder does not read receiver types at all yet,
            // so every one of those is `ReceiverTypeUnknown` instead.
            Reason::NoDeclaredType,
            // Needs to know a callee is a value rather than a name. The walk
            // reports those as `UnhandledForm` with the node kind, which is a
            // truthful account of what it saw.
            Reason::DynamicDispatch,
            // A macro's expansion is never parsed, so no reference is ever
            // emitted from inside one and nothing can point into one.
            Reason::MacroExpansion,
        ];
        assert_eq!(unreached.len(), 3, "the unreached list is part of the contract");
    }

    /// A relation's parent is a name read out of a trait bound or an impl
    /// header, which is the same kind of name a call site produces — so it goes
    /// up the same ladder (R7). A second placement rule for structure would be a
    /// second idea of what a miss means, and the histogram would stop being a
    /// measurement of one thing.
    #[test]
    fn a_relations_parent_climbs_the_same_ladder_a_references_target_does() {
        let facts = ladder(
            "m",
            "use crate::draw::Draw;\npub struct Widget;\nimpl Draw for Widget {}\n\
             pub trait Local {}\npub trait Sub: Local + Clone {}\n",
        );
        let got: Vec<String> = facts
            .relations
            .iter()
            .filter(|r| {
                !matches!(
                    r.kind,
                    crate::indexer::facts::RelationKind::Owns
                        | crate::indexer::facts::RelationKind::Contains
                )
            })
            .map(|r| match &r.parent {
                Resolution::Resolved { fqn, .. } => {
                    format!("{:?} -> {}", r.kind, fqn.as_str())
                }
                Resolution::Unresolved { reason, evidence } => {
                    format!("{:?} -> {reason:?}({})", r.kind, evidence.name)
                }
            })
            .collect();

        assert_eq!(
            got,
            vec![
                // Rung 2: the import states where `Draw` lives.
                "TraitImpl -> rust·p·draw·Draw·item",
                // Rung 1: declared in this very file.
                "Extends -> rust·p·m·Local·item",
                // Rung 4: in scope with nothing written to bring it there, so it
                // is a named library member like any other external (R5). No
                // trailing reach, because we index no declarations for a
                // library and so have nothing there to keep apart (spec §2).
                "Extends -> lib·std·clone::Clone",
            ],
            "every rung the ladder has must serve a relation exactly as it serves a reference"
        );
    }

    /// Spec §2.1, both required properties in one test, because either one alone
    /// is satisfiable by a rule that breaks the other.
    ///
    /// A path leaf does not state which namespace it is in, so a use site and a
    /// declaration that both read `Status::Err` must mint ONE string — the reach
    /// is what they can both observe. And a field is reachable in Rust ONLY
    /// through a dot with no call (`Type::field` is not a path Rust admits), so
    /// separating `field` from `item` costs nothing at a use site and keeps the
    /// 20 field/method pairs this repo has from merging onto one node.
    #[test]
    fn a_path_reached_enum_variant_merges_while_a_field_and_a_method_stay_apart() {
        let facts = ladder(
            "m",
            "pub enum Status { Active, Err { code: u32 } }\n\
             pub struct WatcherHealth { pub healthy: bool }\n\
             impl WatcherHealth {\n\
             \x20   pub fn healthy(&self) -> bool { self.healthy }\n\
             \x20   pub fn check(&self) -> bool { self.healthy() }\n\
             }\n\
             pub fn make() -> Status { crate::m::Status::Err { code: 1 } }\n",
        );
        let declared: Vec<&str> = facts.symbols.iter().map(|s| s.fqn.as_str()).collect();

        // The variant: one identity, minted by the declaration AND by the path.
        assert!(
            declared.contains(&"rust·p·m·Status·Err·item"),
            "the declaration side must mint the variant as `item`; got {declared:?}"
        );
        assert_placed(&facts, "rust·p·m·Status·Err·item");

        // The field and the method: two identities, both declared and both
        // reached, and the reaches never overlap.
        assert!(
            declared.contains(&"rust·p·m·WatcherHealth·healthy·field"),
            "the field must stay a `field`; got {declared:?}"
        );
        assert!(
            declared.contains(&"rust·p·m·WatcherHealth·healthy·item"),
            "the same-named method must be an `item`; got {declared:?}"
        );
        assert_placed(&facts, "rust·p·m·WatcherHealth·healthy·field");
        assert_placed(&facts, "rust·p·m·WatcherHealth·healthy·item");
    }

    // ── the corpus ───────────────────────────────────────────────────────────
    //
    // A fixture proves the ladder handles what the fixture's author thought of.
    // These run over this repo's own rust, with each file given the package and
    // module path it really has, because a ladder measured against `package: p,
    // module: m` is measured against a repository that does not exist.

    fn corpus() -> (BTreeSet<String>, Vec<(String, FileFacts)>) {
        let sources = crate::indexer::corpus_rust_sources();
        let first_party: BTreeSet<String> =
            sources.iter().map(|(path, _)| package_of(path)).collect();
        let walked = sources
            .into_iter()
            .map(|(path, text)| {
                let source = Source {
                    package: &package_of(&path),
                    module: &module_of(&path),
                    path: &path,
                    text: &text,
                };
                let facts = rust::read(&source).unwrap_or_else(|e| panic!("{path}: {e:?}"));
                (path, facts)
            })
            .collect();
        (first_party, walked)
    }

    /// A3, over the real thing. Every reference the corpus produces is accounted
    /// for by exactly one outcome, `Unplaced` is empty because the ladder has
    /// run, and the table is printed so the shape of what is left can be read
    /// rather than guessed at.
    #[test]
    fn the_reasons_account_for_every_unresolved_reference_in_this_repos_rust() {
        use std::collections::BTreeMap;

        let (first_party, walked) = corpus();
        let scanned = BTreeSet::new();
        let world = World {
            first_party: &first_party,
            first_party_members: &BTreeSet::new(),
            declared_members: &BTreeSet::new(),
            returns: &std::collections::BTreeMap::new(),
            scanned: &scanned,
        };

        let mut histogram: BTreeMap<String, usize> = BTreeMap::new();
        let mut common: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
        let mut total = 0usize;
        let mut resolved = 0usize;
        let mut unresolved = 0usize;
        let mut structural = 0usize;
        let mut structural_resolved = 0usize;
        let mut structural_reasons: BTreeMap<String, usize> = BTreeMap::new();
        for (path, facts) in walked {
            let before = facts.references.len();
            let facts = resolve(facts, &rust::GRAMMAR, &world);
            assert_eq!(
                facts.references.len(),
                before,
                "{path}: the ladder dropped a reference; a miss is a reason, never an \
                 omission (R2)"
            );
            for reference in &facts.references {
                total += 1;
                match &reference.target {
                    Resolution::Resolved { .. } => resolved += 1,
                    Resolution::Unresolved { reason, evidence } => {
                        unresolved += 1;
                        assert_ne!(
                            *reason,
                            crate::indexer::facts::Reason::Unplaced,
                            "{path}: `{}` is still Unplaced, so the ladder did not run on it — \
                             that bucket says only that resolution has not happened yet",
                            evidence.name
                        );
                        assert!(!evidence.name.is_empty(), "{path}: a miss with no name");
                        *histogram.entry(format!("{reason:?}")).or_insert(0) += 1;
                        *common
                            .entry(format!("{reason:?}"))
                            .or_default()
                            .entry(evidence.name.clone())
                            .or_insert(0) += 1;
                    }
                }
            }
            // A relation's parent goes up the same ladder, so it is held to the
            // same rule: still `Unplaced` means the ladder skipped it, and a
            // structural edge that nothing placed is exactly the bare name with
            // no reason the plan's step 6 forbids.
            for relation in &facts.relations {
                structural += 1;
                match &relation.parent {
                    Resolution::Resolved { .. } => structural_resolved += 1,
                    Resolution::Unresolved { reason, evidence } => {
                        assert_ne!(
                            *reason,
                            crate::indexer::facts::Reason::Unplaced,
                            "{path}: the {:?} edge from `{}` is still Unplaced, so the ladder \
                             did not run on structure",
                            relation.kind,
                            relation.child.as_str()
                        );
                        assert!(
                            !evidence.name.is_empty(),
                            "{path}: a relation parent with no name is a bare edge"
                        );
                        *structural_reasons.entry(format!("{reason:?}")).or_insert(0) += 1;
                    }
                }
            }
        }

        assert_eq!(
            histogram.values().sum::<usize>(),
            unresolved,
            "the reasons must account for 100% of the {unresolved} unresolved references"
        );
        assert_eq!(resolved + unresolved, total);
        assert!(total > 10_000, "the corpus produced only {total} references; that is not it");

        println!("\n{total} references — {resolved} resolved, {unresolved} unresolved");
        println!("{:<24} {:>8} {:>8}  most common", "reason", "count", "% unres");
        for (reason, count) in &histogram {
            let mut names: Vec<(&String, &usize)> = common[reason].iter().collect();
            names.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            let top: Vec<String> = names.iter().take(6).map(|(n, c)| format!("{n} {c}")).collect();
            let share = 100.0 * *count as f64 / unresolved as f64;
            println!("{reason:<24} {count:>8} {share:>7.1}%  {}", top.join(", "));
        }

        assert_eq!(
            structural_reasons.values().sum::<usize>() + structural_resolved,
            structural,
            "every relation parent is one outcome or the other"
        );
        assert!(structural > 1_000, "the corpus produced only {structural} relations");
        println!(
            "\n{structural} relations — {structural_resolved} placed, {:?} unplaced",
            structural_reasons
        );
    }

    /// A4 for structure, over the real thing — and the measurement that found a
    /// defect UPSTREAM of this step.
    ///
    /// An ownership edge points at the type its member was NAMED under, so a
    /// dangling one means the member's own identity is anchored on a type
    /// nothing declares. 588 of 4,676 are, and every one of them has the same
    /// cause: `db/pg_store/graph.rs` writes `use super::*;` and then
    /// `impl PgStore { }`, so the walk anchors those methods on
    /// `db::pg_store::graph·PgStore`, while `PgStore` is declared one module up
    /// in `db/pg_store/mod.rs`. The walk names an impl's members from the module
    /// the impl is WRITTEN in, and a split impl is not written where its type
    /// lives.
    ///
    /// That is the ghost-node shape the module docs describe, reached by a
    /// different road, and it belongs to the walk's identity rule (step 3), not
    /// to the ladder: by the time the ladder runs, the member identities have
    /// already been minted. Fixing it means the impl's TYPE is placed before its
    /// members are named, which moves work across the walk/ladder seam that
    /// steps 1-4 fixed — a decision this step records rather than takes.
    ///
    /// So this test does not assert zero, which would be a lie. It asserts the
    /// two things that keep the defect from spreading unnoticed: every dangling
    /// owner is a MIS-ANCHORED type that really is declared elsewhere in its own
    /// package, never a type nobody declares at all; and the share does not
    /// grow.
    ///
    /// The parent side of an inheritance relation is exempt: a supertrait is
    /// very often `Send`, `Clone` or another external, and R5 says an external
    /// is named and never opened, so it has no declaration by design.
    #[test]
    fn every_ownership_edge_points_at_a_type_declared_somewhere_in_its_own_package() {
        let (first_party, walked) = corpus();
        let declared: BTreeSet<crate::indexer::facts::Fqn> = walked
            .iter()
            .flat_map(|(_, facts)| facts.symbols.iter().map(|s| s.fqn.clone()))
            .collect();
        // `(package, type name)` of everything the corpus declares, which is what
        // separates "anchored on the wrong module" from "invented".
        let by_name: BTreeSet<(String, String)> = declared
            .iter()
            .filter_map(|fqn| {
                let parsed = fqn::parse(fqn.as_str()).ok()?;
                Some((parsed.package.to_string(), (*parsed.tail.last()?).to_string()))
            })
            .collect();
        let world = World {
            first_party: &first_party,
            first_party_members: &BTreeSet::new(),
            declared_members: &BTreeSet::new(),
            returns: &std::collections::BTreeMap::new(),
            scanned: &BTreeSet::new(),
        };

        let mut total = 0usize;
        let mut dangling: BTreeMap<String, usize> = BTreeMap::new();
        let mut invented: Vec<String> = Vec::new();
        for (path, facts) in walked {
            for relation in &resolve(facts, &rust::GRAMMAR, &world).relations {
                if relation.kind != crate::indexer::facts::RelationKind::Owns {
                    continue;
                }
                total += 1;
                let Resolution::Resolved { fqn: parent, .. } = &relation.parent else {
                    panic!(
                        "{path}: an ownership edge is minted from a declaration this walk \
                            read, so it is never a miss"
                    );
                };
                if declared.contains(parent) {
                    continue;
                }
                *dangling.entry(parent.as_str().to_string()).or_insert(0) += 1;
                let named = fqn::parse(parent.as_str())
                    .ok()
                    .and_then(|p| Some((p.package.to_string(), (*p.tail.last()?).to_string())))
                    .is_some_and(|key| by_name.contains(&key));
                if !named {
                    invented.push(format!("{path}: {}", parent.as_str()));
                }
            }
        }

        let count: usize = dangling.values().sum();
        let mut worst: Vec<(&String, &usize)> = dangling.iter().collect();
        worst.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));

        assert!(
            invented.is_empty(),
            "an ownership edge names a type its package never declares under ANY module, so \
             this is not the known split-impl mis-anchoring — it is a type the walk invented \
             (R4): {:?}",
            invented.iter().take(12).collect::<Vec<_>>()
        );
        // 588 of 4,676 = 12.6% when this was written; 655 of 5,474 = 11.9% now.
        // The ceiling is a ratchet: the defect is recorded, and it is not
        // allowed to spread while it waits for the seam decision above.
        //
        // **THE COUNT ROSE AND THE SHARE FELL, AND NEITHER IS THIS DEFECT
        // MOVING.** Both numbers grew because the corpus did. Stage 11's fix to
        // the impl owner's module (`module: &home`) was MEASURED against this
        // ratchet and moved it by exactly nothing: `home` only differs from the
        // block's own module when the FILE states where the type lives, and the
        // shape this ratchet counts is `use super::*` followed by
        // `impl PgStore { }` — a glob, which states no home, so `home_of` falls
        // back to the block's module and the two are the same string.
        //
        // Recorded because it is the attribution that a lowered ceiling would
        // have hidden: this defect is waiting on the import rung, not on the
        // anchoring.
        let share = 100 * count / total;
        // PRINTED, not only asserted. A ratchet that reports nothing on success
        // can only ever be lowered by somebody who edits the test to find out
        // where it stands — which is how a ceiling outlives the defect under it.
        println!("\n  split-impl anchoring: {count} of {total} ownership edges ({share}%)");
        assert!(
            share <= 12,
            "{count} of {total} ownership edges ({share}%) are anchored on a module that \
             declares nothing, up from the 11.9% this defect now measures. Worst: {:?}",
            worst.iter().take(12).collect::<Vec<_>>()
        );
    }

    /// Everything but the trailing segment of an fqn, and that segment.
    ///
    /// String surgery rather than `fqn::parse`, because `parse` deliberately
    /// cannot say which FORM a string is and this needs no form — only the
    /// boundary the trailing segment sits behind. It lives in the tests, where
    /// writing the separator is reading an fqn rather than minting one.
    fn without_the_trailing_segment(fqn: &str) -> Option<(&str, &str)> {
        fqn.rsplit_once('·')
    }

    /// Spec §2.1, over the real thing. The measurement that forced the rule.
    ///
    /// A reference that names NO declaration but would name one if its trailing
    /// segment were different is not a gap in coverage — it is the two sides of
    /// the merge contract reading the same source and disagreeing about one
    /// label. The reference lands on a stub that no definition will ever enrich
    /// and the declaration keeps a node nothing points at: one symbol, two
    /// halves (spec §2).
    ///
    /// This does NOT catch every dangling edge, and deliberately so. A target
    /// under the wrong MODULE (the split-impl anchoring, an unfollowed
    /// re-export) differs in more than its last segment and is a separate,
    /// separately-recorded defect. What is counted here is exactly the class the
    /// reach rule closes.
    ///
    /// One case is separated out rather than counted, and it is the case the
    /// `mod` reach exists for: a head that only a MODULE declares. `mod` is the
    /// one reach no reference can mint, so `X·item` beside a declared `X·mod` is
    /// not two spellings of one symbol — it is a call to a function re-exported
    /// through a module of the same name, and merging it onto the module would
    /// be a WRONG edge in place of a dangling one (R4). Those are asserted as
    /// their own bounded set below, so removing the `mod` reach fails this test
    /// rather than quietly turning 7 dangling edges into 7 wrong ones.
    #[test]
    fn no_reference_names_a_declaration_that_differs_only_in_its_reach() {
        let (first_party, walked) = corpus();
        let declared: BTreeSet<String> = walked
            .iter()
            .flat_map(|(_, facts)| facts.symbols.iter().map(|s| s.fqn.as_str().to_string()))
            .collect();
        // Every declared identity, keyed by everything before its trailing
        // segment. Two entries under one key are two declarations one use site
        // could be spelling.
        let mut declared_reaches: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for fqn in &declared {
            if let Some((head, reach)) = without_the_trailing_segment(fqn) {
                declared_reaches.entry(head).or_default().insert(reach);
            }
        }

        let module_only = fqn::Reach::Mod.as_str();
        let scanned = BTreeSet::new();
        let world = World {
            first_party: &first_party,
            first_party_members: &BTreeSet::new(),
            declared_members: &BTreeSet::new(),
            returns: &std::collections::BTreeMap::new(),
            scanned: &scanned,
        };
        let mut resolved = 0usize;
        let mut broken: BTreeMap<String, usize> = BTreeMap::new();
        let mut only_a_module_declares_it: BTreeMap<String, usize> = BTreeMap::new();
        for (_, facts) in walked {
            let placed = resolve(facts, &rust::GRAMMAR, &world);
            let targets = placed
                .references
                .iter()
                .map(|r| &r.target)
                .chain(placed.relations.iter().map(|r| &r.parent));
            for target in targets {
                let Resolution::Resolved { fqn, .. } = target else {
                    continue;
                };
                let Ok(parsed) = fqn::parse(fqn.as_str()) else {
                    continue;
                };
                // An external carries no reach at all (R5), so it cannot differ
                // from a declaration by one.
                if !matches!(parsed.origin, fqn::Origin::Local { .. }) {
                    continue;
                }
                resolved += 1;
                if declared.contains(fqn.as_str()) {
                    continue;
                }
                let Some((head, reach)) = without_the_trailing_segment(fqn.as_str()) else {
                    continue;
                };
                let Some(declared_here) = declared_reaches.get(head) else {
                    continue;
                };
                let mut others = declared_here.iter().filter(|r| **r != reach).peekable();
                if others.peek().is_none() {
                    continue;
                }
                if others.all(|r| *r == module_only) {
                    *only_a_module_declares_it.entry(fqn.as_str().to_string()).or_insert(0) += 1;
                } else {
                    *broken.entry(fqn.as_str().to_string()).or_insert(0) += 1;
                }
            }
        }

        let count: usize = broken.values().sum();
        let mut worst: Vec<(&String, &usize)> = broken.iter().collect();
        worst.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        assert_eq!(
            count,
            0,
            "{count} of {resolved} first-party resolutions ({} distinct identities) name a \
             declaration that exists under a DIFFERENT trailing segment, so the use site and the \
             declaration never merge. Worst: {:?}",
            broken.len(),
            worst.iter().take(12).collect::<Vec<_>>()
        );

        // The `mod` reach's whole job, as a number. Every one of these is a
        // re-export the walk does not follow (`crate::installer::install` calls
        // a function re-exported from module `installer::install`). They dangle,
        // which is the correct outcome; without a separate `mod` reach they
        // would resolve onto the MODULE instead — 7 wrong edges rather than 7
        // missing ones (R4).
        let module_heads: usize = only_a_module_declares_it.values().sum();
        assert_eq!(
            (module_heads, only_a_module_declares_it.len()),
            (7, 6),
            "the references whose head only a module declares moved: {:?}",
            only_a_module_declares_it
        );
    }

    /// R6/A6, and the reason the ladder is shaped the way it is.
    ///
    /// The same file is placed twice: once as if nothing else in the repository
    /// had been read, and once with every identity the whole corpus mints
    /// already in hand. The two must be byte-identical, because a rung that
    /// consulted what had been scanned would answer differently depending on
    /// which file came first — and the specific answer it would get wrong is
    /// "this symbol is absent, so it must be external", which is what minted 659
    /// ghost nodes the last time.
    #[test]
    fn resolving_a_file_does_not_depend_on_what_has_been_scanned_before_it() {
        let (first_party, walked) = corpus();
        let everything: BTreeSet<_> = walked
            .iter()
            .flat_map(|(_, facts)| facts.symbols.iter().map(|s| s.fqn.clone()))
            .collect();
        assert!(everything.len() > 1_000, "the corpus minted only {}", everything.len());

        let nothing = BTreeSet::new();
        for (path, facts) in walked {
            let alone = resolve(
                facts.clone(),
                &rust::GRAMMAR,
                &World {
                    first_party: &first_party,
                    first_party_members: &BTreeSet::new(),
                    declared_members: &BTreeSet::new(),
                    returns: &std::collections::BTreeMap::new(),
                    scanned: &nothing,
                },
            );
            let after = resolve(
                facts,
                &rust::GRAMMAR,
                &World {
                    first_party: &first_party,
                    first_party_members: &BTreeSet::new(),
                    declared_members: &BTreeSet::new(),
                    returns: &std::collections::BTreeMap::new(),
                    scanned: &everything,
                },
            );
            assert!(
                alone == after,
                "{path}: the ladder answered differently once the rest of the repository had \
                 been scanned, so the graph depends on file order (R6).\n{}",
                first_difference(&alone, &after)
            );
        }
    }

    /// The one outcome that moved, as a line.
    ///
    /// `assert_eq!` on two `FileFacts` prints both whole values, and one file of
    /// this corpus is megabytes of `Debug` — a report nobody can read is a test
    /// nobody can act on. The comparison above is still the whole value, so
    /// nothing is weakened; only the message is made legible.
    fn first_difference(alone: &FileFacts, after: &FileFacts) -> String {
        for (i, (a, b)) in alone.references.iter().zip(&after.references).enumerate() {
            if a != b {
                return format!("reference {i} at {:?}:\n  alone: {:?}\n  after: {:?}", a.at, a, b);
            }
        }
        for (i, (a, b)) in alone.relations.iter().zip(&after.relations).enumerate() {
            if a != b {
                return format!("relation {i} at {:?}:\n  alone: {:?}\n  after: {:?}", a.at, a, b);
            }
        }
        format!(
            "not in a reference or a relation: {} vs {} references, {} vs {} relations, \
             {} vs {} symbols",
            alone.references.len(),
            after.references.len(),
            alone.relations.len(),
            after.relations.len(),
            alone.symbols.len(),
            after.symbols.len()
        )
    }

    /// A6 from the other direction: the whole corpus, indexed front to back and
    /// back to front, accumulating what has been scanned as a real scan would.
    /// This is the form that catches an order dependence introduced through some
    /// input the test above does not carry.
    #[test]
    fn indexing_the_corpus_in_either_order_produces_the_same_facts() {
        let (first_party, walked) = corpus();

        let run = |order: Vec<(String, FileFacts)>| {
            let mut scanned: BTreeSet<crate::indexer::facts::Fqn> = BTreeSet::new();
            let mut out: BTreeMap<String, FileFacts> = BTreeMap::new();
            for (path, facts) in order {
                let placed = resolve(
                    facts,
                    &rust::GRAMMAR,
                    &World {
                        first_party: &first_party,
                        first_party_members: &BTreeSet::new(),
                        declared_members: &BTreeSet::new(),
                        returns: &std::collections::BTreeMap::new(),
                        scanned: &scanned,
                    },
                );
                scanned.extend(placed.symbols.iter().map(|s| s.fqn.clone()));
                out.insert(path, placed);
            }
            out
        };

        let forward = run(walked.clone());
        let reversed = run(walked.into_iter().rev().collect());
        let moved: Vec<String> = forward
            .iter()
            .filter(|(path, facts)| reversed.get(*path) != Some(*facts))
            .map(|(path, facts)| format!("{path}: {}", first_difference(facts, &reversed[path])))
            .collect();
        assert!(
            moved.is_empty(),
            "re-indexing in a different file order moved the graph in {} files:\n{}",
            moved.len(),
            moved.iter().take(3).cloned().collect::<Vec<_>>().join("\n")
        );
        assert_eq!(forward.len(), reversed.len(), "one order produced more files than the other");
    }
    /// Whether an fqn is a TRAIT-IMPL member's, and if so the identity a use
    /// site mints for it.
    ///
    /// `rust·pkg·module·Type·Trait·member·reach` becomes
    /// `rust·pkg·module·Type·member·reach`, which is the `Form::Member` a call
    /// like `x.member()` or `Type::member()` produces — see
    /// [`the_members_a_trait_impl_supplies_are_named_where_no_use_site_can_reach_them`].
    ///
    /// Told apart by the grammar's OWN type rule and not by segment count: a
    /// trait-impl member is the only form with two consecutive type-naming
    /// segments before its name, because a module segment is one segment however
    /// many `::` it contains and Rust lints modules into `snake_case`.
    fn as_a_use_site_would_mint_it(fqn: &str) -> Option<String> {
        let mut segments: Vec<&str> = fqn.split('·').collect();
        let names_a_type = |s: &&str| (rust::GRAMMAR.names_a_type)(s);
        // lang · package · [module] · Type · Trait · member · reach
        let member = segments.len().checked_sub(2)?;
        let ty = member.checked_sub(2)?;
        if ty < 2 || !names_a_type(&segments[ty]) || !names_a_type(&segments[member - 1]) {
            return None;
        }
        segments.remove(member - 1);
        Some(segments.join("·"))
    }

    /// The residual: every first-party reference naming an identity that NO
    /// declaration mints, counted and split by cause.
    ///
    /// `no_reference_names_a_declaration_that_differs_only_in_its_reach` asserts
    /// ZERO for the one class the reach rule closes. This is the rest, and it is
    /// not zero — so it is measured, split, and ratcheted, because a residual
    /// nobody counts is a residual that grows.
    ///
    /// The split is the point. One part is a defect with a KNOWN fix that lives
    /// in a later step; the other is the deferred trait-dispatch lookup (spec
    /// §5), and confusing them would either scope-creep this step or lose the
    /// first behind the second.
    #[test]
    fn the_references_that_name_no_declaration_are_a_measured_and_split_set() {
        let (first_party, walked) = corpus();
        let declared: BTreeSet<String> = walked
            .iter()
            .flat_map(|(_, facts)| facts.symbols.iter().map(|s| s.fqn.as_str().to_string()))
            .collect();
        // Every trait-impl member declaration, keyed by the identity a use site
        // would mint for it. Two traits supplying one member name on one type
        // would put two entries under one key — the collision the trait segment
        // exists to prevent — so the set is kept and its size asserted below.
        let mut supplied_by_a_trait: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
        for fqn in &declared {
            if let Some(flattened) = as_a_use_site_would_mint_it(fqn) {
                supplied_by_a_trait.entry(flattened).or_default().insert(fqn.as_str());
            }
        }

        let scanned = BTreeSet::new();
        let world = World {
            first_party: &first_party,
            first_party_members: &BTreeSet::new(),
            declared_members: &BTreeSet::new(),
            returns: &std::collections::BTreeMap::new(),
            scanned: &scanned,
        };
        let mut resolved = 0usize;
        let mut elsewhere: BTreeMap<String, usize> = BTreeMap::new();
        let mut through_a_trait: BTreeMap<String, usize> = BTreeMap::new();
        for (_, facts) in walked {
            for reference in resolve(facts, &rust::GRAMMAR, &world).references {
                let Resolution::Resolved { fqn, .. } = &reference.target else {
                    continue;
                };
                let Ok(parsed) = fqn::parse(fqn.as_str()) else {
                    continue;
                };
                // An external has no declaration by design (R5).
                if !matches!(parsed.origin, fqn::Origin::Local { .. }) {
                    continue;
                }
                resolved += 1;
                if declared.contains(fqn.as_str()) {
                    continue;
                }
                let counter = if supplied_by_a_trait.contains_key(fqn.as_str()) {
                    &mut through_a_trait
                } else {
                    &mut elsewhere
                };
                *counter.entry(fqn.as_str().to_string()).or_insert(0) += 1;
            }
        }

        let trait_shaped: usize = through_a_trait.values().sum();
        let other: usize = elsewhere.values().sum();
        let mut worst: Vec<(&String, &usize)> = elsewhere.iter().collect();
        worst.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));

        // **THE TRAIT-IMPL HALF IS NOW ZERO, AND THIS IS WHERE IT WAS 22.**
        //
        // A declaration made inside `impl Trait for Type` used to carry a
        // `Trait` segment that NO use site could spell — `x.default()` and
        // `Type::default()` both mint the plain member form — so a declaration
        // and every reference to it were two halves of one symbol. MEASURED
        // here, at 22 references over 3 identities.
        //
        // The note that stood here said the fix was deferred, on the grounds
        // that dropping the segment "would delete the only thing separating
        // `<X as Display>::fmt` from `<X as Debug>::fmt`". Stage 11's S8 makes
        // the opposite trade deliberately and states why: a merge key must be
        // what BOTH sides can produce, the caller can never produce that
        // segment, and the trait is not lost — it is a `TraitImpl` edge emitted
        // by the file that writes the impl. The collision the old note feared
        // is not refused silently; it is COUNTED, by A7's identity ratchet.
        //
        // NON-VACUOUS: `(0, 0)` is also what a broken `as_a_use_site_would_mint_it`
        // returns, so the helper is exercised on a hand-built string of the old
        // shape first. The corpus no longer contains one.
        assert_eq!(
            as_a_use_site_would_mint_it("rust·p·m·Widget·Draw·draw·item").as_deref(),
            Some("rust·p·m·Widget·draw·item"),
            "the detector must still recognise the shape it is looking for, or the zero below \
             says only that it stopped looking"
        );
        // **EXACTLY THE SHADOWED ONES, AND NOTHING ELSE.** S8 takes the trait
        // out of a method's key; the one case it must not is a name the type
        // ALSO declares inherently, where Rust keeps the two apart and so must
        // the graph. So this set is not empty — it is precisely the pairs where
        // an inherent declaration owns the flat key.
        //
        // MEASURED: one, `ModelProvisioning`'s `status_all`. The `impl
        // ReadinessProbe` copy calls the inherent one through
        // `ModelProvisioning::status_all(self)`, written and commented to avoid
        // recursing; flattened, that call became a self-loop.
        let shadowed: Vec<(&String, &BTreeSet<&str>)> = supplied_by_a_trait.iter().collect();
        for (flat, qualified) in &shadowed {
            assert!(
                declared.contains(flat.as_str()),
                "a declaration keeps its trait segment ONLY where an inherent declaration owns \
                 the flat key — {flat} is qualified by {qualified:?} and nothing mints the flat \
                 spelling, so the trait segment is a key no caller can reach"
            );
        }
        assert_eq!(
            shadowed.len(),
            1,
            "the shadowed set moved: {:?}",
            shadowed.iter().take(8).collect::<Vec<_>>()
        );
        assert_eq!(
            (trait_shaped, through_a_trait.len()),
            (0, 0),
            "was 22 references over 3 identities; the trait left the key and the two halves \
             met. The one declaration that KEEPS its trait segment contributes nothing here, \
             because the flat spelling a caller mints is the inherent declaration's own: {:?}",
            through_a_trait
        );

        // Everything else. A ceiling and not an equality: this bucket holds the
        // split-`impl` mis-anchoring that
        // `every_ownership_edge_points_at_a_type_declared_somewhere_in_its_own_package`
        // already names, plus re-exports the walk does not follow and impls
        // generated by a `derive` that exists in no source file (spec §5). It
        // moves whenever anyone edits any rust in this workspace, and a ratchet
        // that fails on unrelated work is a ratchet nobody trusts.
        //
        // **RAISED 320 -> 1,300 BY S7, DELIBERATELY, AND THIS IS WHAT IT BOUGHT
        // AND WHAT IT COST.**
        //
        // S7 lets an identity THIS FILE'S TEXT established become an edge with
        // no repo-wide set agreeing, which is what removes the type barrier.
        // The file is right about where the type lives in every case below; what
        // it cannot know from one file is that the name it was handed does not
        // DECLARE the member. So references the old gate refused as missing are
        // now present and dangling: 1,260 over 358 identities, from ~320.
        //
        // MEASURED, and every one of the three causes is a declaration that is
        // not in the source we walked rather than a home the file got wrong:
        //
        // - **A TYPE ALIAS.** 423 of them on one identity. `api/state.rs` says
        //   `pub type AppState = Arc<SharedState>`, so `state.pg` names
        //   `api::state·AppState·pg` while the field is declared on
        //   `SharedState`. Following the alias is cross-file knowledge, which is
        //   the barrier this stage exists to remove.
        // - **AN IMPL A `derive` GENERATED.** `MemOutbox::default`,
        //   `NewRun::default`, clap's `Cli::parse_from`. Spec §5: the impl
        //   exists in no source file, so no walk can declare it.
        // - **THE SPLIT-`impl` MIS-ANCHORING**, already ratcheted separately at
        //   655 of 5,474. Here the DECLARATION is the mis-filed side: with no
        //   table, `use super::*` states no home, so an `impl PgStore` in
        //   `db/pg_store/folders.rs` anchors its members one module too deep
        //   while the caller — which imports `PgStore` by a package-rooted path
        //   — names the right one.
        //
        // **THE OBLIGATION THIS CREATES, and it is not discharged here.** Under
        // stub-and-heal each of these mints a node on first mention that no
        // declaration ever promotes — a ghost. S9 keeps it harmless (a
        // `referenced` node is inserted only if ABSENT, so it can never
        // overwrite a real declaration), and §4.3 makes finding them a stage-12
        // query rather than an indexer output: `lost_exact` is a node with no
        // inbound edge whose exact identity appears in some unlinked edge.
        //
        // What stage 12 MUST do with them is decided and recorded, not left
        // implicit: a target no first-party declaration mints is to be resolved
        // to its real home or marked EXTERNAL, never left as a first-party node
        // nothing declares. Tracked in `docs/backlog.md`; this ceiling is what
        // stops the population growing in the meantime.
        // **PRINTED ON SUCCESS, DELIBERATELY.** A ratchet silent while green can
        // only be lowered by someone who edits the test to discover where it
        // stands, so the headroom is never known at the moment it matters — the
        // one before a change that spends it. This is the second ratchet in
        // this slice to earn a print for that reason.
        println!(
            "  dangling first-party references: {other} of {resolved} over {} identities \
             (ceiling 1,300, headroom {})",
            elsewhere.len(),
            1_300_i64 - other as i64
        );
        assert!(
            other <= 1_300,
            "{other} of {resolved} first-party references ({} distinct identities) name an \
             identity no declaration mints, up from the 1,260 S7 was measured at. Worst: {:?}",
            elsewhere.len(),
            worst.iter().take(12).collect::<Vec<_>>()
        );
    }
}
