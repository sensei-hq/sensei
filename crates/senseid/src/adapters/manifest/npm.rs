//! `NpmManifestAdapter` — parses `package.json`.
//!
//! Delegates the actual parsing work to the pure helpers in
//! `indexer::lib_indexer` (`parse_npm_deps`, `npm_local_source`) so this
//! step is genuinely a refactor: no duplicated logic and existing tests keep
//! covering the parser.

use super::workspace::{extract_npm_workspace_patterns, resolve_glob_members};
use super::{FsSignals, ManifestAdapter, ParsedManifest};
use crate::indexer::lib_indexer::{DepVersion, parse_npm_deps};
use crate::types::PackageInfo;
use std::path::Path;

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
            Some(serde_json::Value::Object(o)) => {
                o.get("packages").and_then(|v| v.as_array()).is_some_and(|a| !a.is_empty())
            }
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

    fn stack_labels(&self, content: &str) -> Vec<&'static str> {
        // Framework-detection precedence matches the legacy inline block in
        // scan_logic.rs so historical outputs stay stable. Cheap substring
        // check on the raw text — the pattern is unambiguous enough in practice
        // (a dep named "svelte" is a Svelte project) and avoids parsing JSON
        // for every scanned folder.
        if content.contains("\"svelte\"") || content.contains("\"@sveltejs/kit\"") {
            vec!["svelte"]
        } else if content.contains("\"react\"") {
            vec!["react"]
        } else if content.contains("\"vue\"") {
            vec!["vue"]
        } else if content.contains("\"next\"") {
            vec!["nextjs"]
        } else {
            vec!["typescript"]
        }
    }

    fn infer_role(
        &self,
        parsed: &ParsedManifest,
        content: &str,
        fs: &FsSignals,
    ) -> Option<&'static str> {
        // 1. Tool — a node package that ships a CLI (`bin` field).
        if content.contains("\"bin\"") {
            return Some("tool");
        }

        // 2. Website — a web-app framework (checked before library: an app's
        //    package.json also has a name, but it is not a publishable lib).
        //    For SvelteKit/Vite require an actual `src/routes` app tree.
        let is_web_app = content.contains("\"next\"")
            || content.contains("\"astro\"")
            || (fs.has_src_routes
                && (content.contains("\"@sveltejs/kit\"") || content.contains("\"vite\"")));
        if is_web_app {
            return Some("website");
        }

        // 3. Library — publishable package (has a name plus an entry point).
        let node_lib = parsed.name.is_some()
            && (content.contains("\"exports\"")
                || content.contains("\"main\"")
                || content.contains("\"module\":")
                || content.contains("\"svelte\""));
        if node_lib {
            return Some("library");
        }
        None
    }

    fn detect_workspace_members(&self, repo_root: &Path) -> Vec<PackageInfo> {
        let mut members = Vec::new();

        // 1. Root package.json declares `workspaces`
        if let Ok(content) = std::fs::read_to_string(repo_root.join("package.json"))
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
        {
            for pattern in extract_npm_workspace_patterns(&json) {
                for entry in resolve_glob_members(repo_root, &pattern) {
                    if let Some(member) = package_json_member(repo_root, &entry) {
                        members.push(member);
                    }
                }
            }
        }

        // 2. pnpm-workspace.yaml — only used if no root package.json workspaces
        if members.is_empty()
            && let Ok(content) = std::fs::read_to_string(repo_root.join("pnpm-workspace.yaml"))
            && let Ok(yaml) = serde_yaml::from_str::<serde_json::Value>(&content)
            && let Some(packages) = yaml.get("packages").and_then(|p| p.as_array())
        {
            for p in packages {
                if let Some(pattern) = p.as_str() {
                    for entry in resolve_glob_members(repo_root, pattern) {
                        if let Some(member) = package_json_member(repo_root, &entry) {
                            members.push(member);
                        }
                    }
                }
            }
        }

        members
    }

    /// Read `scripts` from `package.json` and emit one DiscoveredCommand
    /// per entry (#83). `command_line` is the standard `<pm> run <name>`
    /// form — we don't know if the user prefers bun/npm/pnpm here, so we
    /// leave that to the caller (the scan pipeline picks based on lock
    /// file). Category is derived from the script name via the shared
    /// [`command_category::categorise`] classifier.
    fn parse_commands(&self, content: &str) -> Vec<super::DiscoveredCommand> {
        let Ok(pkg) = serde_json::from_str::<serde_json::Value>(content) else { return Vec::new() };
        let Some(scripts) = pkg.get("scripts").and_then(|v| v.as_object()) else {
            return Vec::new();
        };
        scripts
            .iter()
            .filter_map(|(name, _value)| {
                if name.is_empty() {
                    return None;
                }
                Some(super::DiscoveredCommand {
                    raw_name: name.clone(),
                    // Package-manager-agnostic. The runner picks bun/npm/pnpm
                    // at exec time based on lockfile presence.
                    command_line: format!("npm run {name}"),
                    category: super::command_category::categorise(name),
                })
            })
            .collect()
    }
}

/// Build a `PackageInfo` for a workspace-member folder that contains a
/// `package.json`. Returns `None` if the folder has no `package.json`.
fn package_json_member(repo_root: &Path, rel_path: &str) -> Option<PackageInfo> {
    let pkg_json = repo_root.join(rel_path).join("package.json");
    if !pkg_json.exists() {
        return None;
    }
    let pkg = std::fs::read_to_string(&pkg_json)
        .ok()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok());
    let name = pkg
        .as_ref()
        .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| rel_path.to_string());
    let version =
        pkg.as_ref().and_then(|v| v.get("version").and_then(|n| n.as_str()).map(|s| s.to_string()));
    let private =
        pkg.as_ref().and_then(|v| v.get("private").and_then(|b| b.as_bool())).unwrap_or(false);
    Some(PackageInfo {
        name,
        path: rel_path.to_string(),
        version,
        pkg_type: "npm_workspace".to_string(),
        private,
    })
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

    // ── #83 commands surface — parse_commands ──────────────────────────────

    #[test]
    fn parse_commands_extracts_scripts_and_categorises() {
        let a = NpmManifestAdapter;
        let pkg = r#"{
            "name": "app",
            "scripts": {
                "test": "vitest",
                "test:e2e": "playwright test",
                "build": "vite build",
                "lint": "eslint src",
                "custom": "node custom.js"
            }
        }"#;
        let cmds = a.parse_commands(pkg);
        assert_eq!(cmds.len(), 5, "one row per script");

        // Category classification hits the shared vocabulary.
        let by_name: std::collections::HashMap<&str, &super::super::DiscoveredCommand> =
            cmds.iter().map(|c| (c.raw_name.as_str(), c)).collect();
        assert_eq!(by_name["test"].category, Some("test"));
        assert_eq!(by_name["test:e2e"].category, Some("e2e"), "e2e wins over test");
        assert_eq!(by_name["build"].category, Some("build"));
        assert_eq!(by_name["lint"].category, Some("lint"));
        assert_eq!(by_name["custom"].category, None, "unclassified stays None");

        // Command line uses the package-manager-agnostic form.
        assert_eq!(by_name["test"].command_line, "npm run test");
        assert_eq!(by_name["test:e2e"].command_line, "npm run test:e2e");
    }

    #[test]
    fn parse_commands_empty_when_scripts_missing() {
        let a = NpmManifestAdapter;
        assert!(a.parse_commands(r#"{"name":"app"}"#).is_empty());
    }

    #[test]
    fn parse_commands_empty_when_invalid_json() {
        let a = NpmManifestAdapter;
        assert!(a.parse_commands("not json").is_empty());
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
        assert!(
            NpmManifestAdapter
                .is_workspace_root(r#"{ "workspaces": { "packages": ["packages/*"] } }"#)
        );
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

    #[test]
    fn stack_labels_svelte_before_react_vue_next_typescript() {
        // A svelte package.json → "svelte" even when other deps are present.
        assert_eq!(
            NpmManifestAdapter.stack_labels(r#"{"dependencies":{"svelte":"^5"}}"#),
            vec!["svelte"]
        );
        assert_eq!(
            NpmManifestAdapter.stack_labels(r#"{"devDependencies":{"@sveltejs/kit":"^2"}}"#),
            vec!["svelte"]
        );
        assert_eq!(
            NpmManifestAdapter.stack_labels(r#"{"dependencies":{"react":"^18"}}"#),
            vec!["react"]
        );
        assert_eq!(
            NpmManifestAdapter.stack_labels(r#"{"dependencies":{"vue":"^3"}}"#),
            vec!["vue"]
        );
        assert_eq!(
            NpmManifestAdapter.stack_labels(r#"{"dependencies":{"next":"^14"}}"#),
            vec!["nextjs"]
        );
        // No framework markers → generic typescript label.
        assert_eq!(NpmManifestAdapter.stack_labels(r#"{"name":"plain"}"#), vec!["typescript"]);
    }

    #[test]
    fn infer_role_tool_from_bin_field() {
        let content = r#"{"name":"c","bin":{"c":"./c.js"}}"#;
        let parsed = NpmManifestAdapter.parse_manifest(content);
        let fs = FsSignals::default();
        assert_eq!(NpmManifestAdapter.infer_role(&parsed, content, &fs), Some("tool"));
    }

    #[test]
    fn infer_role_website_from_sveltekit_with_routes() {
        let content = r#"{"name":"learn","devDependencies":{"@sveltejs/kit":"^2"}}"#;
        let parsed = NpmManifestAdapter.parse_manifest(content);
        let fs = FsSignals { has_src_routes: true, ..Default::default() };
        assert_eq!(NpmManifestAdapter.infer_role(&parsed, content, &fs), Some("website"));
    }

    #[test]
    fn infer_role_library_when_no_routes_even_with_sveltekit_dep() {
        // A UnoCSS preset that peer-deps @sveltejs/kit but has no `src/routes`
        // is a library, not a website.
        let content = r#"{"name":"@rokkit/unocss","exports":{".":"./i.js"},"peerDependencies":{"@sveltejs/kit":"^2"}}"#;
        let parsed = NpmManifestAdapter.parse_manifest(content);
        let fs = FsSignals::default();
        assert_eq!(NpmManifestAdapter.infer_role(&parsed, content, &fs), Some("library"));
    }

    #[test]
    fn infer_role_none_when_name_present_but_no_entry_point() {
        // `type:module` alone is not enough — needs exports/main/module/svelte.
        let content = r#"{"name":"x","type":"module"}"#;
        let parsed = NpmManifestAdapter.parse_manifest(content);
        assert_eq!(NpmManifestAdapter.infer_role(&parsed, content, &FsSignals::default()), None);
    }
}
