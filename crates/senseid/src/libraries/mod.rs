//! Library intelligence (workstream D) — a library declares the skills/agents/tools
//! it provides via a `sensei.library.json` manifest committed in its OWN repo; sensei
//! ingests that manifest into `sensei.library_skills` / `sensei.library_agents` and
//! associates the capabilities to any project that depends on the library.
//!
//! This is `crate::libraries` — distinct from `crate::api::handlers::libraries` (the
//! HTTP handlers) and `crate::adapters::manifest` (per-ecosystem dependency parsing).

pub mod manifest;
