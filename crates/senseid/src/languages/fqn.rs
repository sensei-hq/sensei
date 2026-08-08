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
        assert_eq!(method("rust", "senseid", "widget", "Widget", "new"), "rust·senseid·widget·Widget·new");
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
        assert_eq!(lib("serde_json", "serde_json", "from_str"), "lib·serde_json·serde_json·from_str");
    }
}
