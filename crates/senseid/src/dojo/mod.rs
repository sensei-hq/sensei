//! Dōjō: the daemon-side foundation for connecting a personal Sensei to one or
//! many company/client/community Dōjōs (the collective-intelligence SaaS layer).
//!
//! Scope of this module (Chunk 4):
//! - [`memberships`] — the local connection model + CRUD orchestration (the
//!   authoritative `dojo.memberships` lives in the Dōjō service DB; Fork 1).
//! - [`routing`] — the pure client-precedence routing decision.
//! - [`client`] — the HTTP/auth seam C6 (publish) and C7 (pull) build on.
//!
//! Not here (later chunks): artifact publish/pull (C6/C7), dereference /
//! anonymise (C5), downstream inbox (C7). The daemon uses a Keychain-backed
//! Bearer token for the Dōjō service — never Supabase (dual-plane auth).

pub mod client;
pub mod memberships;
pub mod routing;
