//! Reading a Rust TYPE out of source text.
//!
//! One owner, and that is the whole point (R7). The definition side reads a
//! type off an impl header (`impl<T> Widget<T>`) and the reference side off a
//! use (`Widget::<u32>::new`, `&Widget`, `crate::widget::Widget`); if each
//! normalised its own way they would mint different strings for one type and
//! never merge (spec §2). Both call in here.
//!
//! It lives beside the Rust walk rather than in `fqn.rs` because it is READING,
//! not encoding: `&mut dyn Trait` is a fact about Rust's grammar and means
//! nothing to the other languages sharing the fqn builder.

use crate::indexer::fqn::{FqnError, Segment};

/// FQN segment separator — the one character a segment may never contain.
/// Duplicated nowhere: this is read from `fqn.rs`, which owns it.
use crate::indexer::fqn::SEPARATOR;

/// A type spelled as a SIMPLE name, or `None`.
///
/// `Config` yes, and `&Config`/`&mut Config` yes — a reference has the
/// referent's methods. `Vec<Config>`, `crate::a::Config`, `[u8; 4]`,
/// `Arc<Config>` no.
///
/// Rejecting rather than truncating is the point. A member of `Vec` is not an
/// identity this grammar mints, and `Vec<Config>` is not a name any declaration
/// carries — so both the truncated and the verbatim spelling would be wrong,
/// and a wrong receiver type mints a wrong key (R4). The narrow rule is what
/// keeps every type this records one a declaration could actually have minted.
pub(super) fn simple_type_name(text: &str) -> Option<String> {
    // A REFERENCE is stripped, because `&Config` has `Config`'s methods —
    // Rust auto-derefs the receiver, so `cfg.script()` on a `&Config` calls
    // `Config::script`. This is a fact about the language, not an inference,
    // and it is most of the reach here: parameters in this workspace are
    // overwhelmingly taken by reference, so without it the signature route
    // reached 57 of 3,873 receivers.
    //
    // `Arc<T>`/`Box<T>` deref too and are deliberately NOT stripped. There the
    // member could belong to the smart pointer OR to `T`, and picking one is a
    // guess that mints a wrong identity half the time (R4). `&` has no such
    // ambiguity: `&T` declares no inherent members of its own.
    let t = text.trim().trim_start_matches('&').trim_start();
    let t = t.strip_prefix("mut ").unwrap_or(t).trim();

    // A GENERIC is typed by its head: `Vec<Config>::push` is `Vec`'s method, and
    // the type argument does not change which type OWNS the member. Refusing the
    // whole spelling reported the type as unknown when it was written down.
    // A PATH names its type in the last segment: `crate::a::Config` owns
    // `Config`'s members, and the module path in front says where it is
    // declared, not what it is. Taken before the generic split so
    // `crate::a::Config<T>` works too.
    let t = t.rsplit("::").next().unwrap_or(t).trim();

    let head = t.split_once('<').map_or(t, |(head, _)| head).trim();

    // `dyn Trait` names the TRAIT, which declares the member. The concrete impl
    // is unknowable — that is what dynamic dispatch means — but WHICH METHOD is
    // called is not in doubt, and refusing the edge loses that to protect
    // against a question nobody asked. Who implements it is answered separately
    // from the `implements` relations the walk already records.
    //
    // `Box<dyn Trait>` reduces the same way, and is the ONE `Box` that is not
    // ambiguous: `Box` declares no inherent method a trait method could be
    // confused with (`Box::new` is associated, not a member).
    if let Some(inner) = t.strip_prefix("dyn ") {
        return simple_type_name(inner.split('+').next().unwrap_or(inner));
    }
    if let Some(inner) = t.strip_prefix("Box<").and_then(|r| r.strip_suffix('>'))
        && inner.trim_start().starts_with("dyn ")
    {
        return simple_type_name(inner);
    }

    // ...EXCEPT a deref wrapper, where the member may be on the INNER type.
    // `arc.method()` is `Arc::method` or `Config::method` and the source does
    // not say which, so taking the head would mint a wrong identity on every
    // call that is really on the inner one (R4). A short, named list rather
    // than a guess: these are the std types whose whole purpose is to be
    // transparent.
    if head != t && DEREF_WRAPPERS.contains(&head) {
        return None;
    }

    let ok = !head.is_empty()
        && head.chars().next().is_some_and(char::is_uppercase)
        && head.chars().all(|c| c.is_alphanumeric() || c == '_');
    ok.then(|| head.to_string())
}

/// The type an element of `collection` has, when the collection states ONE.
///
/// `Vec<Cfg>` yields `Cfg`; `&[Cfg]` yields `Cfg`. A MAP is refused: it yields
/// `(K, V)` pairs, so the binding is a tuple and attributing either half to it
/// is false. Anything not on the list is refused too — a `for` over a custom
/// iterator states its item type on the `Iterator` impl, not here, and guessing
/// would mint a member on whatever type happened to be inside the angle
/// brackets (R4).
pub(super) fn element_type(collection: &str) -> Option<String> {
    let t = collection.trim().trim_start_matches('&').trim_start();
    let t = t.strip_prefix("mut ").unwrap_or(t).trim();

    // A slice or array: `[T]`, `[T; 4]`.
    if let Some(inner) = t.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        return simple_type_name(inner.split(';').next().unwrap_or(inner));
    }

    let (head, inner) = t.split_once('<')?;
    let inner = inner.strip_suffix('>')?;
    // ONE type parameter. A pair means a map, and a map's element is a tuple.
    if inner.contains(',') || !SINGLE_ELEMENT_CONTAINERS.contains(&head.trim()) {
        return None;
    }
    simple_type_name(inner)
}

/// Collections whose element is their single type parameter. Deliberately a
/// list: a map yields pairs, and an unknown generic yields whatever its
/// `Iterator` impl says, which is not stated here.
const SINGLE_ELEMENT_CONTAINERS: &[&str] =
    &["Vec", "VecDeque", "HashSet", "BTreeSet", "BinaryHeap", "Option", "Box", "Rc", "Arc"];

/// Types that deref to their parameter, so a member call on one is ambiguous
/// between the wrapper and the inner type. See [`simple_type_name`].
const DEREF_WRAPPERS: &[&str] = &[
    "Arc",
    "Rc",
    "Box",
    "RefCell",
    "Cell",
    "Mutex",
    "RwLock",
    "Ref",
    "RefMut",
    "Cow",
    "Pin",
    "MutexGuard",
    "RwLockReadGuard",
    "RwLockWriteGuard",
    "ManuallyDrop",
];

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
pub(super) fn type_segment(raw: &str) -> Result<String, FqnError> {
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
pub(super) fn type_path(raw: &str) -> Result<String, FqnError> {
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
    if path.contains(SEPARATOR) {
        return Err(FqnError::SeparatorInSegment {
            segment: Segment::Type,
            value: path.to_string(),
        });
    }
    Ok(path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
