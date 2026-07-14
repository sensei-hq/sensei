//! Collective intelligence — the `global-dojo` contribution side.
//!
//! The best insight sensei can offer is cross-user: "other developers in the
//! same stack keep hitting X; here's what worked." This module owns the
//! **anonymisation** that lets a local insight travel to the global collective
//! without carrying anything project-specific
//! (`docs/spec/pipeline/collective-intelligence.md`).
//!
//! Global-dojo anonymisation is **stricter than a client-Dōjō dereference**:
//! - the deterministic [`crate::dojo::attribution`] strip runs FIRST (the
//!   confidentiality guarantee — never trust the LLM to strip);
//! - the project name is replaced with a project-*shape* descriptor
//!   (`{stack, size, kind}` — never the name);
//! - user identity is replaced with a rotating opaque anonymous id;
//! - prose is optionally rewritten stack-agnostic via the `reasoning` chain for
//!   readability, then re-checked deterministically (the model must not
//!   reintroduce anything).
//!
//! Not here yet: [`promote`] (C8, receiving side) — the pipeline owner-files list
//! it. [`inbox`] (C7, the downstream distribution / Upgrades inbox) now lives
//! here alongside [`anonymize`].

pub mod anonymize;
pub mod inbox;
pub mod preferences;
