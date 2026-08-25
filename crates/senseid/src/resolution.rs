//! `Resolution<T>` — the outcome of resolving an identity (folder→namespace,
//! folder→owning-project, session→membership, …) where **"nothing matched"**
//! and **"more than one matched"** are first-class outcomes, DISTINCT from a
//! confident hit.
//!
//! The point is to make the #109 failure mode unrepresentable: code that used
//! to `unwrap_or(<a default>)` or `.first()`/`.next()` on a lookup — silently
//! attributing work to the wrong project / scope / tenant when the real answer
//! was "unknown" or "ambiguous" — must instead match every arm and **fail
//! closed** (error, hold, or surface "unknown"), never substitute a broad
//! default. `crate::git_identity` already does this by propagating `None` for
//! every unresolved field; this type generalises that discipline.

/// The result of resolving a single identity from some lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution<T> {
    /// Exactly one confident match.
    Resolved(T),
    /// The lookup matched more than one candidate — the caller must NOT guess
    /// which; it has to ask or scope down. `count` is how many matched.
    Ambiguous { count: usize },
    /// Nothing matched.
    Unresolved,
}

impl<T> Resolution<T> {
    /// Collapse a candidate iterator to a resolution: 0 → `Unresolved`, 1 →
    /// `Resolved`, >1 → `Ambiguous`. The safe replacement for
    /// `candidates.into_iter().next()`, which silently picks an arbitrary first
    /// when several match. Doesn't allocate — it consumes at most what it needs.
    pub fn from_unique<I: IntoIterator<Item = T>>(candidates: I) -> Self {
        let mut it = candidates.into_iter();
        match (it.next(), it.next()) {
            (None, _) => Resolution::Unresolved,
            (Some(one), None) => Resolution::Resolved(one),
            // Two seen already; count whatever remains for a useful message.
            (Some(_), Some(_)) => Resolution::Ambiguous { count: 2 + it.count() },
        }
    }

    /// The resolved value, or `None` for `Ambiguous`/`Unresolved`. Use ONLY where
    /// treating "not exactly one" as "no confident answer" is the intended
    /// fail-closed behaviour — never to then substitute a default.
    pub fn resolved(self) -> Option<T> {
        match self {
            Resolution::Resolved(t) => Some(t),
            _ => None,
        }
    }

    pub fn is_resolved(&self) -> bool {
        matches!(self, Resolution::Resolved(_))
    }
    pub fn is_ambiguous(&self) -> bool {
        matches!(self, Resolution::Ambiguous { .. })
    }
    pub fn is_unresolved(&self) -> bool {
        matches!(self, Resolution::Unresolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_unique_maps_zero_one_many() {
        assert_eq!(Resolution::<i32>::from_unique(Vec::new()), Resolution::Unresolved);
        assert_eq!(Resolution::from_unique(vec![7]), Resolution::Resolved(7));
        assert_eq!(Resolution::from_unique(vec![1, 2, 3, 4]), Resolution::Ambiguous { count: 4 });
    }

    #[test]
    fn resolved_yields_value_only_for_single_match() {
        assert_eq!(Resolution::from_unique(vec!["a"]).resolved(), Some("a"));
        // Ambiguous must NOT collapse to a guessed first element.
        assert_eq!(Resolution::from_unique(vec!["a", "b"]).resolved(), None);
        assert_eq!(Resolution::<&str>::from_unique(Vec::new()).resolved(), None);
    }

    #[test]
    fn predicates_are_exclusive() {
        assert!(Resolution::from_unique(vec![1]).is_resolved());
        assert!(Resolution::from_unique(vec![1, 2]).is_ambiguous());
        assert!(Resolution::<i32>::from_unique(Vec::new()).is_unresolved());
    }
}
