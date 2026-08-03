//! Dōjō: the daemon-side foundation for connecting a personal Sensei to one or
//! many company/client/community Dōjōs (the collective-intelligence SaaS layer).
//!
//! Scope of this module:
//! - [`memberships`] (C4) — the local connection model + CRUD orchestration (the
//!   authoritative `dojo.memberships` lives in the Dōjō service DB; Fork 1).
//! - [`routing`] (C4) — the pure client-precedence routing decision.
//! - [`client`] (C4) — the HTTP/auth seam C6 (publish) and C7 (pull) build on.
//! - [`attribution`] (C5) — the deterministic client-work DEREFERENCE: the
//!   confidentiality safety net that strips every known + generic identifier and
//!   fails closed on residual risk. Prose generalisation on top lives in
//!   [`crate::collective::anonymize`].
//!
//! Not here (later chunks): artifact publish/pull (C6/C7), downstream inbox
//! (C7). The daemon uses a Keychain-backed Bearer token for the Dōjō service —
//! never Supabase (dual-plane auth).

pub mod attribution;
pub mod client;
pub mod contribute;
pub mod gate;
pub mod memberships;
pub mod relay_constitution;
pub mod relay_nudge;
pub mod relay_project;
pub mod relay_run_project;
pub mod routing;
