//! Supabase client — the daemon's own authenticated connection to dōjō.
//!
//! The daemon is a PUBLIC OAuth client (it runs on the user's machine, so any
//! embedded secret is readable), which is why sign-in is PKCE with a loopback
//! redirect rather than a client-secret flow.
//!
//! What this replaces: a device-token plane against a bespoke HTTP service that
//! does not exist. Authenticating as the user means row-level security enforces
//! "only repositories this person can reach" on every query, instead of every
//! endpoint remembering to check.

pub mod dojo_auth;
pub mod pkce;
pub mod session;
pub mod settings;
pub mod user_plane;
