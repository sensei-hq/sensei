//! What every language module needs and no language module owns.
//!
//! A rule that would have to be written twice for two languages does not belong
//! in either one (R7). What lands here is the shape of a MISS — the thing a walk
//! says when it read a use site and could not place it — because that shape is a
//! property of [`Resolution`] being total, not a property of any grammar.
//!
//! Deliberately NOT here: anything that reads syntax. `Miss` names a node kind
//! but never looks at a node, so a tree-sitter walk and an oxc walk both build
//! one without either parser leaking into the other's module.
//
// No caller until cutover — see the note in `indexer/mod.rs`.
#![allow(dead_code)]

use crate::indexer::facts::{Evidence, Fqn, Observation, Reason, Resolution};
use crate::indexer::fqn::{FqnError, Reach};

/// What a walk has to say about a use site it did not place: the bucket, the
/// name, the node kind that names the bucket in the histogram, and the material
/// a later pass gets to work with.
///
/// Every use-site arm in every language builds one of these and then emits, so
/// there is no path through any walk on which a use site yields nothing (R2).
pub(super) struct Miss {
    pub reason: Reason,
    pub name: String,
    /// The kind of the node that DEFEATED the walk, which is the anchor's own
    /// kind when the walk read it fine and the inner node's kind when it did
    /// not. This is what a reader sees in the histogram.
    ///
    /// A STRING rather than a parser's node type, because two parsers are in
    /// play: tree-sitter names a Rust node `field_expression`, oxc names the
    /// JavaScript one `StaticMemberExpression`. Both are the same fact — "this
    /// is the shape that defeated me" — and the histogram wants the label, not
    /// the type.
    pub node_kind: String,
    /// How this use site reaches its target (spec §2.1). Stated by every arm,
    /// including the ones that could not read the shape: a call the walk cannot
    /// parse is still reached the way a call is, and the arm that dispatched on
    /// the node kind is the only place that knows it.
    pub reach: Reach,
    pub saw: Vec<Observation>,
}

impl Miss {
    /// The walk read the use site and named it. Placing that name needs the
    /// shared ladder, which a language module is not (R7).
    pub fn unplaced(node_kind: &str, name: &str, reach: Reach, saw: Vec<Observation>) -> Self {
        Self {
            reason: Reason::Unplaced,
            name: readable(name, node_kind),
            node_kind: node_kind.to_string(),
            reach,
            saw,
        }
    }

    /// The walk has no rule for this shape. Named rather than dropped, so the
    /// histogram says what was not understood instead of saying nothing.
    ///
    /// It still states a reach, and that is not a guess: the arm calling this
    /// dispatched on the node kind, so it knows a call is reached like a call
    /// and a field access like a field even when the inside of the node
    /// defeated it.
    pub fn unhandled(node_kind: &str, name: &str, reach: Reach) -> Self {
        Self {
            reason: Reason::UnhandledForm,
            name: readable(name, node_kind),
            node_kind: node_kind.to_string(),
            reach,
            saw: Vec::new(),
        }
    }

    /// A miss with a cause the walk can NAME — a receiver it could not type, a
    /// target chosen at run time. Distinct from [`Miss::unhandled`], which says
    /// only that the shape was not understood.
    pub fn because(
        reason: Reason,
        node_kind: &str,
        name: &str,
        reach: Reach,
        saw: Vec<Observation>,
    ) -> Self {
        Self {
            reason,
            name: readable(name, node_kind),
            node_kind: node_kind.to_string(),
            reach,
            saw,
        }
    }

    /// The unresolved target this miss stands for. One owner, so a reference and
    /// a relation cannot describe the same failure two different ways — the
    /// ladder reads both through the same shape.
    pub fn resolution(self) -> Resolution {
        Resolution::Unresolved {
            reason: self.reason,
            evidence: Evidence {
                name: self.name,
                node_kind: self.node_kind,
                reach: self.reach,
                saw: self.saw,
            },
        }
    }
}

/// An identity the walk CONSIDERED, as evidence — never as a resolution. One
/// that could not even be minted leaves no observation behind rather than a
/// placeholder one.
pub(super) fn considered(minted: Result<Fqn, FqnError>) -> Vec<Observation> {
    minted.map(Observation::Candidate).into_iter().collect()
}

/// Evidence must always name something. In a tree full of ERROR nodes a node's
/// text can be empty, and an unnameable miss is one nobody can act on, so the
/// node kind stands in.
pub(super) fn readable(name: &str, node_kind: &str) -> String {
    let name = name.trim();
    if name.is_empty() { node_kind.to_string() } else { name.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The property the whole design rests on: whatever a walk says about a use
    /// site it could not place, it says SOMETHING (R2, R4). There is no
    /// constructor here that produces an unnamed miss, and the one that could —
    /// a node whose text is empty — falls back to the node kind.
    #[test]
    fn no_miss_is_nameless_whichever_door_it_came_through() {
        let misses = vec![
            Miss::unplaced("call_expression", "handle", Reach::Item, Vec::new()),
            Miss::unplaced("CallExpression", "   ", Reach::Item, Vec::new()),
            Miss::unhandled("field_expression", "", Reach::Field),
            Miss::because(
                Reason::ReceiverTypeUnknown,
                "StaticMemberExpression",
                "",
                Reach::Field,
                vec![Observation::Receiver("ctx.pg()".to_string())],
            ),
        ];
        for miss in misses {
            let node_kind = miss.node_kind.clone();
            match miss.resolution() {
                Resolution::Unresolved { evidence, .. } => {
                    assert!(!evidence.name.is_empty(), "{node_kind} produced a nameless miss");
                    assert!(!evidence.node_kind.is_empty(), "{node_kind} produced no node kind");
                }
                Resolution::Resolved { fqn, .. } => panic!("a miss resolved to {fqn}"),
            }
        }
    }

    /// The reason is chosen by the door, not by the caller passing one twice.
    #[test]
    fn each_door_states_its_own_reason() {
        let unplaced = Miss::unplaced("call_expression", "handle", Reach::Item, Vec::new());
        assert_eq!(unplaced.reason, Reason::Unplaced);
        let unhandled = Miss::unhandled("call_expression", "handle", Reach::Item);
        assert_eq!(unhandled.reason, Reason::UnhandledForm);
        assert!(unhandled.saw.is_empty(), "a shape the walk did not read saw nothing to record");
        let named = Miss::because(
            Reason::DynamicDispatch,
            "call_expression",
            "handle",
            Reach::Item,
            Vec::new(),
        );
        assert_eq!(named.reason, Reason::DynamicDispatch);
    }
}
