//! The fact vocabulary one walk produces (spec §3).
//!
//! Types, and the labels those types spell themselves with. Every rule that
//! INTERPRETS them lives elsewhere: the fqn grammar in `fqn.rs`, the resolution
//! ladder in `resolve.rs`, grammar reading in `lang/`. That split is R7 — a
//! language module owns "how do I read this grammar" and owns nothing else.
//!
//! Two shapes here are load-bearing:
//!
//! - [`Resolution`] is TOTAL. There is no variant meaning "nothing", so a walk
//!   that cannot place a target still has to say something, and the only thing
//!   it can say is [`Resolution::Unresolved`] with a [`Reason`] and the
//!   [`Evidence`] it had in hand (R2, R4). An `Option` here would let a miss be
//!   dropped on the floor, which is the defect this rewrite exists to remove.
//! - Nothing here can be constructed without stating every field. A value that
//!   filled itself in is indistinguishable from one the walk actually read, and
//!   that is how fabricated data enters (R4).
//
// This module has no caller on purpose: the shipped indexer under
// `crate::languages` keeps producing the graph until the rust cutover in step 9
// at stage 10 (`docs/spec/indexer/10-cutover.md`). The allow lets the two coexist
// without either one carrying warnings.
#![allow(dead_code)]

use std::fmt;

use super::fqn::Reach;

// ── identity ─────────────────────────────────────────────────────────────────

/// Where a fact was read from, in the file the walk was given. Columns are
/// carried as well as lines because two references frequently share a line
/// (`a.b().c()`), and a fact that cannot be told apart from its neighbour
/// cannot be counted (A2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Span {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

/// The identity of a symbol — a lookup key minted independently by a definition
/// and by a reference (spec §2).
///
/// The inner string is private and there is no way to build one from arbitrary
/// text: `fqn::define` and `fqn::refer` are the only doors. That is the whole
/// point of the newtype. Two call sites formatting their own string is how the
/// definition side and the reference side drift apart, and the drift is
/// invisible in review because each side looks correct alone.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fqn(String);

impl Fqn {
    /// Wrap an already-encoded fqn. `pub(super)` and named for what the caller
    /// must have done first: `fqn.rs` calls this after the grammar has checked
    /// the segments, and nothing else calls it at all — see the guard test
    /// `no_fqn_is_built_by_string_formatting_outside_this_file`.
    pub(super) fn from_encoded(encoded: String) -> Self {
        Self(encoded)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Fqn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A language the walk can read.
///
/// An enum rather than a string so the leading fqn segment cannot be spelled
/// two ways: `"rust"` on the definition side and `"Rust"` on the reference side
/// would be two graphs that never meet.
///
/// **`.js`, `.ts` and a `.svelte` script block are ONE language here, and that
/// is a merge decision rather than a taxonomy.** They share a module graph: a
/// `.ts` file importing from a `.js` file is ordinary, and if the two sides
/// carried different leading segments the import would mint an identity the
/// declaration never mints and the two would never meet (spec §2). Three
/// adapters read them — they claim different extensions and only one of them
/// has annotations to read — but all three file their symbols under this one
/// label, which is also the label the legacy producer used, so a differential
/// over JS compares like with like.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Language {
    Rust,
    /// JavaScript, TypeScript, and the script block of a Svelte component.
    TypeScript,
    Java,
}

impl Language {
    /// Every language this build can read. Exhaustively matched below, so a new
    /// variant does not compile until it is listed here too.
    pub fn all() -> &'static [Language] {
        &[Language::Rust, Language::TypeScript, Language::Java]
    }

    /// The label this language occupies the leading fqn segment with. Paired
    /// with [`Language::from_label`] here so the two directions cannot drift.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            Self::Java => "java",
        }
    }

    /// The inverse of [`Language::as_str`]. `None` means the label names no
    /// language this build can read — the caller turns that into a typed error
    /// rather than picking one.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "rust" => Some(Self::Rust),
            "typescript" => Some(Self::TypeScript),
            "java" => Some(Self::Java),
            _ => None,
        }
    }
}

// ── declarations (spec §3.1) ─────────────────────────────────────────────────

/// What kind of declaration a [`Symbol`] is.
///
/// The set is spec §3.1 verbatim. Fields, properties and enum variants are in
/// it because "what shape is this data" is a question the graph must answer
/// (A5), and because pattern detection reads field types (R8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    /// A variant of an enum. A member of its enum, not a free item.
    EnumVariant,
    Interface,
    Trait,
    TypeAlias,
    Const,
    /// Distinct from [`SymbolKind::Const`]: a static has an address and a
    /// lifetime, and singleton detection reads exactly that (R8).
    Static,
    Module,
    /// A `macro_rules!` or proc-macro definition. Its own kind because only a
    /// `name!` reaches a macro, and "where is X defined" must answer for a
    /// macro too.
    Macro,
    /// A declared storage slot on a type.
    Field,
    /// An accessor pair that presents as a slot. Kept apart from
    /// [`SymbolKind::Field`] because one is storage and the other is code.
    Property,
}

/// How far a declaration is visible. Read by singleton detection (R8) and by
/// anyone asking what a module's surface is.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Visibility {
    Public,
    Crate,
    /// Visible within a stated scope — Rust's `pub(super)` or `pub(in path)`.
    /// The scope is kept verbatim because narrowing it to a flag would discard
    /// which scope was meant.
    Restricted(String),
    Private,
}

/// A type the language STATES, verbatim as written.
///
/// Total rather than an `Option` because the difference matters downstream: a
/// walk that read `-> Arc<PgStore>` and a walk looking at a function with no
/// return type are two different facts, and only the second one licenses
/// [`Reason::NoDeclaredType`] on a reference through it. Inferring the type of
/// an unannotated value is a non-goal (spec §5).
///
/// Kept verbatim on purpose: normalising to a bare name here would throw away
/// the module path that says WHICH `PgStore` is meant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeclaredType {
    Stated(String),
    Unstated,
}

/// A parameter, carried as a typed prop on its function rather than as a node
/// of its own (D2) — a parameter is not something a reader navigates to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Param {
    pub name: String,
    /// Position in the parameter list, from zero. The name alone does not
    /// identify a parameter: two overloads and a tuple-struct field both need
    /// the position to stay distinct.
    pub position: u32,
    pub declared_type: DeclaredType,
}

/// One declaration (spec §3.1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub fqn: Fqn,
    pub kind: SymbolKind,
    /// The name as declared, without any qualification. The qualified identity
    /// is [`Symbol::fqn`].
    pub name: String,
    pub span: Span,
    pub visibility: Visibility,
    /// The doc comment attached to this declaration. `None` means the source
    /// carries none — this is never where a failed read is parked.
    pub docstring: Option<String>,
    /// The type this declaration STATES: a field's type, a function's return
    /// type. [`DeclaredType::Unstated`] where the language states none.
    pub declared_type: DeclaredType,
    /// Empty for a declaration that takes no parameters and for every kind that
    /// cannot take any. Both are genuinely empty, not a failure to read.
    pub params: Vec<Param>,
}

// ── references (spec §3.2) ───────────────────────────────────────────────────

/// What a use site does with its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefKind {
    Calls,
    Reads,
    Writes,
    /// Builds a value of the target type. Factory detection needs construction
    /// separated from calling (R8).
    Constructs,
    /// Names the target as a type — in a signature, a field, a bound. This is
    /// what makes field and parameter types edges rather than loose strings.
    TypeUse,
    /// Invokes a macro. A distinct kind because what a macro expands to is not
    /// parsed, so a call INSIDE the expansion is not a fact this walk has.
    MacroInvokes,
}

/// Why a reference could not be resolved.
///
/// A closed enum, one variant per distinct cause, and causes are never
/// collapsed: telling them apart is how coverage gets measured (A3). A `String`
/// here would not be exhaustively matchable and the histogram would grow junk
/// categories that nobody can act on.
///
/// The variants carry no payload. What the walk SAW belongs in [`Evidence`]; a
/// reason is the bucket, evidence is the material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reason {
    /// The walk has no rule for this node kind yet. The fallback arm, so that a
    /// form nobody thought about is named in the histogram instead of silently
    /// yielding nothing.
    UnhandledForm,
    /// The language states no type at the point resolution needed one — an
    /// unannotated binding, a closure parameter. Inferring it is a non-goal
    /// (spec §5).
    NoDeclaredType,
    /// A member access whose receiver type this file cannot determine, most
    /// often a chain deeper than the walk can follow.
    ReceiverTypeUnknown,
    /// A bare name with no local declaration and no import that binds it.
    /// NOT the same as external: externality comes from an import (spec §2).
    NoImportInScope,
    /// More than one candidate matched and nothing distinguishes them. Picking
    /// one would be a wrong edge, which is worse than a missing one (R4).
    AmbiguousCandidates,
    /// The target is chosen at run time — a trait object, a function pointer.
    /// No static answer exists, so this is a permanent miss, not a gap.
    DynamicDispatch,
    /// The target would only exist after macro expansion, and expansions are
    /// not parsed.
    MacroExpansion,
    /// Plumbing the grammar deliberately filters — `clone`, `unwrap`,
    /// `toString`. Filtering, not failure: its own reason so a reader can
    /// exclude it without also excluding genuine misses.
    ///
    /// Named for what it IS rather than for the mechanism that catches it. It
    /// was `Denylisted`, which described the list in `Grammar::plumbing` and
    /// left a reader to work out that the list holds plumbing; the field and the
    /// reason now share a name.
    Plumbing,
    /// The target names a member NO first-party type declares anywhere in the
    /// scan, so it cannot be an edge we could ever draw: it is a method on a
    /// library type — `.trim()`, `.collect()`, `.toBe()` — at the boundary R5
    /// says we name and never open.
    ///
    /// Its own reason because the alternative was counting it as a failure.
    /// MEASURED: 23,739 of the 53,224 receivers reported as
    /// [`Reason::ReceiverTypeUnknown`] were this, and the number read as a gap
    /// somebody should close. It is not one — no amount of type inference makes
    /// `.toBe()` a first-party edge — and a bucket that mixes "we dropped
    /// something" with "this is where our world ends" cannot be acted on.
    ///
    /// Decided from the corpus's own declarations, never from a list: a name is
    /// at the boundary exactly when nothing we index declares it.
    ExternalBoundary,
    /// The walk read the use site and minted the candidate identity it names,
    /// but placing that candidate needs the shared resolution ladder, which the
    /// walk is not (R7). The walk never reaches outside the file it was handed,
    /// so this is what a well-understood use site carries between the walk and
    /// the ladder.
    ///
    /// Distinct from every reason above because those state a cause that will
    /// still hold after the ladder runs; this one states only that the ladder
    /// has not run. It is the one bucket that must be EMPTY once resolution is
    /// in place, which is a thing a histogram can check — a reason chosen from
    /// the list above instead would hide the walk's incompleteness behind a
    /// cause that is not true of it (R4).
    Unplaced,
}

impl Reason {
    /// The stable label this reason is written and read under.
    ///
    /// ONE labeling, because there is more than one consumer: `persist` writes
    /// it into `edges.props` and reads it back, and `impact` shows it to a
    /// person. Two matches over the same enum agree on the day they are written
    /// and drift on the day a variant is added to one of them — and the drift
    /// surfaces as a reason that round-trips to `None`, which reads as "this
    /// miss has no reason" rather than as the bug it is.
    pub fn as_label(self) -> &'static str {
        match self {
            Self::UnhandledForm => "unhandled_form",
            Self::NoDeclaredType => "no_declared_type",
            Self::ReceiverTypeUnknown => "receiver_type_unknown",
            Self::NoImportInScope => "no_import_in_scope",
            Self::AmbiguousCandidates => "ambiguous_candidates",
            Self::DynamicDispatch => "dynamic_dispatch",
            Self::MacroExpansion => "macro_expansion",
            Self::Plumbing => "plumbing",
            Self::ExternalBoundary => "external_boundary",
            Self::Unplaced => "unplaced",
        }
    }

    /// The inverse of [`Reason::as_label`]. `None` for a label no variant
    /// claims, which is a corrupt or future row and never a silent default.
    pub fn from_label(label: &str) -> Option<Self> {
        Some(match label {
            "unhandled_form" => Self::UnhandledForm,
            "no_declared_type" => Self::NoDeclaredType,
            "receiver_type_unknown" => Self::ReceiverTypeUnknown,
            "no_import_in_scope" => Self::NoImportInScope,
            "ambiguous_candidates" => Self::AmbiguousCandidates,
            "dynamic_dispatch" => Self::DynamicDispatch,
            "macro_expansion" => Self::MacroExpansion,
            "plumbing" => Self::Plumbing,
            "external_boundary" => Self::ExternalBoundary,
            "unplaced" => Self::Unplaced,
            _ => return None,
        })
    }

    /// Does this miss cast doubt on a first-party edge, or has the ladder
    /// positively placed the site OUTSIDE?
    ///
    /// Two of the ten are verdicts rather than failures. [`Reason::Plumbing`]
    /// is documented as "filtering, not failure" — `.ok()` on a `Result` is
    /// plumbing. [`Reason::ExternalBoundary`] is decided by "nothing we index
    /// declares this name", which is the ladder saying the world ends here —
    /// `.as_deref()` is not an edge anyone lost. Neither is a gap somebody
    /// should close, and counting them as uncertainty makes the graph look
    /// least sure exactly where it is most.
    ///
    /// On the enum rather than beside one consumer, because there are now two
    /// — `impact` excludes them from a blast radius, and the acceptance report
    /// splits the histogram by them — and a partition of this enum written
    /// twice would drift the first time a variant is added.
    ///
    /// MEASURED over this repo: the two account for 45,320 of 97,681 misses,
    /// headed by std methods (`map`, `into`, `as_str`, `collect`) that merely
    /// share a name with something first-party.
    pub fn casts_doubt(self) -> bool {
        !matches!(self, Self::Plumbing | Self::ExternalBoundary)
    }

    /// Every variant, so a test or a report can iterate the whole taxonomy
    /// rather than restate it and fall behind.
    pub const ALL: &'static [Reason] = &[
        Reason::UnhandledForm,
        Reason::NoDeclaredType,
        Reason::ReceiverTypeUnknown,
        Reason::NoImportInScope,
        Reason::AmbiguousCandidates,
        Reason::DynamicDispatch,
        Reason::MacroExpansion,
        Reason::Plumbing,
        Reason::ExternalBoundary,
        Reason::Unplaced,
    ];
}

/// Something the walk saw at the use site and could not turn into a target.
///
/// This is material for a later pass, never an answer. A [`Observation::Candidate`]
/// in particular is a key that was CONSIDERED — treating it as the resolution is
/// exactly the wrong-merge this design refuses.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Observation {
    /// The receiver expression, verbatim source text (`ctx.pg()`).
    Receiver(String),
    /// An import in scope that could have bound this name, verbatim as written.
    ImportInScope(String),
    /// A type the walk read but could not place in the graph.
    UnplacedType(String),
    /// An identity the walk considered and could not prove.
    Candidate(Fqn),
}

/// What the walk had in hand at the moment it could not resolve (spec §3.2).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Evidence {
    /// The bare name at the use site. Always present — a use the walk cannot
    /// even name is not one it can emit.
    pub name: String,
    /// The tree-sitter node kind of the use site. This is what names an
    /// [`Reason::UnhandledForm`] miss in the histogram.
    pub node_kind: String,
    /// HOW this use site reaches its target (spec §2.1) — a function of the use
    /// SYNTAX, so only the walk that read the syntax can state it.
    ///
    /// Here rather than derived downstream from [`RefKind`], and the difference
    /// is not tidiness. `RefKind::Reads` is a path leaf at `a::B::C` and a field
    /// at `x.y`, so the kind alone cannot tell them apart and anything deriving
    /// a reach from it would mint `item` for half this repo's field reads — a
    /// wrong identity, on the side of the merge contract that has no way to
    /// notice (R4, R7).
    pub reach: Reach,
    /// Everything else the walk saw, in the order it saw it. Empty means it saw
    /// nothing beyond the name, which is itself a reading worth having.
    pub saw: Vec<Observation>,
}

/// Which rung of the ladder placed an edge.
///
/// The rungs are not interchangeable, and a consumer handed a bare "resolved"
/// treats the weakest claim the ladder makes exactly like the strongest.
/// [`Rung::DeclaredHere`] is a file pointing at its own declaration — there is
/// nothing to be wrong about. [`Rung::ThroughAGlob`] is a name bound by
/// `use x::*`, where the ladder knows the glob covers the module but the source
/// never wrote the name down. Same `Resolved`, very different evidence.
///
/// It is also the one field that maps an edge back to the code that made it. A
/// rung IS a method on `Ladder`, so a wrong edge tagged `through_a_glob` names
/// `Ladder::through_a_glob` as the thing to go and read — which is what makes a
/// defect report actionable instead of a search.
///
/// Declaration order is CLIMB order, and [`Rung::ALL`] relies on it: reading the
/// list is reading the ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rung {
    /// The file declares the target itself.
    DeclaredHere,
    /// An import in scope binds the head of the path.
    ThroughAnImport,
    /// A glob in scope covers the module the target sits in. The name itself
    /// was never written down, which is what makes this the weakest first-party
    /// rung.
    ThroughAGlob,
    /// A path rooted at this package, needing no import.
    RootedInThisPackage,
    /// A fully-qualified path that leaves the scanned source (R5). Named, never
    /// opened.
    FullyQualifiedExternal,
    /// A name the language puts in scope everywhere. Outranked by a glob, which
    /// could have shadowed it.
    InThePrelude,
}

impl Rung {
    /// The stable label this rung is written and read under. One labeling, for
    /// the reason [`Reason::as_label`] gives.
    pub fn as_label(self) -> &'static str {
        match self {
            Self::DeclaredHere => "declared_here",
            Self::ThroughAnImport => "through_an_import",
            Self::ThroughAGlob => "through_a_glob",
            Self::RootedInThisPackage => "rooted_in_this_package",
            Self::FullyQualifiedExternal => "fully_qualified_external",
            Self::InThePrelude => "in_the_prelude",
        }
    }

    /// The inverse of [`Rung::as_label`]. `None` for a label no rung claims.
    pub fn from_label(label: &str) -> Option<Self> {
        Some(match label {
            "declared_here" => Self::DeclaredHere,
            "through_an_import" => Self::ThroughAnImport,
            "through_a_glob" => Self::ThroughAGlob,
            "rooted_in_this_package" => Self::RootedInThisPackage,
            "fully_qualified_external" => Self::FullyQualifiedExternal,
            "in_the_prelude" => Self::InThePrelude,
            _ => return None,
        })
    }

    /// Every rung, in the order `Ladder::climb` tries them.
    pub const ALL: &'static [Rung] = &[
        Rung::DeclaredHere,
        Rung::ThroughAnImport,
        Rung::ThroughAGlob,
        Rung::RootedInThisPackage,
        Rung::FullyQualifiedExternal,
        Rung::InThePrelude,
    ];
}

/// Where a reference points. Total by construction (spec §3.2, R2).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Resolution {
    Resolved { fqn: Fqn, via: Rung },
    Unresolved { reason: Reason, evidence: Evidence },
}

/// One use site (spec §3.2). Exactly one of these per use site in the AST —
/// a miss is an `Unresolved` target, never an omitted reference (R2, A2).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Reference {
    /// The symbol the use site sits inside.
    pub from: Fqn,
    pub kind: RefKind,
    pub at: Span,
    pub target: Resolution,
}

// ── structure (spec §3.3) ────────────────────────────────────────────────────

/// What one symbol is to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelationKind {
    /// Single inheritance.
    Extends,
    /// Interface implementation.
    Implements,
    /// `impl Trait for Type`. Same shape as [`RelationKind::Implements`], a
    /// different fact, and kept apart so a query can ask for either.
    TraitImpl,
    Mixin,
    Decorates,
    /// Member ownership: a type owns its fields and methods.
    ///
    /// This is what an inherent `impl Foo { }` produces. It is NOT inheritance —
    /// emitting an `Extends` for an inherent impl would be a false edge that
    /// pattern detection reads as real.
    Owns,
}

/// One structural fact, emitted by the same walk that emits symbols (D5).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Relation {
    pub kind: RelationKind,
    /// The declaring side: the subtype, or the owned member.
    pub child: Fqn,
    /// The other side, resolved the same way a call target is. Total, so a
    /// supertype the walk could not place is an `Unresolved` with a reason —
    /// never a bare name with nothing said about it.
    pub parent: Resolution,
    pub at: Span,
}

// ── imports (spec §2) ────────────────────────────────────────────────────────

/// What an import brings into scope.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Binding {
    /// Binds exactly this name — the last path segment, or the alias.
    Name(String),
    /// A glob. It binds an unknown set of names, so a name that MIGHT have come
    /// from here is not proof that it did (R4).
    Glob,
}

/// Whether an import crosses out of the scanned source.
///
/// This is the ONLY thing that decides externality (spec §2). A symbol being
/// absent from what has been scanned so far decides nothing, because what has
/// been scanned depends on file order and resolution must not (R6).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImportOrigin {
    Local,
    External { package: String },
}

/// One import (spec §3).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Import {
    /// The specifier verbatim as written.
    pub path: String,
    pub binds: Binding,
    pub origin: ImportOrigin,
    pub at: Span,
}

// ── the product of one walk (spec §3) ────────────────────────────────────────

/// Everything one parse of one file produced. A file is parsed once and no
/// later stage re-reads its bytes (R1), so whatever is missing from here is
/// missing from the graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileFacts {
    pub language: Language,
    /// The owning crate/package, supplied by the processor from the nearest
    /// manifest.
    pub package: String,
    /// This file's package-relative module path, empty at the package root.
    pub module: String,
    /// Where this file is. A fact of the walk and not an argument the writer
    /// supplies again later, because the identity a file-scope use site is
    /// filed under and the identity the writer hangs imports off must be one
    /// string — and at a crate root, where the module path is empty and a
    /// package may have several, the path is what tells two files apart.
    pub path: String,
    pub symbols: Vec<Symbol>,
    pub references: Vec<Reference>,
    pub relations: Vec<Relation>,
    pub imports: Vec<Import>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::fqn::{self, Form};

    fn a_span() -> Span {
        Span { start_line: 12, start_col: 4, end_line: 12, end_col: 9 }
    }

    /// Built through the one door that mints an [`Fqn`], because nothing else
    /// may — see the guard test in `fqn.rs`.
    fn an_fqn(name: &str) -> Fqn {
        fqn::define(&Form::Item {
            lang: Language::Rust,
            package: "senseid",
            module: "indexer::facts",
            name,
            reach: fqn::Reach::Item,
        })
        .expect("the test's own fqn parts are well formed")
    }

    fn all_languages() -> Vec<Language> {
        Language::all().to_vec()
    }

    fn all_symbol_kinds() -> Vec<SymbolKind> {
        vec![
            SymbolKind::Function,
            SymbolKind::Method,
            SymbolKind::Class,
            SymbolKind::Struct,
            SymbolKind::Enum,
            SymbolKind::EnumVariant,
            SymbolKind::Interface,
            SymbolKind::Trait,
            SymbolKind::TypeAlias,
            SymbolKind::Const,
            SymbolKind::Static,
            SymbolKind::Module,
            SymbolKind::Macro,
            SymbolKind::Field,
            SymbolKind::Property,
        ]
    }

    fn all_visibilities() -> Vec<Visibility> {
        vec![
            Visibility::Public,
            Visibility::Crate,
            Visibility::Restricted("super".to_string()),
            Visibility::Private,
        ]
    }

    fn all_declared_types() -> Vec<DeclaredType> {
        vec![DeclaredType::Stated("Arc<PgStore>".to_string()), DeclaredType::Unstated]
    }

    fn all_ref_kinds() -> Vec<RefKind> {
        vec![
            RefKind::Calls,
            RefKind::Reads,
            RefKind::Writes,
            RefKind::Constructs,
            RefKind::TypeUse,
            RefKind::MacroInvokes,
        ]
    }

    fn all_reasons() -> Vec<Reason> {
        vec![
            Reason::UnhandledForm,
            Reason::NoDeclaredType,
            Reason::ReceiverTypeUnknown,
            Reason::NoImportInScope,
            Reason::AmbiguousCandidates,
            Reason::DynamicDispatch,
            Reason::MacroExpansion,
            Reason::Plumbing,
            Reason::ExternalBoundary,
            Reason::Unplaced,
        ]
    }

    fn all_observations() -> Vec<Observation> {
        vec![
            Observation::Receiver("ctx.pg()".to_string()),
            Observation::ImportInScope("crate::db::PgStore".to_string()),
            Observation::UnplacedType("PgStore".to_string()),
            Observation::Candidate(an_fqn("candidate")),
        ]
    }

    fn an_evidence() -> Evidence {
        Evidence {
            name: "pg".to_string(),
            node_kind: "field_expression".to_string(),
            reach: Reach::Field,
            saw: all_observations(),
        }
    }

    fn all_resolutions() -> Vec<Resolution> {
        vec![
            Resolution::Resolved { fqn: an_fqn("resolved"), via: Rung::DeclaredHere },
            Resolution::Unresolved { reason: Reason::ReceiverTypeUnknown, evidence: an_evidence() },
        ]
    }

    fn all_relation_kinds() -> Vec<RelationKind> {
        vec![
            RelationKind::Extends,
            RelationKind::Implements,
            RelationKind::TraitImpl,
            RelationKind::Mixin,
            RelationKind::Decorates,
            RelationKind::Owns,
        ]
    }

    fn all_bindings() -> Vec<Binding> {
        vec![Binding::Name("PgStore".to_string()), Binding::Glob]
    }

    fn all_import_origins() -> Vec<ImportOrigin> {
        vec![ImportOrigin::Local, ImportOrigin::External { package: "serde_json".to_string() }]
    }

    /// Every match below is wildcard-free on purpose: a new variant stops
    /// compiling here until it is also added to the `all_*` list, so the lists
    /// cannot silently fall behind the enums they claim to enumerate.
    #[test]
    fn every_declaration_fact_variant_is_constructible() {
        for l in all_languages() {
            match l {
                Language::Rust | Language::TypeScript | Language::Java => {}
            }
        }
        assert_eq!(all_languages().len(), 3);

        for k in all_symbol_kinds() {
            match k {
                SymbolKind::Function
                | SymbolKind::Method
                | SymbolKind::Class
                | SymbolKind::Struct
                | SymbolKind::Enum
                | SymbolKind::EnumVariant
                | SymbolKind::Interface
                | SymbolKind::Trait
                | SymbolKind::TypeAlias
                | SymbolKind::Const
                | SymbolKind::Static
                | SymbolKind::Module
                | SymbolKind::Macro
                | SymbolKind::Field
                | SymbolKind::Property => {}
            }
        }
        assert_eq!(all_symbol_kinds().len(), 15);

        for v in all_visibilities() {
            match v {
                Visibility::Public
                | Visibility::Crate
                | Visibility::Restricted(_)
                | Visibility::Private => {}
            }
        }
        assert_eq!(all_visibilities().len(), 4);

        for t in all_declared_types() {
            match t {
                DeclaredType::Stated(_) | DeclaredType::Unstated => {}
            }
        }
        assert_eq!(all_declared_types().len(), 2);
    }

    #[test]
    fn every_reference_fact_variant_is_constructible() {
        for k in all_ref_kinds() {
            match k {
                RefKind::Calls
                | RefKind::Reads
                | RefKind::Writes
                | RefKind::Constructs
                | RefKind::TypeUse
                | RefKind::MacroInvokes => {}
            }
        }
        assert_eq!(all_ref_kinds().len(), 6);

        for r in all_reasons() {
            match r {
                Reason::UnhandledForm
                | Reason::NoDeclaredType
                | Reason::ReceiverTypeUnknown
                | Reason::NoImportInScope
                | Reason::AmbiguousCandidates
                | Reason::DynamicDispatch
                | Reason::MacroExpansion
                | Reason::Plumbing
                | Reason::ExternalBoundary
                | Reason::Unplaced => {}
            }
        }
        assert_eq!(all_reasons().len(), 10);

        for o in all_observations() {
            match o {
                Observation::Receiver(_)
                | Observation::ImportInScope(_)
                | Observation::UnplacedType(_)
                | Observation::Candidate(_) => {}
            }
        }
        assert_eq!(all_observations().len(), 4);
    }

    #[test]
    fn every_relation_and_import_variant_is_constructible() {
        for k in all_relation_kinds() {
            match k {
                RelationKind::Extends
                | RelationKind::Implements
                | RelationKind::TraitImpl
                | RelationKind::Mixin
                | RelationKind::Decorates
                | RelationKind::Owns => {}
            }
        }
        assert_eq!(all_relation_kinds().len(), 6);

        for b in all_bindings() {
            match b {
                Binding::Name(_) | Binding::Glob => {}
            }
        }
        assert_eq!(all_bindings().len(), 2);

        for o in all_import_origins() {
            match o {
                ImportOrigin::Local | ImportOrigin::External { .. } => {}
            }
        }
        assert_eq!(all_import_origins().len(), 2);
    }

    /// Every label round-trips, and no two languages share one.
    ///
    /// The label is the LEADING SEGMENT of every identity the language mints
    /// (spec §2), so a label that does not survive a round trip files a symbol
    /// under a language nothing reads back, and two languages sharing a label
    /// merge two graphs that have no business meeting.
    #[test]
    fn every_language_label_round_trips_and_is_unique() {
        let mut seen: Vec<&str> = Vec::new();
        for language in Language::all() {
            let label = language.as_str();
            assert_eq!(
                Language::from_label(label),
                Some(*language),
                "{label} does not read back as the language that wrote it"
            );
            assert!(!seen.contains(&label), "two languages both spell themselves {label}");
            seen.push(label);
        }
        assert_eq!(seen.len(), all_languages().len(), "`all` and the round trip disagree");
        assert_eq!(Language::from_label("COBOL"), None, "an unknown label names no language");
    }

    /// R2 in one assertion. `Resolution` is total: the walk has exactly two
    /// things it may say about a target, and both of them say something. There
    /// is no third, empty variant a miss could be parked in and no `None` it
    /// could collapse to, so a reference cannot be dropped on the floor.
    #[test]
    fn resolution_has_exactly_two_variants_and_neither_is_empty() {
        let resolutions = all_resolutions();
        assert_eq!(resolutions.len(), 2, "Resolution must have exactly two variants");

        for r in resolutions {
            match r {
                Resolution::Resolved { fqn, .. } => {
                    assert!(!fqn.as_str().is_empty(), "a resolved target names a symbol");
                }
                Resolution::Unresolved { reason, evidence } => {
                    assert!(
                        all_reasons().contains(&reason),
                        "{reason:?} must be one of the closed set of causes"
                    );
                    assert!(
                        !evidence.name.is_empty(),
                        "a miss carries what the walk saw, never a bare nothing"
                    );
                    assert!(
                        !evidence.node_kind.is_empty(),
                        "the node kind is what names the miss in the histogram"
                    );
                }
            }
        }
    }

    /// One `FileFacts` holding one of every shape, which is the whole product of
    /// a single walk (spec §3). Written out longhand because there is no
    /// constructor to lean on: a defaulted field is indistinguishable from one
    /// the walk actually read, and that is how fabricated data enters (R4).
    #[test]
    fn one_walk_produces_one_value_carrying_every_shape() {
        let symbol = Symbol {
            fqn: an_fqn("Widget"),
            kind: SymbolKind::Struct,
            name: "Widget".to_string(),
            span: a_span(),
            visibility: Visibility::Public,
            docstring: Some("A widget.".to_string()),
            declared_type: DeclaredType::Unstated,
            params: vec![Param {
                name: "width".to_string(),
                position: 0,
                declared_type: DeclaredType::Stated("u32".to_string()),
            }],
        };
        let reference = Reference {
            from: an_fqn("Widget"),
            kind: RefKind::Calls,
            at: a_span(),
            target: Resolution::Unresolved {
                reason: Reason::UnhandledForm,
                evidence: an_evidence(),
            },
        };
        let relation = Relation {
            kind: RelationKind::TraitImpl,
            child: an_fqn("Widget"),
            parent: Resolution::Resolved { fqn: an_fqn("Display"), via: Rung::DeclaredHere },
            at: a_span(),
        };
        let import = Import {
            path: "crate::db::PgStore".to_string(),
            binds: Binding::Name("PgStore".to_string()),
            origin: ImportOrigin::Local,
            at: a_span(),
        };

        let facts = FileFacts {
            language: Language::Rust,
            package: "senseid".to_string(),
            module: "indexer::facts".to_string(),
            path: "src/indexer/facts.rs".to_string(),
            symbols: vec![symbol],
            references: vec![reference],
            relations: vec![relation],
            imports: vec![import],
        };

        assert_eq!(facts.symbols.len(), 1);
        assert_eq!(facts.references.len(), 1);
        assert_eq!(facts.relations.len(), 1);
        assert_eq!(facts.imports.len(), 1);
        assert_eq!(facts.symbols[0].params.len(), 1, "a parameter is a prop, not a node (D2)");
    }

    /// The needles are assembled rather than written out, because this test
    /// lives in one of the files it reads and a literal would match itself.
    #[test]
    fn no_option_stands_in_for_a_resolution() {
        let banned = [
            format!("Option<{}>", "Fqn"),
            format!("Option<{}>", "Resolution"),
            format!("Option<&{}>", "Fqn"),
        ];
        let mut read = 0;
        for (path, body) in crate::indexer::guard_sources() {
            read += 1;
            for needle in &banned {
                assert!(
                    !body.contains(needle.as_str()),
                    "{path} uses `{needle}`: a miss is Unresolved with a reason, never a None (R2)"
                );
            }
        }
        assert!(read > 0, "the guard read no files, so it would have passed vacuously");
    }

    /// A derived or hand-written default hands out a value nothing observed,
    /// and a caller cannot tell it from one the walk read (R4).
    ///
    /// Matched on the two SPELLINGS that do it rather than on the word, and the
    /// difference is not pedantry: a plain substring search for `Default` also
    /// matches `ExportDefaultDeclaration`, which is what a JavaScript reader has
    /// to call `export default`. A guard that a second language cannot satisfy
    /// without renaming that language's keywords is a guard that will be
    /// deleted, so it is narrowed to the thing it protects:
    ///
    /// - a DERIVED or hand-written `Default`, which hands out a whole value;
    /// - `unwrap_or_default`, which turns a failed read into a zero nobody can
    ///   tell from a real one.
    ///
    /// Calling `::default()` on a third-party type is neither — an arena
    /// allocator is not data the walk claims to have read — and is left alone.
    #[test]
    fn nothing_defaults_a_value_it_did_not_read() {
        // Assembled, because this file is one of the ones being read and a
        // literal would match itself.
        // BOTH spellings. `unwrap_or_default()` and `unwrap_or("")` are the
        // same act on a `&str`, and the guard matched only the first — so
        // `javascript.rs`'s `text_of` turned an unreadable span into an empty
        // string that went on to NAME a symbol, under a guard written to
        // forbid exactly that.
        let swallowed = format!("unwrap_or_def{}", "ault");
        let empty_string = format!("unwrap_or({}{})", '"', '"');
        let derived = format!("Def{}", "ault");
        let mut read = 0;
        for (path, body) in crate::indexer::guard_sources() {
            read += 1;
            let body = crate::indexer::outside_tests(&body);
            assert!(
                !body.contains(swallowed.as_str()),
                "{path} uses `{swallowed}`: a failed read turned into a zero is one a caller \
                 cannot tell from a real one (R4)"
            );
            assert!(
                !body.contains(empty_string.as_str()),
                "{path} uses `{empty_string}`: an empty string a failed read produced is one a \
                 caller cannot tell from a name the source actually carried (R4)"
            );
            for line in body.lines() {
                let trimmed = line.trim_start();
                let hands_out_a_whole_value = (trimmed.starts_with("#[derive(")
                    || trimmed.starts_with("impl "))
                    && line.contains(derived.as_str());
                assert!(
                    !hands_out_a_whole_value,
                    "{path} derives or implements `{derived}`: a value nothing observed is \
                     fabricated data that a caller cannot tell from one the walk read (R4)\n  {line}"
                );
            }
        }
        assert!(read > 0, "the guard read no files, so it would have passed vacuously");
    }

    /// Every rung has prose too, and every piece of rung prose has a rung.
    ///
    /// Its own domain rather than sharing `code_graph`, because `precedence` is
    /// scoped per domain and means different things: for a reason it is "fix
    /// this first", for a rung it is CLIMB ORDER — which rung outranks which.
    /// One domain would make that number answer two questions.
    ///
    /// Every rung is `normal`: a placed edge is not a fault and not a refusal,
    /// it is the ladder working. The table's CHECK then forces remedy and actor
    /// to be null, which is correct — there is nothing to do about a success.
    #[test]
    fn every_rung_is_explained_by_a_seeded_reason_code() {
        const DOMAIN: &str = "code_graph_rung";
        let seed = include_str!("../../../../database/import/staging/reason_codes.jsonl");
        let rows: Vec<serde_json::Value> = seed
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("every seed line is JSON"))
            .filter(|row: &serde_json::Value| row["domain"] == DOMAIN)
            .collect();

        for rung in Rung::ALL {
            let code = rung.as_label();
            let row = rows
                .iter()
                .find(|r| r["code"] == code)
                .unwrap_or_else(|| panic!("{DOMAIN}.{code} has no row in reason_codes.jsonl"));
            for field in ["summary", "detail"] {
                assert!(
                    row[field].as_str().is_some_and(|s| !s.trim().is_empty()),
                    "{code}.{field} is empty prose"
                );
            }
            assert_eq!(row["kind"], "normal", "{code}: a placed edge is not a fault");
            assert!(
                row["remedy"].as_str().unwrap_or("").is_empty()
                    && row["actor"].as_str().unwrap_or("").is_empty(),
                "{code} is `normal`, so reason_codes_normal_is_silent rejects a remedy or actor"
            );
        }
        for row in &rows {
            let code = row["code"].as_str().unwrap_or_default();
            assert!(Rung::from_label(code).is_some(), "{DOMAIN}.{code} names no rung");
        }
        assert_eq!(rows.len(), Rung::ALL.len(), "one row per rung, no more");

        // Precedence IS climb order, so the seeded order must be the enum's.
        let mut seeded: Vec<(i64, &str)> = rows
            .iter()
            .map(|r| (r["precedence"].as_i64().unwrap_or(0), r["code"].as_str().unwrap_or("")))
            .collect();
        seeded.sort_unstable();
        assert_eq!(
            seeded.iter().map(|(_, c)| *c).collect::<Vec<_>>(),
            Rung::ALL.iter().map(|r| r.as_label()).collect::<Vec<_>>(),
            "seeded precedence disagrees with the order Ladder::climb tries the rungs"
        );
    }

    /// Every reason a miss can carry has PROSE, and every piece of prose has a
    /// reason.
    ///
    /// `Reason::as_label` gives a consumer a token; `sensei.reason_codes` is
    /// what turns that token into something a person or a model can act on.
    /// They are in two files, so the only thing keeping them in step is this
    /// test. Without it the failure is silent and one-directional: a variant
    /// added here surfaces to every reader as a bare `unhandled_form` with no
    /// explanation, which reads as a system that has nothing to say rather than
    /// as prose somebody forgot to write.
    ///
    /// Asserted against the SEED FILE rather than a list restated here, for the
    /// reason `the_refresh_window_is_wider_than_the_check_interval` gives: a
    /// copy keeps agreeing with itself.
    #[test]
    fn every_reason_is_explained_by_a_seeded_reason_code() {
        const DOMAIN: &str = "code_graph";
        let seed = include_str!("../../../../database/import/staging/reason_codes.jsonl");

        let rows: Vec<serde_json::Value> = seed
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("every seed line is JSON"))
            .filter(|row: &serde_json::Value| row["domain"] == DOMAIN)
            .collect();

        for reason in Reason::ALL {
            let code = reason.as_label();
            let row = rows
                .iter()
                .find(|r| r["code"] == code)
                .unwrap_or_else(|| panic!("{DOMAIN}.{code} has no row in reason_codes.jsonl"));
            for field in ["summary", "detail"] {
                let text = row[field].as_str().unwrap_or("");
                assert!(!text.trim().is_empty(), "{code}.{field} is empty prose");
            }
            let kind = row["kind"].as_str().unwrap_or("");
            assert!(
                matches!(kind, "normal" | "refusal" | "fault"),
                "{code} has kind {kind:?}, which sensei.reason_kind does not have"
            );
            // The CHECK on the table, asserted here so a bad row fails in the
            // suite rather than at deploy.
            if kind == "normal" {
                assert!(
                    row["remedy"].as_str().unwrap_or("").is_empty()
                        && row["actor"].as_str().unwrap_or("").is_empty(),
                    "{code} is `normal` but names a remedy or an actor — \
                     reason_codes_normal_is_silent rejects that"
                );
            }
        }

        // And nothing the other way: prose for a reason that no longer exists
        // is prose no reader will ever see, which is how a vocabulary rots.
        for row in &rows {
            let code = row["code"].as_str().unwrap_or_default();
            assert!(
                Reason::from_label(code).is_some(),
                "{DOMAIN}.{code} is seeded but no Reason variant produces it"
            );
        }

        let mut precedences: Vec<i64> =
            rows.iter().filter_map(|r| r["precedence"].as_i64()).collect();
        precedences.sort_unstable();
        let before = precedences.len();
        precedences.dedup();
        assert_eq!(before, precedences.len(), "reason_codes is UNIQUE (domain, precedence)");
        assert_eq!(rows.len(), Reason::ALL.len(), "one row per reason, no more");
    }
}
