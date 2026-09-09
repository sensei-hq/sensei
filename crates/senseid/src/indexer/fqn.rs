//! Indexer v2 — the one place an FQN string is built (spec §2, R7).
//!
//! An FQN is a LOOKUP KEY, minted independently by a reference and by a
//! definition. If the two sides mint different strings for one symbol they
//! never merge and the graph holds two half-symbols instead of one, so the
//! grammar lives here and only here. [`define`] and [`refer`] are the two
//! doors; they are separate so that a special case can only ever be added to
//! one of them, and `the_definition_side_and_the_reference_side_mint_byte_identical_fqns`
//! is what notices when someone does.
//!
//! Grammar (`·` = U+00B7 MIDDLE DOT, which no identifier in a supported grammar
//! can contain, so a segment boundary is never ambiguous):
//!
//! | form | encoding |
//! |---|---|
//! | [`Form::Item`] | `<lang>·<package>·<module>·<name>·<reach>` |
//! | [`Form::Member`] | `<lang>·<package>·<module>·<Type>·<member>·<reach>` |
//! | [`Form::TraitMember`] | `<lang>·<package>·<module>·<Type>·<Trait>·<member>·<reach>` |
//! | [`Form::Lib`] | `lib·<package>·<member>` |
//!
//! The trailing [`Reach`] is what keeps two declarations Rust allows to share a
//! name from merging onto one node — see [`Reach`] for why the spec §2 sketch,
//! which has no such segment, is not enough. It goes last so that a prefix
//! query still gathers every member of a type regardless of reach.
//!
//! `module` is ONE segment that may itself contain `::` (`api::handlers::codebase`)
//! and may be empty at the package root; an empty `module` is dropped rather
//! than left as a doubled separator. So is an empty `Lib` member, which is how a
//! bare crate reference is spelled. Every other segment is required, and a
//! builder handed an empty one returns [`FqnError::EmptySegment`] instead of a
//! shorter string that means a different symbol.
//!
//! Externals are named, never opened (R5): a library member is
//! `lib·<package>·<member>` and no dependency source is parsed to produce it.
//
// This module has no caller on purpose — see the note in `mod.rs`.
#![allow(dead_code)]

use super::facts::{Fqn, Language};

/// FQN segment separator — U+00B7 MIDDLE DOT.
///
/// Private, so the compiler — not a convention — stops another module from
/// assembling an fqn out of parts.
const SEP: char = '·';

/// The leading segment of an external symbol, standing where a language would
/// be. An external has no language of ours because we never open it (R5).
const LIB: &str = "lib";

/// Which segment of the grammar a build or parse complaint is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Segment {
    Package,
    Module,
    Type,
    Trait,
    /// An item's name, a type's member, a library's member.
    Member,
}

/// HOW a use site gets to a name — the last segment of every local form
/// (spec §2.1, D7).
///
/// A key needs a trailing discriminator because Rust permits two declarations
/// to share one name in one scope: a `healthy` field beside a `healthy()`
/// getter, `pub mod config;` beside `pub fn config()`. Both are in this repo
/// today. The fqn is the merge key (spec §2), so without this segment those two
/// mint one string, one overwrites the other, and every reference to either
/// lands on the wrong one about half the time — a wrong edge, which is worse
/// than a missing one (R4).
///
/// The discriminator is the REACH and not the Rust NAMESPACE, and the
/// difference is forced rather than stylistic. `Foo::Bar` is spelled
/// identically whether `Bar` is an enum variant, an associated const, an
/// associated type or a type inside a module, so at a path leaf the namespace
/// is not observable. Worse for any repair on the reference side: one
/// declaration can occupy BOTH namespaces — the Rust Reference files an enum
/// variant and a unit struct under the type namespace and their constructors
/// under the value namespace — so there is no single true namespace for the
/// DECLARATION side to mint either.
///
/// Each value below is decided by something the declaration side and the use
/// site can both observe, so a use site mints exactly ONE candidate and there
/// is no ambiguity for a resolver to adjudicate. What the collapse gives up —
/// a type and a function sharing a name in one scope — is not argued away, it
/// is CHECKED: see A7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reach {
    /// Anything reached by a path segment, a bare name, or a dotted call:
    /// types, traits, type aliases, associated types, functions, methods,
    /// consts, statics, enum variants.
    Item,
    /// Reached by `x.name` or `x.0` — a dot with NO call — or by a
    /// struct-literal or struct-pattern field position. Its own reach because a
    /// field is reachable no other way: `Type::field` is not a path Rust
    /// admits, so `field` and [`Reach::Item`] never overlap and a field beside
    /// a same-named method stays two symbols.
    Field,
    /// Reached by `name!`. `macro_rules!` and proc macros.
    Macro,
    /// Reached by nothing: a module appears in an identity as the `module`
    /// segment, never as a target.
    ///
    /// Its own reach for a measured reason, not an aesthetic one. Collapse the
    /// type and value namespaces while leaving modules in the collapsed bucket
    /// and `crate::installer::install(..)` — a call to a function re-exported
    /// from a module of the same name — lands on the MODULE: a wrong edge where
    /// today there is a dangling one. 7 such references in this repo, 6 distinct
    /// identities. It costs nothing, because no reference ever mints `mod`.
    Mod,
}

impl Reach {
    /// The label this reach occupies the trailing segment with. Paired with
    /// [`Reach::from_label`] here so the two directions cannot drift.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Item => "item",
            Self::Field => "field",
            Self::Macro => "macro",
            Self::Mod => "mod",
        }
    }

    /// The inverse of [`Reach::as_str`]. `None` means the label names no reach
    /// a builder here could have written — the caller turns that into a typed
    /// error rather than picking one.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "item" => Some(Self::Item),
            "field" => Some(Self::Field),
            "macro" => Some(Self::Macro),
            "mod" => Some(Self::Mod),
            _ => None,
        }
    }
}

/// Why an fqn could not be built or read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FqnError {
    /// A required segment was empty.
    ///
    /// Dropping it would not produce a broken string, it would produce a
    /// PERFECTLY VALID string naming a different symbol: `package: ""` with
    /// `module: "m"` and `package: "m"` with `module: ""` both collapse to
    /// `rust·m·n`. An identity that cannot be built is an error (R4).
    EmptySegment { segment: Segment },
    /// A segment carried the separator, which would make the encoding split
    /// back into more parts than it was built from.
    SeparatorInSegment { segment: Segment, value: String },
    /// [`parse`] was handed something no builder here could have produced —
    /// too few segments, or an empty one.
    NotAnFqn { encoded: String },
    /// [`parse`] read a leading segment naming no language this build can read.
    /// An error rather than a guess: a symbol filed under the wrong language is
    /// a wrong-merge waiting to happen.
    UnknownLanguage { found: String },
    /// [`parse`] read a trailing segment naming no reach a builder here could
    /// have written. Guessing one would merge two declarations that the reach
    /// exists to keep apart.
    UnknownReach { found: String },
    /// [`type_segment`] was handed source text that names no type — a tuple, a
    /// slice, a unit. There is no segment to mint and inventing one would be
    /// fabrication (R4).
    NotATypeName { value: String },
}

/// The forms of the grammar. One variant per form, and [`encode`] has one arm
/// per variant, so each form's encoding is written down exactly once (R7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Form<'a> {
    /// A free item: function, type, trait, const, static, module.
    Item { lang: Language, package: &'a str, module: &'a str, name: &'a str, reach: Reach },
    /// A member of a type: field, inherent method, associated fn, enum variant.
    /// [`Reach`] is what keeps a field and a same-named method apart.
    Member {
        lang: Language,
        package: &'a str,
        module: &'a str,
        ty: &'a str,
        member: &'a str,
        reach: Reach,
    },
    /// A member supplied by a trait impl. The trait qualifier is what keeps
    /// `Display::fmt` and `Debug::fmt` on one type from becoming one symbol.
    TraitMember {
        lang: Language,
        package: &'a str,
        module: &'a str,
        ty: &'a str,
        tr: &'a str,
        member: &'a str,
        reach: Reach,
    },
    /// An external symbol (R5). `member` may be empty, which names the crate
    /// itself.
    ///
    /// No reach, and the asymmetry is deliberate: the reach exists to keep two
    /// of OUR declarations from merging, and we index no declarations for a
    /// library — an external is named by use and never opened (R5, D3). A
    /// library member is also reached through a whole path (`sync::Mutex::new`),
    /// which has no single answer anyway.
    Lib { package: &'a str, member: &'a str },
}

/// Mint the fqn of a DECLARATION.
///
/// Identical to [`refer`] by construction and by test. Two named doors, because
/// the sides are written by different code at different times and the merge
/// contract is that they agree; one door would leave nothing for the byte-
/// identity test to compare and no place where a divergence would show up.
pub fn define(form: &Form<'_>) -> Result<Fqn, FqnError> {
    encode(form)
}

/// Mint the fqn a USE SITE is looking for. See [`define`].
pub fn refer(form: &Form<'_>) -> Result<Fqn, FqnError> {
    encode(form)
}

/// The single encoder. Both doors lead here; nothing else does.
fn encode(form: &Form<'_>) -> Result<Fqn, FqnError> {
    let segments: Vec<&str> = match form {
        Form::Item { lang, package, module, name, reach } => {
            check(Segment::Package, package, Required::Yes)?;
            check(Segment::Module, module, Required::No)?;
            check(Segment::Member, name, Required::Yes)?;
            vec![lang.as_str(), package, module, name, reach.as_str()]
        }
        Form::Member { lang, package, module, ty, member, reach } => {
            check(Segment::Package, package, Required::Yes)?;
            check(Segment::Module, module, Required::No)?;
            check(Segment::Type, ty, Required::Yes)?;
            check(Segment::Member, member, Required::Yes)?;
            vec![lang.as_str(), package, module, ty, member, reach.as_str()]
        }
        Form::TraitMember { lang, package, module, ty, tr, member, reach } => {
            check(Segment::Package, package, Required::Yes)?;
            check(Segment::Module, module, Required::No)?;
            check(Segment::Type, ty, Required::Yes)?;
            check(Segment::Trait, tr, Required::Yes)?;
            check(Segment::Member, member, Required::Yes)?;
            vec![lang.as_str(), package, module, ty, tr, member, reach.as_str()]
        }
        Form::Lib { package, member } => {
            check(Segment::Package, package, Required::Yes)?;
            check(Segment::Member, member, Required::No)?;
            vec![LIB, package, member]
        }
    };
    Ok(Fqn::from_encoded(join(&segments)))
}

/// Whether the grammar allows a segment to be empty. Only `module` and a
/// library `member` are optional; see the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Required {
    Yes,
    No,
}

/// The two ways a segment can be unusable, checked in one place so every form
/// rejects them identically.
fn check(segment: Segment, value: &str, required: Required) -> Result<(), FqnError> {
    if value.is_empty() {
        return match required {
            Required::Yes => Err(FqnError::EmptySegment { segment }),
            Required::No => Ok(()),
        };
    }
    if value.contains(SEP) {
        return Err(FqnError::SeparatorInSegment { segment, value: value.to_string() });
    }
    Ok(())
}

/// Join with [`SEP`], dropping empty segments so an absent optional one never
/// leaves a doubled separator behind.
fn join(segments: &[&str]) -> String {
    let mut out = String::new();
    for segment in segments {
        if segment.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(SEP);
        }
        out.push_str(segment);
    }
    out
}

/// Reduce the source text of a type to the one segment that names it.
///
/// The single owner of that rule (R7). The two sides read a type out of
/// different places and get different text for one type: the definition side
/// sees an impl header (`impl<T> Widget<T>`), the reference side sees a use
/// (`Widget::<u32>::new`, `&Widget`, `crate::widget::Widget`). If each
/// normalised its own way they would mint different strings and never merge, so
/// neither does — both call this.
///
/// It strips what decorates a type and keeps the head of its path. It is NOT
/// resolution: `Self` and a bare `Widget` come back verbatim, because deciding
/// WHICH `Widget` is meant needs the import table and belongs to the ladder.
pub fn type_segment(raw: &str) -> Result<String, FqnError> {
    let path = type_path(raw)?;
    Ok(path.rsplit("::").next().unwrap_or(&path).trim().to_string())
}

/// The same reduction as [`type_segment`], stopping one step earlier: it keeps
/// the PATH that leads to the type instead of only the name at its end.
///
/// Both are needed and both are here, because the name alone cannot say WHICH
/// `Widget` is meant while the path can: `crate::widget::Widget` states its own
/// root, and a reference that kept only `Widget` has thrown that away. Discarding
/// something already parsed is the failure this rewrite is measuring, so the walk
/// records the path as evidence and the resolution ladder reads it back.
pub fn type_path(raw: &str) -> Result<String, FqnError> {
    let not_a_type = || FqnError::NotATypeName { value: raw.to_string() };

    let mut rest = raw.trim();
    // Decorations, in any order and any number: `&mut &'a dyn Trait` is legal.
    loop {
        let before = rest;
        for prefix in ["&", "*const ", "*mut ", "*", "dyn ", "impl ", "mut "] {
            rest = rest.strip_prefix(prefix).unwrap_or(rest).trim_start();
        }
        if rest.starts_with('\'') {
            // A lifetime argument decorates the type without naming it.
            rest = rest[1..]
                .trim_start_matches(|c: char| c.is_alphanumeric() || c == '_')
                .trim_start();
        }
        if rest == before {
            break;
        }
    }

    // Generic arguments belong to the use, not to the identity: `Widget<T>` and
    // `Widget` are one type.
    let head = rest.split('<').next().unwrap_or(rest).trim();
    // `Widget::<u32>` leaves a turbofish's `::` dangling once the arguments go.
    let path = head.trim_end_matches(':').trim();
    let name = path.rsplit("::").next().unwrap_or(path).trim();

    if name.is_empty() {
        return Err(not_a_type());
    }
    // A tuple, a slice, a unit or a fn pointer names no single type. Rust
    // identifiers start with a letter or an underscore, so anything else here
    // is not a name to mint a segment from.
    if !name.starts_with(|c: char| c.is_alphabetic() || c == '_') {
        return Err(not_a_type());
    }
    if path.contains(SEP) {
        return Err(FqnError::SeparatorInSegment {
            segment: Segment::Type,
            value: path.to_string(),
        });
    }
    Ok(path.to_string())
}

/// Whether an fqn names something of ours or something we only ever name (R5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    /// Local symbols carry the reach they are named through; see [`Reach`].
    Local { lang: Language, reach: Reach },
    /// An external has no reach — see [`Form::Lib`].
    Lib,
}

/// What [`parse`] could prove about an fqn.
///
/// There is no `form` here, and that is deliberate. Dropping the empty module
/// segment makes the encoding non-injective — a package-root method and a
/// module-level item encode alike — so the form is not recoverable from the
/// string. Reporting a guessed one would be fabrication (R4); reporting the
/// segments is what the string actually carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed<'a> {
    pub origin: Origin,
    pub package: &'a str,
    /// Everything between the package and the trailing reach, in order. Empty
    /// for a bare crate.
    pub tail: Vec<&'a str>,
}

/// Read a built fqn back into the parts it can prove.
///
/// Takes a `&str` rather than an [`Fqn`] because the strings that need reading
/// back come out of storage, where nothing has yet vouched for them. It hands
/// back borrowed segments and never an [`Fqn`], so it is not a second way to
/// mint an identity.
pub fn parse(encoded: &str) -> Result<Parsed<'_>, FqnError> {
    let segments: Vec<&str> = encoded.split(SEP).collect();
    let malformed = || FqnError::NotAnFqn { encoded: encoded.to_string() };
    if segments.iter().any(|s| s.is_empty()) {
        return Err(malformed());
    }

    let (head, rest) = segments.split_first().ok_or_else(malformed)?;
    let (package, tail) = rest.split_first().ok_or_else(malformed)?;

    if *head == LIB {
        return Ok(Parsed { origin: Origin::Lib, package, tail: tail.to_vec() });
    }
    let lang = Language::from_label(head)
        .ok_or_else(|| FqnError::UnknownLanguage { found: (*head).to_string() })?;
    // `<lang>·<package>·<reach>` alone names no symbol — every local form has
    // at least a name between the package and the reach.
    let (reach_label, tail) = tail.split_last().ok_or_else(malformed)?;
    if tail.is_empty() {
        return Err(malformed());
    }
    let reach = Reach::from_label(reach_label)
        .ok_or_else(|| FqnError::UnknownReach { found: (*reach_label).to_string() })?;
    Ok(Parsed { origin: Origin::Local { lang, reach }, package, tail: tail.to_vec() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::facts::Language;

    /// The merge contract, as a table.
    ///
    /// A definition and a reference to the same symbol are minted by different
    /// code at different times; if the two strings differ by one byte they never
    /// merge onto one node and the graph carries two half-symbols instead of
    /// one. Every form of the grammar appears here, and each row is checked from
    /// both sides.
    fn table() -> Vec<(&'static str, Form<'static>, &'static str)> {
        vec![
            (
                "free fn in a nested module",
                Form::Item {
                    lang: Language::Rust,
                    package: "senseid",
                    module: "api::handlers::codebase",
                    name: "language_for_ext",
                    reach: Reach::Item,
                },
                "rust·senseid·api::handlers::codebase·language_for_ext·item",
            ),
            (
                "free fn at the crate root",
                Form::Item {
                    lang: Language::Rust,
                    package: "sensei-cli",
                    module: "",
                    name: "main",
                    reach: Reach::Item,
                },
                "rust·sensei-cli·main·item",
            ),
            (
                "a module declaration, the one thing no reference ever reaches",
                Form::Item {
                    lang: Language::Rust,
                    package: "senseid",
                    module: "api",
                    name: "handlers",
                    reach: Reach::Mod,
                },
                "rust·senseid·api·handlers·mod",
            ),
            (
                "inherent method",
                Form::Member {
                    lang: Language::Rust,
                    package: "senseid",
                    module: "widget",
                    ty: "Widget",
                    member: "new",
                    reach: Reach::Item,
                },
                "rust·senseid·widget·Widget·new·item",
            ),
            (
                "method on a crate-root type",
                Form::Member {
                    lang: Language::Rust,
                    package: "senseid",
                    module: "",
                    ty: "Config",
                    member: "load",
                    reach: Reach::Item,
                },
                "rust·senseid·Config·load·item",
            ),
            (
                "a field, which shares its type, module and package with a method",
                Form::Member {
                    lang: Language::Rust,
                    package: "senseid",
                    module: "widget",
                    ty: "Widget",
                    member: "width",
                    reach: Reach::Field,
                },
                "rust·senseid·widget·Widget·width·field",
            ),
            (
                "a field whose name a method on the same type also carries",
                Form::Member {
                    lang: Language::Rust,
                    package: "senseid",
                    module: "widget",
                    ty: "Widget",
                    member: "new",
                    reach: Reach::Field,
                },
                "rust·senseid·widget·Widget·new·field",
            ),
            (
                "enum variant",
                Form::Member {
                    lang: Language::Rust,
                    package: "senseid",
                    module: "indexer::facts",
                    ty: "Reason",
                    member: "UnhandledForm",
                    reach: Reach::Item,
                },
                "rust·senseid·indexer::facts·Reason·UnhandledForm·item",
            ),
            (
                "trait-impl method",
                Form::TraitMember {
                    lang: Language::Rust,
                    package: "senseid",
                    module: "widget",
                    ty: "Widget",
                    tr: "Display",
                    member: "fmt",
                    reach: Reach::Item,
                },
                "rust·senseid·widget·Widget·Display·fmt·item",
            ),
            (
                "trait-impl method on a crate-root type",
                Form::TraitMember {
                    lang: Language::Rust,
                    package: "senseid",
                    module: "",
                    ty: "Config",
                    tr: "Debug",
                    member: "fmt",
                    reach: Reach::Item,
                },
                "rust·senseid·Config·Debug·fmt·item",
            ),
            (
                "a macro definition, which only a `name!` can reach",
                Form::Item {
                    lang: Language::Rust,
                    package: "senseid",
                    module: "macros",
                    name: "bail",
                    reach: Reach::Macro,
                },
                "rust·senseid·macros·bail·macro",
            ),
            (
                "external member",
                Form::Lib { package: "serde_json", member: "from_str" },
                "lib·serde_json·from_str",
            ),
            (
                "bare external crate, no member",
                Form::Lib { package: "tokio", member: "" },
                "lib·tokio",
            ),
            (
                "external member reached through a path",
                Form::Lib { package: "tokio", member: "sync::Mutex::new" },
                "lib·tokio·sync::Mutex::new",
            ),
        ]
    }

    /// The single load-bearing test of the whole rewrite. If it fails, nothing
    /// built on top of it can work: definitions and references never merge.
    #[test]
    fn the_definition_side_and_the_reference_side_mint_byte_identical_fqns() {
        for (what, form, expected) in table() {
            let defined = define(&form).unwrap_or_else(|e| panic!("{what}: define failed: {e:?}"));
            let referred = refer(&form).unwrap_or_else(|e| panic!("{what}: refer failed: {e:?}"));

            assert_eq!(
                defined.as_str().as_bytes(),
                referred.as_str().as_bytes(),
                "{what}: the definition side and the reference side disagree, so the two \
                 will never merge onto one node"
            );
            assert_eq!(defined.as_str(), expected, "{what}: definition side");
            assert_eq!(referred.as_str(), expected, "{what}: reference side");
        }
    }

    /// A crate-root module and a bare external crate are the two places the
    /// grammar allows an empty segment. Both sides must drop it the same way —
    /// one side emitting a doubled separator is the same failure as one side
    /// spelling a name differently.
    #[test]
    fn an_empty_optional_segment_is_dropped_identically_on_both_sides() {
        let empty_module = Form::Member {
            lang: Language::Rust,
            package: "senseid",
            module: "",
            ty: "Config",
            member: "load",
            reach: Reach::Item,
        };
        let bare_crate = Form::Lib { package: "tokio", member: "" };

        for (what, form) in [("crate-root module", empty_module), ("bare crate", bare_crate)] {
            let defined = define(&form).unwrap_or_else(|e| panic!("{what}: {e:?}"));
            let referred = refer(&form).unwrap_or_else(|e| panic!("{what}: {e:?}"));
            assert_eq!(defined.as_str(), referred.as_str(), "{what}");
            assert!(
                !defined.as_str().contains("··"),
                "{what}: an empty segment left a doubled separator in {defined}"
            );
        }
    }

    /// `Display::fmt` and `Debug::fmt` are two different functions on one type.
    /// Without the trait qualifier they would be one node and every call to
    /// either would point at the wrong one half the time.
    #[test]
    fn a_trait_qualifier_keeps_two_impls_of_one_member_distinct() {
        let display = define(&Form::TraitMember {
            lang: Language::Rust,
            package: "senseid",
            module: "widget",
            ty: "Widget",
            tr: "Display",
            member: "fmt",
            reach: Reach::Item,
        })
        .expect("well-formed");
        let debug = define(&Form::TraitMember {
            lang: Language::Rust,
            package: "senseid",
            module: "widget",
            ty: "Widget",
            tr: "Debug",
            member: "fmt",
            reach: Reach::Item,
        })
        .expect("well-formed");
        assert_ne!(display, debug, "two traits, one member name, one type: still two symbols");
    }

    /// Dropping an empty segment is what keeps a crate-root symbol from carrying
    /// a doubled separator — but dropped blindly it also merges two different
    /// symbols. `package: ""` with `module: "m"` and `package: "m"` with
    /// `module: ""` would both encode to `rust·m·n`. An identity that cannot be
    /// built is an error, never a shorter string that means something else (R4).
    #[test]
    fn an_empty_required_segment_is_an_error_not_a_collapsed_fqn() {
        let cases = vec![
            (
                "no package",
                Form::Item {
                    lang: Language::Rust,
                    package: "",
                    module: "m",
                    name: "n",
                    reach: Reach::Item,
                },
                Segment::Package,
            ),
            (
                "no name",
                Form::Item {
                    lang: Language::Rust,
                    package: "p",
                    module: "m",
                    name: "",
                    reach: Reach::Item,
                },
                Segment::Member,
            ),
            (
                "no type",
                Form::Member {
                    lang: Language::Rust,
                    package: "p",
                    module: "m",
                    ty: "",
                    member: "x",
                    reach: Reach::Item,
                },
                Segment::Type,
            ),
            (
                "no member",
                Form::Member {
                    lang: Language::Rust,
                    package: "p",
                    module: "m",
                    ty: "T",
                    member: "",
                    reach: Reach::Item,
                },
                Segment::Member,
            ),
            (
                "no trait",
                Form::TraitMember {
                    lang: Language::Rust,
                    package: "p",
                    module: "m",
                    ty: "T",
                    tr: "",
                    member: "x",
                    reach: Reach::Item,
                },
                Segment::Trait,
            ),
            ("no lib package", Form::Lib { package: "", member: "x" }, Segment::Package),
        ];

        for (what, form, segment) in cases {
            assert_eq!(define(&form), Err(FqnError::EmptySegment { segment }), "{what}: define");
            assert_eq!(refer(&form), Err(FqnError::EmptySegment { segment }), "{what}: refer");
        }
    }

    /// A segment carrying the separator would encode to a string that parses
    /// back into a different number of parts than it was built from, so the
    /// encoding would stop being reversible.
    #[test]
    fn a_separator_inside_a_segment_is_an_error_not_an_ambiguous_fqn() {
        let form = Form::Item {
            lang: Language::Rust,
            package: "senseid",
            module: "a·b",
            name: "n",
            reach: Reach::Item,
        };
        let expected = Err(FqnError::SeparatorInSegment {
            segment: Segment::Module,
            value: "a·b".to_string(),
        });
        assert_eq!(define(&form), expected);
        assert_eq!(refer(&form), expected);
    }

    /// Re-encoding what `parse` returned must reproduce the input byte for byte,
    /// for every form in the table.
    #[test]
    fn a_built_fqn_parses_back_into_the_parts_it_was_built_from() {
        for (what, form, expected) in table() {
            let built = define(&form).unwrap_or_else(|e| panic!("{what}: {e:?}"));
            let parsed = parse(built.as_str()).unwrap_or_else(|e| panic!("{what}: {e:?}"));

            let (head, reach) = match parsed.origin {
                Origin::Local { lang, reach } => (lang.as_str(), reach.as_str()),
                Origin::Lib => (LIB, ""),
            };
            let mut segments = vec![head, parsed.package];
            segments.extend(parsed.tail.iter().copied());
            segments.push(reach);
            assert_eq!(join(&segments), expected, "{what}: round trip");
        }
    }

    #[test]
    fn parse_reports_the_parts_a_string_actually_carries() {
        let local = parse("rust·senseid·widget·Widget·new·item").expect("well-formed");
        assert_eq!(local.origin, Origin::Local { lang: Language::Rust, reach: Reach::Item });
        assert_eq!(local.package, "senseid");
        assert_eq!(local.tail, vec!["widget", "Widget", "new"]);

        let lib = parse("lib·tokio·sync::Mutex::new").expect("well-formed");
        assert_eq!(lib.origin, Origin::Lib);
        assert_eq!(lib.package, "tokio");
        assert_eq!(lib.tail, vec!["sync::Mutex::new"]);

        let bare = parse("lib·tokio").expect("a bare crate reference is a whole fqn");
        assert_eq!(bare.origin, Origin::Lib);
        assert_eq!(bare.package, "tokio");
        assert!(bare.tail.is_empty(), "a bare crate has no member, and that is not a failure");
    }

    /// Dropping empty segments makes the encoding non-injective: a crate-root
    /// method and a module-level item encode alike, and nothing in the string
    /// says which was meant. `parse` therefore reports the segments it can prove
    /// and never a form it would have to guess (R4). Recorded here so no later
    /// pass is written against a form the string does not carry.
    #[test]
    fn two_forms_can_encode_alike_so_parse_never_guesses_the_form() {
        let crate_root_method = define(&Form::Member {
            lang: Language::Rust,
            package: "p",
            module: "",
            ty: "Config",
            member: "load",
            reach: Reach::Item,
        })
        .expect("well-formed");
        let item_in_a_module = define(&Form::Item {
            lang: Language::Rust,
            package: "p",
            module: "Config",
            name: "load",
            reach: Reach::Item,
        })
        .expect("well-formed");

        assert_eq!(
            crate_root_method.as_str(),
            item_in_a_module.as_str(),
            "the empty-module drop makes these one string"
        );
        assert_eq!(
            parse(crate_root_method.as_str()).expect("well-formed").tail,
            vec!["Config", "load"]
        );
    }

    #[test]
    fn parse_rejects_a_string_no_builder_here_could_have_produced() {
        for encoded in ["", "rust", "rust·senseid", "rust·senseid·item", "rust··x·item", "lib"]
        {
            assert_eq!(
                parse(encoded),
                Err(FqnError::NotAnFqn { encoded: encoded.to_string() }),
                "`{encoded}` is not a whole fqn"
            );
        }
        assert_eq!(
            parse("python·p·x·item"),
            Err(FqnError::UnknownLanguage { found: "python".to_string() }),
            "a language this build cannot read is an error, not a silent Rust"
        );
        assert_eq!(
            parse("rust·p·m·x"),
            Err(FqnError::UnknownReach { found: "x".to_string() }),
            "a trailing segment no builder could have written is an error, not a guessed reach"
        );
        assert_eq!(
            parse("rust·p·m·x·ty"),
            Err(FqnError::UnknownReach { found: "ty".to_string() }),
            "`ty` and `val` were the NAMESPACE this segment used to carry; a string still \
             spelling one is a stale identity, not a reach this build can read"
        );
    }

    /// Rust lets a field and a method share a name on one type, and lets a
    /// module and a function share a name in one scope. The fqn is the merge key
    /// (spec §2), so a grammar that cannot tell them apart merges two different
    /// declarations onto one node and every reference to either points at the
    /// wrong one about half the time — a wrong edge, which is worse than a
    /// missing one (R4).
    ///
    /// Both are real collisions in this repo: `WatcherHealth` has a `healthy`
    /// field and a `healthy()` getter, and `sensei-bootstrap` has
    /// `pub mod config;` and `pub fn config()` at its crate root. What keeps
    /// each pair apart has changed, and the two rows now prove different things:
    /// the field is separated by the DOT it is reached through, and the module
    /// by the fact that nothing reaches a module at all. Neither is separated by
    /// a namespace any more, because a path leaf does not state one (§2.1).
    #[test]
    fn two_declarations_that_rust_allows_to_share_a_name_mint_distinct_fqns() {
        let field = define(&Form::Member {
            lang: Language::Rust,
            package: "senseid",
            module: "watcher::root_watcher",
            ty: "WatcherHealth",
            member: "healthy",
            reach: Reach::Field,
        })
        .expect("well-formed");
        let getter = define(&Form::Member {
            lang: Language::Rust,
            package: "senseid",
            module: "watcher::root_watcher",
            ty: "WatcherHealth",
            member: "healthy",
            reach: Reach::Item,
        })
        .expect("well-formed");
        assert_ne!(field, getter, "a field and a same-named method are two symbols, not one");

        let module = define(&Form::Item {
            lang: Language::Rust,
            package: "sensei-bootstrap",
            module: "",
            name: "config",
            reach: Reach::Mod,
        })
        .expect("well-formed");
        let function = define(&Form::Item {
            lang: Language::Rust,
            package: "sensei-bootstrap",
            module: "",
            name: "config",
            reach: Reach::Item,
        })
        .expect("well-formed");
        assert_ne!(module, function, "a module and a same-named fn are two symbols, not one");
    }

    /// The reference side reads a type out of source text that carries generics,
    /// references and a module path; the definition side reads it out of an impl
    /// header that carries different ones. `Widget<T>` and `Widget` are the same
    /// type and must produce the same segment, or the two sides never merge.
    #[test]
    fn one_owner_normalises_a_type_into_its_fqn_segment() {
        for (raw, expected) in [
            ("Widget", "Widget"),
            ("Widget<T>", "Widget"),
            ("Widget::<u32>", "Widget"),
            ("&Widget", "Widget"),
            ("&mut Widget<T>", "Widget"),
            ("&'a Widget", "Widget"),
            ("*const Widget", "Widget"),
            ("dyn Draw", "Draw"),
            ("impl Draw", "Draw"),
            ("crate::widget::Widget", "Widget"),
            ("std::collections::HashMap<String, u32>", "HashMap"),
            ("Box<Widget>", "Box"),
            ("Self", "Self"),
        ] {
            assert_eq!(type_segment(raw).as_deref(), Ok(expected), "normalising `{raw}`");
        }
    }

    /// The path a type is reached through is a fact the source states and the
    /// name at its end is not: `Widget` alone cannot say which `Widget`, while
    /// `crate::widget::Widget` roots itself. Both reductions strip the same
    /// decorations, so a caller that wants one and a caller that wants the other
    /// cannot disagree about what a type's text says.
    #[test]
    fn the_path_to_a_type_survives_the_same_reduction_that_produces_its_name() {
        for (raw, path, segment) in [
            ("Widget", "Widget", "Widget"),
            ("&mut crate::widget::Widget<T>", "crate::widget::Widget", "Widget"),
            ("std::collections::HashMap<String, u32>", "std::collections::HashMap", "HashMap"),
            ("super::Widget", "super::Widget", "Widget"),
            ("dyn crate::draw::Draw", "crate::draw::Draw", "Draw"),
            ("Widget::<u32>", "Widget", "Widget"),
        ] {
            assert_eq!(type_path(raw).as_deref(), Ok(path), "the path of `{raw}`");
            assert_eq!(type_segment(raw).as_deref(), Ok(segment), "the name of `{raw}`");
        }
        for raw in ["(u32, u32)", "[u8]", "()", "&[u8]", ""] {
            assert!(
                matches!(type_path(raw), Err(FqnError::NotATypeName { .. })),
                "`{raw}` names no type, so it has no path either"
            );
        }
    }

    /// A tuple, a slice and a unit are types with no name, so there is no
    /// segment to mint. Inventing one would be fabrication (R4).
    #[test]
    fn a_type_with_no_name_is_an_error_not_an_invented_segment() {
        for raw in ["(u32, u32)", "[u8]", "()", "&[u8]", ""] {
            assert!(
                matches!(type_segment(raw), Err(FqnError::NotATypeName { .. })),
                "`{raw}` names no type, so it has no fqn segment"
            );
        }
    }

    /// Two call sites building an fqn by hand is how the definition and
    /// reference sides drift apart, and it is invisible in review because each
    /// side looks right on its own. The separator is private to this file so the
    /// compiler blocks the easy route; this closes the rest.
    #[test]
    fn no_fqn_is_built_by_string_formatting_outside_this_file() {
        // The separator, spelled the ways rust source can spell it. The plain
        // character is the accident; the escapes are the evasion.
        let separators = [
            SEP.to_string(),
            format!("\\u{{{:x}}}", SEP as u32),
            format!("\\u{{{:X}}}", SEP as u32),
        ];
        let mut read = 0;
        for (path, body) in crate::indexer::guard_sources() {
            if path == "fqn.rs" {
                continue;
            }
            read += 1;
            let body = crate::indexer::outside_tests(&body);
            for separator in &separators {
                assert!(
                    !body.contains(separator.as_str()),
                    "{path} contains the fqn separator as `{separator}`: every fqn is built by \
                     fqn::define or fqn::refer, so the two sides cannot disagree"
                );
            }
            // Unqualified, so importing it under another name does not evade
            // the guard the way `Fqn::from_encoded` alone would. The definition
            // itself is removed first: `facts.rs` DECLARES the constructor, and
            // declaring it is not calling it.
            let calls = body.replace("fn from_encoded", "");
            assert!(
                !calls.contains("from_encoded"),
                "{path} mints an Fqn directly, bypassing the grammar and its checks"
            );
        }
        assert!(read > 0, "the guard read no files, so it would have passed vacuously");
    }
}
