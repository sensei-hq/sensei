//! SDLC metrics (spec `docs/spec/daemon/tracker-sdlc-metrics.md`): what the AI
//! sessions linked to a ticket did — builds, tests, reviews, holds — derived
//! from captured hook events and read alongside the tracker's own history.

pub mod classify;
pub mod settings;
pub mod signals;
pub mod ticket;
