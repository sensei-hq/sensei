//! The contract between extraction and persistence.
//!
//! Extraction produces facts; persistence writes them. Today each emit arm in
//! `process.rs` states its own resolution ladder inline, and two of them derive
//! a lib package from an fqn in two different ways — one hardcoding `'·'`, the
//! other using [`crate::languages::fqn::SEP`]. If that separator ever changed,
//! one would silently break. This module owns the shared pieces.
//!
//! Scoped deliberately: types and pure functions only. The persister that
//! consumes them is the next increment, and the emit arms migrate after that.

/// What to do when a target cannot be resolved.
///
/// Two variants, because the code has two distinct behaviours. The ADR listed
/// three — `LeaveUnresolved`, `CreateStub`, `RequireUnambiguous` — but
/// `RequireUnambiguous` describes how a LOOKUP is performed, not what happens on
/// a miss: the doc-symbol arm's miss behaviour is `LeaveUnresolved`, identical
/// to the file arm's. A variant that names a lookup strategy in a
/// miss-policy enum is a category error, and one with no distinct behaviour is
/// worse than the inline code it replaces.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum OnMiss {
    /// Write the edge with `target_name` set and no `target_id`. The truthful
    /// unresolved shape: a consumer can tell it apart from a resolved edge.
    LeaveUnresolved,
    /// Create a placeholder node under the target fqn, which the real
    /// definition ENRICHES when it is indexed. This is what makes resolution
    /// order-independent — measured, 99.5% of java inheritance relations have
    /// their parent in another file.
    CreateStub {
        /// The node kind the placeholder carries, so an enrich need not correct
        /// it. A supertype is a `class`; a call target is a `function`.
        kind: &'static str,
    },
}

/// The `<package>` segment of a `lib·<package>·<path>·<member>` fqn.
///
/// One owner for what `process.rs` derived twice. Returns `None` rather than
/// `""` on a shape that carries no package: an empty package would be written
/// into a `lib_package` node's name, and the CLAUDE.md rule against fabricating
/// on a miss applies to a defaulted identity as much as to a defaulted value.
pub fn lib_package_of(fqn: &str) -> Option<&str> {
    let sep = crate::languages::fqn::SEP;
    let mut segs = fqn.split(sep);
    if segs.next()? != "lib" {
        return None;
    }
    match segs.next() {
        Some(p) if !p.is_empty() => Some(p),
        _ => None,
    }
}

/// Where an edge points, and what the persister may do about a miss.
///
/// Three shapes because the emit arms have three, each with a DIFFERENT write:
/// an external target mints a `lib·` node, an internal one is reused or stubbed,
/// and an unresolvable one carries only its name.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TargetRef {
    /// An external dependency. Carries `name` EXPLICITLY rather than deriving it
    /// from the fqn's last segment: `upsert_lib_node_by_fqn` writes it to
    /// `nodes.name`, and 6,910 live `lib_symbol` rows depend on it. The last
    /// segment happens to equal the name in every current construction, which
    /// is exactly why deriving it would go unnoticed when it stops being true.
    Lib { fqn: String, name: String, package: String },
    /// A target inside the indexed tree. Reused if already known, otherwise
    /// handled per [`OnMiss`].
    Internal { fqn: String, name: String, on_miss: OnMiss },
    /// Nothing to point at. The edge records the name and no target_id — the
    /// truthful unresolved shape, never a guess.
    Unresolvable { name: String },
}

/// One edge to write.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct EdgeFact {
    pub source_id: uuid::Uuid,
    pub target: TargetRef,
    /// The `sensei.edge_kind` label.
    pub kind: &'static str,
    /// Merged into `edges.props`. Empty for the arms that stamp nothing —
    /// measured, `calls`/`imports`/`references` are 0% stamped while
    /// `extends`/`implements` are 100%, and a persister that unified that
    /// would regress one side or the other.
    pub props: serde_json::Value,
}

/// Which emit ARM each edge came from — test-only observability.
///
/// Several arms are indistinguishable in final state: an internal target
/// resolved from `fqn_ids` and one created as a stub both end as a resolved
/// edge to an enriched node. So a check that classifies rows AFTER the fact
/// cannot prove which branch ran, and a refactor that dropped the in-file fast
/// path would leave the graph byte-identical and be caught by nothing.
///
/// A thread-local rather than a return value: the emit block is deep inside
/// `process_file` and threading a counter through it would change production
/// signatures to serve a test. `#[cfg(test)]` so there is no shipped cost.
#[cfg(test)]
pub mod arm_tally {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    thread_local! {
        static TALLY: RefCell<BTreeMap<&'static str, usize>> = const { RefCell::new(BTreeMap::new()) };
    }

    pub(crate) fn bump(arm: &'static str) {
        TALLY.with(|t| *t.borrow_mut().entry(arm).or_insert(0) += 1);
    }

    pub(crate) fn reset() {
        TALLY.with(|t| t.borrow_mut().clear());
    }

    /// Read and clear. Tests assert against a HARDCODED arm list — counting
    /// what was observed and asserting each count >= 1 is a tautology.
    pub(crate) fn take() -> BTreeMap<&'static str, usize> {
        TALLY.with(|t| std::mem::take(&mut *t.borrow_mut()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The package is the SECOND segment, and a package-less shape yields None.
    ///
    /// `process.rs` used `.nth(1).unwrap_or("")` in both copies, so a malformed
    /// fqn produced an empty package that would be written as a `lib_package`
    /// node's name. `None` forces the caller to decide instead.
    ///
    /// Breaking mutation: return `Some("")` instead of `None` for the empty
    /// case, or take `nth(0)` instead of the second segment.
    #[test]
    fn lib_package_of_takes_the_second_segment_and_refuses_a_package_less_fqn() {
        let sep = crate::languages::fqn::SEP;
        assert_eq!(lib_package_of(&format!("lib{sep}serde{sep}ser{sep}Serialize")), Some("serde"));
        // The real shape in the live graph, verified: lib·json·json·load.
        assert_eq!(lib_package_of(&format!("lib{sep}json{sep}json{sep}load")), Some("json"));
        assert_eq!(
            lib_package_of(&format!("lib{sep}{sep}Foo")),
            None,
            "an empty package segment is not a package"
        );
        assert_eq!(lib_package_of("lib"), None, "no package segment at all");
        assert_eq!(
            lib_package_of(&format!("rust{sep}mycrate{sep}m{sep}Foo")),
            None,
            "a non-lib fqn has no lib package"
        );
    }

    /// `CreateStub` must carry its kind, and there must be exactly two variants.
    ///
    /// The stub kind is load-bearing: a supertype stub is a `class`, a call
    /// target stub a `function`, and swapping them writes the wrong kind into
    /// the graph — the mutation that increment 0's golden was built to catch.
    ///
    /// Breaking mutation: drop the `kind` field, or add a third variant with no
    /// distinct behaviour.
    #[test]
    fn on_miss_has_two_variants_and_the_stub_states_its_kind() {
        let policies = [OnMiss::LeaveUnresolved, OnMiss::CreateStub { kind: "class" }];
        assert_eq!(policies.len(), 2);
        assert_ne!(
            OnMiss::CreateStub { kind: "class" },
            OnMiss::CreateStub { kind: "function" },
            "the stub kind is part of the policy, not incidental"
        );
        match &policies[1] {
            OnMiss::CreateStub { kind } => assert_eq!(*kind, "class"),
            OnMiss::LeaveUnresolved => panic!("wrong variant"),
        }
    }
}
