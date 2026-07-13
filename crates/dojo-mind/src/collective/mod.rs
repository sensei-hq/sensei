//! Collective-intelligence services on the Dōjō (receiving) side.
//!
//! [`promote`] is the triage/promote stage that CLOSES the contribute →
//! triage → approve → distribute loop: it moves `submitted` artifacts to
//! `published` (auto-approving the ones that clear the bar, queueing the rest
//! for a maintainer) so that downstream pull (C7) can distribute them.

pub mod promote;
