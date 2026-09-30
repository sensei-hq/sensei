//! How good is the graph this indexer produces, measured over real source.
//!
//! Both modules here answer that, from opposite ends, and both are harnesses
//! rather than runtime code: they run as tests, print a decomposition, and
//! assert a threshold.
//!
//! - [`acceptance`] — §6's criteria and R8. What the graph must achieve.
//! - [`reachability`] — the two barriers a good graph clears: a declaration
//!   should be the callee end of some edge, and WHICH end the caller sits on
//!   says a different thing. What the graph must not lose.
//!
//! **Thresholds, not comparisons.** Neither asserts at zero. The point of each
//! is the decomposition: a graph is good when every miss has a NAME, and the
//! names are few and checkable. A bare number going up tells nobody which
//! language regressed.
//!
//! - [`corpus`] — the one reader both of them, and `impact`, `persist` and
//!   `resolve`, measure over. A second copy would not be a second measurement;
//!   it would be two numbers that drift apart and then disagree about which
//!   language got worse.

pub mod acceptance;
pub mod corpus;
pub mod reachability;
