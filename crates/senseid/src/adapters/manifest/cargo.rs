//! `CargoManifestAdapter` — parses `Cargo.toml`.
//!
//! Delegates dep parsing to `indexer::lib_indexer::parse_cargo_deps` so this
//! step remains a refactor. Workspace detection reads the `[workspace]`
//! section directly.

use super::{FsSignals, ManifestAdapter, ParsedManifest};
use crate::indexer::lib_indexer::{parse_cargo_deps, DepVersion};

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
        content
            .parse::<toml::Value>()
            .ok()
            .and_then(|v| v.get("workspace").cloned())
            .is_some()
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
            description: pkg
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
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
        assert_eq!(
            CargoManifestAdapter.stack_labels("[workspace]\nmembers=[]"),
            vec!["rust"]
        );
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
        assert_eq!(
            CargoManifestAdapter.infer_role(&parsed, lib_content, &fs),
            Some("library")
        );
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
