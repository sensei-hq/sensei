//! `GoManifestAdapter` — parses `go.mod`.
//!
//! **New capability** — go.mod dep versions were never captured before this
//! step. Parses the minimal set the daemon needs: the `module` line for
//! identity, and `require` directives (both block-form and single-line) for
//! dependencies. `replace` directives are ignored for the first pass; they
//! rewrite the resolver graph but don't add or drop entries.
//!
//! go.mod is not a workspace root — Go workspaces live in a separate
//! `go.work` file — so `is_workspace_root` always returns false.
//! `detect_workspace_members` reads `go.work` at the repo root to enumerate
//! the workspace's members.

use super::{ManifestAdapter, ParsedManifest};
use crate::indexer::lib_indexer::{DepVersion, clean_version};
use crate::types::PackageInfo;
use std::path::Path;

pub struct GoManifestAdapter;

impl ManifestAdapter for GoManifestAdapter {
    fn manifest_filenames(&self) -> &[&'static str] {
        &["go.mod"]
    }

    fn ecosystem(&self) -> &'static str {
        "go"
    }

    fn parse_dependencies(&self, content: &str) -> Vec<DepVersion> {
        let mut deps = Vec::new();
        let mut in_require_block = false;

        for raw_line in content.lines() {
            let line = strip_go_line_comment(raw_line).trim();
            if line.is_empty() {
                continue;
            }

            if in_require_block {
                if line == ")" {
                    in_require_block = false;
                    continue;
                }
                if let Some((name, version)) = parse_require_pair(line) {
                    deps.push(dep_version(name, version, false));
                }
                continue;
            }

            if let Some(rest) = line.strip_prefix("require") {
                let rest = rest.trim_start();
                if rest.starts_with('(') {
                    in_require_block = true;
                    continue;
                }
                // Single-line require: `require github.com/foo/bar v1.2.3`
                if let Some((name, version)) = parse_require_pair(rest) {
                    deps.push(dep_version(name, version, false));
                }
            }
        }

        deps
    }

    fn is_workspace_root(&self, _content: &str) -> bool {
        // Go workspaces live in `go.work`, not `go.mod`. Anything else is a
        // regular module. The go.work adapter lives outside this trait for
        // now (see config/detector.rs).
        false
    }

    fn parse_manifest(&self, content: &str) -> ParsedManifest {
        let name = content
            .lines()
            .map(strip_go_line_comment)
            .map(|l| l.trim())
            .find_map(|line| line.strip_prefix("module").map(|rest| rest.trim().to_string()))
            .filter(|s| !s.is_empty());
        ParsedManifest {
            name,
            version: None,     // go.mod doesn't declare a module version
            description: None, // no description field in go.mod
        }
    }

    fn stack_labels(&self, _content: &str) -> Vec<&'static str> {
        vec!["go"]
    }

    fn detect_workspace_members(&self, repo_root: &Path) -> Vec<PackageInfo> {
        let Ok(content) = std::fs::read_to_string(repo_root.join("go.work")) else {
            return Vec::new();
        };
        let mut members = Vec::new();
        // Parse the `use ( ./dir1 ./dir2 )` block plus stand-alone `use ./dir`
        // lines that go.work also supports.
        let mut in_use = false;
        for raw_line in content.lines() {
            let line = strip_go_line_comment(raw_line).trim();
            if line.is_empty() {
                continue;
            }
            if let Some(rest) = line.strip_prefix("use") {
                let rest = rest.trim_start();
                if rest.starts_with('(') {
                    in_use = true;
                    continue;
                }
                // Single-line: `use ./sub`
                if let Some(member) = go_work_member(repo_root, rest) {
                    members.push(member);
                }
                continue;
            }
            if in_use {
                if line == ")" {
                    in_use = false;
                    continue;
                }
                if let Some(member) = go_work_member(repo_root, line) {
                    members.push(member);
                }
            }
        }
        members
    }

    /// Conventional Go verbs. `go test ./...` (recursive) is the canonical
    /// answer to "run the tests"; `go vet` catches common bugs; `go fmt`
    /// enforces formatting; `go mod tidy` cleans deps. These are the same
    /// across every Go project.
    fn parse_commands(&self, _content: &str) -> Vec<super::DiscoveredCommand> {
        super::conventional_commands(
            "go",
            &[
                ("test ./...", "test"),
                ("build ./...", "build"),
                ("vet ./...", "lint"),
                ("fmt ./...", "format"),
                ("mod tidy", "run"),
                ("run .", "run"),
            ],
        )
    }
}

/// Build a `PackageInfo` for a `use ./dir` entry in a go.work file. Returns
/// `None` when the referenced folder has no `go.mod`.
fn go_work_member(repo_root: &Path, entry: &str) -> Option<PackageInfo> {
    let dir = entry.trim_start_matches("./").trim();
    if dir.is_empty() {
        return None;
    }
    let go_mod = repo_root.join(dir).join("go.mod");
    if !go_mod.exists() {
        return None;
    }
    let name = std::fs::read_to_string(&go_mod)
        .ok()
        .and_then(|c| {
            c.lines()
                .map(strip_go_line_comment)
                .map(|l| l.trim().to_string())
                .find_map(|line| line.strip_prefix("module").map(|rest| rest.trim().to_string()))
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| dir.to_string());
    Some(PackageInfo {
        name,
        path: dir.to_string(),
        version: None,
        pkg_type: "go_module".to_string(),
        private: false,
    })
}

/// Strip a `//` line comment (preserving anything before it).
fn strip_go_line_comment(line: &str) -> &str {
    line.split_once("//").map(|(before, _)| before).unwrap_or(line)
}

/// Parse a `name version` pair (with optional trailing indirect marker) into
/// its two parts. Returns `None` for malformed lines.
fn parse_require_pair(line: &str) -> Option<(&str, &str)> {
    let mut parts = line.split_whitespace();
    let name = parts.next()?;
    let version = parts.next()?;
    Some((name, version))
}

fn dep_version(name: &str, version: &str, dev: bool) -> DepVersion {
    // Go module versions carry a `v` prefix by convention (`v1.2.3` is the
    // tag, `1.2.3` is the semver). Strip it so the version column stays
    // consistent with npm / cargo / pypi and the version-conflict view can
    // compare across ecosystems without a per-ecosystem hack.
    let stripped = version.trim_start_matches('v');
    DepVersion {
        lib_name: name.to_string(),
        version: clean_version(stripped),
        raw_version: version.to_string(),
        source: "go.mod".into(),
        dev,
        local_source: None, // go.mod local deps use `replace`, not `require`
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecosystem_and_filenames() {
        assert_eq!(GoManifestAdapter.ecosystem(), "go");
        assert_eq!(GoManifestAdapter.manifest_filenames(), &["go.mod"]);
    }

    #[test]
    fn parse_dependencies_from_require_block() {
        let src = r#"
            module github.com/example/svc

            go 1.21

            require (
                github.com/spf13/cobra v1.5.0
                github.com/sirupsen/logrus v1.9.0
            )
        "#;
        let deps = GoManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].lib_name, "github.com/spf13/cobra");
        assert_eq!(deps[0].version, "1.5.0");
        assert_eq!(deps[0].source, "go.mod");
        assert!(!deps[0].dev);
    }

    #[test]
    fn parse_dependencies_from_single_line_require() {
        let src = r#"
            module github.com/example/svc

            require github.com/gin-gonic/gin v1.9.0
        "#;
        let deps = GoManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].lib_name, "github.com/gin-gonic/gin");
        assert_eq!(deps[0].version, "1.9.0");
    }

    #[test]
    fn parse_dependencies_mixes_block_and_single_line() {
        let src = r#"
            module m

            require (
                github.com/a/a v1.0.0
            )

            require github.com/b/b v2.0.0
        "#;
        let deps = GoManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 2);
        let names: Vec<&str> = deps.iter().map(|d| d.lib_name.as_str()).collect();
        assert!(names.contains(&"github.com/a/a"));
        assert!(names.contains(&"github.com/b/b"));
    }

    #[test]
    fn parse_dependencies_strips_comments_and_indirect_marker() {
        // `// indirect` markers are common on transitive requires. Comments
        // are stripped so only the name+version reach the parser.
        let src = r#"
            require (
                github.com/a/a v1.0.0 // indirect
                github.com/b/b v2.0.0
            )
        "#;
        let deps = GoManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].version, "1.0.0");
        assert_eq!(deps[1].version, "2.0.0");
    }

    #[test]
    fn parse_dependencies_ignores_replace_directives() {
        // `replace` directives rewrite the resolver graph — they don't add
        // deps by themselves, so the first pass ignores them.
        let src = r#"
            require github.com/a/a v1.0.0

            replace github.com/a/a => ../local
        "#;
        let deps = GoManifestAdapter.parse_dependencies(src);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].lib_name, "github.com/a/a");
    }

    #[test]
    fn parse_dependencies_empty_for_module_only_file() {
        let src = "module github.com/example/x\n\ngo 1.21\n";
        assert!(GoManifestAdapter.parse_dependencies(src).is_empty());
    }

    #[test]
    fn is_workspace_root_always_false_for_go_mod() {
        // Go workspaces live in go.work, not go.mod.
        assert!(!GoManifestAdapter.is_workspace_root(""));
        assert!(!GoManifestAdapter.is_workspace_root("module x\n"));
    }

    #[test]
    fn parse_manifest_reads_module_name() {
        let src = "module github.com/example/svc\n\ngo 1.21\n";
        let p = GoManifestAdapter.parse_manifest(src);
        assert_eq!(p.name.as_deref(), Some("github.com/example/svc"));
        assert!(p.version.is_none());
        assert!(p.description.is_none());
    }

    #[test]
    fn parse_manifest_defaults_when_no_module_line() {
        let p = GoManifestAdapter.parse_manifest("");
        assert!(p.name.is_none());
    }

    #[test]
    fn stack_labels_always_go() {
        assert_eq!(GoManifestAdapter.stack_labels(""), vec!["go"]);
    }
}
