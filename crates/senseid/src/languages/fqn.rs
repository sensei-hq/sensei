//! Language-agnostic FQN (fully-qualified name) grammar — the SCIP/LSIF *moniker*
//! encoded to one stable string (plan 0.1). Every definition AND every reference
//! computes its FQN through these builders so the two sides agree and
//! `upsert_node_by_fqn` merges them onto one node. This module is pure and shared;
//! only the per-language *name resolution* that feeds it (which module, which
//! enclosing type, which trait) is language-specific.
//!
//! Grammar (`·` = U+00B7 MIDDLE DOT — no Rust/TS/Python identifier can contain it):
//!   - free fn / const / type def : `<lang>·<package>·<module>·<name>`
//!   - inherent method / assoc fn : `<lang>·<package>·<module>·<Type>·<member>`
//!   - trait-impl method          : `<lang>·<package>·<module>·<Type>·<Trait>·<member>`
//!     (the trait qualifier disambiguates `Display::fmt` vs `Debug::fmt` on one type)
//!   - external lib symbol        : `lib·<package>·<path>·<member>`
//!
//! `module` is a single segment that may itself contain `::` (the crate-relative
//! module chain, e.g. `api::handlers::codebase`) and may be empty (crate root);
//! empty segments are dropped when encoding.

use crate::types::SymbolKind;

/// FQN segment separator — U+00B7 MIDDLE DOT.
pub const SEP: char = '·';

/// A definition carrying its canonical FQN — the language-agnostic shape every
/// per-language producer emits. Phase 3 turns each into an `upsert_node_by_fqn`
/// definition (enrich) call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FqnDefinition {
    pub fqn: String,
    pub name: String,
    pub kind: SymbolKind,
    pub line_start: u32,
    pub line_end: u32,
    pub is_exported: bool,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    /// Enclosing type name for a method (e.g. `Widget` for `Widget::new`).
    pub parent_type: Option<String>,
    /// FQN of this def's STRUCTURAL parent (D5c): a method's enclosing TYPE, so
    /// the emit path nests it under the type node (not the flat file node). `None`
    /// for a top-level item, which nests under the file's module container.
    pub parent_fqn: Option<String>,
    /// The declared return type, verbatim as written (`&crate::db::PgStore`,
    /// `Arc<PgStore>`, `Result<T, E>`). `None` for a function returning unit, or
    /// a language/kind where the notion does not apply.
    ///
    /// Carried here because THIS is the pass that mints the node. The type was
    /// already being extracted in a different pass over the same file and then
    /// dropped, so the graph knew every function's name, signature and parent
    /// but not what it returned — and that one absent field is what blocks
    /// transitive receiver resolution: `ctx.pg().method()` needs the type of
    /// `pg`'s return value to know which `method` is being called. Every other
    /// hop in that chain is already a key we can mint.
    ///
    /// Kept VERBATIM on purpose. Normalising to a bare type name here would
    /// discard the module path that says which `PgStore` is meant; unwrapping
    /// `Arc`/`Result`/`Option` is the resolver's job, and `base_type_name`
    /// already owns that rule.
    pub return_type: Option<String>,
}

/// What a call site SAW of its receiver, expressed as a graph KEY.
///
/// A method call is the one reference shape whose target no import can name:
/// measured on the live graph, all 28,069 unresolved rust `calls` edges have a
/// lowercase `target_name`. The producer had the receiver in hand at the moment
/// it gave up and threw it away, so nothing downstream could finish the job.
/// This carries it instead.
///
/// The payload is an FQN on purpose: a later resolver needs the node table and
/// nothing else — no file path, no re-parse, no second pass over source.
///
/// There is deliberately no "probably" variant. A receiver whose type the file
/// cannot name carries NO hint and the reference stays unresolved, because a
/// plausible key the resolver cannot tell apart from a real one is exactly how
/// ghost nodes get minted.
///
/// ONE variant, because only one shape is ever storable. Props are stamped only
/// on an UNRESOLVED call (see `process.rs`), and the two producer arms that know
/// the receiver's type outright — `self` inside an `impl`, a receiver bound to a
/// first-party type — are the same two arms that already mint a target. A
/// "receiver type" hint therefore attached only to calls that carried an fqn,
/// and every one was computed and dropped on the floor.
///
/// Deleted rather than wired up. What it would buy is real — 2,142 rust `calls`
/// edges in the live `sensei` folder point at a STUB (a target node with no
/// file path), and the receiver's type is a second way to reach the definition
/// those stubs stand in for. What it would cost is re-pointing calls that are
/// ALREADY correctly resolved, which is a strictly worse failure than a stub:
/// the edge would move only when the second lookup disagreed with the first,
/// and nothing here can say which of the two was right. Reaching those 2,142
/// wants a lookup that cannot wrong-merge, not a hint that can.
///
/// Reach, measured by replaying the rust producer over this repo's own 364 rust
/// files with each file's real package and module (30,051 references emitted,
/// 17,067 resolved / 12,984 not): 599 references gain a hint, and every one of
/// them is unresolved — that is the new reach, references that had nothing but a
/// bare method name. 12,385 unresolved references still carry none.
///
/// Every unresolved rust reference is a member call — the `field_expression`
/// arm is the only one that yields an unresolved target — so that last bucket
/// is entirely receivers this file cannot name: a chain deeper than one link
/// (`a.b().c().d()`, where the inner hop is itself unresolved and so has no key
/// to hand on) or an identifier the binding map never learned a type for
/// (`let x = ctx.pg();`). Both are the same defect one level down and they
/// unlock together: once a resolver can turn a hint into a concrete type,
/// feeding that type back into the binding map is what reaches them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiverHint {
    /// The receiver is the value another call returned — this is THAT call's
    /// target FQN (`rust·senseid·tasks::executor·TaskContext·pg`). The resolver
    /// reads the declared return type off that node ([`FqnDefinition::return_type`])
    /// to learn the receiver's type, then finds the member under it.
    ReturnOf(String),
}

impl ReceiverHint {
    /// The `edges.props` key this variant is stored under.
    ///
    /// A call is emitted in one place and completed in another — the producer
    /// has the receiver, the resolver has the graph — so the key name lives
    /// HERE, next to the variant, rather than as string literals that can drift
    /// apart across `process.rs` and `pg_store::graph`.
    const PROPS_KEY: &'static str = "receiver_return_of";

    /// Encode for `edges.props`. Only ever stamped on an UNRESOLVED call — a
    /// resolved edge already names its target, and recording the receiver
    /// alongside it would be a second answer to a question already answered.
    pub fn to_props(&self) -> serde_json::Value {
        let Self::ReturnOf(fqn) = self;
        serde_json::json!({ Self::PROPS_KEY: fqn })
    }

    /// Decode from `edges.props`. `None` when the edge carries no hint, which
    /// is every call emitted before this existed and every receiver the
    /// producer could not name — both stay unresolved, as they are.
    pub fn from_props(props: &serde_json::Value) -> Option<Self> {
        props.get(Self::PROPS_KEY).and_then(|v| v.as_str()).map(|f| Self::ReturnOf(f.to_string()))
    }
}

/// A reference (call-site) resolved to a target FQN. `target_fqn = None` means the
/// producer could not resolve the target to a concrete symbol (e.g. a method call
/// on a receiver whose type is out of the bounded binding→type scope, plan 0.7) —
/// deliberately NOT guessed, so it never wrong-merges. `target_name` (the bare
/// last-segment) is kept for the language-scoped bare-name fallback during the
/// transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FqnReference {
    pub caller_fqn: String,
    pub caller_line: u32,
    pub target_fqn: Option<String>,
    pub target_name: String,
    /// True when the target resolves to an external dependency (`lib·…`).
    pub is_lib: bool,
    /// What the call site saw of the receiver, when it saw something nameable.
    /// `None` for every non-member call, for an external receiver type (no
    /// definition in this graph to read a return type from), and for a receiver
    /// the file genuinely cannot place. Inert to target matching — the emit path
    /// still branches on `target_fqn`/`is_lib` alone.
    pub receiver: Option<ReceiverHint>,
}

/// One type's relation to a supertype, resolved the same way a call target is.
///
/// `parent_fqn = None` means the producer could not resolve the supertype and
/// deliberately did NOT guess — the emit path stores `target_name` only, which
/// is the truthful unresolved shape. Guessing here is how a bare name gets
/// matched to a same-named type in another language or of another kind: a
/// confident wrong answer, which is worse than an unresolved one.
///
/// `is_lib` splits the emit path exactly as it does for [`FqnReference`]: an
/// external supertype becomes a `lib·` node (so "which of our models extend
/// `pydantic.BaseModel`" is answerable), an internal one a graph node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRelation {
    /// FQN of the subtype — the type that declares the relation.
    pub child_fqn: String,
    /// FQN of the supertype, or `None` when unresolvable.
    pub parent_fqn: Option<String>,
    /// Bare last segment of the supertype, always present.
    pub parent_name: String,
    /// True when the supertype resolves to an external dependency (`lib·…`).
    pub is_lib: bool,
    pub relation: crate::types::RelationKind,
}

/// Output of a per-language FQN producer over one file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FqnFileOutput {
    pub defs: Vec<FqnDefinition>,
    pub refs: Vec<FqnReference>,
    /// The owning crate/package (for the D5c module container's fqn).
    pub package: String,
    /// This file's crate-relative module path (empty at the crate root). The emit
    /// path materialises a `module` container node for it, nested under the file.
    pub module: String,
    /// Inheritance facts declared in this file. Empty for a language with no
    /// inheritance producer yet — distinct from a language that HAS one and
    /// found none, which is why the capability is declared separately rather
    /// than inferred from this being empty.
    pub relations: Vec<TypeRelation>,
}

/// Per-file context a producer needs: the owning crate/package name (from the
/// nearest manifest, supplied by the Phase-3 processor) and this file's
/// crate-relative module path (e.g. `widget` or `api::handlers::codebase`; empty
/// for the crate root).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFqnContext {
    pub package: String,
    pub module: String,
}

/// Join non-empty segments with [`SEP`]. Empty segments (e.g. a crate-root
/// module) are dropped so the encoding never emits a doubled separator.
fn encode(segments: &[&str]) -> String {
    let mut out = String::new();
    for seg in segments {
        if seg.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(SEP);
        }
        out.push_str(seg);
    }
    out
}

/// Free function, constant, or type definition (struct/enum/trait/type alias):
/// `<lang>·<package>·<module>·<name>`.
pub fn item(lang: &str, package: &str, module: &str, name: &str) -> String {
    encode(&[lang, package, module, name])
}

/// Inherent method or associated function: `<lang>·<package>·<module>·<Type>·<member>`.
/// `module` is the TYPE's canonical module (the anchoring rule, plan 0.1), not the
/// file the `impl` block lives in.
pub fn method(lang: &str, package: &str, module: &str, ty: &str, member: &str) -> String {
    encode(&[lang, package, module, ty, member])
}

/// Trait-impl method: `<lang>·<package>·<module>·<Type>·<Trait>·<member>`. The
/// trait qualifier keeps `Display::fmt` and `Debug::fmt` on the same type distinct.
pub fn trait_method(
    lang: &str,
    package: &str,
    module: &str,
    ty: &str,
    tr: &str,
    member: &str,
) -> String {
    encode(&[lang, package, module, ty, tr, member])
}

/// External (dependency) symbol: `lib·<package>·<path>·<member>`. `member` may be
/// empty for a bare-crate reference.
pub fn lib(package: &str, path: &str, member: &str) -> String {
    encode(&["lib", package, path, member])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sep_is_middot() {
        assert_eq!(SEP, '\u{00B7}');
        // No common identifier char equals it.
        assert_ne!(SEP, '.');
        assert_ne!(SEP, ':');
    }

    #[test]
    fn item_free_fn() {
        assert_eq!(item("rust", "senseid", "widget", "make"), "rust·senseid·widget·make");
    }

    /// The hint is written by the emit path and read by the resolver, in
    /// different modules, days apart in the pipeline. What it carries is a
    /// METHOD fqn to read a return type from, and the key name is the contract
    /// that keeps the two ends agreeing across that gap.
    #[test]
    fn a_receiver_hint_round_trips_through_edge_props() {
        let hint = ReceiverHint::ReturnOf("rust·senseid·tasks::executor·TaskContext·pg".into());
        assert_eq!(
            ReceiverHint::from_props(&hint.to_props()),
            Some(hint.clone()),
            "{hint:?} must survive the round trip through edges.props"
        );
        assert_eq!(
            ReceiverHint::from_props(&serde_json::json!({})),
            None,
            "an edge with no hint yields no hint — never a defaulted one"
        );
        assert_eq!(
            ReceiverHint::from_props(&serde_json::json!({ "relation": "trait_impl" })),
            None,
            "another writer's props key is not a receiver hint"
        );
    }

    #[test]
    fn item_crate_root_drops_empty_module() {
        // src/main.rs (crate root) → no module segment, no doubled separator.
        assert_eq!(item("rust", "sensei-cli", "", "main"), "rust·sensei-cli·main");
    }

    #[test]
    fn item_nested_module_is_one_segment() {
        // The crate-relative module chain stays `::`-joined inside a single segment.
        assert_eq!(
            item("rust", "senseid", "api::handlers::codebase", "language_for_ext"),
            "rust·senseid·api::handlers::codebase·language_for_ext"
        );
    }

    #[test]
    fn inherent_method() {
        assert_eq!(
            method("rust", "senseid", "widget", "Widget", "new"),
            "rust·senseid·widget·Widget·new"
        );
    }

    #[test]
    fn trait_method_carries_trait_qualifier() {
        assert_eq!(
            trait_method("rust", "senseid", "fmtmod", "Foo", "Display", "fmt"),
            "rust·senseid·fmtmod·Foo·Display·fmt"
        );
        // Same type + member, different trait → distinct FQN (the disambiguation).
        assert_ne!(
            trait_method("rust", "senseid", "fmtmod", "Foo", "Display", "fmt"),
            trait_method("rust", "senseid", "fmtmod", "Foo", "Debug", "fmt")
        );
    }

    #[test]
    fn lib_symbol() {
        assert_eq!(
            lib("serde_json", "serde_json", "from_str"),
            "lib·serde_json·serde_json·from_str"
        );
    }
}

/// Finders every language's producer tests need over an [`FqnFileOutput`].
///
/// One definition, because there were FIVE byte-identical copies of `def_fqn`
/// and `ref_to` — typescript, java, python, rust_lang, sql — and kotlin had
/// NEITHER, which is the gap a kotlin producer test hits first. There was no
/// relation finder at all: the same
/// `out.relations.iter().find(|r| r.parent_name == n)` closure was hand-written
/// three times, in java twice and python once.
///
/// `#[cfg(test)]`, so none of this is reachable from a shipped path.
#[cfg(test)]
pub(crate) mod finders {
    use super::{FqnDefinition, FqnFileOutput, FqnReference, TypeRelation};

    /// The whole definition by name, for asserting on fields other than the fqn.
    /// Panics with the def list, because a missing def usually means the producer
    /// emitted nothing and the list is what tells you what it did emit.
    pub(crate) fn def_of<'a>(out: &'a FqnFileOutput, name: &str) -> &'a FqnDefinition {
        out.defs
            .iter()
            .find(|d| d.name == name)
            .unwrap_or_else(|| panic!("no def named `{name}` in {:?}", out.defs))
    }

    /// A definition's fqn by name, or `"<no-def>"` — a sentinel rather than a
    /// panic, so a test can assert a definition is ABSENT without catching.
    pub(crate) fn def_fqn<'a>(out: &'a FqnFileOutput, name: &str) -> &'a str {
        out.defs.iter().find(|d| d.name == name).map(|d| d.fqn.as_str()).unwrap_or("<no-def>")
    }

    /// A reference by target name. Panics with the whole ref list, because a
    /// missing ref is almost always a producer that emitted nothing and the list
    /// is what tells you which.
    pub(crate) fn ref_to<'a>(out: &'a FqnFileOutput, target_name: &str) -> &'a FqnReference {
        out.refs
            .iter()
            .find(|r| r.target_name == target_name)
            .unwrap_or_else(|| panic!("no ref to `{target_name}` in {:?}", out.refs))
    }

    /// A relation by SUPERTYPE name — the missing third finder.
    pub(crate) fn rel_to<'a>(out: &'a FqnFileOutput, parent_name: &str) -> &'a TypeRelation {
        out.relations
            .iter()
            .find(|r| r.parent_name == parent_name)
            .unwrap_or_else(|| panic!("no relation to `{parent_name}` in {:?}", out.relations))
    }
}
