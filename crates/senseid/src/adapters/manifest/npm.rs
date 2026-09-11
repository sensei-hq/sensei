//! `NpmManifestAdapter` — parses `package.json`.
//!
//! Delegates the actual parsing work to the pure helpers in
//! `indexer::lib_indexer` (`parse_npm_deps`, `npm_local_source`) so this
//! step is genuinely a refactor: no duplicated logic and existing tests keep
//! covering the parser.

use super::workspace::{extract_npm_workspace_patterns, resolve_glob_members};
use super::{FsSignals, ManifestAdapter, ParsedManifest, PinnedVersion};
use crate::indexer::lib_indexer::{DepVersion, parse_npm_deps};
use crate::types::PackageInfo;
use std::path::Path;

pub struct NpmManifestAdapter;

impl ManifestAdapter for NpmManifestAdapter {
    fn manifest_filenames(&self) -> &[&'static str] {
        &["package.json"]
    }

    fn lockfile_filenames(&self) -> &[&'static str] {
        // `bun.lockb` is deliberately absent — it is a BINARY format, and
        // listing it would claim a reader this adapter does not have.
        &["package-lock.json", "bun.lock", "npm-shrinkwrap.json"]
    }

    /// npm has several lockfile formats sharing no grammar, so the FILENAME
    /// selects the reader.
    ///
    /// - `bun.lock` — JSONC. `packages` maps a bare name to an array whose
    ///   first element is `name@version`.
    /// - `package-lock.json` / `npm-shrinkwrap.json` — strict JSON. `packages`
    ///   maps an INSTALL PATH (`node_modules/x`) to an object with `version`.
    ///
    /// `yarn.lock` and `pnpm-lock.yaml` are not JSON at all and are not
    /// claimed above; a repo using one gets manifest ranges until a reader
    /// for it exists, which is a named gap rather than a silent wrong answer.
    fn parse_lockfile(&self, filename: &str, content: &str) -> Vec<PinnedVersion> {
        match filename {
            "bun.lock" => parse_bun_lock(content),
            "package-lock.json" | "npm-shrinkwrap.json" => parse_npm_lock(content),
            _ => Vec::new(),
        }
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

    fn root_package(&self, repo_root: &Path) -> Option<PackageInfo> {
        // Same reuse as cargo: `package_json_member` already reads
        // `"private": true`, which is what excludes rokkit's and kavach's
        // roots while keeping a publishable one.
        package_json_member(repo_root, "")
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

/// Split `name@version` from the RIGHT.
///
/// A scoped package starts with `@` (`@rokkit/ui@1.4.1`), so splitting on the
/// first `@` gives an empty name and `rokkit/ui@1.4.1` as the version. Only
/// the last `@` separates the two.
fn split_name_at_version(spec: &str) -> Option<(String, String)> {
    let at = spec.rfind('@').filter(|i| *i > 0)?;
    Some((spec[..at].to_string(), spec[at + 1..].to_string()))
}

/// Strip JSONC to JSON: line comments and trailing commas, both of which
/// `serde_json` rejects outright.
///
/// Bun writes trailing commas. Without this a real `bun.lock` parses to
/// nothing and every npm version silently stays a manifest range floor —
/// the failure looks exactly like "this project has no dependencies".
/// Quote- and escape-aware, so a comma or `//` inside a string survives.
fn jsonc_to_json(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut in_str = false;
    let mut escaped = false;
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if in_str {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_str = true;
                out.push(c);
            }
            '/' if chars.peek() == Some(&'/') => {
                for n in chars.by_ref() {
                    if n == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ',' => {
                // Look ahead past whitespace: a comma before `}` or `]` is
                // trailing and must go.
                let mut ws = String::new();
                while let Some(&n) = chars.peek() {
                    if n.is_whitespace() {
                        ws.push(n);
                        chars.next();
                    } else {
                        break;
                    }
                }
                match chars.peek() {
                    Some('}') | Some(']') => out.push_str(&ws),
                    _ => {
                        out.push(',');
                        out.push_str(&ws);
                    }
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Order pins by name, then version — NOT by the order the file happened to
/// list them in (R6/A6). Callers index by name so nothing depends on the
/// order functionally, which is exactly why an unstable one would go
/// unnoticed until two runs of the same input produced different output.
/// A lockfile entry whose "version" is a PROTOCOL, not a release.
///
/// bun records a workspace member as `kavach@link:kavach`, so splitting on the
/// last `@` yields `link:kavach` — which then travels as a pin and lands in
/// `library_versions` as a version key no dependency can ever match. MEASURED:
/// exactly that row existed.
///
/// These entries are first-party siblings, which `local_source` already routes
/// to `folder_dependencies`; they are not registry releases and do not belong
/// in a pin table at all.
fn is_protocol_not_a_version(v: &str) -> bool {
    ["link:", "workspace:", "file:", "portal:", "npm:", "patch:"].iter().any(|p| v.starts_with(p))
}

fn sort_pins(pins: &mut [PinnedVersion]) {
    pins.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
}

/// `bun.lock`: `packages` maps a bare name to `[ "name@version", ... ]`.
fn parse_bun_lock(content: &str) -> Vec<PinnedVersion> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&jsonc_to_json(content)) else {
        return Vec::new();
    };
    let Some(packages) = v.get("packages").and_then(|p| p.as_object()) else {
        return Vec::new();
    };
    let mut pins: Vec<PinnedVersion> = packages
        .iter()
        .filter_map(|(_, entry)| {
            let spec = entry.as_array()?.first()?.as_str()?;
            let (name, version) = split_name_at_version(spec)?;
            if is_protocol_not_a_version(&version) {
                return None;
            }
            Some(PinnedVersion { name, version })
        })
        .collect();
    sort_pins(&mut pins);
    pins
}

/// `package-lock.json` v2/v3: `packages` maps an INSTALL PATH to an object
/// carrying `version`. The root project is the empty key and is skipped — it
/// is not a dependency of itself.
fn parse_npm_lock(content: &str) -> Vec<PinnedVersion> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(content) else {
        return Vec::new();
    };
    let Some(packages) = v.get("packages").and_then(|p| p.as_object()) else {
        return Vec::new();
    };
    let mut pins: Vec<PinnedVersion> = packages
        .iter()
        .filter_map(|(path, entry)| {
            // "node_modules/a/node_modules/b" is package b. Take the segment
            // after the LAST `node_modules/`, which also keeps a scope intact.
            let name = path.rsplit_once("node_modules/").map(|(_, n)| n)?;
            if name.is_empty() {
                return None;
            }
            Some(PinnedVersion {
                name: name.to_string(),
                version: entry.get("version")?.as_str()?.to_string(),
            })
        })
        .collect();
    sort_pins(&mut pins);
    pins
}

#[cfg(test)]
mod lockfile_tests {
    use super::*;
    use crate::adapters::manifest::PinnedVersion;

    fn bun(content: &str) -> Vec<PinnedVersion> {
        NpmManifestAdapter.parse_lockfile("bun.lock", content)
    }
    fn npm(content: &str) -> Vec<PinnedVersion> {
        NpmManifestAdapter.parse_lockfile("package-lock.json", content)
    }

    #[test]
    fn bun_lock_reads_the_version_off_the_name_at_version_entry() {
        // bun.lock's `packages` map keys on the bare name; the version lives
        // inside the first array element as `name@version`.
        let lock = r#"{
  "lockfileVersion": 1,
  "packages": {
    "@rokkit/ui": ["@rokkit/ui@1.4.1", "", {}, "sha512-x"],
    "svelte": ["svelte@5.39.5", "", {}, "sha512-y"]
  }
}"#;
        assert_eq!(
            bun(lock),
            vec![
                PinnedVersion { name: "@rokkit/ui".into(), version: "1.4.1".into() },
                PinnedVersion { name: "svelte".into(), version: "5.39.5".into() },
            ]
        );
    }

    #[test]
    fn bun_lock_tolerates_trailing_commas_because_it_is_jsonc_not_json() {
        // Bun writes JSONC. `serde_json` rejects a trailing comma outright, so
        // a strict parse returns ZERO pins on a real file and every version
        // silently stays a manifest range floor.
        let lock = "{\n  \"packages\": {\n    \"a\": [\"a@1.0.0\", \"\", {}, \"sha\"],\n  },\n}";
        assert_eq!(bun(lock), vec![PinnedVersion { name: "a".into(), version: "1.0.0".into() }]);
    }

    #[test]
    fn a_scoped_name_keeps_its_at_and_only_the_last_at_splits_the_version() {
        // `@rokkit/ui@1.4.1` — splitting on the FIRST `@` yields an empty name
        // and "rokkit/ui@1.4.1" as the version.
        let lock = r#"{"packages": {"@scope/pkg": ["@scope/pkg@2.0.0", "", {}, "sha"]}}"#;
        assert_eq!(
            bun(lock),
            vec![PinnedVersion { name: "@scope/pkg".into(), version: "2.0.0".into() }]
        );
    }

    #[test]
    fn npm_lock_reads_versions_from_the_packages_map_keyed_by_node_modules_path() {
        // package-lock v2/v3 keys on the INSTALL PATH, not the package name.
        // The root entry has an empty key and no version — it is the project.
        let lock = r#"{
  "lockfileVersion": 3,
  "packages": {
    "": {"name": "root", "version": "1.0.0"},
    "node_modules/lodash": {"version": "4.17.21"},
    "node_modules/@babel/core": {"version": "7.24.0"}
  }
}"#;
        assert_eq!(
            npm(lock),
            vec![
                PinnedVersion { name: "@babel/core".into(), version: "7.24.0".into() },
                PinnedVersion { name: "lodash".into(), version: "4.17.21".into() },
            ]
        );
    }

    #[test]
    fn a_nested_node_modules_path_resolves_to_the_innermost_package_name() {
        // "node_modules/a/node_modules/b" is package b, not "a/node_modules/b".
        let lock = r#"{"packages": {"node_modules/a/node_modules/b": {"version": "2.0.0"}}}"#;
        assert_eq!(npm(lock), vec![PinnedVersion { name: "b".into(), version: "2.0.0".into() }]);
    }

    #[test]
    fn an_entry_with_no_version_is_skipped_not_defaulted() {
        let lock =
            r#"{"packages": {"node_modules/a": {}, "node_modules/b": {"version": "1.0.0"}}}"#;
        assert_eq!(npm(lock), vec![PinnedVersion { name: "b".into(), version: "1.0.0".into() }]);
    }

    #[test]
    fn a_workspace_entry_is_not_a_pin_because_its_version_is_a_protocol() {
        // bun writes `kavach@link:kavach` for a workspace member. Split on the
        // last `@` that reads as version `link:kavach`, which then travels as
        // a pin into `library_versions` — a version key nothing can match.
        let lock = r#"{"packages": {
            "kavach": ["kavach@link:kavach", "", {}, ""],
            "real":   ["real@1.2.3", "", {}, "sha"]
        }}"#;
        assert_eq!(bun(lock), vec![PinnedVersion { name: "real".into(), version: "1.2.3".into() }]);
    }

    #[test]
    fn a_malformed_lockfile_yields_nothing_rather_than_panicking() {
        assert!(bun("not json at all {{{").is_empty());
        assert!(npm("").is_empty());
    }

    #[test]
    fn it_accepts_the_four_npm_lockfiles_and_not_a_manifest() {
        for f in ["package-lock.json", "bun.lock"] {
            assert!(NpmManifestAdapter.accepts_lockfile(f), "{f}");
        }
        assert!(!NpmManifestAdapter.accepts_lockfile("Cargo.lock"));
        assert!(!NpmManifestAdapter.accepts("bun.lock"), "accepts() answers for manifests only");
    }
}
