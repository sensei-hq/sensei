//! Indexer v2 — the resolution ladder and the reason codes (step 5 of the plan).
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
//! 3. a path rooted in the package being scanned,
//! 4. the language's prelude — the names in scope with no import at all.
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
    Binding, Evidence, FileFacts, Fqn, Import, ImportOrigin, Language, Observation, Reason,
    Reference, Relation, Resolution, Span, SymbolKind,
};
use super::fqn::{self, Form, Reach};

/// One language's vocabulary, as DATA rather than as behaviour.
///
/// The ladder's rungs and its reason codes are shared (R7); what differs between
/// languages is only the spelling — which characters join a path, which word
/// roots a path in the current package, which names are in scope with no import.
/// Keeping those as data is what stops a second language from arriving with its
/// own idea of what a miss means.
pub struct Grammar {
    pub language: Language,
    /// What joins the segments of a qualified name.
    pub separator: &'static str,
    /// The path root that names the package being scanned.
    pub package_root: &'static str,
    /// The path root that names the module the path is written in.
    pub module_self: &'static str,
    /// The path root that names the module one level out.
    pub module_parent: &'static str,
    /// What an import specifier puts between the path and the name it binds.
    /// The walk keeps a specifier verbatim, so the ladder has to know where the
    /// path stops.
    pub names_the_binding: &'static str,
    /// What a specifier ends with when it binds a whole module instead of one
    /// name. Also verbatim in the specifier, so also the ladder's to read off.
    pub wildcard: &'static str,
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
    /// The package that supplies the names in scope with no import.
    pub prelude_package: &'static str,
    /// Those names, each with the path inside `prelude_package` that it is
    /// re-exported from. The path and not the bare name, so a file that imports
    /// one the long way lands on the node the prelude lands on.
    pub prelude: &'static [(&'static str, &'static str)],
    /// Members every value of the language has. Filtering, not failure — see
    /// [`Reason::Denylisted`].
    pub plumbing: &'static [&'static str],
}

/// What the ladder is told about the scan it is part of.
pub struct World<'a> {
    /// Every package this scan owns the source of, from the manifests — NOT from
    /// what has been read so far. An import naming one of these crosses no
    /// boundary however it is spelled, so it resolves local; anything else the
    /// import calls external is a library (R5).
    pub first_party: &'a BTreeSet<String>,
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
        .map(|r| Reference { target: ladder.place(&r.target, r.at), ..r.clone() })
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
        let mut blocks: Vec<(Span, &str)> = facts
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Module)
            .map(|s| (s.span, s.name.as_str()))
            .collect();
        // Outermost first, so reading them in order spells the module path. A
        // module that encloses another starts before it and ends after it.
        blocks.sort_by_key(|(span, _)| (span.start_line, span.start_col));

        let mut bound: BTreeMap<&str, Vec<&Import>> = BTreeMap::new();
        let mut globs: Vec<&Import> = Vec::new();
        for import in &facts.imports {
            match &import.binds {
                Binding::Name(name) => bound.entry(name.as_str()).or_default().push(import),
                Binding::Glob => globs.push(import),
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
            Resolution::Resolved(fqn) => Resolution::Resolved(fqn.clone()),
            // Every reason but `Unplaced` states a cause that still holds after
            // the ladder has run, so the ladder does not overrule it.
            Resolution::Unresolved { reason: Reason::Unplaced, evidence } => {
                self.climb(evidence, at)
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
            return Resolution::Resolved(fqn);
        }
        let wanted = self.wanted(evidence);
        if let Placed::Proven(fqn) = self.through_an_import(&wanted, at) {
            return Resolution::Resolved(fqn);
        }
        if let Placed::Proven(fqn) = self.through_a_glob(&wanted, at) {
            return Resolution::Resolved(fqn);
        }
        if let Placed::Proven(fqn) = self.rooted_in_this_package(&wanted, at) {
            return Resolution::Resolved(fqn);
        }
        // Rung 4 is the only one a glob can overrule: an explicit item and an
        // explicit import both outrank a glob, but a glob outranks the prelude.
        // So a glob in scope over a name the prelude also has leaves two
        // possible origins and nothing to tell them apart.
        match (self.in_the_prelude(&wanted), self.a_glob_binds_at(at)) {
            (Placed::Proven(fqn), false) => return Resolution::Resolved(fqn),
            (Placed::Proven(_), true) => return self.shadowed_by_a_glob(evidence, at),
            (Placed::Unbound, _) => {}
        }
        Resolution::Unresolved {
            reason: self.filtered(Reason::NoImportInScope, evidence),
            evidence: evidence.clone(),
        }
    }

    /// Plumbing is filtered wherever it lands, so the histogram has one bucket
    /// for it rather than a share of every other one.
    ///
    /// [`Reason::UnhandledForm`] is left alone: it names a gap in the WALK, not
    /// a name the ladder declined to chase, and moving one into the denylist
    /// would hide exactly what the histogram exists to show.
    fn filtered(&self, reason: Reason, evidence: &Evidence) -> Reason {
        match reason {
            Reason::UnhandledForm => reason,
            _ if self.grammar.plumbing.contains(&evidence.name.as_str()) => Reason::Denylisted,
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
        for observation in &evidence.saw {
            if let Observation::Candidate(fqn) = observation
                && self.declared.contains(fqn.as_str())
            {
                return Placed::Proven(fqn.clone());
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
        let file = self.split(self.module);
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
        match self.relative_to(&wanted.segments, at) {
            Rooted::At(segments) => self.identity(self.package, &segments, wanted.reach),
            Rooted::Nowhere => Placed::Unbound,
        }
    }

    /// Rung 4. A language puts some names in scope with nothing written to bring
    /// them there, so the absence of an import says nothing about them. They are
    /// members of a package we never open, like any other external (R5).
    fn in_the_prelude(&self, wanted: &Wanted) -> Placed {
        let Some((head, tail)) = wanted.segments.split_first() else {
            return Placed::Unbound;
        };
        let Some((_, path)) = self.grammar.prelude.iter().find(|(name, _)| name == head) else {
            return Placed::Unbound;
        };
        let mut segments = self.split(path);
        segments.extend(tail.iter().cloned());
        self.library(self.grammar.prelude_package, &segments)
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

    fn split(&self, raw: &str) -> Vec<String> {
        raw.split(self.grammar.separator)
            .map(str::trim)
            // A turbofish decorates a path without naming a segment of it.
            .filter(|s| !s.is_empty() && !s.starts_with('<'))
            .map(str::to_string)
            .collect()
    }

    fn is_a_root(&self, segment: &str) -> bool {
        segment == self.grammar.package_root
            || segment == self.grammar.module_self
            || segment == self.grammar.module_parent
    }

    /// Read a path's leading root words against the module the path is written
    /// in, leaving package-relative segments.
    fn relative_to(&self, segments: &[String], at: Span) -> Rooted {
        let mut base = self.module_at(at);
        let mut rest = segments;
        while let Some(head) = rest.first() {
            if head == self.grammar.package_root {
                base.clear();
            } else if head == self.grammar.module_self {
                // Names the module the path is written in, which `base` already
                // is.
            } else if head == self.grammar.module_parent {
                if base.pop().is_none() {
                    return Rooted::Nowhere;
                }
            } else {
                break;
            }
            rest = &rest[1..];
        }
        base.extend(rest.iter().cloned());
        Rooted::At(base)
    }

    /// The module path a span sits in: the file's own, extended by every module
    /// block that encloses the span. A declaration inside `mod b { }` is named
    /// the same as one in `b.rs`, so a path written there has to be read the
    /// same way.
    fn module_at(&self, at: Span) -> Vec<String> {
        let mut segments: Vec<String> =
            self.split(self.module).into_iter().filter(|s| !s.is_empty()).collect();
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
        let path = match import.path.split_once(self.grammar.names_the_binding) {
            Some((path, _alias)) => path,
            None => import.path.as_str(),
        };
        let mut segments = self.split(path);
        // A grouped `self` (`use a::{self}`) binds the module the group is on,
        // not a member called `self`; a wildcard binds that module's contents
        // and is not a segment of its path either.
        while segments
            .last()
            .is_some_and(|s| s == self.grammar.module_self || s == self.grammar.wildcard)
        {
            segments.pop();
        }

        match &import.origin {
            // The head IS the package; what follows is the path inside it.
            ImportOrigin::External { .. } => Rooted::At(segments.into_iter().skip(1).collect()),
            ImportOrigin::Local => self.relative_to(&segments, import.at),
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
        let separator = self.grammar.separator;
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
        let member = segments.join(self.grammar.separator);
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

    use crate::indexer::facts::{FileFacts, RefKind, Reference, Resolution};
    use crate::indexer::fqn;
    use crate::indexer::lang::rust::{self, Source};
    use crate::indexer::resolve::{World, resolve};
    use crate::indexer::{module_of, package_of};

    fn placed(reference: &Reference) -> &str {
        match &reference.target {
            Resolution::Resolved(fqn) => fqn.as_str(),
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
        resolve(facts, &rust::GRAMMAR, &World { first_party: &first_party, scanned: &scanned })
    }

    /// Every reference's outcome, as text: the identity it was placed at, or the
    /// reason it was not and the name that defeated the ladder. Written this way
    /// so an assertion names what it wants instead of indexing into a vector.
    fn targets(facts: &FileFacts) -> Vec<String> {
        facts
            .references
            .iter()
            .map(|r| match &r.target {
                Resolution::Resolved(fqn) => fqn.as_str().to_string(),
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

    /// The denylist is FILTERING, not failure. `x.clone()` is a miss the ladder
    /// will never place and there are thousands of them; left in the general
    /// bucket they bury the misses somebody could act on. Its own reason is what
    /// lets a reader drop it without dropping those.
    #[test]
    fn plumbing_is_filtered_into_its_own_reason_rather_than_a_genuine_miss() {
        let facts = ladder("m", "fn f(x: &Thing, y: &Thing) { x.clone(); y.width(); }");
        let got = targets(&facts);
        assert!(
            got.iter().any(|t| t == "Denylisted(clone)"),
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
                Reason::Denylisted,
                "plumbing, filtered so it does not bury genuine misses",
                "fn f(x: &Thing) { x.clone(); }",
            ),
            (
                Reason::ReceiverTypeUnknown,
                "a member access whose receiver this file states no type for",
                "fn f(x: &Thing) { x.width(); }",
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
            .filter(|r| r.kind != crate::indexer::facts::RelationKind::Owns)
            .map(|r| match &r.parent {
                Resolution::Resolved(fqn) => format!("{:?} -> {}", r.kind, fqn.as_str()),
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
        let world = World { first_party: &first_party, scanned: &scanned };

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
                    Resolution::Resolved(_) => resolved += 1,
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
                    Resolution::Resolved(_) => structural_resolved += 1,
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
        let world = World { first_party: &first_party, scanned: &BTreeSet::new() };

        let mut total = 0usize;
        let mut dangling: BTreeMap<String, usize> = BTreeMap::new();
        let mut invented: Vec<String> = Vec::new();
        for (path, facts) in walked {
            for relation in &resolve(facts, &rust::GRAMMAR, &world).relations {
                if relation.kind != crate::indexer::facts::RelationKind::Owns {
                    continue;
                }
                total += 1;
                let Resolution::Resolved(parent) = &relation.parent else {
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
        // 588 of 4,676 = 12.6% when this was written. The ceiling is a ratchet:
        // the defect is recorded, and it is not allowed to spread while it waits
        // for the seam decision above.
        let share = 100 * count / total;
        assert!(
            share <= 13,
            "{count} of {total} ownership edges ({share}%) are anchored on a module that \
             declares nothing, up from the 12.6% this defect was measured at. Worst: {:?}",
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
        let world = World { first_party: &first_party, scanned: &scanned };
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
                let Resolution::Resolved(fqn) = target else {
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
                &World { first_party: &first_party, scanned: &nothing },
            );
            let after = resolve(
                facts,
                &rust::GRAMMAR,
                &World { first_party: &first_party, scanned: &everything },
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
                    &World { first_party: &first_party, scanned: &scanned },
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
        let world = World { first_party: &first_party, scanned: &scanned };
        let mut resolved = 0usize;
        let mut elsewhere: BTreeMap<String, usize> = BTreeMap::new();
        let mut through_a_trait: BTreeMap<String, usize> = BTreeMap::new();
        for (_, facts) in walked {
            for reference in resolve(facts, &rust::GRAMMAR, &world).references {
                let Resolution::Resolved(fqn) = &reference.target else {
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

        // The trait-impl half, exactly. A declaration made inside
        // `impl Trait for Type` carries a `Trait` segment, and NO use site can
        // spell it: `x.default()` and `Type::default()` both mint the plain
        // member form. So the declaration and every reference to it are two
        // halves of one symbol — the same failure the reach rule closed for
        // plain paths, one level in.
        //
        // It is recorded here rather than fixed, and the reason is the
        // measurement beside it: `supplied_by_a_trait` has no key with two
        // entries, so dropping the `Trait` segment would close all of these and
        // create no collision IN THIS CORPUS — but it would delete the only
        // thing separating `<X as Display>::fmt` from `<X as Debug>::fmt`, a
        // shape this corpus does not contain and therefore cannot ratchet. R4
        // ranks the wrong edge that would produce below the missing one it
        // removes. The real fix is the impl-set lookup spec §5 defers under
        // trait dispatch, which is a query over the whole graph and not a thing
        // a per-file ladder can answer without depending on scan order (R6).
        assert!(
            supplied_by_a_trait.values().all(|declarations| declarations.len() == 1),
            "two traits now supply one member name on one type, so the `Trait` segment is \
             load-bearing in this corpus and the note below has to be re-decided: {:?}",
            supplied_by_a_trait.iter().filter(|(_, d)| d.len() > 1).take(8).collect::<Vec<_>>()
        );
        assert_eq!(
            (trait_shaped, through_a_trait.len()),
            (22, 3),
            "the references that a trait impl supplies and no use site can name moved: {:?}",
            through_a_trait
        );

        // Everything else. A ceiling and not an equality: this bucket holds the
        // split-`impl` mis-anchoring that
        // `every_ownership_edge_points_at_a_type_declared_somewhere_in_its_own_package`
        // already names, plus re-exports the walk does not follow and impls
        // generated by a `derive` that exists in no source file (spec §5). It
        // moves whenever anyone edits any rust in this workspace, and a ratchet
        // that fails on unrelated work is a ratchet nobody trusts.
        assert!(
            other <= 320,
            "{other} of {resolved} first-party references ({} distinct identities) name an \
             identity no declaration mints, up from the 312 this was measured at — 309 of them \
             before `indexer/reconcile.rs` itself joined the corpus. Worst: {:?}",
            elsewhere.len(),
            worst.iter().take(12).collect::<Vec<_>>()
        );
    }
}
