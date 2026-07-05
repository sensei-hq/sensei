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
use crate::types::PackageInfo;
use std::path::Path;

mod cargo;
mod go;
mod maven;
mod npm;
mod pyproject;
pub(crate) mod workspace;

/// Adapter for a specific ecosystem's manifest format.
pub trait ManifestAdapter: Send + Sync {
    /// Manifest filenames this adapter recognises (typically one).
    fn manifest_filenames(&self) -> &[&'static str];

    /// Ecosystem slug matching the `sensei.library_ecosystem` DDL enum.
    /// One of `"npm"`, `"cargo"`, `"pypi"`, `"go"`, `"maven"`.
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

    /// Stack labels this manifest indicates (e.g. `["rust"]`, `["svelte"]`,
    /// `["typescript"]`, `["python"]`, `["go"]`). Framework detection (React,
    /// Svelte, Vue, Next) for npm happens here so the label vocabulary lives
    /// with the ecosystem that owns it. Default: the ecosystem slug.
    fn stack_labels(&self, _content: &str) -> Vec<&'static str> {
        vec![self.ecosystem()]
    }

    /// Infer a folder's semantic `folder_role` (`"library"` / `"tool"` /
    /// `"website"` / `"docs"`) from its manifest and filesystem signals.
    /// Returns `None` to leave the role unset. Default: `None` — ecosystems
    /// without a role-inference rule (Go, Python today) don't classify.
    fn infer_role(
        &self,
        _parsed: &ParsedManifest,
        _content: &str,
        _fs: &FsSignals,
    ) -> Option<&'static str> {
        None
    }

    /// Enumerate workspace-member packages of a repo root. Default: none —
    /// ecosystems that don't have a workspace concept (pyproject today) return
    /// an empty vector. Called by `config::detector::detect_workspace_members`.
    fn detect_workspace_members(&self, _repo_root: &Path) -> Vec<PackageInfo> {
        Vec::new()
    }
}

/// Identity metadata for a manifest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedManifest {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}

/// Filesystem-side signals the caller supplies to `infer_role`. Kept in one
/// struct so the trait method stays stable when new signals are added.
#[derive(Debug, Clone, Default)]
pub struct FsSignals {
    /// `src/lib.rs` exists (Rust library entry point).
    pub has_lib_rs: bool,
    /// `src/routes/` exists (SvelteKit / web-app route tree).
    pub has_src_routes: bool,
    /// Folder's own directory name — reserved for future adapters that
    /// classify by folder name (e.g. `docs/` → docs). Today the docs fallback
    /// lives at the `classify_role` layer since it isn't tied to any manifest.
    #[allow(dead_code)]
    pub dir_name: String,
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
pub fn registered_adapters() -> &'static [&'static dyn ManifestAdapter] {
    &[
        &npm::NpmManifestAdapter,
        &cargo::CargoManifestAdapter,
        &pyproject::PyprojectManifestAdapter,
        &go::GoManifestAdapter,
        &maven::MavenManifestAdapter,
    ]
}

/// Every distinct manifest filename any registered adapter recognises. Used by
/// filesystem walks (subproject discovery, manifest scanning) that need to
/// know "is there a known manifest in this directory?" without knowing which
/// ecosystem it belongs to.
pub fn all_manifest_filenames() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = registered_adapters()
        .iter()
        .flat_map(|a| a.manifest_filenames().iter().copied())
        .collect();
    out.sort();
    out.dedup();
    out
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

    #[test]
    fn dispatch_returns_maven_for_pom_xml() {
        let a = manifest_adapter_for_filename("pom.xml").unwrap();
        assert_eq!(a.ecosystem(), "maven");
    }

    #[test]
    fn all_manifest_filenames_includes_every_ecosystem() {
        let names = all_manifest_filenames();
        for expected in ["Cargo.toml", "package.json", "pyproject.toml", "go.mod", "pom.xml"] {
            assert!(names.contains(&expected), "missing {expected} in {names:?}");
        }
    }
}
