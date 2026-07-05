//! `MavenManifestAdapter` — parses `pom.xml` (#86).
//!
//! Maven's manifest is XML, but the shape sensei cares about is well-scoped
//! and repetitive: top-level `<groupId>` / `<artifactId>` / `<version>` /
//! `<description>` / `<packaging>`, a `<dependencies>` list of `<dependency>`
//! entries, and (for multi-module reactors) a `<modules>` list. That's
//! parseable cleanly with the regex crate we already carry — pulling in
//! `quick-xml` would be a heavier dep for what amounts to five regex-guarded
//! scoops.
//!
//! Coordinate naming: Maven identifies a library as `groupId:artifactId`.
//! We follow suit so the library rows produced here match how the ecosystem
//! itself names artifacts, and so re-scanning across pom.xml versions stays
//! idempotent (versions live in `DepVersion.version`, not the name).
//!
//! Not covered in v1:
//! - Property interpolation (`${my.prop}`) — versions with `${...}` land as
//!   the literal string. Maven itself resolves these against `<properties>`;
//!   we can add that in a follow-up once a real corpus needs it.
//! - `<dependencyManagement>` — only `<dependencies>` is read today (that
//!   is what actually pulls a jar). BOM-only `<dependencyManagement>` blocks
//!   would need distinguishing "declared version" from "used version".
//! - `<parent>` inheritance across pom.xml files.

use super::{ManifestAdapter, ParsedManifest};
use crate::indexer::lib_indexer::DepVersion;
use crate::types::PackageInfo;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct MavenManifestAdapter;

impl ManifestAdapter for MavenManifestAdapter {
    fn manifest_filenames(&self) -> &[&'static str] {
        &["pom.xml"]
    }

    fn ecosystem(&self) -> &'static str {
        "maven"
    }

    fn parse_dependencies(&self, content: &str) -> Vec<DepVersion> {
        // Restrict to the top-level `<dependencies>` block so we don't pick
        // up `<dependencyManagement>` entries. The naive non-greedy regex
        // matches the FIRST `<dependencies>` in the file which is inside
        // `<dependencyManagement>` when both are present — strip that
        // wrapper block out first so only the real `<dependencies>` remains.
        let stripped = strip_top_level_block(content, "dependencyManagement");
        let deps_block = extract_top_level_block(&stripped, "dependencies").unwrap_or_default();
        let mut out = Vec::new();
        for dep in extract_dependency_entries(&deps_block) {
            let group = element_text(&dep, "groupId").unwrap_or_default();
            let artifact = element_text(&dep, "artifactId").unwrap_or_default();
            if group.is_empty() || artifact.is_empty() {
                continue;
            }
            let version = element_text(&dep, "version").unwrap_or_else(|| "*".to_string());
            let scope = element_text(&dep, "scope");
            let dev = matches!(scope.as_deref(), Some("test") | Some("provided"));
            out.push(DepVersion {
                lib_name: format!("{group}:{artifact}"),
                version: clean_maven_version(&version),
                raw_version: version,
                source: "pom.xml".into(),
                dev,
                local_source: None,
            });
        }
        out
    }

    fn is_workspace_root(&self, content: &str) -> bool {
        // A multi-module reactor sets <packaging>pom</packaging> AND declares
        // <modules>. Either alone isn't enough (a plain pom-packaged parent
        // with no modules is just an inheritance root, not a workspace).
        element_text(content, "packaging").as_deref() == Some("pom")
            && extract_top_level_block(content, "modules").is_some()
    }

    fn parse_manifest(&self, content: &str) -> ParsedManifest {
        let group = element_text(content, "groupId");
        let artifact = element_text(content, "artifactId");
        let version = element_text(content, "version");
        let description = element_text(content, "description");
        // Name = "groupId:artifactId" so the identity keys the same way as
        // dependency coordinates. If either half is missing we return None
        // rather than a partial coord.
        let name = match (group, artifact) {
            (Some(g), Some(a)) if !g.is_empty() && !a.is_empty() => Some(format!("{g}:{a}")),
            _ => None,
        };
        ParsedManifest {
            name,
            version: version.filter(|v| !v.is_empty()),
            description: description.filter(|d| !d.is_empty()),
        }
    }

    fn stack_labels(&self, _content: &str) -> Vec<&'static str> {
        vec!["java", "maven"]
    }

    fn detect_workspace_members(&self, repo_root: &Path) -> Vec<PackageInfo> {
        let Ok(content) = std::fs::read_to_string(repo_root.join("pom.xml")) else {
            return Vec::new();
        };
        if !self.is_workspace_root(&content) {
            return Vec::new();
        }
        let Some(modules_block) = extract_top_level_block(&content, "modules") else {
            return Vec::new();
        };

        // Modules are `<module>subdir</module>` — plain relative paths (no
        // globs; Maven doesn't do glob workspace patterns like npm does).
        // Skip any listed module dir that doesn't exist on disk or doesn't
        // carry its own pom.xml — a stale `<module>` entry from a prior
        // reactor shape shouldn't break enumeration.
        let mut out = Vec::new();
        for path in extract_module_paths(&modules_block) {
            let dir = repo_root.join(&path);
            if !dir.is_dir() || !dir.join("pom.xml").exists() {
                continue;
            }
            let pom = std::fs::read_to_string(dir.join("pom.xml")).unwrap_or_default();
            let parsed = self.parse_manifest(&pom);
            let name = parsed.name.unwrap_or_else(|| path.clone());
            let version = parsed.version;
            out.push(PackageInfo {
                name,
                path,
                version,
                pkg_type: "maven_module".to_string(),
                // Maven doesn't have an npm `"private": true` flag; assume
                // multi-module reactor children are publishable.
                private: false,
            });
        }
        out
    }
}

// ── Extraction helpers ─────────────────────────────────────────────────────

/// Extract the inner text of the first `<tag>…</tag>` in `content`.
/// Returns `None` when the tag is absent or self-closing.
fn element_text(content: &str, tag: &str) -> Option<String> {
    let re = element_re(tag);
    re.captures(content).map(|c| c.get(1).unwrap().as_str().trim().to_string())
}

/// Compile-cached regex for `<tag>...</tag>` (non-greedy inner).
fn element_re(tag: &str) -> Regex {
    // Escape the tag name so `<dep>` doesn't accidentally match a longer name.
    Regex::new(&format!(r"<{tag}>([\s\S]*?)</{tag}>", tag = regex::escape(tag))).unwrap()
}

/// Extract the content between a top-level `<tag>…</tag>` — used for the
/// enclosing `<dependencies>` / `<modules>` blocks so nested `<dependency>` /
/// `<module>` entries are matched only inside their proper parent.
fn extract_top_level_block(content: &str, tag: &str) -> Option<String> {
    element_re(tag).captures(content).map(|c| c.get(1).unwrap().as_str().to_string())
}

/// Delete every `<tag>…</tag>` block from `content`. Used to pull the
/// `<dependencyManagement>` wrapper out before scanning for the real
/// `<dependencies>` — otherwise the FIRST `<dependencies>` the naive
/// non-greedy regex matches is the nested BOM one, not the deps that
/// actually pull jars.
fn strip_top_level_block(content: &str, tag: &str) -> String {
    element_re(tag).replace_all(content, "").into_owned()
}

/// Every `<dependency>...</dependency>` entry inside a dependencies block.
fn extract_dependency_entries(deps_block: &str) -> Vec<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"<dependency>([\s\S]*?)</dependency>").unwrap());
    re.captures_iter(deps_block).map(|c| c.get(1).unwrap().as_str().to_string()).collect()
}

/// Every `<module>path</module>` entry inside a modules block.
fn extract_module_paths(modules_block: &str) -> Vec<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"<module>([\s\S]*?)</module>").unwrap());
    re.captures_iter(modules_block)
        .map(|c| c.get(1).unwrap().as_str().trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Strip Maven-specific version noise for the `clean` field. Unresolved
/// `${prop}` interpolations are preserved literally so a reader can spot
/// them; version ranges like `[1.0,2.0)` also pass through unchanged.
fn clean_maven_version(raw: &str) -> String {
    raw.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecosystem_and_filenames() {
        assert_eq!(MavenManifestAdapter.ecosystem(), "maven");
        assert_eq!(MavenManifestAdapter.manifest_filenames(), &["pom.xml"]);
    }

    #[test]
    fn parse_manifest_extracts_coord_and_description() {
        let src = r#"<?xml version="1.0" encoding="UTF-8"?>
            <project>
              <modelVersion>4.0.0</modelVersion>
              <groupId>com.example</groupId>
              <artifactId>my-app</artifactId>
              <version>1.2.3</version>
              <description>A sample Maven app.</description>
            </project>"#;
        let p = MavenManifestAdapter.parse_manifest(src);
        assert_eq!(p.name.as_deref(), Some("com.example:my-app"));
        assert_eq!(p.version.as_deref(), Some("1.2.3"));
        assert_eq!(p.description.as_deref(), Some("A sample Maven app."));
    }

    #[test]
    fn parse_manifest_none_when_group_or_artifact_missing() {
        let no_group = "<project><artifactId>orphan</artifactId></project>";
        assert!(MavenManifestAdapter.parse_manifest(no_group).name.is_none());
        let no_artifact = "<project><groupId>com.example</groupId></project>";
        assert!(MavenManifestAdapter.parse_manifest(no_artifact).name.is_none());
    }

    #[test]
    fn parse_dependencies_reads_dependencies_block() {
        let src = r#"<project>
            <dependencies>
              <dependency>
                <groupId>org.springframework</groupId>
                <artifactId>spring-core</artifactId>
                <version>6.1.0</version>
              </dependency>
              <dependency>
                <groupId>org.junit.jupiter</groupId>
                <artifactId>junit-jupiter</artifactId>
                <version>5.10.0</version>
                <scope>test</scope>
              </dependency>
            </dependencies>
        </project>"#;
        let deps = MavenManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 2);
        let by = |n: &str| deps.iter().find(|d| d.lib_name == n).unwrap();
        let spring = by("org.springframework:spring-core");
        assert_eq!(spring.version, "6.1.0");
        assert!(!spring.dev);
        let junit = by("org.junit.jupiter:junit-jupiter");
        assert!(junit.dev, "test-scoped deps are dev");
    }

    #[test]
    fn parse_dependencies_ignores_dependency_management() {
        let src = r#"<project>
            <dependencyManagement>
              <dependencies>
                <dependency>
                  <groupId>bom.example</groupId>
                  <artifactId>bom</artifactId>
                  <version>1.0.0</version>
                </dependency>
              </dependencies>
            </dependencyManagement>
            <dependencies>
              <dependency>
                <groupId>real.example</groupId>
                <artifactId>real-lib</artifactId>
                <version>2.0.0</version>
              </dependency>
            </dependencies>
        </project>"#;
        let deps = MavenManifestAdapter.parse_dependencies(src);
        // BOM entries (dependencyManagement) must NOT surface as deps —
        // they declare a version but don't actually pull anything.
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].lib_name, "real.example:real-lib");
    }

    #[test]
    fn parse_dependencies_defaults_version_star_when_missing() {
        let src = r#"<project>
            <dependencies>
              <dependency>
                <groupId>g</groupId>
                <artifactId>a</artifactId>
              </dependency>
            </dependencies>
        </project>"#;
        let deps = MavenManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].version, "*");
    }

    #[test]
    fn is_workspace_root_requires_packaging_pom_and_modules() {
        let both = r#"<project>
            <packaging>pom</packaging>
            <modules><module>core</module></modules>
        </project>"#;
        assert!(MavenManifestAdapter.is_workspace_root(both));

        let packaging_only = "<project><packaging>pom</packaging></project>";
        assert!(!MavenManifestAdapter.is_workspace_root(packaging_only));

        let modules_only = "<project><modules><module>x</module></modules></project>";
        assert!(!MavenManifestAdapter.is_workspace_root(modules_only));
    }

    #[test]
    fn detect_workspace_members_returns_child_pom_coords() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<project>
                <packaging>pom</packaging>
                <modules>
                  <module>core</module>
                  <module>api</module>
                </modules>
            </project>"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("core")).unwrap();
        std::fs::write(
            dir.path().join("core/pom.xml"),
            r#"<project>
                <groupId>com.example</groupId>
                <artifactId>core</artifactId>
                <version>1.0</version>
            </project>"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("api")).unwrap();
        std::fs::write(
            dir.path().join("api/pom.xml"),
            r#"<project>
                <groupId>com.example</groupId>
                <artifactId>api</artifactId>
                <version>1.0</version>
            </project>"#,
        )
        .unwrap();

        let members = MavenManifestAdapter.detect_workspace_members(dir.path());
        assert_eq!(members.len(), 2);
        let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"com.example:core"));
        assert!(names.contains(&"com.example:api"));
        assert!(members.iter().all(|m| m.pkg_type == "maven_module"));
    }

    #[test]
    fn detect_workspace_members_skips_module_paths_without_pom_xml() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<project>
                <packaging>pom</packaging>
                <modules>
                  <module>stale</module>
                </modules>
            </project>"#,
        )
        .unwrap();
        // No stale/ dir on disk — must not crash and must return empty.
        let members = MavenManifestAdapter.detect_workspace_members(dir.path());
        assert!(members.is_empty());
    }
}
