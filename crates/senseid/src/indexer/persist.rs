//! Indexer v2 — the emit path from [`FileFacts`] to the database (step 7).
//!
//! One rule shapes everything here: **a fact that the walk captured must not be
//! able to stop reaching storage without the build breaking** (R3, R9). The
//! defect this replaces was not a bug in a line of code — `extract_return_type`
//! ran correctly on every function for months. It was a SHAPE: `upsert_node`
//! took eight positional arguments, so the return type had nowhere to go, and
//! nothing anywhere said so.
//!
//! So every hop between a fact and a row is a struct-to-struct conversion whose
//! source is destructured with every field NAMED:
//!
//! ```text
//!   Symbol ──► SymbolRow ──► NodeColumns ──► sensei.nodes
//!                                        ◄── sensei.nodes ──► SymbolRow
//! ```
//!
//! Add a field to `Symbol` and the build stops at [`SymbolRow::of`] until
//! someone decides where it goes. The same type serves both directions, so a
//! round trip compares what came out of Postgres with what the walk produced,
//! and not one copy of the encoder with another.
//! `every_conversion_between_a_fact_and_a_row_names_every_field` is what keeps
//! that property once `..` becomes an available way to silence the error.
//!
//! ## Which column, and which prop
//!
//! Where a column already means what a fact means, the fact goes in it. Where
//! none does, it goes in `props`, the jsonb column the schema documents as
//! "extensible metadata" and where a declared return type already lives
//! (`set_node_return_type`). Three facts land in BOTH, which is not redundancy:
//!
//! | fact | column | prop | why both |
//! |---|---|---|---|
//! | kind | `node_kind` | `symbol_kind` | `node_kind` has no `trait`, `static` or `macro`, so three pairs collapse in it |
//! | visibility | `is_exported` | `visibility` | a boolean cannot hold four states |
//! | span | `line_start`/`line_end` | `span_columns` | there is no column for a column |
//!
//! The column is what the existing graph queries read; the prop is what makes
//! the round trip lossless. Widening `node_kind` or `edge_kind` is a DDL change
//! to tables the SHIPPED indexer writes, so it is step 9's decision and not
//! this step's.
//!
//! Where a fact lands in both, the read path reads the COLUMN and checks it
//! against the prop through an inverse written independently of the encoder
//! ([`node_kind_holds`], [`is_exported_holds`], [`edge_kind_holds_uses`]). Left
//! out, the round trip compares the encoder with itself: measured, three
//! one-line inversions of production column mappings — a function filed as a
//! method, the export boolean flipped, a call filed as a reference — passed the
//! entire workspace suite while changing what every graph query would answer.
//!
//! ## An edge is a relationship, not an occurrence — but occurrences have a file
//!
//! `edges` is keyed `(folder, source, target, kind)`, so two calls to one
//! function from one function are ONE row. The occurrences are grouped in RUST,
//! before the write, so one file's complete list goes out in one statement.
//!
//! That is not enough on its own, because two FILES can produce one edge —
//! `crates/mcp` has a `lib.rs` and a `main.rs` — and `props = props ||
//! EXCLUDED.props` REPLACES a key rather than appending, so the second write
//! erased the first's occurrences with nothing counting the loss. So the list is
//! an OBJECT KEYED BY FILE, merged by
//! [`crate::db::pg_store::PgStore::merge_v2_edge_occurrences`]. The same `||`
//! then does both jobs: it replaces this file's list, so a re-scan drops spans
//! that have moved, and it leaves every other file's alone. What that still
//! cannot cover is counted — see [`EdgeCollision`].
//!
//! ## What this cannot keep apart, and whose it is
//!
//! Two declarations that mint one fqn are one node. That is not persistence
//! being lossy — an fqn IS the lookup key (spec §2) and `nodes_unique_fqn` is
//! the merge contract itself. But it is still a declaration with no row, so
//! [`write`] RETURNS the collisions rather than letting them be silent, and
//! `the_identities_this_repos_rust_cannot_keep_apart_are_a_known_and_bounded_set`
//! bounds them at the 19 this repository produces and names their cause.
//
// This module has no caller on purpose — see the note in `indexer/mod.rs`.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::db::pg_store::{EdgeColumns, NodeColumns, PgStore};
use crate::graph_facts::{EdgeFact, OnMiss, TargetRef};

use super::facts::{
    Binding, DeclaredType, Evidence, FileFacts, Import, ImportOrigin, Language, Observation, Param,
    Reason, RefKind, Reference, Relation, RelationKind, Resolution, Span, Symbol, SymbolKind,
    Visibility,
};
use super::fqn::{self, Origin, Reach};

// ── the wire format (R3, R9) ─────────────────────────────────────────────────

/// One declaration, as the database holds it.
///
/// The same type on the way in and on the way out, deliberately: a separate
/// write shape and read shape are two hand-copied enumerations of one thing, and
/// the round-trip test could then compare two values that agree with each other
/// and with nothing the walk produced.
///
/// Identity is a `String` and not an [`super::facts::Fqn`]. `fqn::parse`'s own
/// documentation says the form is not recoverable from the encoding — the empty
/// module segment is dropped, so a package-root method and a module-level item
/// encode alike — so re-minting one here would be a second, weaker door onto
/// identity. Comparing the encoded strings is the merge contract exactly as the
/// database sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRow {
    pub fqn: String,
    pub kind: SymbolKind,
    pub name: String,
    pub span: Span,
    pub visibility: Visibility,
    pub docstring: Option<String>,
    pub declared_type: DeclaredType,
    pub params: Vec<Param>,
    pub file_path: String,
    pub language: Language,
}

impl SymbolRow {
    /// The row a [`Symbol`] becomes. Destructured EXHAUSTIVELY (R9): a field
    /// added to `Symbol` stops this compiling until someone decides which column
    /// or which prop it lands in. A positional argument list is the version of
    /// this that keeps compiling and silently stops carrying the value — which
    /// is how `extract_return_type` ran on every function for months while no
    /// return type reached a column.
    pub fn of(symbol: &Symbol, file_path: &str, language: Language) -> Self {
        let Symbol { fqn, kind, name, span, visibility, docstring, declared_type, params } = symbol;
        Self {
            fqn: fqn.as_str().to_string(),
            kind: *kind,
            name: name.clone(),
            span: *span,
            visibility: visibility.clone(),
            docstring: docstring.clone(),
            declared_type: declared_type.clone(),
            params: params.clone(),
            file_path: file_path.to_string(),
            language,
        }
    }
}

/// Where a use site points, as the database holds it. Mirrors
/// [`super::facts::Resolution`] and is total for the same reason (R2): there is
/// no shape here meaning "nothing", so a miss cannot be written as an absence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetRow {
    Resolved(String),
    Unresolved { reason: Reason, evidence: EvidenceRow },
}

/// [`super::facts::Evidence`], with any identity it mentions as the string the
/// database holds — see [`SymbolRow`] for why an [`super::facts::Fqn`] is not
/// re-minted on the way out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRow {
    pub name: String,
    pub node_kind: String,
    pub reach: Reach,
    pub saw: Vec<ObservationRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationRow {
    Receiver(String),
    ImportInScope(String),
    UnplacedType(String),
    Candidate(String),
}

/// One use site, as the database holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceRow {
    pub from: String,
    pub kind: RefKind,
    pub at: Span,
    pub target: TargetRow,
}

impl ReferenceRow {
    /// Destructured EXHAUSTIVELY (R9) — see [`SymbolRow::of`].
    pub fn of(reference: &Reference) -> Self {
        let Reference { from, kind, at, target } = reference;
        Self {
            from: from.as_str().to_string(),
            kind: *kind,
            at: *at,
            target: target_row_of(target),
        }
    }
}

fn target_row_of(target: &Resolution) -> TargetRow {
    match target {
        Resolution::Resolved(fqn) => TargetRow::Resolved(fqn.as_str().to_string()),
        Resolution::Unresolved { reason, evidence } => {
            let Evidence { name, node_kind, reach, saw } = evidence;
            TargetRow::Unresolved {
                reason: *reason,
                evidence: EvidenceRow {
                    name: name.clone(),
                    node_kind: node_kind.clone(),
                    reach: *reach,
                    saw: saw.iter().map(observation_row_of).collect(),
                },
            }
        }
    }
}

fn observation_row_of(observation: &Observation) -> ObservationRow {
    match observation {
        Observation::Receiver(text) => ObservationRow::Receiver(text.clone()),
        Observation::ImportInScope(text) => ObservationRow::ImportInScope(text.clone()),
        Observation::UnplacedType(text) => ObservationRow::UnplacedType(text.clone()),
        Observation::Candidate(fqn) => ObservationRow::Candidate(fqn.as_str().to_string()),
    }
}

/// One structural fact, as the database holds it (spec §3.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationRow {
    pub kind: RelationKind,
    pub child: String,
    pub parent: TargetRow,
    pub at: Span,
}

impl RelationRow {
    /// Destructured EXHAUSTIVELY (R9) — see [`SymbolRow::of`].
    pub fn of(relation: &Relation) -> Self {
        let Relation { kind, child, parent, at } = relation;
        Self {
            kind: *kind,
            child: child.as_str().to_string(),
            parent: target_row_of(parent),
            at: *at,
        }
    }
}

/// One import, as the database holds it (spec §2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRow {
    /// The symbol the specifier was written in. An [`Import`] does not carry one
    /// — an import belongs to the FILE — so the file's own module identity is
    /// supplied by the writer, which is the only place that knows it.
    pub from: String,
    pub path: String,
    pub binds: BindingRow,
    pub origin: OriginRow,
    pub at: Span,
}

impl ImportRow {
    /// Destructured EXHAUSTIVELY (R9) — see [`SymbolRow::of`].
    pub fn of(import: &Import, from: &str) -> Self {
        let Import { path, binds, origin, at } = import;
        Self {
            from: from.to_string(),
            path: path.clone(),
            binds: match binds {
                Binding::Name(name) => BindingRow::Name(name.clone()),
                Binding::Glob => BindingRow::Glob,
            },
            origin: match origin {
                ImportOrigin::Local => OriginRow::Local,
                ImportOrigin::External { package } => OriginRow::External(package.clone()),
            },
            at: *at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingRow {
    Name(String),
    Glob,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginRow {
    Local,
    External(String),
}

/// Everything one folder's v2 rows read back as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stored {
    pub symbols: Vec<SymbolRow>,
    pub references: Vec<ReferenceRow>,
    pub relations: Vec<RelationRow>,
    pub imports: Vec<ImportRow>,
}

/// The `sensei.nodes` write a [`SymbolRow`] becomes.
///
/// Destructured EXHAUSTIVELY (R9), so a field added to `SymbolRow` stops the
/// build here until someone decides which column or which prop carries it.
///
/// Three fields land in a column AND in a prop, and that is not redundancy:
/// `kind` collapses in `node_kind` (see [`node_kind_label`]), `visibility`
/// narrows to a boolean in `is_exported`, and the span's COLUMNS have no column
/// at all. In each case the column is what the existing graph queries read and
/// the prop is what survives the round trip.
fn node_columns_of(row: &SymbolRow) -> NodeColumns {
    let SymbolRow {
        fqn,
        kind,
        name,
        span,
        visibility,
        docstring,
        declared_type,
        params,
        file_path,
        language,
    } = row;
    NodeColumns {
        fqn: fqn.clone(),
        kind: node_kind_label(*kind).to_string(),
        name: name.clone(),
        file_path: Some(file_path.clone()),
        language: Some(language.as_str().to_string()),
        line_start: Some(line_number(span.start_line)),
        line_end: Some(line_number(span.end_line)),
        is_exported: *visibility == Visibility::Public,
        docstring: docstring.clone(),
        props: serde_json::json!({
            "symbol_kind": symbol_kind_label(*kind),
            "visibility": visibility_props(visibility),
            "declared_type": declared_type_prop(declared_type),
            "params": params_prop(params),
            "span_columns": [span.start_col, span.end_col],
        }),
    }
}

/// A line number as the column type holds it.
///
/// Saturating rather than erroring: `line_start` is one of `nodes`' identity
/// columns, and a file with more than two billion lines is not a case worth a
/// failure path. tree-sitter's own row is a `usize` read from a buffer that had
/// to fit in memory first.
fn line_number(line: u32) -> i32 {
    i32::try_from(line).unwrap_or(i32::MAX)
}

// ── labels (the spelling storage uses) ───────────────────────────────────────

/// The `sensei.node_kind` a symbol is filed under.
///
/// NOT injective, and that is why the exact kind is also a prop: `node_kind` has
/// no `trait`, no `static` and no `macro`, so three pairs collapse here. The
/// column is what the existing graph queries read; the prop is what the round
/// trip reads. Extending the enum is a DDL change on a table the shipped
/// indexer writes, which is step 9's decision and not this step's.
fn node_kind_label(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function => "function",
        SymbolKind::Method => "method",
        SymbolKind::Class => "class",
        SymbolKind::Struct => "struct",
        SymbolKind::Enum => "enum",
        SymbolKind::EnumVariant => "enum_variant",
        SymbolKind::Interface => "interface",
        SymbolKind::Trait => "interface",
        SymbolKind::TypeAlias => "type",
        SymbolKind::Const => "const",
        SymbolKind::Static => "const",
        SymbolKind::Module => "module",
        SymbolKind::Macro => "function",
        SymbolKind::Field => "field",
        SymbolKind::Property => "property",
    }
}

/// The exact [`SymbolKind`], as the prop spells it. Paired with
/// [`symbol_kind_from_label`] here so the two directions cannot drift — the
/// idiom `Reach::as_str`/`Reach::from_label` already uses.
fn symbol_kind_label(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function => "function",
        SymbolKind::Method => "method",
        SymbolKind::Class => "class",
        SymbolKind::Struct => "struct",
        SymbolKind::Enum => "enum",
        SymbolKind::EnumVariant => "enum_variant",
        SymbolKind::Interface => "interface",
        SymbolKind::Trait => "trait",
        SymbolKind::TypeAlias => "type_alias",
        SymbolKind::Const => "const",
        SymbolKind::Static => "static",
        SymbolKind::Module => "module",
        SymbolKind::Macro => "macro",
        SymbolKind::Field => "field",
        SymbolKind::Property => "property",
    }
}

/// Which [`SymbolKind`]s a `node_kind` COLUMN value can hold — the inverse of
/// [`node_kind_label`], and a set because that mapping is not injective.
///
/// Written independently of [`node_kind_label`] ON PURPOSE, and not factored
/// into a table both directions read. The round trip's job is to compare what
/// Postgres holds with what the WALK produced; a single shared table would put
/// the same statement on both sides, and inverting it would move the encoder
/// and the check together — the round trip would then agree with itself while
/// the graph filed every function as a method. It is the same reason
/// [`Reach::as_str`] and [`Reach::from_label`] are two matches and not one map.
fn node_kind_holds(label: &str) -> Option<&'static [SymbolKind]> {
    Some(match label {
        "function" => &[SymbolKind::Function, SymbolKind::Macro][..],
        "method" => &[SymbolKind::Method],
        "class" => &[SymbolKind::Class],
        "struct" => &[SymbolKind::Struct],
        "enum" => &[SymbolKind::Enum],
        "enum_variant" => &[SymbolKind::EnumVariant],
        "interface" => &[SymbolKind::Interface, SymbolKind::Trait],
        "type" => &[SymbolKind::TypeAlias],
        "const" => &[SymbolKind::Const, SymbolKind::Static],
        "module" => &[SymbolKind::Module],
        "field" => &[SymbolKind::Field],
        "property" => &[SymbolKind::Property],
        _ => return None,
    })
}

/// Whether the `is_exported` COLUMN agrees with the visibility the props carry.
///
/// A boolean cannot hold four states, so `props.visibility` is the exact one and
/// the column is a narrowing of it. Stated here as its own exhaustive match
/// rather than by calling the narrowing back, for the reason in
/// [`node_kind_holds`]: a shared expression is the encoder checking itself.
fn is_exported_holds(exported: bool, visibility: &Visibility) -> bool {
    match visibility {
        Visibility::Public => exported,
        Visibility::Crate | Visibility::Restricted(_) | Visibility::Private => !exported,
    }
}

fn symbol_kind_from_label(label: &str) -> Option<SymbolKind> {
    Some(match label {
        "function" => SymbolKind::Function,
        "method" => SymbolKind::Method,
        "class" => SymbolKind::Class,
        "struct" => SymbolKind::Struct,
        "enum" => SymbolKind::Enum,
        "enum_variant" => SymbolKind::EnumVariant,
        "interface" => SymbolKind::Interface,
        "trait" => SymbolKind::Trait,
        "type_alias" => SymbolKind::TypeAlias,
        "const" => SymbolKind::Const,
        "static" => SymbolKind::Static,
        "module" => SymbolKind::Module,
        "macro" => SymbolKind::Macro,
        "field" => SymbolKind::Field,
        "property" => SymbolKind::Property,
        _ => return None,
    })
}

// ── edges: one row per RELATIONSHIP, carrying every occurrence of it ─────────

/// What an edge points at, as the two unique indexes on `sensei.edges` key it:
/// a resolved edge by its target node, an unresolved one by the name it could
/// not place.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum TargetKey {
    Proven(String),
    Named(String),
}

/// One fact that landed on an edge.
///
/// An edge's identity is `(folder, source, target, kind)` — a RELATIONSHIP, not
/// an occurrence — so two calls to one function from one function are one row.
/// Grouping happens HERE, in Rust, before the write: the alternative is two
/// writes that collide, and `props = props || EXCLUDED.props` replaces a key
/// rather than appending to it, so the second occurrence would silently
/// overwrite the first. Writing the complete list in one statement is also what
/// makes a re-scan REPLACE the list instead of accumulating spans that have
/// since moved.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Occurrence {
    /// A use site (spec §3.2).
    Use { kind: RefKind, at: Span, outcome: Outcome },
    /// A structural fact (spec §3.3).
    Structure { kind: RelationKind, at: Span, outcome: Outcome },
    /// An import (spec §2). It brings a name into scope rather than using one,
    /// so there is no target for the ladder to have proven or missed.
    Brought { binds: BindingRow, origin: OriginRow, at: Span },
}

/// What the ladder concluded at one occurrence.
///
/// [`Outcome::Proven`] carries no identity on purpose. WHICH target was proven
/// is the edge's `target_id` — a column — so reading it back out of props would
/// be the round trip comparing a copy of the props with itself instead of with
/// the edge.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Outcome {
    Proven,
    Missed { reason: Reason, evidence: EvidenceRow },
}

/// One edge to write: the relationship, and every fact that produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct EdgeRow {
    source: String,
    kind: &'static str,
    target: TargetKey,
    occurrences: Vec<Occurrence>,
}

/// Group one file's facts into the edges they become.
///
/// A `BTreeMap` and not a `HashMap`: the write order is the graph's write order,
/// and a graph that differs by hash seed between two runs of the same scan is
/// the order dependence R6 forbids.
fn edge_rows_of(facts: &FileFacts, file: &str) -> Vec<EdgeRow> {
    let mut grouped: BTreeMap<(String, &'static str, TargetKey), Vec<Occurrence>> = BTreeMap::new();

    for reference in &facts.references {
        let Reference { from, kind, at, target } = reference;
        let (key, outcome) = target_key_and_outcome(target);
        grouped
            .entry((from.as_str().to_string(), reference_edge_kind(*kind), key))
            .or_default()
            .push(Occurrence::Use { kind: *kind, at: *at, outcome });
    }

    for relation in &facts.relations {
        let Relation { kind, child, parent, at } = relation;
        let (key, outcome) = target_key_and_outcome(parent);
        grouped
            .entry((child.as_str().to_string(), relation_edge_kind(*kind), key))
            .or_default()
            .push(Occurrence::Structure { kind: *kind, at: *at, outcome });
    }

    for import in &facts.imports {
        let row = ImportRow::of(import, file);
        let ImportRow { from, path, binds, origin, at } = row;
        grouped
            .entry((from, "imports", TargetKey::Named(path)))
            .or_default()
            .push(Occurrence::Brought { binds, origin, at });
    }

    grouped
        .into_iter()
        .map(|((source, kind, target), occurrences)| EdgeRow { source, kind, target, occurrences })
        .collect()
}

fn target_key_and_outcome(target: &Resolution) -> (TargetKey, Outcome) {
    match target_row_of(target) {
        TargetRow::Resolved(fqn) => (TargetKey::Proven(fqn), Outcome::Proven),
        TargetRow::Unresolved { reason, evidence } => {
            (TargetKey::Named(evidence.name.clone()), Outcome::Missed { reason, evidence })
        }
    }
}

/// The `sensei.edge_kind` a use site is filed under.
///
/// Coarser than [`RefKind`], because the enum is a shared table the shipped
/// indexer writes and widening it is a DDL change (step 9's decision). The exact
/// kind is carried per occurrence, so nothing is lost — but `calls` is kept for
/// calls alone, so `get_callers` keeps meaning what it means: a construction is
/// not a call, and counting it as one would be a wrong edge for factory
/// detection to read (R8).
fn reference_edge_kind(kind: RefKind) -> &'static str {
    match kind {
        RefKind::Calls | RefKind::MacroInvokes => "calls",
        RefKind::Reads | RefKind::Writes | RefKind::Constructs | RefKind::TypeUse => "references",
    }
}

/// The `sensei.edge_kind` a structural fact is filed under, on the same terms
/// as [`reference_edge_kind`]: the enum is shared and widening it is step 9's.
///
/// Ownership is `references` and NOT `extends`, which is the whole point of
/// [`RelationKind::Owns`] being a separate kind: an inherent `impl Foo { }` owns
/// its members and inherits nothing, and an `extends` row for it would be a
/// false inheritance edge that pattern detection reads as real.
fn relation_edge_kind(kind: RelationKind) -> &'static str {
    match kind {
        RelationKind::Extends => "extends",
        RelationKind::Implements | RelationKind::TraitImpl | RelationKind::Mixin => "implements",
        RelationKind::Decorates | RelationKind::Owns => "references",
    }
}

fn relation_kind_label(kind: RelationKind) -> &'static str {
    match kind {
        RelationKind::Extends => "extends",
        RelationKind::Implements => "implements",
        RelationKind::TraitImpl => "trait_impl",
        RelationKind::Mixin => "mixin",
        RelationKind::Decorates => "decorates",
        RelationKind::Owns => "owns",
    }
}

/// Which [`RefKind`]s an `edge_kind` COLUMN value can hold — the inverse of
/// [`reference_edge_kind`], written independently for the reason in
/// [`node_kind_holds`].
fn edge_kind_holds_uses(label: &str) -> &'static [RefKind] {
    match label {
        "calls" => &[RefKind::Calls, RefKind::MacroInvokes][..],
        "references" => &[RefKind::Reads, RefKind::Writes, RefKind::Constructs, RefKind::TypeUse],
        _ => &[],
    }
}

/// Which [`RelationKind`]s an `edge_kind` COLUMN value can hold — the inverse of
/// [`relation_edge_kind`], written independently for the same reason.
fn edge_kind_holds_structure(label: &str) -> &'static [RelationKind] {
    match label {
        "extends" => &[RelationKind::Extends][..],
        "implements" => &[RelationKind::Implements, RelationKind::TraitImpl, RelationKind::Mixin],
        "references" => &[RelationKind::Decorates, RelationKind::Owns],
        _ => &[],
    }
}

fn relation_kind_from_label(label: &str) -> Option<RelationKind> {
    Some(match label {
        "extends" => RelationKind::Extends,
        "implements" => RelationKind::Implements,
        "trait_impl" => RelationKind::TraitImpl,
        "mixin" => RelationKind::Mixin,
        "decorates" => RelationKind::Decorates,
        "owns" => RelationKind::Owns,
        _ => return None,
    })
}

fn ref_kind_label(kind: RefKind) -> &'static str {
    match kind {
        RefKind::Calls => "calls",
        RefKind::Reads => "reads",
        RefKind::Writes => "writes",
        RefKind::Constructs => "constructs",
        RefKind::TypeUse => "type_use",
        RefKind::MacroInvokes => "macro_invokes",
    }
}

fn ref_kind_from_label(label: &str) -> Option<RefKind> {
    Some(match label {
        "calls" => RefKind::Calls,
        "reads" => RefKind::Reads,
        "writes" => RefKind::Writes,
        "constructs" => RefKind::Constructs,
        "type_use" => RefKind::TypeUse,
        "macro_invokes" => RefKind::MacroInvokes,
        _ => return None,
    })
}

fn reason_label(reason: Reason) -> &'static str {
    match reason {
        Reason::UnhandledForm => "unhandled_form",
        Reason::NoDeclaredType => "no_declared_type",
        Reason::ReceiverTypeUnknown => "receiver_type_unknown",
        Reason::NoImportInScope => "no_import_in_scope",
        Reason::AmbiguousCandidates => "ambiguous_candidates",
        Reason::DynamicDispatch => "dynamic_dispatch",
        Reason::MacroExpansion => "macro_expansion",
        Reason::Denylisted => "denylisted",
        Reason::Unplaced => "unplaced",
    }
}

fn reason_from_label(label: &str) -> Option<Reason> {
    Some(match label {
        "unhandled_form" => Reason::UnhandledForm,
        "no_declared_type" => Reason::NoDeclaredType,
        "receiver_type_unknown" => Reason::ReceiverTypeUnknown,
        "no_import_in_scope" => Reason::NoImportInScope,
        "ambiguous_candidates" => Reason::AmbiguousCandidates,
        "dynamic_dispatch" => Reason::DynamicDispatch,
        "macro_expansion" => Reason::MacroExpansion,
        "denylisted" => Reason::Denylisted,
        "unplaced" => Reason::Unplaced,
        _ => return None,
    })
}

fn span_prop(at: Span) -> serde_json::Value {
    serde_json::json!([at.start_line, at.start_col, at.end_line, at.end_col])
}

fn span_from_prop(value: &serde_json::Value) -> Option<Span> {
    let parts = value.as_array()?;
    let at = |i: usize| -> Option<u32> { u32::try_from(parts.get(i)?.as_u64()?).ok() };
    Some(Span { start_line: at(0)?, start_col: at(1)?, end_line: at(2)?, end_col: at(3)? })
}

fn evidence_prop(evidence: &EvidenceRow) -> serde_json::Value {
    let EvidenceRow { name, node_kind, reach, saw } = evidence;
    serde_json::json!({
        "name": name,
        "node_kind": node_kind,
        "reach": reach.as_str(),
        "saw": saw.iter().map(observation_prop).collect::<Vec<_>>(),
    })
}

fn evidence_from_prop(value: &serde_json::Value) -> Option<EvidenceRow> {
    Some(EvidenceRow {
        name: value.get("name")?.as_str()?.to_string(),
        node_kind: value.get("node_kind")?.as_str()?.to_string(),
        // The reach the WALK stated (spec §2.1). It is what a later pass needs
        // to re-mint the candidate this miss was about, so losing it here would
        // make the evidence unusable for the thing evidence is for.
        reach: Reach::from_label(value.get("reach")?.as_str()?)?,
        saw: value
            .get("saw")?
            .as_array()?
            .iter()
            .map(observation_from_prop)
            .collect::<Option<Vec<_>>>()?,
    })
}

fn observation_prop(observation: &ObservationRow) -> serde_json::Value {
    let (saw, text) = match observation {
        ObservationRow::Receiver(text) => ("receiver", text),
        ObservationRow::ImportInScope(text) => ("import_in_scope", text),
        ObservationRow::UnplacedType(text) => ("unplaced_type", text),
        ObservationRow::Candidate(text) => ("candidate", text),
    };
    serde_json::json!({ "saw": saw, "text": text })
}

fn observation_from_prop(value: &serde_json::Value) -> Option<ObservationRow> {
    let text = value.get("text")?.as_str()?.to_string();
    Some(match value.get("saw")?.as_str()? {
        "receiver" => ObservationRow::Receiver(text),
        "import_in_scope" => ObservationRow::ImportInScope(text),
        "unplaced_type" => ObservationRow::UnplacedType(text),
        "candidate" => ObservationRow::Candidate(text),
        _ => return None,
    })
}

fn occurrence_prop(occurrence: &Occurrence) -> serde_json::Value {
    match occurrence {
        Occurrence::Use { kind, at, outcome } => with_outcome(
            serde_json::json!({
                "fact": "use",
                "kind": ref_kind_label(*kind),
                "at": span_prop(*at),
            }),
            outcome,
        ),
        Occurrence::Structure { kind, at, outcome } => with_outcome(
            serde_json::json!({
                "fact": "structure",
                "kind": relation_kind_label(*kind),
                "at": span_prop(*at),
            }),
            outcome,
        ),
        Occurrence::Brought { binds, origin, at } => serde_json::json!({
            "fact": "brought",
            "binds": match binds {
                BindingRow::Name(name) => serde_json::json!({ "binds": "name", "name": name }),
                BindingRow::Glob => serde_json::json!({ "binds": "glob" }),
            },
            "origin": match origin {
                OriginRow::Local => serde_json::json!({ "origin": "local" }),
                OriginRow::External(package) => {
                    serde_json::json!({ "origin": "external", "package": package })
                }
            },
            "at": span_prop(*at),
        }),
    }
}

fn with_outcome(mut prop: serde_json::Value, outcome: &Outcome) -> serde_json::Value {
    if let Outcome::Missed { reason, evidence } = outcome {
        prop["reason"] = serde_json::Value::String(reason_label(*reason).to_string());
        prop["evidence"] = evidence_prop(evidence);
    }
    prop
}

fn binds_from_prop(value: &serde_json::Value) -> Option<BindingRow> {
    Some(match value.get("binds")?.as_str()? {
        "name" => BindingRow::Name(value.get("name")?.as_str()?.to_string()),
        "glob" => BindingRow::Glob,
        _ => return None,
    })
}

fn origin_from_prop(value: &serde_json::Value) -> Option<OriginRow> {
    Some(match value.get("origin")?.as_str()? {
        "local" => OriginRow::Local,
        "external" => OriginRow::External(value.get("package")?.as_str()?.to_string()),
        _ => return None,
    })
}

/// The outcome an occurrence prop states.
///
/// A prop with no `reason` is a PROVEN occurrence, and that is not an absence
/// standing in for a value: the edge's own `target_id` says which target, so
/// `Proven` has nothing left to carry. A prop with a `reason` but no evidence is
/// a malformed row and errors rather than being read as a proof.
fn outcome_from_prop(value: &serde_json::Value) -> Option<Outcome> {
    let Some(reason) = value.get("reason") else {
        return Some(Outcome::Proven);
    };
    Some(Outcome::Missed {
        reason: reason_from_label(reason.as_str()?)?,
        evidence: evidence_from_prop(value.get("evidence")?)?,
    })
}

fn occurrence_from_prop(value: &serde_json::Value) -> Option<Occurrence> {
    match value.get("fact")?.as_str()? {
        "use" => Some(Occurrence::Use {
            kind: ref_kind_from_label(value.get("kind")?.as_str()?)?,
            at: span_from_prop(value.get("at")?)?,
            outcome: outcome_from_prop(value)?,
        }),
        "structure" => Some(Occurrence::Structure {
            kind: relation_kind_from_label(value.get("kind")?.as_str()?)?,
            at: span_from_prop(value.get("at")?)?,
            outcome: outcome_from_prop(value)?,
        }),
        "brought" => Some(Occurrence::Brought {
            binds: binds_from_prop(value.get("binds")?)?,
            origin: origin_from_prop(value.get("origin")?)?,
            at: span_from_prop(value.get("at")?)?,
        }),
        _ => None,
    }
}

/// Every occurrence of one edge that ONE FILE contributes.
///
/// Written under the file's own key rather than into a flat list, because the
/// edge is keyed `(folder, source, target, kind)` and two files can produce the
/// same edge — `crates/mcp`'s `lib.rs` and `main.rs` share a module path, so
/// their imports hang off one file identity. jsonb `||` replaces a key rather
/// than appending, and that is exactly the behaviour wanted PER FILE (a re-scan
/// must drop spans that have moved) and exactly the wrong behaviour ACROSS
/// files (the second write would erase the first). Keying by file gets both:
/// see [`crate::db::pg_store::PgStore::merge_v2_edge_occurrences`].
fn occurrences_prop(row: &EdgeRow) -> serde_json::Value {
    serde_json::Value::Array(row.occurrences.iter().map(occurrence_prop).collect())
}

fn edge_props(row: &EdgeRow) -> serde_json::Value {
    let mut props = serde_json::json!({});
    // `props.relation` is the EXISTING discriminant for what an `extends` or
    // `implements` row means — the `edges` table comment names it, and
    // `prune_mislabelled_containment_extends` DELETES any `extends` row that
    // lacks it. Stamping it is therefore not decoration: without it every
    // inheritance edge v2 writes would be collected as legacy debris.
    //
    // Read off the first structural occurrence. A second one would have to
    // disagree with it to matter, and it cannot: the grouping key already
    // includes the edge kind, and no Rust source states two DIFFERENT
    // inheritance kinds between one pair of types.
    if let Some(kind) = row.occurrences.iter().find_map(|occurrence| match occurrence {
        Occurrence::Structure { kind, .. } => Some(*kind),
        Occurrence::Use { .. } | Occurrence::Brought { .. } => None,
    }) {
        props["relation"] = serde_json::Value::String(relation_kind_label(kind).to_string());
    }
    props
}

// ── writing ──────────────────────────────────────────────────────────────────

/// What one file's write did.
///
/// The two collision lists are the point of returning anything at all.
/// Everything else here is a count a caller may log; a collision is a fact that
/// has no row of its own and never will, and a caller that cannot see it is a
/// caller for whom the fact silently disappeared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub symbols: usize,
    pub edges: usize,
    pub collisions: Vec<Collision>,
    pub edge_collisions: Vec<EdgeCollision>,
    /// The edge ROWS this file's occurrences now sit in.
    ///
    /// Rows and not groups, so `edge_rows.len()` is below `edges` exactly when
    /// two of this file's groups landed on one row — the case
    /// [`EdgeCollision`] names.
    ///
    /// Here because reconcile cannot compute it: an edge's row is decided by
    /// the database, through `insert_edge_with_props`' conflict key and through
    /// whatever `upsert_node_by_fqn` did with the source identity. The only way
    /// to know which rows a write landed on is to be the write. Without it,
    /// "which of this file's old edges did it stop producing" has to be
    /// re-derived from the facts, which is a second copy of `edge_rows_of` and
    /// its target resolution — two producers of one answer (R7).
    pub edge_rows: BTreeSet<uuid::Uuid>,
}

/// One identity that more than one declaration in a single file minted.
///
/// The collapse belongs to the fqn GRAMMAR and not to persistence: an fqn is a
/// lookup key (spec §2) and `nodes_unique_fqn` merely applies it, so two
/// declarations that mint one key ARE one node by definition. What persistence
/// owes is the count — see
/// `the_identities_this_repos_rust_cannot_keep_apart_are_a_known_and_bounded_set`
/// for what produces them and whose fix it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collision {
    pub fqn: String,
    pub declarations: usize,
}

/// Two of ONE file's edge groups that the database keyed as one row.
///
/// Occurrences are keyed by file, so two FILES contributing to one edge is no
/// longer a loss. One file contributing twice still would be: the second merge
/// writes this file's key again and replaces what the first put there. It
/// cannot happen through the grouping — `edge_rows_of` emits one group per
/// `(source, kind, target)` — but it can happen BELOW it, because two identities
/// can end up on one node when `upsert_node_by_fqn` recovers from a
/// `nodes_unique_identity` conflict by re-pointing an existing row's fqn.
///
/// Rare, and precisely the kind of rare that goes unnoticed for months. So it is
/// counted rather than assumed away: the write remembers which edge rows it has
/// already merged into and names the second one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeCollision {
    pub source: String,
    pub kind: &'static str,
    /// What the FIRST group pointed at, and what the second did. Both, because
    /// the whole point is that two different targets reached one row.
    pub targets: [String; 2],
}

/// Persist one file's facts (R3, D1).
///
/// Symbols first, so an edge whose source or target is declared in this file
/// lands on the real node rather than on a stub of it.
pub async fn write(
    store: &PgStore,
    folder_id: &uuid::Uuid,
    facts: &FileFacts,
) -> Result<Written, String> {
    // The path comes from the FACTS and is not a second argument: the walk
    // already used it to mint the identity every file-scope use site is filed
    // under, and a writer given its own copy could be handed a different one.
    let file_path = facts.path.as_str();
    let language = facts.language.as_str();
    let mut known: HashMap<String, uuid::Uuid> = HashMap::new();
    let mut declarations: BTreeMap<String, usize> = BTreeMap::new();
    let owner_of = owners(facts);
    for symbol in &facts.symbols {
        let row = SymbolRow::of(symbol, file_path, facts.language);
        // The owner's ROW has to exist before the member's, because `parent_id`
        // is one of the six columns `nodes_unique_identity` keys on. Reached
        // through `node_for`, so an owner this file does not declare is the same
        // reference-before-definition stub an edge target is — which is what
        // keeps the answer independent of whether the `impl` was written above
        // or below the type (R6).
        let parent = match owner_of.get(row.fqn.as_str()) {
            Some(owner) => Some(node_for(store, folder_id, &mut known, owner, language).await?),
            None => None,
        };
        let id = store.upsert_v2_symbol(folder_id, &node_columns_of(&row), parent.as_ref()).await?;
        *declarations.entry(row.fqn.clone()).or_default() += 1;
        known.insert(row.fqn, id);
    }
    let collisions: Vec<Collision> = declarations
        .into_iter()
        .filter(|(_, declarations)| *declarations > 1)
        .map(|(fqn, declarations)| Collision { fqn, declarations })
        .collect();
    let mut edges = 0usize;
    let mut edge_collisions: Vec<EdgeCollision> = Vec::new();
    // Which edge row each of this file's groups landed on. Two groups on one row
    // is the one way an occurrence list can still be lost — see [`EdgeCollision`].
    let mut merged: HashMap<uuid::Uuid, String> = HashMap::new();

    for row in edge_rows_of(facts, &file_identity(facts)?) {
        let source_id = node_for(store, folder_id, &mut known, &row.source, language).await?;
        let props = edge_props(&row);
        let target = target_ref_of(&row.target)?;
        let fact = EdgeFact { source_id, target, kind: row.kind, props };
        let edge_id = store.persist_edge_fact(folder_id, &fact, &known, Some(language)).await?;
        let names = match &row.target {
            TargetKey::Proven(fqn) => fqn.clone(),
            TargetKey::Named(name) => name.clone(),
        };
        if let Some(first) = merged.insert(edge_id, names.clone()) {
            edge_collisions.push(EdgeCollision {
                source: row.source.clone(),
                kind: row.kind,
                targets: [first, names],
            });
        }
        // The occurrences go out SEPARATELY and keyed by this file, so a re-scan
        // of it replaces its own list while another file's survives.
        store.merge_v2_edge_occurrences(&edge_id, file_path, &occurrences_prop(&row)).await?;
        edges += 1;
    }
    Ok(Written {
        symbols: facts.symbols.len(),
        edges,
        collisions,
        edge_collisions,
        edge_rows: merged.into_keys().collect(),
    })
}

/// The type each declaration is a member OF, read off the ownership relations
/// the same walk produced (spec §3.3).
///
/// Read off the RELATIONS rather than re-derived from the fqn, for the reason
/// `Owner` exists in the walk at all: a member's identity does not spell its
/// owner's. An enum variant's container is `Enum::Variant`, which is not the
/// spelling the variant's own identity uses, and a trait-impl member's identity
/// carries a trait segment its type's does not. Re-minting an owner from a
/// member's key would produce a parent no declaration ever minted.
///
/// An `Owns` whose parent the ladder could not place is skipped, not defaulted:
/// a member parented on a guess is a containment the source does not state (R4).
fn owners(facts: &FileFacts) -> HashMap<&str, &str> {
    facts
        .relations
        .iter()
        .filter(|relation| relation.kind == RelationKind::Owns)
        .filter_map(|relation| match &relation.parent {
            Resolution::Resolved(parent) => Some((relation.child.as_str(), parent.as_str())),
            Resolution::Unresolved { .. } => None,
        })
        .collect()
}

/// The node an identity names, creating the STUB shape if nothing has written
/// it yet.
///
/// One function for the two places that need it — an edge's source and a
/// member's owner — because both are the same reference-before-definition case
/// the merge contract handles, and two copies would be two stub policies. A use
/// site can sit in the FILE itself rather than in a declaration the walk emitted
/// (a `use`, a const initialiser at file scope); a member's owner can be a type
/// declared in another file, or below the `impl` in this one. In every case the
/// identity is one some declaration mints, so this is not a node invented to
/// hang a row on.
async fn node_for(
    store: &PgStore,
    folder_id: &uuid::Uuid,
    known: &mut HashMap<String, uuid::Uuid>,
    fqn: &str,
    language: &str,
) -> Result<uuid::Uuid, String> {
    if let Some(id) = known.get(fqn) {
        return Ok(*id);
    }
    // Ask before writing, so a re-scan of an unchanged file does not stamp
    // `modified_at` on a row it is about to write back unaltered. A file's own
    // module node comes through here on every file-scope use site, so without
    // this every file churns at least one row per pass.
    let id = match store.v2_node_unchanged_by_reference(folder_id, fqn, Some(language)).await? {
        Some(id) => id,
        None => {
            let (kind, name) = stub_kind_and_name(fqn)?;
            store.upsert_node_by_fqn(folder_id, fqn, kind, &name, Some(language), None).await?
        }
    };
    known.insert(fqn.to_string(), id);
    Ok(id)
}

/// The identity of the file itself, which is the identity of the module it
/// declares — what an import hangs off, and what a use site at file scope
/// belongs to.
///
/// Dispatched to the language module because the rule is the language's: a Rust
/// `mod.rs` names the directory it sits in and not itself. Delegated rather than
/// re-derived, so the file's identity has ONE owner and the `mod x;` that names
/// this file lands on the same node.
fn file_identity(facts: &FileFacts) -> Result<String, String> {
    file_identity_of(facts.language, &facts.package, &facts.module, &facts.path)
}

/// [`file_identity`] for a caller that has no facts.
///
/// Reconcile needs it for a file that is GONE from disk: there was no parse, so
/// there are no facts, and the file's own module identity is still the source of
/// every file-scope edge it emitted. Taking the four parts rather than the facts
/// is what lets both callers reach the one derivation instead of the deleted
/// file getting a second, weaker one.
pub(crate) fn file_identity_of(
    language: Language,
    package: &str,
    module: &str,
    path: &str,
) -> Result<String, String> {
    let fqn = match language {
        Language::Rust => super::lang::rust::file_fqn(package, module, path),
    };
    fqn.map(|fqn| fqn.as_str().to_string())
        .map_err(|e| format!("{path} ({package}::{module}) has no file identity: {e:?}"))
}

/// The `node_kind` a stub carries when its identity does not state one.
///
/// [`Reach::Item`] is every path-reachable declaration at once — a type, a
/// trait, a function, a const, an enum variant — so an `item` stub's kind is
/// genuinely NOT KNOWN until the declaration is indexed. Two rules meet here
/// and both are stated rather than hidden.
///
/// The reference's [`RefKind`] must not fill the gap. "`Constructs`, therefore
/// a type" is precisely the inference that produced the identity break spec
/// §2.1 records, and a guess written into a row outlives the guess whenever no
/// declaration ever arrives.
///
/// The column is `not null` and `sensei.node_kind` has no value meaning "not
/// yet known", so it holds a PLACEHOLDER — chosen once here, never per use
/// site. `parameter` and not a plausible `type` or `function`, because a
/// plausible one is a value no caller can tell from a real reading, which is
/// the one thing a failure path may not produce. This one can be told apart
/// two ways: v2 never mints `parameter` for any declaration (a parameter is a
/// typed prop on its function, D2), and no query in this tree selects it, so
/// nothing counts a stub as a classification it does not have. The row is also
/// marked as a stub by the schema's own means — `upsert_node_by_fqn` writes it
/// `resolved = false` with a null `file_path` — and replaces `kind` wholesale
/// the moment the declaration lands. Widening the enum is step 9's decision,
/// on the table the shipped indexer writes.
const KIND_NOT_YET_KNOWN: &str = "parameter";

/// The node kind and name a placeholder for `fqn` carries.
///
/// READ OFF THE IDENTITY and never guessed from the use site: the trailing
/// segment of every local fqn is the REACH the name is reached through, so
/// [`Reach::Field`] is a field and [`Reach::Mod`] a module whatever the
/// reference looked like. Where the reach states no kind, this says so — see
/// [`KIND_NOT_YET_KNOWN`].
///
/// [`Reach::Macro`] has no `node_kind` of its own — the enum has no `macro` —
/// so it takes the one a macro is invoked like. That is a collapse and not a
/// guess: the identity DOES state that the target is a macro, and only the
/// column cannot spell it. Named here rather than hidden, and it disappears
/// when step 9 decides the enum.
pub(crate) fn stub_kind_and_name(fqn: &str) -> Result<(&'static str, String), String> {
    let parsed = fqn::parse(fqn).map_err(|e| format!("{fqn} is not an fqn: {e:?}"))?;
    let name = parsed
        .tail
        .last()
        .map(|segment| segment.rsplit("::").next().unwrap_or(segment).to_string())
        .unwrap_or_else(|| parsed.package.to_string());
    let kind = match parsed.origin {
        Origin::Lib => "lib_symbol",
        Origin::Local { reach: Reach::Item, .. } => KIND_NOT_YET_KNOWN,
        Origin::Local { reach: Reach::Field, .. } => "field",
        Origin::Local { reach: Reach::Macro, .. } => "function",
        Origin::Local { reach: Reach::Mod, .. } => "module",
    };
    Ok((kind, name))
}

/// The write policy for one edge's target — the SHARED one
/// (`crate::graph_facts`), so v2 does not arrive with a second idea of what a
/// miss means.
///
/// A proven first-party target that has not been indexed yet becomes a STUB and
/// not an unresolved edge, and that is R6: whether the edge resolves must not
/// depend on which file the scan reached first.
fn target_ref_of(target: &TargetKey) -> Result<TargetRef, String> {
    match target {
        TargetKey::Named(name) => Ok(TargetRef::Unresolvable { name: name.clone() }),
        TargetKey::Proven(fqn) => {
            let parsed = fqn::parse(fqn).map_err(|e| format!("{fqn} is not an fqn: {e:?}"))?;
            let (kind, name) = stub_kind_and_name(fqn)?;
            Ok(match parsed.origin {
                Origin::Lib => {
                    TargetRef::Lib { fqn: fqn.clone(), name, package: parsed.package.to_string() }
                }
                Origin::Local { .. } => TargetRef::Internal {
                    fqn: fqn.clone(),
                    name,
                    on_miss: OnMiss::CreateStub { kind },
                },
            })
        }
    }
}

// ── reading back ─────────────────────────────────────────────────────────────

/// Read one folder's v2 rows back into the shapes they were written from.
///
/// Every miss is an error and never a substituted value: a decoded row that
/// filled in a blank would make the round-trip test pass on data the database
/// does not hold, which is the one thing it exists to catch (R4).
pub async fn read_back(store: &PgStore, folder_id: &uuid::Uuid) -> Result<Stored, String> {
    let symbols = store
        .v2_definition_nodes(folder_id)
        .await?
        .iter()
        .map(symbol_row_from_columns)
        .collect::<Result<Vec<_>, String>>()?;
    let mut stored =
        Stored { symbols, references: Vec::new(), relations: Vec::new(), imports: Vec::new() };
    for edge in store.v2_edges(folder_id).await? {
        read_edge_into(&edge, &mut stored)?;
    }
    Ok(stored)
}

/// Every fact one edge row records.
///
/// The target comes from the EDGE — `target_id`'s fqn, or `target_name` — and
/// never from the props, so this reads what the graph will answer with rather
/// than a copy of what was written next to it. The `kind` COLUMN is read on the
/// same terms: it is what `get_callers` filters on, so a mapping that filed a
/// call under `references` has to fail here rather than be reproduced out of
/// props that agree with it.
fn read_edge_into(edge: &EdgeColumns, stored: &mut Stored) -> Result<(), String> {
    let EdgeColumns { source_fqn, kind: column, target_fqn, target_name, props } = edge;
    // An OBJECT keyed by file, not a flat list — see `occurrences_prop`. A flat
    // list is the shape this reader's writer no longer produces, so it is an
    // error rather than something to read half of.
    let by_file = props
        .get("occurrences")
        .and_then(|v| v.as_object())
        .ok_or_else(|| format!("{source_fqn}: an edge with no occurrences is not a v2 edge"))?;
    let occurrences: Vec<&serde_json::Value> = by_file
        .values()
        .map(|of_one_file| {
            of_one_file.as_array().ok_or_else(|| {
                format!("{source_fqn}: a file's occurrences must be a list, got {of_one_file}")
            })
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .flatten()
        .collect();

    // A proven occurrence on an edge pointing at nothing is a contradiction, not
    // something to paper over: it would read back as a resolution the graph
    // cannot follow. Same the other way — a miss with no name recorded is a
    // reference that vanished into an anonymous row.
    let proven = |what: &str| {
        target_fqn.clone().ok_or_else(|| format!("{source_fqn}: a proven {what} points at no node"))
    };
    let missed = |what: &str, reason, evidence| match target_name {
        Some(_) => Ok(TargetRow::Unresolved { reason, evidence }),
        None => Err(format!("{source_fqn}: a missed {what} carries no target name")),
    };

    for prop in occurrences {
        let occurrence = occurrence_from_prop(prop)
            .ok_or_else(|| format!("{source_fqn}: unreadable occurrence {prop}"))?;
        // The column and the occurrence must agree about what this edge IS. The
        // column is coarser (see `reference_edge_kind`), so the check is
        // membership and not equality — but it is checked, because otherwise
        // nothing reads the column at all and inverting the mapping that writes
        // it changes the graph while every test still passes.
        let filed_as = |allowed: &[&dyn std::fmt::Debug], ok: bool| -> Result<(), String> {
            if ok {
                return Ok(());
            }
            Err(format!(
                "{source_fqn}: the edge_kind column says {column:?}, which does not hold                  {allowed:?} — the column and the occurrence disagree about what this edge is"
            ))
        };
        match occurrence {
            Occurrence::Use { kind, at, outcome } => {
                filed_as(&[&kind], edge_kind_holds_uses(column).contains(&kind))?;
                let target = match outcome {
                    Outcome::Proven => TargetRow::Resolved(proven("use site")?),
                    Outcome::Missed { reason, evidence } => missed("use site", reason, evidence)?,
                };
                stored.references.push(ReferenceRow { from: source_fqn.clone(), kind, at, target });
            }
            Occurrence::Structure { kind, at, outcome } => {
                filed_as(&[&kind], edge_kind_holds_structure(column).contains(&kind))?;
                let parent = match outcome {
                    Outcome::Proven => TargetRow::Resolved(proven("relation")?),
                    Outcome::Missed { reason, evidence } => missed("relation", reason, evidence)?,
                };
                stored.relations.push(RelationRow { kind, child: source_fqn.clone(), parent, at });
            }
            Occurrence::Brought { binds, origin, at } => {
                filed_as(&[&"an import"], column == "imports")?;
                // The specifier IS the target name — an import names a path, and
                // placing that path is the ladder's job, done per NAME rather
                // than per specifier. Minting an identity for it here would be a
                // second, weaker resolver (R7).
                let path = target_name
                    .clone()
                    .ok_or_else(|| format!("{source_fqn}: an import row carries no specifier"))?;
                stored.imports.push(ImportRow {
                    from: source_fqn.clone(),
                    path,
                    binds,
                    origin,
                    at,
                });
            }
        }
    }
    Ok(())
}

/// One `sensei.nodes` row, as the walk produced it.
///
/// Two of these fields land in a COLUMN as well as in a prop, and both columns
/// are read here. That is the difference between a round trip and an encoder
/// agreeing with itself: `node_kind` is what the graph's own queries filter on
/// and `is_exported` is what "what is this module's surface" answers with, so a
/// mapping that filed every function as a method, or inverted the export
/// boolean, would change the graph while props — written by the same encoder —
/// still read back byte-perfect. Measured before this check existed: three such
/// one-line inversions survived the whole suite.
///
/// Neither column can simply BE the value. `node_kind` collapses
/// trait/interface, static/const and macro/function, and a boolean cannot hold
/// four visibilities. So the prop is the value and the column is CHECKED
/// against it, through inverses written independently of the encoders —
/// [`node_kind_holds`] and [`is_exported_holds`]. A disagreement is an error
/// and never a preference for one side (R4).
fn symbol_row_from_columns(columns: &NodeColumns) -> Result<SymbolRow, String> {
    let NodeColumns {
        fqn,
        kind: node_kind,
        name,
        file_path,
        language,
        line_start,
        line_end,
        is_exported,
        docstring,
        props,
    } = columns;

    let missing = |what: &str| format!("{fqn}: {what} never reached the database");

    let kind_label = props.get("symbol_kind").and_then(|v| v.as_str()).ok_or_else(|| {
        missing("the exact symbol kind (props.symbol_kind); the node_kind column collapses trait/interface, static/const and macro/function, so it cannot stand in")
    })?;
    let kind = symbol_kind_from_label(kind_label)
        .ok_or_else(|| format!("{fqn}: props.symbol_kind names no kind: {kind_label:?}"))?;
    let holds = node_kind_holds(node_kind).ok_or_else(|| {
        format!("{fqn}: the node_kind column says {node_kind:?}, which no v2 symbol is filed under")
    })?;
    if !holds.contains(&kind) {
        return Err(format!(
            "{fqn}: the node_kind column says {node_kind:?}, which holds {holds:?}, but the walk \
             read a {kind:?} — the column the graph queries is not the kind the walk saw"
        ));
    }

    let file_path = file_path.clone().ok_or_else(|| missing("the file path"))?;
    let language_label = language.as_deref().ok_or_else(|| missing("the language"))?;
    let language = Language::from_label(language_label)
        .ok_or_else(|| format!("{fqn}: language names no language v2 reads: {language_label:?}"))?;

    let start_line = line_start.ok_or_else(|| missing("the first line"))?;
    let end_line = line_end.ok_or_else(|| missing("the last line"))?;
    let cols = props
        .get("span_columns")
        .and_then(|v| v.as_array())
        .ok_or_else(|| missing("the span's columns (props.span_columns)"))?;
    let column = |i: usize| -> Result<u32, String> {
        cols.get(i)
            .and_then(serde_json::Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| format!("{fqn}: props.span_columns[{i}] is not a column number"))
    };
    let span = Span {
        start_line: u32::try_from(start_line)
            .map_err(|_| format!("{fqn}: line_start {start_line} is not a line number"))?,
        start_col: column(0)?,
        end_line: u32::try_from(end_line)
            .map_err(|_| format!("{fqn}: line_end {end_line} is not a line number"))?,
        end_col: column(1)?,
    };

    let visibility =
        visibility_from_props(props).ok_or_else(|| missing("the visibility (props.visibility)"))?;
    if !is_exported_holds(*is_exported, &visibility) {
        return Err(format!(
            "{fqn}: the is_exported column says {is_exported}, but the walk read {visibility:?} — \
             the column that answers \"what is this module's surface\" disagrees with the \
             visibility the source states"
        ));
    }
    let declared_type = declared_type_from_props(props, "declared_type")
        .ok_or_else(|| missing("the declared type (props.declared_type)"))?;
    let params =
        params_from_props(props).ok_or_else(|| missing("the parameters (props.params)"))?;

    Ok(SymbolRow {
        fqn: fqn.clone(),
        kind,
        name: name.clone(),
        span,
        visibility,
        docstring: docstring.clone(),
        declared_type,
        params,
        file_path,
        language,
    })
}

fn visibility_props(visibility: &Visibility) -> serde_json::Value {
    match visibility {
        Visibility::Public => serde_json::json!({ "seen_from": "public" }),
        Visibility::Crate => serde_json::json!({ "seen_from": "crate" }),
        Visibility::Restricted(scope) => {
            serde_json::json!({ "seen_from": "restricted", "scope": scope })
        }
        Visibility::Private => serde_json::json!({ "seen_from": "private" }),
    }
}

fn visibility_from_props(props: &serde_json::Value) -> Option<Visibility> {
    let visibility = props.get("visibility")?;
    match visibility.get("seen_from")?.as_str()? {
        "public" => Some(Visibility::Public),
        "crate" => Some(Visibility::Crate),
        "restricted" => {
            Some(Visibility::Restricted(visibility.get("scope")?.as_str()?.to_string()))
        }
        "private" => Some(Visibility::Private),
        _ => None,
    }
}

/// A stated type is its text; an unstated one is JSON `null`.
///
/// The two are told apart by the JSON type and not by emptiness: a language may
/// state the empty string, and collapsing that onto "stated nothing" would put a
/// value the walk read into the bucket that licenses
/// [`super::facts::Reason::NoDeclaredType`].
fn declared_type_prop(declared: &DeclaredType) -> serde_json::Value {
    match declared {
        DeclaredType::Stated(text) => serde_json::Value::String(text.clone()),
        DeclaredType::Unstated => serde_json::Value::Null,
    }
}

fn declared_type_from_props(props: &serde_json::Value, key: &str) -> Option<DeclaredType> {
    match props.get(key)? {
        serde_json::Value::Null => Some(DeclaredType::Unstated),
        serde_json::Value::String(text) => Some(DeclaredType::Stated(text.clone())),
        _ => None,
    }
}

/// Parameters are props on their function and never nodes (D2) — so this is
/// where they live, and the spec's own word for them is what the column is.
fn params_prop(params: &[Param]) -> serde_json::Value {
    serde_json::Value::Array(
        params
            .iter()
            .map(|param| {
                let Param { name, position, declared_type } = param;
                serde_json::json!({
                    "name": name,
                    "position": position,
                    "declared_type": declared_type_prop(declared_type),
                })
            })
            .collect(),
    )
}

fn params_from_props(props: &serde_json::Value) -> Option<Vec<Param>> {
    props
        .get("params")?
        .as_array()?
        .iter()
        .map(|param| {
            Some(Param {
                name: param.get("name")?.as_str()?.to_string(),
                position: u32::try_from(param.get("position")?.as_u64()?).ok()?,
                declared_type: declared_type_from_props(param, "declared_type")?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::db::pg_store::PgStore;
    use crate::db::pg_store::tests::create_test_folder;
    use crate::indexer::facts::{
        DeclaredType, FileFacts, Language, Param, Reason, RelationKind, Symbol,
    };
    use crate::indexer::lang::rust::{self, Source};
    use crate::indexer::persist;
    use crate::indexer::resolve::{World, resolve};

    /// One file carrying one of everything the round-trip has to preserve: a
    /// stated return type, a stated field type, a doc comment, three
    /// visibilities, parameters, a resolved call, a call nothing here declares,
    /// and a trait impl.
    const FIXTURE: &str = r#"
use std::collections::BTreeMap;

/// A gadget.
pub struct Gadget {
    /// How wide it is.
    pub width: u32,
    pub(crate) label: String,
    seen: BTreeMap<u32, u32>,
}

pub trait Shape: Sized + Nowhere {
    fn area(&self) -> u32;
}

impl Gadget {
    /// Build one.
    pub fn new(width: u32, label: String) -> Gadget {
        Gadget { width, label, seen: BTreeMap::new() }
    }

    fn widen(&self) -> u32 {
        widest(self.width)
    }

    fn borrowed(&self) -> u32 {
        nowhere_at_all(self.width)
    }
}

impl Shape for Gadget {
    fn area(&self) -> u32 {
        self.width
    }
}

pub fn widest(a: u32) -> u32 {
    a
}
"#;

    const FILE_PATH: &str = "src/gadget.rs";

    /// The fixture, walked and then placed against the shared ladder — the same
    /// two steps the processor will run at cutover, so what persistence is
    /// handed here is what it will be handed then.
    fn walked() -> FileFacts {
        walk_of("gadget", FILE_PATH, FIXTURE)
    }

    fn walk_of(module: &str, path: &str, text: &str) -> FileFacts {
        let facts = rust::read(&Source { package: "senseid", module, path, text })
            .expect("the fixture parses");
        let first_party: BTreeSet<String> = ["senseid".to_string()].into_iter().collect();
        let scanned = BTreeSet::new();
        resolve(facts, &rust::GRAMMAR, &World { first_party: &first_party, scanned: &scanned })
    }

    async fn a_folder(store: &PgStore, test: &str) -> uuid::Uuid {
        create_test_folder(store, &format!("v2_persist_{test}_{}", uuid::Uuid::new_v4())).await
    }

    /// R3, at the seam that lost data before. Every field of every [`Symbol`]
    /// the walk produced is read back OUT OF POSTGRES and compared to what went
    /// in. The `let Symbol { .. }` destructure is what keeps the list complete:
    /// a new field stops this compiling until it is checked here too (R9).
    #[tokio::test]
    async fn every_field_of_every_symbol_survives_the_round_trip() {
        let facts = walked();
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "symbols").await;

        persist::write(&store, &folder, &facts).await.expect("the facts persist");
        let stored = persist::read_back(&store, &folder).await.expect("the rows read back");

        assert_eq!(
            stored.symbols.len(),
            facts.symbols.len(),
            "every symbol the walk produced must be a row"
        );
        for symbol in &facts.symbols {
            let Symbol { fqn, kind, name, span, visibility, docstring, declared_type, params } =
                symbol;
            let row = stored
                .symbols
                .iter()
                .find(|r| r.fqn == fqn.as_str())
                .unwrap_or_else(|| panic!("{fqn} reached no row"));
            assert_eq!(row.kind, *kind, "{fqn}: kind");
            assert_eq!(row.name, *name, "{fqn}: name");
            assert_eq!(row.span, *span, "{fqn}: span");
            assert_eq!(row.visibility, *visibility, "{fqn}: visibility");
            assert_eq!(row.docstring, *docstring, "{fqn}: docstring");
            assert_eq!(row.declared_type, *declared_type, "{fqn}: declared type");
            assert_eq!(row.params, *params, "{fqn}: params");
            assert_eq!(row.file_path, FILE_PATH, "{fqn}: file path");
            assert_eq!(row.language, Language::Rust, "{fqn}: language");
        }
    }

    /// The exact seam the previous implementation lost data at, named on its
    /// own so a regression there reads as itself and not as "some field of some
    /// symbol changed".
    ///
    /// `extract_return_type` ran on every function for months and the value was
    /// dropped, because `upsert_node`'s eight positional arguments had no slot
    /// for it. So this asserts the stated types are in the DATABASE, by value,
    /// for a return type AND for a field type — and that a declaration which
    /// states none reads back as [`DeclaredType::Unstated`] rather than as a
    /// blank that could have been either.
    #[tokio::test]
    async fn a_stated_type_reaches_the_database() {
        let facts = walked();
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "declared_type").await;

        persist::write(&store, &folder, &facts).await.expect("the facts persist");
        let stored = persist::read_back(&store, &folder).await.expect("the rows read back");

        let stated = |fqn: &str| -> DeclaredType {
            stored
                .symbols
                .iter()
                .find(|r| r.fqn == fqn)
                .unwrap_or_else(|| panic!("{fqn} reached no row"))
                .declared_type
                .clone()
        };

        assert_eq!(
            stated("rust·senseid·gadget·widest·item"),
            DeclaredType::Stated("u32".to_string()),
            "a function's return type must reach the database"
        );
        assert_eq!(
            stated("rust·senseid·gadget·Gadget·new·item"),
            DeclaredType::Stated("Gadget".to_string()),
            "a method's return type must reach the database"
        );
        assert_eq!(
            stated("rust·senseid·gadget·Gadget·seen·field"),
            DeclaredType::Stated("BTreeMap<u32, u32>".to_string()),
            "a field's declared type must reach the database, verbatim"
        );
        assert_eq!(
            stated("rust·senseid·gadget·Gadget·item"),
            DeclaredType::Unstated,
            "a struct states no type, and that reads back as Unstated and not as a blank"
        );

        // A parameter is a typed prop on its function (D2), so its type travels
        // the same seam and is checked at the same place.
        let new_params = stored
            .symbols
            .iter()
            .find(|r| r.fqn == "rust·senseid·gadget·Gadget·new·item")
            .expect("Gadget::new reached no row")
            .params
            .clone();
        assert_eq!(
            new_params,
            vec![
                Param {
                    name: "width".to_string(),
                    position: 0,
                    declared_type: DeclaredType::Stated("u32".to_string()),
                },
                Param {
                    name: "label".to_string(),
                    position: 1,
                    declared_type: DeclaredType::Stated("String".to_string()),
                },
            ],
            "every parameter's stated type must reach the database"
        );
    }

    /// Debug text as the sort key: it prints every field, so ordering by it is
    /// canonical, and a difference reads as a difference in the value rather
    /// than in the order two vectors happened to be built in.
    fn ordered<T: std::fmt::Debug>(mut rows: Vec<T>) -> Vec<T> {
        rows.sort_by_key(|row| format!("{row:?}"));
        rows
    }

    /// Compare two whole sets of rows, but REPORT only the first row they differ
    /// on. `assert_eq!` on the corpus prints both vectors in full — tens of
    /// megabytes — and a failure nobody can read is a failure nobody can act on.
    /// Nothing is weakened: every element is still compared.
    fn same_rows<T: std::fmt::Debug + PartialEq>(actual: Vec<T>, expected: Vec<T>, what: &str) {
        let actual = ordered(actual);
        let expected = ordered(expected);
        for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
            assert!(a == e, "{what}[{i}] differs\n  stored: {a:?}\n  walked: {e:?}");
        }
        assert_eq!(
            actual.len(),
            expected.len(),
            "{what}: {} stored, {} walked; first extra: {:?}",
            actual.len(),
            expected.len(),
            if actual.len() > expected.len() {
                actual.get(expected.len()).map(|r| format!("{r:?}"))
            } else {
                expected.get(actual.len()).map(|r| format!("{r:?}"))
            }
        );
    }

    /// R2 carried through to storage. The walk emits one [`Reference`] per use
    /// site and a miss is a reason rather than an omission; if persistence then
    /// drops the misses, the whole measurement is undone at the last step.
    #[tokio::test]
    async fn every_field_of_every_reference_survives_the_round_trip() {
        let facts = walked();
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "references").await;

        persist::write(&store, &folder, &facts).await.expect("the facts persist");
        let stored = persist::read_back(&store, &folder).await.expect("the rows read back");

        let expected: Vec<persist::ReferenceRow> =
            facts.references.iter().map(persist::ReferenceRow::of).collect();
        assert!(!expected.is_empty(), "the fixture must exercise some references");
        same_rows(stored.references.clone(), expected, "every use site the walk saw");
    }

    /// The named verification: an unresolved reference is a ROW carrying its
    /// reason, never an absence. `nowhere_at_all` is declared nowhere and
    /// imported by nothing, so the ladder refuses it — and that refusal, with
    /// its reason and its evidence, is what has to be in the database.
    #[tokio::test]
    async fn an_unresolved_reference_is_a_row_carrying_its_reason() {
        let facts = walked();
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "unresolved").await;

        persist::write(&store, &folder, &facts).await.expect("the facts persist");
        let stored = persist::read_back(&store, &folder).await.expect("the rows read back");

        let misses: Vec<&persist::ReferenceRow> = stored
            .references
            .iter()
            .filter(|r| {
                matches!(&r.target, persist::TargetRow::Unresolved { evidence, .. }
                                             if evidence.name == "nowhere_at_all")
            })
            .collect();
        assert_eq!(misses.len(), 1, "the one call nothing declares must be exactly one row");

        let persist::TargetRow::Unresolved { reason, evidence } = &misses[0].target else {
            unreachable!("filtered above");
        };
        assert_eq!(
            *reason,
            Reason::NoImportInScope,
            "the reason the ladder gave must be the reason the row carries"
        );
        assert_eq!(evidence.node_kind, "identifier", "the evidence must survive too");
        assert_eq!(
            misses[0].from, "rust·senseid·gadget·Gadget·borrowed·item",
            "and it must still say which symbol the use site sat in"
        );

        // Not an absence: the row exists, and it names the target it could not
        // place rather than pointing at a node that would be a guess.
        let named: Vec<String> = store
            .v2_edges(&folder)
            .await
            .expect("the edges read back")
            .into_iter()
            .filter(|e| e.target_name.as_deref() == Some("nowhere_at_all"))
            .map(|e| format!("{}->{:?}", e.source_fqn, e.target_fqn))
            .collect();
        assert_eq!(
            named,
            vec!["rust·senseid·gadget·Gadget·borrowed·item->None".to_string()],
            "an unresolved edge carries the name and no target id — never a guessed node"
        );
    }

    /// Structure is emitted by the same walk (D5) and travels the same seam. A
    /// relation whose parent the ladder could not place is an unresolved ROW
    /// with its reason, exactly as a reference is — never a bare name.
    #[tokio::test]
    async fn every_field_of_every_relation_survives_the_round_trip() {
        let facts = walked();
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "relations").await;

        persist::write(&store, &folder, &facts).await.expect("the facts persist");
        let stored = persist::read_back(&store, &folder).await.expect("the rows read back");

        let expected: Vec<persist::RelationRow> =
            facts.relations.iter().map(persist::RelationRow::of).collect();
        assert!(!expected.is_empty(), "the fixture must exercise some relations");
        same_rows(stored.relations.clone(), expected, "every structural fact the walk saw");

        // The two shapes named in the plan's step 6, now as rows: an inherent
        // impl owns its members and inherits nothing, a trait impl is an edge.
        let kinds: Vec<String> = ordered(
            stored
                .relations
                .iter()
                .filter(|r| r.kind != RelationKind::Owns)
                .map(|r| format!("{:?} {:?}", r.kind, r.parent))
                .collect(),
        );
        assert_eq!(
            kinds,
            vec![
                "Extends Resolved(\"lib·std·marker::Sized\")".to_string(),
                "Extends Unresolved { reason: NoImportInScope, evidence: EvidenceRow { name: \"Nowhere\", node_kind: \"type_identifier\", reach: Item, saw: [Candidate(\"rust·senseid·gadget·Nowhere·item\"), UnplacedType(\"Nowhere\")] } }".to_string(),
                "TraitImpl Resolved(\"rust·senseid·gadget·Shape·item\")".to_string(),
            ],
            "the inherent impl contributes no inheritance edge; the trait impl and the \
             supertrait bound each contribute one, and the unplaceable one keeps its reason"
        );
    }

    /// An import is what decides externality (spec §2), so losing one loses the
    /// evidence the ladder's answers rest on.
    #[tokio::test]
    async fn every_import_survives_the_round_trip() {
        let facts = walked();
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "imports").await;

        persist::write(&store, &folder, &facts).await.expect("the facts persist");
        let stored = persist::read_back(&store, &folder).await.expect("the rows read back");

        let expected: Vec<persist::ImportRow> = facts
            .imports
            .iter()
            .map(|import| persist::ImportRow::of(import, "rust·senseid·gadget·mod"))
            .collect();
        assert!(!expected.is_empty(), "the fixture must exercise some imports");
        same_rows(stored.imports.clone(), expected, "every import the walk saw");
    }

    /// A2, carried to the last step. The walk counts every use site; this counts
    /// what reached storage. Aggregate on purpose — the per-fact tests above say
    /// the VALUES survive, and this says the COUNT does, which is the property a
    /// grouping bug breaks without changing any single value.
    #[tokio::test]
    async fn no_fact_the_walk_produced_is_missing_from_the_database() {
        let facts = walked();
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "counts").await;

        persist::write(&store, &folder, &facts).await.expect("the facts persist");
        let stored = persist::read_back(&store, &folder).await.expect("the rows read back");

        assert_eq!(
            (
                stored.symbols.len(),
                stored.references.len(),
                stored.relations.len(),
                stored.imports.len()
            ),
            (
                facts.symbols.len(),
                facts.references.len(),
                facts.relations.len(),
                facts.imports.len()
            ),
            "symbols, references, relations, imports"
        );
    }

    /// The safety property, at the read boundary. v2 has no caller, but the
    /// cutover puts BOTH indexers' rows in one folder for one language at a
    /// time (spec §7), so v2's reader has to be able to tell them apart. A
    /// reader that swallowed a v1 edge would report facts v2 never produced,
    /// and a differential harness (step 8) built on it would be measuring
    /// itself.
    #[tokio::test]
    async fn a_v1_edge_in_the_same_folder_is_neither_read_as_a_v2_fact_nor_an_error() {
        let facts = walked();
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "coexist").await;
        persist::write(&store, &folder, &facts).await.expect("the facts persist");
        let v2_only = persist::read_back(&store, &folder).await.expect("the rows read back");

        // A v1 edge: the shipped indexer's shape — no props, a bare target name.
        let source = store
            .node_id_by_fqn(&folder, "rust·senseid·gadget·widest·item")
            .await
            .expect("the lookup runs")
            .expect("the fixture declares it");
        store
            .insert_edge(&folder, &source, None, Some("something_v1_saw"), None, "calls")
            .await
            .expect("the v1 edge inserts");

        let after = persist::read_back(&store, &folder).await.expect("the rows still read back");
        assert_eq!(after, v2_only, "a v1 edge must change nothing v2 reads");
    }

    /// R9 as a property of the SOURCE, because it is not a property of any
    /// value.
    ///
    /// Every conversion from a fact to a row destructures its source with every
    /// field NAMED. The compiler enforces that today — a field added to `Symbol`
    /// stops the build — but only until someone silences that build error with
    /// `..`, and from that moment the field is captured by the walk and never
    /// written, with nothing failing anywhere. That is precisely the shape of
    /// the defect this rewrite exists to remove, so it gets a guard rather than
    /// a convention.
    ///
    /// This is also the substantive version of the plan's "use a struct, not
    /// positional args": a positional list is dangerous only because it silently
    /// drops a field, and naming every field is what makes dropping loud.
    ///
    /// `field: _` is banned alongside `..`, and that half is not pedantry. It is
    /// how `NodeColumns.kind` and `NodeColumns.is_exported` came to be discarded
    /// on the way back: both were bound to `_` and reconstructed from props the
    /// same writer wrote, so the round trip compared the encoder with itself and
    /// three one-line inversions of production column mappings — a function
    /// filed as a method, the export boolean flipped, a call filed as a
    /// reference — passed the whole suite. `..` and `: _` are the same act.
    #[test]
    fn every_conversion_between_a_fact_and_a_row_names_every_field() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let sources = [
            ("indexer/persist.rs", root.join("src/indexer/persist.rs")),
            ("db/pg_store/indexer_v2.rs", root.join("src/db/pg_store/indexer_v2.rs")),
        ];
        // Every type that crosses the fact/row boundary in either direction.
        let required = [
            "Symbol",
            "Reference",
            "Relation",
            "Import",
            "Evidence",
            "Param",
            "SymbolRow",
            "ImportRow",
            "EvidenceRow",
            "NodeColumns",
            "EdgeColumns",
        ];

        let mut checked = 0;
        for ty in required {
            let needle = format!("let {ty} {{");
            let mut found = 0;
            for (name, path) in &sources {
                let body = std::fs::read_to_string(path)
                    .unwrap_or_else(|e| panic!("cannot read {name}: {e}"));
                let body = crate::indexer::outside_tests(&body).to_string();
                let mut from = 0;
                while let Some(at) = body[from..].find(&needle) {
                    let start = from + at;
                    let end =
                        start + body[start..].find('}').expect("a destructure closes its brace");
                    let destructure = &body[start..end];
                    for dropped in ["..", ": _"] {
                        assert!(
                            !destructure.contains(dropped),
                            "{name}: `{}` drops fields with `{dropped}` — every field of a fact \
                             must be placed, or it stops reaching the database and nothing fails \
                             (R9). Binding a COLUMN to `_` and rebuilding it from props is the \
                             same act: it makes the round trip compare the encoder with itself",
                            destructure.split_whitespace().collect::<Vec<_>>().join(" ")
                        );
                    }
                    found += 1;
                    checked += 1;
                    from = end;
                }
            }
            assert!(found > 0, "nothing destructures a {ty}, so this guard does not cover it");
        }
        assert!(checked >= required.len(), "the guard read {checked} destructures");
    }

    /// The one thing one file's write CANNOT keep apart, said out loud.
    ///
    /// Two declarations that mint one fqn are one node, because
    /// `nodes_unique_fqn` is the merge contract (spec §2) and an fqn is a lookup
    /// key by definition. That collapse is the fqn GRAMMAR's, not persistence's,
    /// and persistence's duty is to make it countable rather than silent — a
    /// symbol that vanishes with nothing said is the exact defect this rewrite
    /// exists to remove.
    ///
    /// The fixture is the real shape, not an invented one: a `static` declared
    /// inside a function body is minted as if it sat in the module, so two
    /// functions with a `RE` apiece mint one identity.
    #[tokio::test]
    async fn two_declarations_that_mint_one_identity_are_reported_and_not_silently_dropped() {
        let facts = walk_of(
            "collide",
            "src/collide.rs",
            "pub fn a() -> u32 { static RE: u32 = 1; RE }\n\
             pub fn b() -> u32 { static RE: u32 = 2; RE }",
        );
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "collide").await;

        let written = persist::write(&store, &folder, &facts).await.expect("the facts persist");

        assert_eq!(
            written.collisions,
            vec![persist::Collision {
                fqn: "rust·senseid·collide·RE·item".to_string(),
                declarations: 2,
            }],
            "the write must NAME the identity two declarations shared"
        );
        assert_eq!(written.symbols, facts.symbols.len(), "every symbol was still offered");

        let stored = persist::read_back(&store, &folder).await.expect("the rows read back");
        assert_eq!(
            stored.symbols.iter().filter(|s| s.name == "RE").count(),
            1,
            "one identity is one row — which is what makes the count above the loss"
        );
    }

    /// The bound on that loss over this repo's whole rust, so it cannot grow
    /// unnoticed while the grammar fix is scheduled.
    ///
    /// Needs no database: it is a property of the identities the WALK mints, and
    /// the reason it lives beside persistence is that persistence is where the
    /// consequence lands.
    #[tokio::test]
    async fn the_identities_this_repos_rust_cannot_keep_apart_are_a_known_and_bounded_set() {
        use std::collections::BTreeMap;

        let mut symbols = 0usize;
        let mut lost = 0usize;
        let mut named: Vec<String> = Vec::new();
        for (path, text) in crate::indexer::corpus_rust_sources() {
            let package = crate::indexer::package_of(&path);
            let module = crate::indexer::module_of(&path);
            let facts = rust::read(&Source {
                package: &package,
                module: &module,
                path: &path,
                text: &text,
            })
            .unwrap_or_else(|e| panic!("{path}: {e:?}"));
            let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
            for symbol in &facts.symbols {
                symbols += 1;
                *seen.entry(symbol.fqn.as_str()).or_default() += 1;
            }
            for (fqn, declarations) in seen {
                if declarations > 1 {
                    lost += declarations - 1;
                    named.push(format!("{fqn} x{declarations}"));
                }
            }
        }

        // The count and not the proportion: the total moves whenever anyone
        // edits any rust file in this repo, and a ratchet that drifts with
        // unrelated work is a ratchet nobody trusts.
        assert_eq!(
            lost,
            19,
            "a declaration inside a FUNCTION BODY is minted as if it sat in the module, so two \
             functions with a `static RE` apiece mint one identity — as do two `const _`, which \
             bind no name at all. Every one of the {} affected identities is a function-local \
             item or an anonymous const, out of {symbols} symbols:\n  {}\n\
             This is the fqn grammar's to fix (step 2/3), not persistence's: the walk mints the \
             identity and `nodes_unique_fqn` merely applies it. The ratchet is here because \
             persistence is where the consequence becomes a lost row.",
            named.len(),
            named.join("\n  ")
        );
    }

    /// A7, over the whole corpus and across files.
    ///
    /// The guard that lets spec §2.1 drop the type/value discriminator. The old
    /// trailing segment prevented ONE collision class — a type beside a
    /// same-named value — and said nothing about any other, so "no two
    /// declarations share an identity" was an assumption. This checks it, and a
    /// violation NAMES both sides instead of letting one silently overwrite the
    /// other.
    ///
    /// It is also an A6 obligation: which of two colliding declarations survives
    /// is decided by which file the scan reached last.
    ///
    /// Across files, not per file, because that is where the reach rule could
    /// have cost something — a module and a function sharing a name need not sit
    /// in one file.
    ///
    /// The assertion is the SET and not a count, so a new collision arrives
    /// spelled out. That is what makes the guard load-bearing: dropping the
    /// type/value distinction and leaving modules in the collapsed bucket adds
    /// exactly one entry here, `rust·sensei-bootstrap·config·item`, measured by
    /// doing it — which is what earned modules their own reach rather than
    /// having it asserted.
    ///
    /// The 15 below are all one defect and it is NOT the reach rule's: a
    /// declaration inside a FUNCTION BODY is minted as if it sat in the module,
    /// so two functions each holding a `static RE` mint one identity — as do a
    /// `const _`, which binds no name at all, and two crate roots of one package
    /// (`build.rs` beside `src/main.rs`, two integration-test files), which both
    /// reduce to the empty module path. Every entry is one of those three. They
    /// belong to the walk's identity rule, and this is the ratchet that keeps
    /// them from spreading while they wait.
    #[test]
    fn no_two_declarations_in_this_repos_rust_mint_one_identity() {
        use std::collections::{BTreeMap, BTreeSet};

        let mut sites: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut symbols = 0usize;
        for (path, text) in crate::indexer::corpus_rust_sources() {
            let package = crate::indexer::package_of(&path);
            let module = crate::indexer::module_of(&path);
            let facts = rust::read(&Source {
                package: &package,
                module: &module,
                path: &path,
                text: &text,
            })
            .unwrap_or_else(|e| panic!("{path}: {e:?}"));
            let file = path.rsplit("crates/").next().unwrap_or(&path).to_string();
            for symbol in &facts.symbols {
                symbols += 1;
                sites.entry(symbol.fqn.as_str().to_string()).or_default().insert(format!(
                    "{:?} {} at {file}:{}",
                    symbol.kind, symbol.name, symbol.span.start_line
                ));
            }
        }

        // Function-local items and anonymous consts, minted as if they sat in
        // the module; plus the two identities a package with more than one crate
        // root produces.
        let known = [
            "rust·senseid·adapters::manifest::gradle·RE·item",
            "rust·senseid·adapters::manifest::ruby·RE·item",
            "rust·senseid·adapters::manifest::swiftpm·RE·item",
            "rust·senseid·api::handlers::model_provisioning·provision_status·item",
            "rust·senseid·api::handlers::observatory·COPY_CAP·item",
            "rust·senseid·base_url·item",
            "rust·senseid·db::pg_store::metrics·PgStore·Row·item",
            "rust·senseid·dojo::client::tests·inbox·item",
            "rust·senseid·dojo::client::tests·session·item",
            "rust·senseid·main·item",
            "rust·senseid·run_limits·ABBR·item",
            "rust·senseid·tasks::handlers::embed·_·item",
            "rust·senseid·tasks::handlers::process::tests·ARMS·item",
            "rust·senseid·tasks::handlers::process::tests·status_of·item",
            "rust·senseid·tasks::handlers::publish_run::tests·session·item",
        ];

        let collided: Vec<&String> =
            sites.iter().filter(|(_, sites)| sites.len() > 1).map(|(fqn, _)| fqn).collect();
        let named = |fqns: &[&String]| -> String {
            fqns.iter()
                .map(|fqn| {
                    format!(
                        "{fqn}\n      {}",
                        sites[*fqn].iter().cloned().collect::<Vec<_>>().join("\n      ")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n  ")
        };
        let arrived: Vec<&String> =
            collided.iter().filter(|fqn| !known.contains(&fqn.as_str())).copied().collect();
        assert!(
            arrived.is_empty(),
            "{} identity/identities of {symbols} are NEWLY minted by more than one declaration, \
             so one overwrites the other and which one wins depends on scan order (A6, A7):\n  {}",
            arrived.len(),
            named(&arrived)
        );
        let gone: Vec<&str> =
            known.iter().filter(|fqn| !collided.iter().any(|c| c == fqn)).copied().collect();
        assert!(
            gone.is_empty(),
            "these no longer collide, so the list has stopped describing the corpus and would \
             hide the next real one: {gone:?}"
        );
    }

    /// Corpus files the round trip below keeps whatever the spread does.
    ///
    /// A spread reaches a COLLAPSING identity only by luck, and what the round
    /// trip needs from real files is exactly the shapes nobody writes into a
    /// fixture. `adapters/manifest/gradle.rs` declares three function-local
    /// `static RE`s; a declaration inside a function body is minted as if it sat
    /// in the module, so the three mint one identity and two rows are lost —
    /// the collapse `Written::collisions` exists to count.
    const ALWAYS_SAMPLED: &[&str] = &["adapters/manifest/gradle.rs"];

    /// Whether a corpus path is in the fixed sample the round trip below uses.
    ///
    /// FNV-1a spelled out rather than `DefaultHasher`, whose output std does not
    /// promise to keep stable between releases — a sample that silently re-draws
    /// itself on a toolchain upgrade is the positional bug again with a longer
    /// fuse.
    fn every_thirteenth(path: &str) -> bool {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in path.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
        hash.is_multiple_of(13)
    }

    /// The same round trip, over REAL FILES from this repository.
    ///
    /// A fixture proves persistence handles what the fixture's author thought
    /// of. This proves it handles what is there — repeated calls to one target
    /// from one function, two declarations of one name at one line, a file whose
    /// every reference misses. Those are the shapes that break a grouping, and
    /// none of them appear in a fixture anyone writes on purpose.
    ///
    /// Every file goes into ONE folder, so cross-file merges happen exactly as
    /// they will in a real scan.
    #[tokio::test]
    async fn no_fact_is_lost_when_real_files_from_this_repo_are_persisted() {
        let sources = crate::indexer::corpus_rust_sources();
        // Roughly a thirteenth of the corpus: a spread across crates and sizes
        // that is FIXED rather than randomly sampled, so a failure is
        // reproducible. Every file would be a truer measurement and a
        // several-minute test; the shapes this is here to catch — a repeated
        // call, two declarations at one line — appear many times over in a
        // thirteenth of it.
        //
        // Plus the files that carry the shape by construction — see
        // [`ALWAYS_SAMPLED`]. A spread reaches a collapsing identity only by
        // luck, and the luck ran out the moment a file was added to this
        // workspace.
        //
        // Selected by each PATH rather than by position. `step_by` was
        // positional, and the corpus is this workspace's own source, so adding
        // one file to it re-drew the whole sample from that file onward and
        // moved the count below — a failure with nothing wrong behind it.
        // Keyed on the path, adding a file can only add that file.
        let sample: Vec<&(String, String)> = sources
            .iter()
            .filter(|(path, _)| {
                every_thirteenth(path) || ALWAYS_SAMPLED.iter().any(|tail| path.ends_with(tail))
            })
            .collect();
        assert!(sample.len() > 25, "{} files is not a spread", sample.len());
        for tail in ALWAYS_SAMPLED {
            assert!(
                sample.iter().any(|(path, _)| path.ends_with(tail)),
                "{tail} is named as always sampled but is not in the corpus, so the shape it                  was named for is no longer exercised here"
            );
        }

        let first_party: BTreeSet<String> =
            sources.iter().map(|(path, _)| crate::indexer::package_of(path)).collect();
        let scanned = BTreeSet::new();
        let world = World { first_party: &first_party, scanned: &scanned };

        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "corpus").await;

        // One row per IDENTITY, last write winning — which is not this test
        // being lenient, it is what an fqn MEANS (spec §2) and what
        // `nodes_unique_fqn` enforces. The declarations that collapse are
        // counted below, so the leniency has a number attached to it.
        let mut symbols: std::collections::BTreeMap<String, persist::SymbolRow> =
            std::collections::BTreeMap::new();
        let mut references = Vec::new();
        let mut relations = Vec::new();
        let mut imports = Vec::new();
        let mut collisions = 0usize;
        for (path, text) in &sample {
            let package = crate::indexer::package_of(path);
            let module = crate::indexer::module_of(path);
            let facts = rust::read(&Source { package: &package, module: &module, path, text })
                .unwrap_or_else(|e| panic!("{path}: {e:?}"));
            let facts = resolve(facts, &rust::GRAMMAR, &world);
            let file = rust::file_fqn(&package, &module, path)
                .unwrap_or_else(|e| panic!("{path}: {e:?}"))
                .as_str()
                .to_string();

            for symbol in &facts.symbols {
                let row = persist::SymbolRow::of(symbol, path, facts.language);
                symbols.insert(row.fqn.clone(), row);
            }
            references.extend(facts.references.iter().map(persist::ReferenceRow::of));
            relations.extend(facts.relations.iter().map(persist::RelationRow::of));
            imports.extend(facts.imports.iter().map(|i| persist::ImportRow::of(i, &file)));

            let written = persist::write(&store, &folder, &facts)
                .await
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            collisions += written.collisions.iter().map(|c| c.declarations - 1).sum::<usize>();
            // Occurrences are keyed by file, so two FILES on one edge is no
            // longer a loss. Two of ONE file's groups on one row still would be
            // — see `EdgeCollision`. Asserted here rather than assumed, over
            // real files, because the count being zero is exactly what makes
            // "the loss cannot happen" a measurement instead of a claim.
            assert!(
                written.edge_collisions.is_empty(),
                "{path}: two of one file's edge groups landed on one row, so the second \
                 replaced the first's occurrences: {:?}",
                written.edge_collisions
            );
        }
        assert_eq!(
            collisions, 2,
            "the sample's share of the bounded set the ratchet above names — the two rows the \
             three `static RE`s in the always-sampled `gradle.rs` collapse into one"
        );

        let stored = persist::read_back(&store, &folder).await.expect("the rows read back");
        same_rows(stored.symbols, symbols.into_values().collect(), "symbols");
        same_rows(stored.references, references, "references");
        same_rows(stored.relations, relations, "relations");
        same_rows(stored.imports, imports, "imports");
    }

    /// The reason grouping exists. An edge's identity is a RELATIONSHIP, so two
    /// calls to one function from one function are ONE row — and both spans have
    /// to be on it, because `props = props || EXCLUDED.props` REPLACES a key
    /// rather than appending, so two separate writes would leave only the last.
    #[tokio::test]
    async fn two_calls_to_one_target_from_one_symbol_are_one_edge_carrying_both() {
        let facts = walk_of(
            "twice",
            "src/twice.rs",
            "pub fn once() -> u32 { 1 }\npub fn caller() -> u32 { once() + once() }",
        );
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "twice").await;
        persist::write(&store, &folder, &facts).await.expect("the facts persist");

        let edges: Vec<_> = store
            .v2_edges(&folder)
            .await
            .expect("the edges read back")
            .into_iter()
            .filter(|e| e.target_fqn.as_deref() == Some("rust·senseid·twice·once·item"))
            .collect();
        assert_eq!(edges.len(), 1, "one relationship, one row");

        let stored = persist::read_back(&store, &folder).await.expect("the rows read back");
        let spans: Vec<_> = ordered(
            stored
                .references
                .iter()
                .filter(|r| {
                    r.target
                        == persist::TargetRow::Resolved("rust·senseid·twice·once·item".to_string())
                })
                .map(|r| (r.at.start_line, r.at.start_col))
                .collect(),
        );
        assert_eq!(spans, vec![(2, 25), (2, 34)], "both call sites survive on the one row");
    }

    /// The same property ACROSS FILES, which grouping inside one write cannot
    /// give.
    ///
    /// `edge_rows_of` groups per file, so one write emits one complete
    /// occurrence list — but an edge's identity is `(folder, source, target,
    /// kind)`, which two files can both produce. The second write then merges
    /// with `props = props || EXCLUDED.props`, and jsonb `||` REPLACES the
    /// `occurrences` key wholesale, so the first file's occurrences disappear
    /// with nothing counting them.
    ///
    /// This is not hypothetical: `crates/mcp` has both a `lib.rs` and a
    /// `main.rs`, both reduce to the crate-root module path, and the imports of
    /// each hang off that one file identity. Two walked, one stored.
    #[tokio::test]
    async fn two_files_that_emit_one_edge_each_keep_their_own_occurrences() {
        let first =
            walk_of("shared", "src/first.rs", "use std::collections::BTreeMap;\npub fn a() {}");
        let second = walk_of(
            "shared",
            "src/second.rs",
            "\n\nuse std::collections::BTreeMap;\npub fn b() {}",
        );
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "two_files").await;

        persist::write(&store, &folder, &first).await.expect("the first persists");
        persist::write(&store, &folder, &second).await.expect("the second persists");

        let stored = persist::read_back(&store, &folder).await.expect("the rows read back");
        let lines: Vec<u32> = ordered(
            stored
                .imports
                .iter()
                .filter(|i| i.path == "std::collections::BTreeMap")
                .map(|i| i.at.start_line)
                .collect(),
        );
        assert_eq!(
            lines,
            vec![1, 3],
            "both files import it, so both occurrences must be on the one edge; one file's \
             write must not erase the other's"
        );
    }

    /// The identity a file hangs its file-scope facts off must name the FILE.
    ///
    /// `crates/mcp` has a `lib.rs` beside a `main.rs`. Both are crate roots, so
    /// both reduce to the empty module path, so both mint one file identity —
    /// and every import and file-scope use site of the two lands on one node, as
    /// if one file had written them. Two files are not one symbol (spec §2), and
    /// the collision cannot be left silent.
    #[tokio::test]
    async fn two_crate_roots_of_one_package_do_not_share_one_file_identity() {
        let lib = walk_of("", "src/lib.rs", "use std::collections::BTreeMap;\npub fn shared() {}");
        let main = walk_of("", "src/main.rs", "use serde_json::Value;\npub fn main() {}");
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "two_roots").await;

        persist::write(&store, &folder, &lib).await.expect("the lib root persists");
        persist::write(&store, &folder, &main).await.expect("the bin root persists");

        let stored = persist::read_back(&store, &folder).await.expect("the rows read back");
        let owners: Vec<String> = ordered(
            stored.imports.iter().map(|i| format!("{} imports {}", i.from, i.path)).collect(),
        );
        assert_eq!(
            owners,
            vec![
                "rust·senseid·crate·lib·mod imports std::collections::BTreeMap".to_string(),
                "rust·senseid·crate·main·mod imports serde_json::Value".to_string(),
            ],
            "the two crate roots are two files and must be two identities, or every file-scope \
             fact of one is filed under the other. `crate` is a reserved word, so neither can \
             collide with a `mod` declaration"
        );
    }

    /// The other half of the same rule: a RE-SCAN of one file replaces that
    /// file's occurrences instead of accumulating them.
    ///
    /// Accumulating would be the obvious way to fix the test above and it would
    /// be wrong — a span that has since moved would stay on the edge forever,
    /// and the list would grow by one copy per scan. Both properties hold
    /// because occurrences are keyed BY FILE.
    #[tokio::test]
    async fn re_indexing_one_file_replaces_its_occurrences_rather_than_appending() {
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "rescan").await;

        let before =
            walk_of("rescan", "src/rescan.rs", "pub fn once() {}\npub fn caller() { once(); }");
        persist::write(&store, &folder, &before).await.expect("the first scan");
        // The same file, with the call one line further down.
        let after =
            walk_of("rescan", "src/rescan.rs", "pub fn once() {}\n\npub fn caller() { once(); }");
        persist::write(&store, &folder, &after).await.expect("the re-scan");

        let stored = persist::read_back(&store, &folder).await.expect("the rows read back");
        let lines: Vec<u32> = ordered(
            stored
                .references
                .iter()
                .filter(|r| {
                    r.target
                        == persist::TargetRow::Resolved("rust·senseid·rescan·once·item".to_string())
                })
                .map(|r| r.at.start_line)
                .collect(),
        );
        assert_eq!(
            lines,
            vec![3],
            "the re-scan must replace this file's occurrences; a span that has moved must not \
             survive as a second one"
        );
    }

    /// The merge contract (spec §2) through the persistence layer. A reference
    /// minted before the definition exists must land on the SAME node the
    /// definition later enriches; a second node would split one symbol in two
    /// and every query would see half of it.
    ///
    /// Two files, definition SECOND, which is the order that breaks a
    /// naive implementation.
    #[tokio::test]
    async fn a_definition_arriving_after_a_reference_merges_onto_the_same_row() {
        let caller = "pub fn ring() -> u32 { crate::bell::toll() }";
        let callee = "pub fn toll() -> u32 { 1 }";
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "merge").await;

        persist::write(&store, &folder, &walk_of("caller", "src/caller.rs", caller))
            .await
            .expect("the caller persists");
        let after_reference = store
            .node_id_by_fqn(&folder, "rust·senseid·bell·toll·item")
            .await
            .expect("the lookup runs")
            .expect("a reference to a not-yet-indexed definition creates the node it names");

        // The stub's kind, before any declaration has been read. `toll` is
        // reached by a CALL, and the tempting answer is therefore `function` —
        // which is the inference spec §2.1 forbids, and which would be right
        // here and wrong for every enum variant. `item` states no kind, so the
        // stub states none either.
        async fn kind_of(store: &PgStore, folder: &uuid::Uuid, what: &str) -> String {
            let row: (String,) = sqlx_core::query_as::query_as(
                "SELECT kind::text FROM sensei.nodes WHERE folder_id = $1 AND fqn = $2",
            )
            .bind(folder)
            .bind("rust·senseid·bell·toll·item")
            .fetch_one(store.pool())
            .await
            .unwrap_or_else(|e| panic!("the kind {what} the definition: {e}"));
            row.0
        }
        assert_eq!(
            kind_of(&store, &folder, "before").await,
            "parameter",
            "an `item` stub must not claim a kind the identity does not state; `parameter` is \
             the placeholder v2 never mints for a declaration (D2), so it cannot be read as one"
        );

        persist::write(&store, &folder, &walk_of("bell", "src/bell.rs", callee))
            .await
            .expect("the callee persists");
        let after_definition = store
            .node_id_by_fqn(&folder, "rust·senseid·bell·toll·item")
            .await
            .expect("the lookup runs")
            .expect("the definition must land on a node");

        assert_eq!(
            after_reference, after_definition,
            "the definition merged onto the reference's node instead of creating a second"
        );
        assert_eq!(
            kind_of(&store, &folder, "after").await,
            "function",
            "the real kind arrives with the declaration and replaces the placeholder"
        );

        let rows: (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id = $1 AND name = 'toll'",
        )
        .bind(folder)
        .fetch_one(store.pool())
        .await
        .expect("the count runs");
        assert_eq!(rows.0, 1, "one symbol, one row");

        // And the merge is an ENRICHMENT, not a rename: the stub had no file,
        // the merged row has one and carries what the walk read there.
        let stored = persist::read_back(&store, &folder).await.expect("the rows read back");
        let toll = stored
            .symbols
            .iter()
            .find(|s| s.fqn == "rust·senseid·bell·toll·item")
            .expect("the merged node must read back as a definition");
        assert_eq!(toll.file_path, "src/bell.rs");
        assert_eq!(toll.declared_type, DeclaredType::Stated("u32".to_string()));
    }

    /// `nodes_unique_identity` is `(folder_id, file_path, kind, name, parent_id,
    /// line_start)` with NULLS NOT DISTINCT, so `parent_id` is the ONLY column
    /// that separates two same-named members of two different types written on
    /// one line.
    ///
    /// Written with `parent_id = NULL` they are one row, and the loss is
    /// invisible from above: the two fqns are DISTINCT, so
    /// [`persist::Written::collisions`] counts nothing, and the second insert
    /// falls into `adopt_node_by_identity`, which re-points the first row's fqn
    /// at the second. One declaration is gone and every reference to it lands
    /// on the other type's member — a wrong edge (R4), not a missing one.
    ///
    /// One line is not a contrivance for its own sake; it is the smallest thing
    /// that isolates `parent_id`. Two members on two lines are separated by
    /// `line_start` and would pass whatever `parent_id` held.
    #[tokio::test]
    async fn two_same_named_members_of_two_types_on_one_line_are_two_rows() {
        let facts = walk_of(
            "twins",
            "src/twins.rs",
            "pub struct A { pub v: u32 } pub struct B { pub v: u32 }",
        );
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "twins").await;

        let written = persist::write(&store, &folder, &facts).await.expect("the facts persist");
        assert_eq!(
            written.collisions,
            vec![],
            "the two fields mint two identities, so nothing above the database sees a collapse"
        );

        let stored = persist::read_back(&store, &folder).await.expect("the rows read back");
        let fields: BTreeSet<&str> =
            stored.symbols.iter().filter(|s| s.name == "v").map(|s| s.fqn.as_str()).collect();
        assert_eq!(
            fields,
            ["rust·senseid·twins·A·v·field", "rust·senseid·twins·B·v·field"]
                .into_iter()
                .collect::<BTreeSet<&str>>(),
            "`A::v` and `B::v` are two declarations; with a null parent_id the identity index \
             keyed them as one row and one of them was silently re-pointed at the other"
        );
    }

    /// The `parent_id` column, read back by IDENTITY and compared to the
    /// ownership relations the walk read.
    ///
    /// The test above proves the column is doing its job at the identity index;
    /// this proves it holds the right value. Both are needed: a `parent_id` set
    /// to any old node would separate `A::v` from `B::v` just as well and would
    /// still be a containment the source does not state.
    ///
    /// Every `Owns` the walk produced, not a chosen sample, so a member kind
    /// that stops being parented fails here.
    #[tokio::test]
    async fn every_ownership_relation_the_walk_read_is_a_parent_id_in_the_database() {
        use crate::indexer::facts::Resolution;

        let facts = walked();
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "containment").await;
        persist::write(&store, &folder, &facts).await.expect("the facts persist");

        let containment: std::collections::BTreeMap<String, Option<String>> = store
            .v2_containment(&folder)
            .await
            .expect("the containment reads back")
            .into_iter()
            .collect();

        let owned: Vec<(&str, &str)> = facts
            .relations
            .iter()
            .filter(|r| r.kind == RelationKind::Owns)
            .filter_map(|r| match &r.parent {
                Resolution::Resolved(parent) => Some((r.child.as_str(), parent.as_str())),
                Resolution::Unresolved { .. } => None,
            })
            .collect();
        assert!(
            owned.len() >= 5,
            "the fixture must actually own some members, or this asserts nothing: {owned:?}"
        );

        for (child, parent) in &owned {
            assert_eq!(
                containment.get(*child).map(Option::as_deref),
                Some(Some(*parent)),
                "{child} is a member of {parent} in the facts, and `nodes.parent_id` must say so"
            );
        }

        // And the other way: a free item is a member of NOTHING, so the column
        // is not simply being filled with whatever was in hand.
        let free = "rust·senseid·gadget·widest·item";
        assert_eq!(
            containment.get(free).map(Option::as_deref),
            Some(None),
            "{free} is declared at file scope and is owned by no type"
        );
    }

    /// Every place a reference to an EXTERNAL puts the package, read back.
    ///
    /// `target_ref_of` derives one string — the package segment of a `lib·` fqn
    /// — and that one string reaches four writes: the container's `fqn`, the
    /// container's `name`, `props.package` on the container, and
    /// `props.package` on the symbol. Nothing read any of them, so inverting the
    /// derivation (taking the member's name, or the fqn's first segment) left
    /// the whole suite green while `list_dependencies` — which answers "what
    /// does this repo depend on" off `lib_package.name` — reported nonsense.
    ///
    /// Asserted as WHOLE ROWS rather than field by field, so a fifth write
    /// appearing on either row has to be looked at rather than skipped past.
    #[tokio::test]
    async fn a_reference_to_an_external_names_its_package_in_all_four_places() {
        use crate::db::pg_store::LibColumns;

        let facts = walked();
        let store = PgStore::connect_test().await.expect("the test database must be reachable");
        let folder = a_folder(&store, "externals").await;
        persist::write(&store, &folder, &facts).await.expect("the facts persist");

        let lib = store.v2_lib_nodes(&folder).await.expect("the external rows read back");
        let container = lib
            .iter()
            .find(|row| row.kind == "lib_package")
            .expect("an external reference mints a package container");
        assert_eq!(
            *container,
            LibColumns {
                fqn: "lib·std".to_string(),
                kind: "lib_package".to_string(),
                name: "std".to_string(),
                package: Some("std".to_string()),
                parent_fqn: None,
            },
            "three of the four package writes are on the container: its fqn, its name and its \
             props.package"
        );

        // The fixture's only external is `std::collections::BTreeMap`, brought
        // in by a `use` and then constructed — so both an import edge and a
        // construction edge reach the same symbol.
        let symbol = lib
            .iter()
            .find(|row| row.kind == "lib_symbol" && row.name == "BTreeMap")
            .unwrap_or_else(|| panic!("the fixture's external symbol reached no row: {lib:?}"));
        assert_eq!(
            *symbol,
            LibColumns {
                fqn: "lib·std·collections::BTreeMap".to_string(),
                kind: "lib_symbol".to_string(),
                name: "BTreeMap".to_string(),
                package: Some("std".to_string()),
                parent_fqn: Some("lib·std".to_string()),
            },
            "the fourth write is props.package on the symbol, and the symbol hangs under the \
             container the same package minted"
        );
    }
}
