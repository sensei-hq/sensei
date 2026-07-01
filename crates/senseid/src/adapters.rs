//! Sibling to `LanguageAdapter` — adapters for the manifest / config / build
//! layer.
//!
//! Each ecosystem (npm, cargo, pyproject, go) implements a `ManifestAdapter`
//! and is registered in the small dispatch table at the bottom. The rest of
//! the daemon consults `manifest_adapter_for_filename` instead of hardcoding
//! `if path == "package.json"` chains — see the 2026-07-01 gap analysis for
//! the ten spaghetti sites this replaces.
//!
//! T1b Step 3 lands the trait skeleton + `NpmManifestAdapter`. Steps 4/5/6
//! add cargo / pyproject / go.mod impls. Step 7 migrates
//! `indexer::lib_indexer::extract_dep_versions` to route through the
//! dispatch. Later steps migrate the workspace / stack / role / metadata
//! callers.

pub mod config;
pub mod manifest;
