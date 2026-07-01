//! `CargoManifestAdapter` — parses `Cargo.toml`.
//!
//! Delegates dep parsing to `indexer::lib_indexer::parse_cargo_deps` so this
//! step remains a refactor. Workspace detection reads the `[workspace]`
//! section directly.

use super::{ManifestAdapter, ParsedManifest};
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
}
