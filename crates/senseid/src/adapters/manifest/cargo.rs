//! `CargoManifestAdapter` — parses `Cargo.toml`.
//!
//! Delegates dep parsing to `indexer::lib_indexer::parse_cargo_deps` so this
//! step remains a refactor. Workspace detection reads the `[workspace]`
//! section directly.

use super::workspace::resolve_glob_members;
use super::{FsSignals, ManifestAdapter, ParsedManifest};
use crate::indexer::lib_indexer::{DepVersion, parse_cargo_deps};
use crate::types::PackageInfo;
use std::path::Path;

/// Common directories where standalone crates live in repos WITHOUT a root
/// `Cargo.toml` workspace. Scanning these lets us catch e.g. sensei's
/// `crates/*` layout even when the root Cargo isn't a workspace root.
const CARGO_FALLBACK_DIRS: &[&str] = &["crates", "rust", "lib", "services"];

pub struct CargoManifestAdapter;

impl ManifestAdapter for CargoManifestAdapter {
    fn manifest_filenames(&self) -> &[&'static str] {
        &["Cargo.toml"]
    }

    fn ecosystem(&self) -> &'static str {
        "cargo"
    }

    fn parse_dependencies(&self, content: &str) -> Vec<DepVersion> {
        let Ok(cargo) = content.parse::<toml::Value>() else {
            return Vec::new();
        };
        parse_cargo_deps(&cargo)
    }

    fn is_workspace_root(&self, content: &str) -> bool {
        content.parse::<toml::Value>().ok().and_then(|v| v.get("workspace").cloned()).is_some()
    }

    fn parse_manifest(&self, content: &str) -> ParsedManifest {
        let Ok(cargo) = content.parse::<toml::Value>() else {
            return ParsedManifest::default();
        };
        // A Cargo workspace root has no `[package]`; skip identity in that case.
        let Some(pkg) = cargo.get("package") else {
            return ParsedManifest::default();
        };
        ParsedManifest {
            name: pkg.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            version: pkg.get("version").and_then(|v| v.as_str()).map(|s| s.to_string()),
            description: pkg.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
        }
    }

    fn stack_labels(&self, _content: &str) -> Vec<&'static str> {
        vec!["rust"]
    }

    fn infer_role(
        &self,
        _parsed: &ParsedManifest,
        content: &str,
        fs: &FsSignals,
    ) -> Option<&'static str> {
        // Explicit [[bin]] target → tool. A bare `main.rs` is deliberately not
        // enough: a daemon (e.g. senseid, axum-based) is a binary too, so
        // requiring an explicit bin declaration avoids mislabeling a backend
        // service as a CLI tool.
        if content.contains("[[bin]]") {
            return Some("tool");
        }
        // Library crate: `src/lib.rs` present or explicit `[lib]` section.
        if fs.has_lib_rs || content.contains("[lib]") {
            return Some("library");
        }
        None
    }

    fn detect_workspace_members(&self, repo_root: &Path) -> Vec<PackageInfo> {
        let mut members = Vec::new();
        let mut declared_member_paths: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        // 1. `[workspace]` members declared in the root Cargo.toml.
        if let Ok(content) = std::fs::read_to_string(repo_root.join("Cargo.toml"))
            && let Ok(toml_val) = content.parse::<toml::Value>()
            && let Some(workspace) = toml_val.get("workspace")
            && let Some(ws_members) = workspace.get("members").and_then(|m| m.as_array())
        {
            for m in ws_members {
                if let Some(pattern) = m.as_str() {
                    for entry in resolve_glob_members(repo_root, pattern) {
                        if let Some(member) = cargo_toml_member(repo_root, &entry) {
                            declared_member_paths.insert(entry.clone());
                            members.push(member);
                        }
                    }
                }
            }
        }

        // 2. Standalone crates: scan common dirs for `Cargo.toml` even without
        //    a root workspace. Catches repos like sensei where `crates/` has
        //    Rust packages but the top-level manifest isn't a workspace.
        //    Skip any path already registered as a workspace member above.
        for dir_name in CARGO_FALLBACK_DIRS {
            let dir = repo_root.join(dir_name);
            if !dir.is_dir() {
                continue;
            }
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                if !entry.path().is_dir() {
                    continue;
                }
                if !entry.path().join("Cargo.toml").exists() {
                    continue;
                }
                let rel_path = format!("{}/{}", dir_name, entry.file_name().to_string_lossy());
                if declared_member_paths.contains(&rel_path) {
                    continue;
                }
                if let Some(member) = cargo_toml_member(repo_root, &rel_path) {
                    // Fallback crates without a `[package]` name are ignored
                    // to keep parity with the pre-refactor detector.
                    if member.name != rel_path {
                        members.push(member);
                    }
                }
            }
        }

        members
    }

    /// Conventional Cargo verbs — every Cargo.toml presents the same set
    /// (clippy is standard-issue enough to count). Custom `.cargo/config.toml`
    /// aliases could be layered in via a follow-up; the conventional set is
    /// what covers 90%+ of "how do I test this Rust project" answers.
    fn parse_commands(&self, _content: &str) -> Vec<super::DiscoveredCommand> {
        super::conventional_commands(
            "cargo",
            &[
                ("test", "test"),
                ("build", "build"),
                ("check", "typecheck"),
                ("clippy", "lint"),
                ("fmt", "format"),
                ("bench", "bench"),
                ("doc", "docs"),
                ("run", "run"),
            ],
        )
    }
}

/// A Cargo package is non-publishable when `publish = false` or `publish = []`.
fn cargo_publish_disabled(pkg: Option<&toml::Value>) -> bool {
    match pkg.and_then(|p| p.get("publish")) {
        Some(toml::Value::Boolean(false)) => true,
        Some(toml::Value::Array(a)) => a.is_empty(),
        _ => false,
    }
}

/// Build a `PackageInfo` for a workspace-member folder that contains a
/// `Cargo.toml`. Returns `None` if the folder has no `Cargo.toml`.
fn cargo_toml_member(repo_root: &Path, rel_path: &str) -> Option<PackageInfo> {
    let cargo_toml = repo_root.join(rel_path).join("Cargo.toml");
    if !cargo_toml.exists() {
        return None;
    }
    let manifest =
        std::fs::read_to_string(&cargo_toml).ok().and_then(|c| c.parse::<toml::Value>().ok());
    let pkg = manifest.as_ref().and_then(|v| v.get("package"));
    let name = pkg
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| rel_path.to_string());
    let version =
        pkg.and_then(|p| p.get("version")).and_then(|n| n.as_str()).map(|s| s.to_string());
    let private = cargo_publish_disabled(pkg);
    Some(PackageInfo {
        name,
        path: rel_path.to_string(),
        version,
        pkg_type: "cargo_crate".to_string(),
        private,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecosystem_and_filenames() {
        assert_eq!(CargoManifestAdapter.ecosystem(), "cargo");
        assert_eq!(CargoManifestAdapter.manifest_filenames(), &["Cargo.toml"]);
    }

    #[test]
    fn parse_dependencies_tags_path_deps_and_reads_string_versions() {
        let src = r#"
            [dependencies]
            serde = "1.0"
            gateway = { path = "../gateway" }
            tokio = { version = "1" }

            [dev-dependencies]
            tempfile = "3"
        "#;
        let deps = CargoManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 4);
        let by_name = |n: &str| deps.iter().find(|d| d.lib_name == n).unwrap();
        assert!(by_name("serde").local_source.is_none());
        assert_eq!(by_name("gateway").local_source.as_deref(), Some("../gateway"));
        assert!(by_name("tokio").local_source.is_none());
        assert!(by_name("tempfile").dev);
    }

    #[test]
    fn parse_dependencies_empty_for_invalid_toml() {
        assert!(CargoManifestAdapter.parse_dependencies("not toml [").is_empty());
    }

    #[test]
    fn is_workspace_root_true_for_workspace_section() {
        let src = r#"
            [workspace]
            members = ["crates/*"]
        "#;
        assert!(CargoManifestAdapter.is_workspace_root(src));
    }

    #[test]
    fn is_workspace_root_false_for_package_only_manifest() {
        let src = r#"
            [package]
            name = "x"
            version = "1.0"
        "#;
        assert!(!CargoManifestAdapter.is_workspace_root(src));
    }

    #[test]
    fn is_workspace_root_false_for_invalid_toml() {
        assert!(!CargoManifestAdapter.is_workspace_root("not toml ["));
    }

    #[test]
    fn parse_manifest_extracts_package_metadata() {
        let src = r#"
            [package]
            name = "senseid"
            version = "0.2.23"
            description = "Sensei indexer daemon"
        "#;
        let p = CargoManifestAdapter.parse_manifest(src);
        assert_eq!(p.name.as_deref(), Some("senseid"));
        assert_eq!(p.version.as_deref(), Some("0.2.23"));
        assert_eq!(p.description.as_deref(), Some("Sensei indexer daemon"));
    }

    #[test]
    fn parse_manifest_default_for_workspace_root() {
        // A workspace root has no `[package]` — identity is undefined.
        let src = r#"[workspace]
            members = ["crates/*"]
        "#;
        assert_eq!(CargoManifestAdapter.parse_manifest(src), ParsedManifest::default());
    }

    #[test]
    fn stack_labels_always_rust() {
        assert_eq!(CargoManifestAdapter.stack_labels(""), vec!["rust"]);
        assert_eq!(CargoManifestAdapter.stack_labels("[workspace]\nmembers=[]"), vec!["rust"]);
    }

    #[test]
    fn infer_role_tool_from_explicit_bin() {
        let content = "[package]\nname=\"c\"\n\n[[bin]]\nname=\"c\"";
        let parsed = CargoManifestAdapter.parse_manifest(content);
        let fs = FsSignals::default();
        assert_eq!(CargoManifestAdapter.infer_role(&parsed, content, &fs), Some("tool"));
    }

    #[test]
    fn infer_role_library_from_lib_rs_or_lib_section() {
        let content = "[package]\nname=\"core\"";
        let parsed = CargoManifestAdapter.parse_manifest(content);
        let fs = FsSignals { has_lib_rs: true, ..Default::default() };
        assert_eq!(CargoManifestAdapter.infer_role(&parsed, content, &fs), Some("library"));

        let lib_content = "[package]\nname=\"c\"\n\n[lib]";
        let parsed = CargoManifestAdapter.parse_manifest(lib_content);
        let fs = FsSignals::default();
        assert_eq!(CargoManifestAdapter.infer_role(&parsed, lib_content, &fs), Some("library"));
    }

    #[test]
    fn infer_role_none_for_service_daemon_without_bin_or_lib() {
        // A daemon binary (main.rs, no [[bin]]/[lib], server deps) stays unclassified.
        let content = "[package]\nname=\"senseid\"\n\n[dependencies]\naxum = \"0.7\"";
        let parsed = CargoManifestAdapter.parse_manifest(content);
        let fs = FsSignals::default();
        assert_eq!(CargoManifestAdapter.infer_role(&parsed, content, &fs), None);
    }
}
