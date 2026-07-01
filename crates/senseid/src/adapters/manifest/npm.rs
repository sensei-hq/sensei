//! `NpmManifestAdapter` — parses `package.json`.
//!
//! Delegates the actual parsing work to the pure helpers in
//! `indexer::lib_indexer` (`parse_npm_deps`, `npm_local_source`) so this
//! step is genuinely a refactor: no duplicated logic and existing tests keep
//! covering the parser.

use super::{ManifestAdapter, ParsedManifest};
use crate::indexer::lib_indexer::{parse_npm_deps, DepVersion};

pub struct NpmManifestAdapter;

impl ManifestAdapter for NpmManifestAdapter {
    fn manifest_filenames(&self) -> &[&'static str] {
        &["package.json"]
    }

    fn ecosystem(&self) -> &'static str {
        "npm"
    }

    fn parse_dependencies(&self, content: &str) -> Vec<DepVersion> {
        let Ok(pkg) = serde_json::from_str::<serde_json::Value>(content) else {
            return Vec::new();
        };
        parse_npm_deps(&pkg)
    }

    fn is_workspace_root(&self, content: &str) -> bool {
        // Either an array under `workspaces` or an object with a `packages`
        // array (yarn workspaces-config format). Both count as workspace roots.
        let Ok(pkg) = serde_json::from_str::<serde_json::Value>(content) else {
            return false;
        };
        match pkg.get("workspaces") {
            Some(serde_json::Value::Array(a)) => !a.is_empty(),
            Some(serde_json::Value::Object(o)) => o
                .get("packages")
                .and_then(|v| v.as_array())
                .is_some_and(|a| !a.is_empty()),
            _ => false,
        }
    }

    fn parse_manifest(&self, content: &str) -> ParsedManifest {
        let Ok(pkg) = serde_json::from_str::<serde_json::Value>(content) else {
            return ParsedManifest::default();
        };
        ParsedManifest {
            name: pkg.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            version: pkg.get("version").and_then(|v| v.as_str()).map(|s| s.to_string()),
            description: pkg.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecosystem_and_filenames() {
        let a = NpmManifestAdapter;
        assert_eq!(a.ecosystem(), "npm");
        assert_eq!(a.manifest_filenames(), &["package.json"]);
    }

    #[test]
    fn parse_dependencies_matches_lib_indexer_parse_npm_deps() {
        let pkg = r#"{
            "dependencies": {
                "d3": "^7.0.0",
                "@rokkit/actions": "link:../actions"
            },
            "devDependencies": { "vitest": "^1.0.0" }
        }"#;
        let deps = NpmManifestAdapter.parse_dependencies(pkg);
        assert_eq!(deps.len(), 3);
        let by_name = |n: &str| deps.iter().find(|d| d.lib_name == n).unwrap();
        assert_eq!(by_name("d3").local_source, None);
        assert_eq!(by_name("@rokkit/actions").local_source, Some("../actions".into()));
        assert!(by_name("vitest").dev);
    }

    #[test]
    fn parse_dependencies_returns_empty_for_invalid_json() {
        assert!(NpmManifestAdapter.parse_dependencies("{ not json").is_empty());
    }

    #[test]
    fn is_workspace_root_recognises_array_workspaces() {
        assert!(NpmManifestAdapter.is_workspace_root(r#"{ "workspaces": ["packages/*"] }"#));
    }

    #[test]
    fn is_workspace_root_recognises_object_workspaces_packages() {
        assert!(NpmManifestAdapter
            .is_workspace_root(r#"{ "workspaces": { "packages": ["packages/*"] } }"#));
    }

    #[test]
    fn is_workspace_root_false_for_no_workspaces_field() {
        assert!(!NpmManifestAdapter.is_workspace_root(r#"{ "name": "x" }"#));
    }

    #[test]
    fn is_workspace_root_false_for_empty_workspaces_array() {
        assert!(!NpmManifestAdapter.is_workspace_root(r#"{ "workspaces": [] }"#));
    }

    #[test]
    fn is_workspace_root_false_for_invalid_json() {
        assert!(!NpmManifestAdapter.is_workspace_root("{ not json"));
    }

    #[test]
    fn parse_manifest_extracts_name_version_description() {
        let pkg = r#"{
            "name": "@rokkit/ui",
            "version": "1.3.1",
            "description": "Rokkit UI components"
        }"#;
        let p = NpmManifestAdapter.parse_manifest(pkg);
        assert_eq!(p.name.as_deref(), Some("@rokkit/ui"));
        assert_eq!(p.version.as_deref(), Some("1.3.1"));
        assert_eq!(p.description.as_deref(), Some("Rokkit UI components"));
    }

    #[test]
    fn parse_manifest_defaults_for_missing_fields() {
        let p = NpmManifestAdapter.parse_manifest(r#"{}"#);
        assert_eq!(p, ParsedManifest::default());
    }
}
