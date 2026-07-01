// Scaffolding: consumers wire in during Steps 7+ (extract_dep_versions
// migration, scan_logic/detector migrations). Remove this allow when the
// first non-test caller lands.
#![allow(dead_code)]

//! `ManifestAdapter` trait + ecosystem impls.
//!
//! The daemon used to switch on manifest filenames inline in ten places
//! (`libraries.rs`, `scan_logic.rs`, `detector.rs`, `external_links.rs`,
//! `summary.rs`, etc.). This module holds one adapter per ecosystem and a
//! single dispatch function.
//!
//! Dispatch is static — `manifest_adapter_for_filename(filename)` returns a
//! `&'static dyn ManifestAdapter` when the filename is recognised, and each
//! impl is a zero-sized type so there's no allocation on the hot path.

use crate::indexer::lib_indexer::DepVersion;

mod cargo;
mod go;
mod npm;
mod pyproject;

/// Adapter for a specific ecosystem's manifest format.
pub trait ManifestAdapter: Send + Sync {
    /// Manifest filenames this adapter recognises (typically one).
    fn manifest_filenames(&self) -> &[&'static str];

    /// Ecosystem slug matching the `sensei.library_ecosystem` DDL enum.
    /// One of `"npm"`, `"cargo"`, `"pypi"`, `"go"`.
    fn ecosystem(&self) -> &'static str;

    /// Parse the raw contents of a manifest into dependency entries.
    ///
    /// Delegates the local-source protocol detection to shared helpers so
    /// npm `link:`/`workspace:`/`file:` and Cargo `path=` all set
    /// `DepVersion.local_source` the same way.
    fn parse_dependencies(&self, content: &str) -> Vec<DepVersion>;

    /// True if this manifest content declares a workspace root (npm
    /// `workspaces`, `Cargo.toml [workspace]`, `pnpm-workspace.yaml`, `go.work`).
    fn is_workspace_root(&self, content: &str) -> bool;

    /// Parse identity metadata (name, version, description) from a manifest.
    fn parse_manifest(&self, content: &str) -> ParsedManifest;
}

/// Identity metadata for a manifest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedManifest {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}

/// Pick the adapter for a manifest filename.
///
/// Returns `None` for unknown filenames — callers fall back to the legacy
/// path until every ecosystem is migrated.
pub fn manifest_adapter_for_filename(filename: &str) -> Option<&'static dyn ManifestAdapter> {
    for a in registered_adapters() {
        if a.manifest_filenames().contains(&filename) {
            return Some(*a);
        }
    }
    None
}

/// All registered ManifestAdapter impls. Add new ecosystems here.
fn registered_adapters() -> &'static [&'static dyn ManifestAdapter] {
    &[
        &npm::NpmManifestAdapter,
        &cargo::CargoManifestAdapter,
        &pyproject::PyprojectManifestAdapter,
        &go::GoManifestAdapter,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_returns_npm_for_package_json() {
        let a = manifest_adapter_for_filename("package.json").unwrap();
        assert_eq!(a.ecosystem(), "npm");
    }

    #[test]
    fn dispatch_returns_none_for_unknown_filename() {
        assert!(manifest_adapter_for_filename("BUILD.bazel").is_none());
        assert!(manifest_adapter_for_filename("").is_none());
    }

    #[test]
    fn dispatch_returns_cargo_for_cargo_toml() {
        let a = manifest_adapter_for_filename("Cargo.toml").unwrap();
        assert_eq!(a.ecosystem(), "cargo");
    }

    #[test]
    fn dispatch_returns_pyproject_for_pyproject_toml() {
        let a = manifest_adapter_for_filename("pyproject.toml").unwrap();
        assert_eq!(a.ecosystem(), "pypi");
    }

    #[test]
    fn dispatch_returns_go_for_go_mod() {
        let a = manifest_adapter_for_filename("go.mod").unwrap();
        assert_eq!(a.ecosystem(), "go");
    }
}
