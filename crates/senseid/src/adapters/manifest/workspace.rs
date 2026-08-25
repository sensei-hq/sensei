//! Shared workspace-resolution helpers used by adapters that expose
//! `detect_workspace_members`. Kept in one place so npm, pnpm and Cargo all
//! resolve `packages/*` glob patterns and package.json metadata identically.

use std::path::Path;

/// Resolve a glob pattern like `"packages/*"` against `repo_path`, returning
/// relative directory paths that matched. Trailing `/*` and `/**` are the
/// only patterns supported — that's the shape workspace configs actually
/// carry (`packages/*`, `crates/*`, `apps/*`). Exact-path patterns without a
/// glob are also accepted verbatim.
pub(crate) fn resolve_glob_members(repo_path: &Path, pattern: &str) -> Vec<String> {
    let mut results = Vec::new();
    let clean = pattern.trim_end_matches("/**").trim_end_matches("/*").trim_end_matches('/');
    let target_dir = repo_path.join(clean);
    if target_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&target_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !name.starts_with('.') {
                        results.push(format!("{clean}/{name}"));
                    }
                }
            }
        }
    } else if target_dir.exists() {
        // Exact path (no glob)
        results.push(clean.to_string());
    }
    results
}

/// Extract workspace glob patterns from a package.json `workspaces` field.
/// Accepts both the plain array form and the Yarn-style
/// `{"workspaces":{"packages":[…]}}` object form.
pub(crate) fn extract_npm_workspace_patterns(pkg: &serde_json::Value) -> Vec<String> {
    let ws = pkg.get("workspaces");
    if let Some(arr) = ws.and_then(|w| w.as_array()) {
        return arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
    }
    if let Some(obj) = ws.and_then(|w| w.as_object())
        && let Some(packages) = obj.get("packages").and_then(|p| p.as_array())
    {
        return packages.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
    }
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn resolve_glob_expands_star_pattern() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("packages/a")).unwrap();
        std::fs::create_dir_all(dir.path().join("packages/b")).unwrap();
        let mut members = resolve_glob_members(dir.path(), "packages/*");
        members.sort();
        assert_eq!(members, vec!["packages/a", "packages/b"]);
    }

    #[test]
    fn resolve_glob_handles_dot_slash_prefix() {
        // Rokkit-style pattern with a leading `./` — every real monorepo we
        // hit in the wild writes at least some patterns this way (#63).
        // Before this test the returned entries carried the `./` prefix,
        // which downstream `package_json_member(repo, "./packages/actions")`
        // still resolved correctly, so this locks the current behaviour.
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("packages/a")).unwrap();
        std::fs::create_dir_all(dir.path().join("packages/b")).unwrap();
        let mut members = resolve_glob_members(dir.path(), "./packages/*");
        members.sort();
        assert_eq!(members, vec!["./packages/a", "./packages/b"]);
    }

    #[test]
    fn resolve_glob_skips_hidden_dirs() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("packages/a")).unwrap();
        std::fs::create_dir_all(dir.path().join("packages/.hidden")).unwrap();
        let members = resolve_glob_members(dir.path(), "packages/*");
        assert_eq!(members, vec!["packages/a"]);
    }

    #[test]
    fn resolve_glob_returns_empty_for_unresolvable_pattern() {
        let dir = TempDir::new().unwrap();
        // Pattern points at a folder that doesn't exist → no members.
        assert!(resolve_glob_members(dir.path(), "packages/*").is_empty());
    }

    #[test]
    fn extract_patterns_from_array_form() {
        let pkg: serde_json::Value =
            serde_json::from_str(r#"{"workspaces":["packages/*","apps/*"]}"#).unwrap();
        let patterns = extract_npm_workspace_patterns(&pkg);
        assert_eq!(patterns, vec!["packages/*", "apps/*"]);
    }

    #[test]
    fn extract_patterns_from_yarn_object_form() {
        let pkg: serde_json::Value =
            serde_json::from_str(r#"{"workspaces":{"packages":["packages/*"]}}"#).unwrap();
        let patterns = extract_npm_workspace_patterns(&pkg);
        assert_eq!(patterns, vec!["packages/*"]);
    }

    #[test]
    fn extract_patterns_empty_for_missing_field() {
        let pkg: serde_json::Value = serde_json::from_str(r#"{"name":"x"}"#).unwrap();
        assert!(extract_npm_workspace_patterns(&pkg).is_empty());
    }
}
