//! Indexer v2 — one module per language, each owning exactly one thing: how to
//! read that language's grammar (R7).
//!
//! Everything a language module is NOT allowed to own lives above it: the fqn
//! grammar in `fqn.rs`, the fact vocabulary in `facts.rs`, and — from step 5 —
//! the resolution ladder and the reason codes. A rule that would have to be
//! written twice for two languages does not belong here.
//
// These modules have no caller on purpose — see the note in `indexer/mod.rs`.

pub mod rust;
