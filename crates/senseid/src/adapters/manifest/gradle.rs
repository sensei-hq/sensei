//! `GradleManifestAdapter` — `build.gradle` / `build.gradle.kts` (#87).
//!
//! Gradle build scripts are imperative — Groovy DSL or Kotlin DSL — so a
//! "full" parse would need a Groovy / Kotlin interpreter. In practice
//! dependencies live in a `dependencies { ... }` block and follow a small
//! set of well-known shapes. This adapter scans for those shapes and
//! extracts the coordinate string; anything more elaborate (computed
//! version literals, closures) is skipped and left to a follow-up.
//!
//! Coordinate + ecosystem: Gradle downloads from Maven Central and uses
//! `groupId:artifactId:version`, so the ecosystem slug is `"maven"` —
//! matching #86. A jar pulled by Gradle and the same jar pulled by Maven
//! resolve to the SAME library row.
//!
//! Not covered in v1 (documented so the next iteration doesn't rediscover):
//! - `group:`/`name:`/`version:` map-style deps
//!   (`implementation group: 'x', name: 'y', version: 'z'`)
//! - Kotlin DSL `dependencies { platform("..."); implementation(kotlin("..."))
//!   }`
//! - Version catalog (`libs.versions.toml`) references — a separate manifest
//! - Property interpolation (`${springVersion}`)

use super::{ManifestAdapter, ParsedManifest};
use crate::indexer::lib_indexer::DepVersion;
use crate::types::PackageInfo;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct GradleManifestAdapter;

impl ManifestAdapter for GradleManifestAdapter {
    fn manifest_filenames(&self) -> &[&'static str] {
        &["build.gradle", "build.gradle.kts", "settings.gradle", "settings.gradle.kts"]
    }

    fn ecosystem(&self) -> &'static str {
        // Gradle uses Maven Central + Maven-style coordinates. A jar pulled
        // via Gradle and via Maven point at the SAME library row.
        "maven"
    }

    fn parse_dependencies(&self, content: &str) -> Vec<DepVersion> {
        // Strip `//` line comments and `/* … */` block comments so a
        // commented-out `implementation "foo"` doesn't surface as a dep.
        let stripped = strip_comments(content);
        let re = dep_re();
        let mut out = Vec::new();
        for cap in re.captures_iter(&stripped) {
            let config = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let coord = cap.get(2).map(|m| m.as_str().trim()).unwrap_or("");
            let Some(dep) = parse_coordinate(coord, config) else { continue };
            out.push(dep);
        }
        out
    }

    fn is_workspace_root(&self, _content: &str) -> bool {
        // `settings.gradle`/`.kts` at a directory always marks a Gradle
        // multi-project reactor. We can't tell from content alone WHICH
        // file this is (the caller feeds us the string), so return false
        // here and let `detect_workspace_members` do the real detection
        // via the filesystem.
        false
    }

    fn parse_manifest(&self, content: &str) -> ParsedManifest {
        // Gradle's identity is `rootProject.name` in settings.gradle plus
        // optional `group` / `version` at the top of build.gradle. Prefer
        // the rootProject.name line when present (works for both Groovy
        // and Kotlin DSL — the LHS + `=` is stable across both).
        let name = extract_root_project_name(content);
        let version = extract_top_level_kv(content, "version");
        let description = extract_top_level_kv(content, "description");
        ParsedManifest { name, version, description }
    }

    fn stack_labels(&self, _content: &str) -> Vec<&'static str> {
        vec!["java", "gradle"]
    }

    fn detect_workspace_members(&self, repo_root: &Path) -> Vec<PackageInfo> {
        // A Gradle reactor is any dir whose settings.gradle (or .kts) makes
        // one or more `include` calls. Read whichever file exists — Gradle
        // considers them equivalent for our purposes.
        let content = ["settings.gradle", "settings.gradle.kts"]
            .iter()
            .filter_map(|name| std::fs::read_to_string(repo_root.join(name)).ok())
            .next();
        let Some(content) = content else {
            return Vec::new();
        };

        let root_name = extract_root_project_name(&content);
        let mut out = Vec::new();
        for module_path in extract_gradle_includes(&content) {
            // Gradle `include(':foo:bar')` maps to `foo/bar/` on disk. A
            // leading `:` is dropped; separators become `/`.
            let rel = module_path.trim_start_matches(':').replace(':', "/");
            let dir = repo_root.join(&rel);
            let build_gradle = dir.join("build.gradle");
            let build_gradle_kts = dir.join("build.gradle.kts");
            let build = if build_gradle.exists() {
                build_gradle
            } else if build_gradle_kts.exists() {
                build_gradle_kts
            } else {
                continue; // stale include entry with no build script
            };
            let script = std::fs::read_to_string(&build).unwrap_or_default();
            // Try build.gradle's own identity first, then fall back to the
            // gradle path segment. When rootProject.name is set at the
            // reactor level, downstream code can still tie the module to
            // its parent via the `group` or configuration inheritance.
            let name = extract_root_project_name(&script)
                .or_else(|| {
                    // `project(':foo').name = 'renamed'` at the reactor
                    // level is exotic; not covered. Default = last segment.
                    dir.file_name().and_then(|n| n.to_str()).map(String::from)
                })
                .unwrap_or_else(|| rel.clone());
            let version = extract_top_level_kv(&script, "version");
            out.push(PackageInfo {
                name: qualified_name(&root_name, &name),
                path: rel,
                version,
                pkg_type: "gradle_module".to_string(),
                private: false,
            });
        }
        out
    }

    /// Conventional Gradle tasks. Wrapper form (`./gradlew`) is what most
    /// repos actually use so we prefer that; a caller can still swap in
    /// `gradle` on read if the folder lacks the wrapper script.
    fn parse_commands(&self, _content: &str) -> Vec<super::DiscoveredCommand> {
        super::conventional_commands(
            "./gradlew",
            &[
                ("test", "test"),
                ("build", "build"),
                ("assemble", "build"),
                ("clean", "run"),
                ("check", "typecheck"),
            ],
        )
    }
}

// ── Coordinate + regex helpers ─────────────────────────────────────────────

/// Regex matching `<config>(...) "group:artifact:version"` or
/// `<config> 'group:artifact:version'`. `<config>` is one of the well-known
/// Gradle configurations that surface actual dependencies.
///
/// The alternation is deliberately narrow: `implementation` matches a
/// method-call form (Kotlin DSL — `implementation("…")`) and a Groovy
/// name-with-space form (`implementation '…'`).
fn dep_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?m)^\s*(implementation|api|compileOnly|runtimeOnly|testImplementation|testCompileOnly|testRuntimeOnly|annotationProcessor|kapt)\s*[(]?\s*['"]([^'"]+)['"]"#,
        )
        .unwrap()
    })
}

/// Parse `group:artifact:version` (Maven coord form) into a `DepVersion`.
/// Returns `None` for entries that aren't a valid three-part coord —
/// `project(':core')` shorthand for local project deps yields something
/// like `":core"` here and is filtered out (project→project edges are the
/// separate `#41` merge concern).
fn parse_coordinate(coord: &str, config: &str) -> Option<DepVersion> {
    if coord.starts_with(':') || coord.contains('/') || coord.contains(' ') {
        return None;
    }
    let parts: Vec<&str> = coord.splitn(3, ':').collect();
    if parts.len() < 2 {
        return None;
    }
    let group = parts[0].trim();
    let artifact = parts[1].trim();
    if group.is_empty() || artifact.is_empty() {
        return None;
    }
    let version = parts.get(2).map(|s| s.trim().to_string()).unwrap_or_else(|| "*".to_string());
    Some(DepVersion {
        lib_name: format!("{group}:{artifact}"),
        version: version.clone(),
        raw_version: version,
        source: "build.gradle".into(),
        dev: is_test_config(config),
        local_source: None,
    })
}

fn is_test_config(config: &str) -> bool {
    config.starts_with("test") || config.starts_with("androidTest")
}

/// Drop `//` line comments and `/* ... */` block comments so a commented-out
/// dependency doesn't leak into `parse_dependencies`.
///
/// This does NOT respect string escapes (a `//` inside a string literal is
/// treated as a comment start). That's a simplification consistent with the
/// other regex-guarded scanners in this crate — Gradle scripts rarely embed
/// `//` in strings, and dropping accidentally is safer than including a
/// commented dep.
fn strip_comments(content: &str) -> String {
    static BLOCK: OnceLock<Regex> = OnceLock::new();
    static LINE: OnceLock<Regex> = OnceLock::new();
    let block = BLOCK.get_or_init(|| Regex::new(r"(?s)/\*.*?\*/").unwrap());
    let no_block = block.replace_all(content, "");
    let line = LINE.get_or_init(|| Regex::new(r"//[^\n]*").unwrap());
    line.replace_all(&no_block, "").into_owned()
}

/// Match `rootProject.name = "…"` (Kotlin DSL) or `rootProject.name = '…'`
/// (Groovy). Optional whitespace + optional Kotlin `=` colon typing are
/// tolerated.
fn extract_root_project_name(content: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r#"rootProject\.name\s*=\s*['"]([^'"]+)['"]"#).unwrap());
    re.captures(content).map(|c| c.get(1).unwrap().as_str().trim().to_string())
}

/// Match top-level `<key> = "value"` (Groovy) or `<key>.set("value")` /
/// `val <key> = "value"` (Kotlin). Only the first form is picked up in v1.
fn extract_top_level_kv(content: &str, key: &str) -> Option<String> {
    let re =
        Regex::new(&format!(r#"(?m)^\s*{key}\s*=\s*['"]([^'"]+)['"]"#, key = regex::escape(key)))
            .ok()?;
    let stripped = strip_comments(content);
    re.captures(&stripped).map(|c| c.get(1).unwrap().as_str().to_string())
}

/// Every path passed to `include(...)` in settings.gradle. Supports:
/// - `include ':foo'`
/// - `include(':foo')`
/// - `include ':foo', ':bar'`
/// - `include(':foo', ':bar')`
fn extract_gradle_includes(content: &str) -> Vec<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let stripped = strip_comments(content);
    // Find each include(...) call, then extract every quoted string inside
    // its argument list. Two-step because Regex's `(?m)` doesn't do
    // recursive matching.
    let call_re = RE.get_or_init(|| Regex::new(r"(?m)^\s*include\s*\(?([^\n)]+)\)?\s*$").unwrap());
    let arg_re = Regex::new(r#"['"]([^'"]+)['"]"#).unwrap();
    let mut out = Vec::new();
    for call in call_re.captures_iter(&stripped) {
        let args = call.get(1).map(|m| m.as_str()).unwrap_or("");
        for arg in arg_re.captures_iter(args) {
            let raw = arg.get(1).unwrap().as_str().trim();
            if !raw.is_empty() {
                out.push(raw.to_string());
            }
        }
    }
    out
}

/// If a reactor rootProject name is known, expose modules as
/// `rootProject:module` so multiple reactors don't collide on the same
/// bare submodule name (`:core` in two different repos are distinct
/// projects). Falls back to the bare module name.
fn qualified_name(root: &Option<String>, module: &str) -> String {
    match root {
        Some(r) if !r.is_empty() => format!("{r}:{module}"),
        _ => module.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecosystem_and_filenames() {
        let a = GradleManifestAdapter;
        assert_eq!(a.ecosystem(), "maven");
        assert_eq!(
            a.manifest_filenames(),
            &["build.gradle", "build.gradle.kts", "settings.gradle", "settings.gradle.kts"],
        );
    }

    #[test]
    fn parse_dependencies_reads_groovy_dsl_string_form() {
        let src = r#"
            dependencies {
                implementation 'org.springframework:spring-core:6.1.0'
                testImplementation 'org.junit.jupiter:junit-jupiter:5.10.0'
            }
        "#;
        let deps = GradleManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 2);
        let by = |n: &str| deps.iter().find(|d| d.lib_name == n).unwrap();
        assert_eq!(by("org.springframework:spring-core").version, "6.1.0");
        assert!(!by("org.springframework:spring-core").dev);
        assert!(by("org.junit.jupiter:junit-jupiter").dev, "test config → dev");
    }

    #[test]
    fn parse_dependencies_reads_kotlin_dsl_method_call_form() {
        let src = r#"
            dependencies {
                implementation("org.springframework:spring-core:6.1.0")
                testImplementation("org.junit.jupiter:junit-jupiter:5.10.0")
            }
        "#;
        let deps = GradleManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 2);
        assert!(deps.iter().any(|d| d.lib_name == "org.springframework:spring-core"));
        assert!(deps.iter().any(|d| d.lib_name == "org.junit.jupiter:junit-jupiter"));
    }

    #[test]
    fn parse_dependencies_skips_commented_lines() {
        let src = r#"
            dependencies {
                // implementation "hidden.line:comment:1.0"
                /* implementation "hidden.block:comment:1.0" */
                implementation "real:lib:1.0"
            }
        "#;
        let deps = GradleManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].lib_name, "real:lib");
    }

    #[test]
    fn parse_dependencies_skips_local_project_shorthand() {
        // `project(':core')` isn't a Maven coord — it's a local
        // project→project edge and belongs to project_dependencies, not
        // referenced_libraries.
        let src = r#"
            dependencies {
                implementation project(':core')
                implementation 'x:y:1.0'
            }
        "#;
        let deps = GradleManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].lib_name, "x:y");
    }

    #[test]
    fn parse_dependencies_defaults_version_star_when_absent() {
        // Version catalogs supply the version later — the coord looks like
        // "group:artifact" without a version segment.
        let src = r#"
            dependencies {
                implementation 'g:a'
            }
        "#;
        let deps = GradleManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].version, "*");
    }

    #[test]
    fn parse_manifest_reads_root_project_name() {
        let src = r#"
            rootProject.name = 'my-app'
            version = '1.4.2'
        "#;
        let p = GradleManifestAdapter.parse_manifest(src);
        assert_eq!(p.name.as_deref(), Some("my-app"));
        assert_eq!(p.version.as_deref(), Some("1.4.2"));
    }

    #[test]
    fn parse_manifest_reads_kotlin_dsl_root_project_name() {
        let src = r#"
            rootProject.name = "kts-app"
        "#;
        let p = GradleManifestAdapter.parse_manifest(src);
        assert_eq!(p.name.as_deref(), Some("kts-app"));
    }

    #[test]
    fn extract_gradle_includes_supports_all_syntaxes() {
        let single_no_parens = "include ':foo'";
        assert_eq!(extract_gradle_includes(single_no_parens), vec![":foo"]);

        let single_parens = "include(':bar')";
        assert_eq!(extract_gradle_includes(single_parens), vec![":bar"]);

        let multi_no_parens = "include ':foo', ':bar'";
        assert_eq!(extract_gradle_includes(multi_no_parens), vec![":foo", ":bar"]);

        let multi_parens = "include(':foo', ':bar')";
        assert_eq!(extract_gradle_includes(multi_parens), vec![":foo", ":bar"]);
    }

    #[test]
    fn detect_workspace_members_enumerates_included_subprojects() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("settings.gradle"),
            "rootProject.name = 'my-app'\n\
             include ':core', ':api'\n",
        )
        .unwrap();
        for sub in ["core", "api"] {
            std::fs::create_dir_all(dir.path().join(sub)).unwrap();
            std::fs::write(dir.path().join(sub).join("build.gradle"), "// stub\nversion = '1.0'\n")
                .unwrap();
        }

        let members = GradleManifestAdapter.detect_workspace_members(dir.path());
        assert_eq!(members.len(), 2);
        let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
        // Qualified with rootProject.name so cross-reactor names don't collide.
        assert!(names.contains(&"my-app:core"));
        assert!(names.contains(&"my-app:api"));
        assert!(members.iter().all(|m| m.pkg_type == "gradle_module"));
    }

    #[test]
    fn detect_workspace_members_skips_stale_include_paths() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("settings.gradle"),
            "rootProject.name = 'r'\ninclude ':stale'\n",
        )
        .unwrap();
        // No `stale/` on disk — must not crash and must return empty.
        assert!(GradleManifestAdapter.detect_workspace_members(dir.path()).is_empty());
    }

    #[test]
    fn detect_workspace_members_handles_kts_settings() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("settings.gradle.kts"),
            r#"rootProject.name = "kts-app"
               include(":core")
            "#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("core")).unwrap();
        std::fs::write(dir.path().join("core").join("build.gradle.kts"), "// stub\n").unwrap();
        let members = GradleManifestAdapter.detect_workspace_members(dir.path());
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name, "kts-app:core");
    }
}
