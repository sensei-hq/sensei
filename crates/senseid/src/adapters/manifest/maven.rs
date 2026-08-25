//! `MavenManifestAdapter` — parses `pom.xml` (#86).
//!
//! Uses `quick-xml` via the shared [`super::xml`] streaming walker so real
//! pom.xml quirks (`xmlns`, comments, CDATA, self-closing tags) are handled
//! by an XML parser rather than by regex.
//!
//! Coordinate naming: Maven identifies a library as `groupId:artifactId`.
//! We follow suit so the library rows produced here match how the ecosystem
//! itself names artifacts, and so re-scanning across pom.xml versions stays
//! idempotent (versions live in `DepVersion.version`, not the name).
//!
//! Not covered in v1 (documented so a follow-up doesn't rediscover them):
//! - Property interpolation (`${my.prop}`) — versions with `${...}` land as
//!   the literal string. Maven itself resolves these against `<properties>`;
//!   we can add that in a follow-up once a real corpus needs it.
//! - `<dependencyManagement>` — only `<dependencies>` is read today (that
//!   is what actually pulls a jar). BOM-only `<dependencyManagement>` blocks
//!   would need distinguishing "declared version" from "used version".
//! - `<parent>` inheritance across pom.xml files.

use super::xml::{XmlEvent, XmlPath, walk, walk_leaves};
use super::{ManifestAdapter, ParsedManifest};
use crate::indexer::lib_indexer::DepVersion;
use crate::types::PackageInfo;
use std::path::Path;

pub struct MavenManifestAdapter;

impl ManifestAdapter for MavenManifestAdapter {
    fn manifest_filenames(&self) -> &[&'static str] {
        &["pom.xml"]
    }

    fn ecosystem(&self) -> &'static str {
        "maven"
    }

    fn parse_dependencies(&self, content: &str) -> Vec<DepVersion> {
        // Collect every `<dependency>` under `project/dependencies/dependency`
        // (NOT under `project/dependencyManagement/…`, which declares
        // versions but doesn't actually pull jars). Enter/Exit brackets on
        // the walker let a fresh PartialDep start at every `<dependency>`
        // Enter — that's the only reliable way to partition repeated
        // same-name children.
        let mut deps: Vec<PartialDep> = Vec::new();
        let mut current: Option<PartialDep> = None;

        let _ = walk(content, |path: &XmlPath<'_>, evt: XmlEvent<'_>| {
            let at_dependency = path.is(&["project", "dependencies", "dependency"]);
            match (at_dependency, evt) {
                (true, XmlEvent::Enter(_)) => current = Some(PartialDep::default()),
                (true, XmlEvent::Exit) => {
                    if let Some(d) = current.take() {
                        deps.push(d);
                    }
                }
                (false, XmlEvent::Leaf(text)) => {
                    let Some(d) = current.as_mut() else { return };
                    // Only pick up leaf children ONE level below <dependency>
                    // — a nested `<version>` under `<exclusions><exclusion>`
                    // (rare, but legal) would otherwise clobber the top-level
                    // one.
                    let inside_dependency = path.0.len() == 4
                        && path.0[0] == "project"
                        && path.0[1] == "dependencies"
                        && path.0[2] == "dependency";
                    if !inside_dependency {
                        return;
                    }
                    match path.0[3].as_str() {
                        "groupId" => d.group = Some(text.to_string()),
                        "artifactId" => d.artifact = Some(text.to_string()),
                        "version" => d.version = Some(text.to_string()),
                        "scope" => d.scope = Some(text.to_string()),
                        _ => {}
                    }
                }
                _ => {}
            }
        });

        deps.into_iter()
            .filter_map(|d| {
                let group = d.group?;
                let artifact = d.artifact?;
                if group.is_empty() || artifact.is_empty() {
                    return None;
                }
                let version = d.version.unwrap_or_else(|| "*".to_string());
                let dev = matches!(d.scope.as_deref(), Some("test") | Some("provided"));
                Some(DepVersion {
                    lib_name: format!("{group}:{artifact}"),
                    version: version.trim().to_string(),
                    raw_version: version,
                    source: "pom.xml".into(),
                    dev,
                    local_source: None,
                })
            })
            .collect()
    }

    fn is_workspace_root(&self, content: &str) -> bool {
        // A multi-module reactor sets <packaging>pom</packaging> AND declares
        // <modules>. Either alone isn't enough (a plain pom-packaged parent
        // with no modules is just an inheritance root, not a workspace).
        let mut packaging_is_pom = false;
        let mut has_modules = false;
        let _ = walk_leaves(content, |p: &XmlPath<'_>, t: &str| {
            if p.is(&["project", "packaging"]) && t == "pom" {
                packaging_is_pom = true;
            } else if p.is(&["project", "modules", "module"]) {
                has_modules = true;
            }
        });
        packaging_is_pom && has_modules
    }

    fn parse_manifest(&self, content: &str) -> ParsedManifest {
        // Read only the TOP-LEVEL identity (`project/*`), not the `<parent>`
        // block — otherwise a child pom.xml that omits its own groupId but
        // inherits from `<parent><groupId>` would take the parent's id
        // instead of correctly reporting None.
        let mut group: Option<String> = None;
        let mut artifact: Option<String> = None;
        let mut version: Option<String> = None;
        let mut description: Option<String> = None;
        let _ = walk_leaves(content, |p: &XmlPath<'_>, t: &str| {
            if p.is(&["project", "groupId"]) {
                group = Some(t.to_string());
            } else if p.is(&["project", "artifactId"]) {
                artifact = Some(t.to_string());
            } else if p.is(&["project", "version"]) {
                version = Some(t.to_string());
            } else if p.is(&["project", "description"]) {
                description = Some(t.to_string());
            }
        });

        // Identity = `groupId:artifactId`. Missing either → None rather than
        // a partial coord.
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
        // Collect module paths under `project/modules/module`.
        let mut modules: Vec<String> = Vec::new();
        let _ = walk_leaves(&content, |p: &XmlPath<'_>, t: &str| {
            if p.is(&["project", "modules", "module"]) && !t.is_empty() {
                modules.push(t.to_string());
            }
        });

        // Maven module paths are plain relative dirs (not glob patterns);
        // stale entries (dir missing or no child pom.xml) are silently
        // skipped so a broken reactor doesn't crash enumeration.
        let mut out = Vec::new();
        for path in modules {
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

    /// Conventional Maven lifecycle verbs. `test` / `package` / `install` /
    /// `clean` / `verify` cover the ~90% of "how do I build this" answers.
    fn parse_commands(&self, _content: &str) -> Vec<super::DiscoveredCommand> {
        super::conventional_commands(
            "mvn",
            &[
                ("test", "test"),
                ("compile", "build"),
                ("package", "build"),
                ("install", "build"),
                ("verify", "test"),
                ("clean", "run"),
            ],
        )
    }
}

#[derive(Default)]
struct PartialDep {
    group: Option<String>,
    artifact: Option<String>,
    version: Option<String>,
    scope: Option<String>,
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
    fn parse_manifest_handles_namespaced_pom() {
        // Real poms declare xmlns; the local-name walker must still resolve.
        let src = r#"<?xml version="1.0"?>
            <project xmlns="http://maven.apache.org/POM/4.0.0">
              <groupId>com.example</groupId>
              <artifactId>ns-app</artifactId>
              <version>2.0</version>
            </project>"#;
        let p = MavenManifestAdapter.parse_manifest(src);
        assert_eq!(p.name.as_deref(), Some("com.example:ns-app"));
    }

    #[test]
    fn parse_manifest_ignores_parent_identity() {
        // A child pom without its own groupId inherits from <parent>. The
        // ADAPTER should NOT surface the parent's id as this artifact's
        // name — that would give the wrong library row on scan.
        let src = r#"<project>
              <parent>
                <groupId>com.parent</groupId>
                <artifactId>parent-pom</artifactId>
                <version>1.0</version>
              </parent>
              <artifactId>child</artifactId>
            </project>"#;
        let p = MavenManifestAdapter.parse_manifest(src);
        // Only artifactId present at project level; group missing → no name.
        assert!(p.name.is_none());
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
    fn parse_dependencies_ignores_xml_comments() {
        // Regex-based extraction would trip on a <dependency> hidden inside
        // an XML comment; the XML parser correctly skips it.
        let src = r#"<project>
            <dependencies>
              <!--<dependency><groupId>hidden</groupId><artifactId>a</artifactId></dependency>-->
              <dependency>
                <groupId>real</groupId>
                <artifactId>lib</artifactId>
                <version>1.0</version>
              </dependency>
            </dependencies>
        </project>"#;
        let deps = MavenManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].lib_name, "real:lib");
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
