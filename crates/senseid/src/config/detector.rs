use std::path::Path;

/// Detect workspace members / sub-packages in a monorepo.
/// Returns (name, relative_path, pkg_type) for each discovered member.
pub fn detect_workspace_members(repo_path: &Path) -> Vec<crate::types::PackageInfo> {
    let mut members = Vec::new();

    // 1. package.json workspaces
    if let Ok(content) = std::fs::read_to_string(repo_path.join("package.json"))
        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            let patterns = extract_workspace_patterns(&json);
            for pattern in &patterns {
                for entry in resolve_glob_members(repo_path, pattern) {
                    let pkg_json = repo_path.join(&entry).join("package.json");
                    if pkg_json.exists() {
                        let name = std::fs::read_to_string(&pkg_json).ok()
                            .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                            .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                            .unwrap_or_else(|| entry.clone());
                        let version = std::fs::read_to_string(&pkg_json).ok()
                            .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                            .and_then(|v| v.get("version").and_then(|n| n.as_str()).map(|s| s.to_string()));
                        members.push(crate::types::PackageInfo {
                            name,
                            path: entry,
                            version,
                            pkg_type: "npm_workspace".to_string(),
                        });
                    }
                }
            }
        }

    // 2. pnpm-workspace.yaml
    if members.is_empty()
        && let Ok(content) = std::fs::read_to_string(repo_path.join("pnpm-workspace.yaml"))
            && let Ok(yaml) = serde_yaml::from_str::<serde_json::Value>(&content)
                && let Some(packages) = yaml.get("packages").and_then(|p| p.as_array()) {
                    for p in packages {
                        if let Some(pattern) = p.as_str() {
                            for entry in resolve_glob_members(repo_path, pattern) {
                                let pkg_json = repo_path.join(&entry).join("package.json");
                                if pkg_json.exists() {
                                    let name = std::fs::read_to_string(&pkg_json).ok()
                                        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                                        .and_then(|v| v.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                                        .unwrap_or_else(|| entry.clone());
                                    members.push(crate::types::PackageInfo {
                                        name,
                                        path: entry,
                                        version: None,
                                        pkg_type: "npm_workspace".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }

    // 3. Cargo.toml workspace members
    if let Ok(content) = std::fs::read_to_string(repo_path.join("Cargo.toml"))
        && let Ok(toml_val) = content.parse::<toml::Value>()
            && let Some(workspace) = toml_val.get("workspace")
                && let Some(ws_members) = workspace.get("members").and_then(|m| m.as_array()) {
                    for m in ws_members {
                        if let Some(pattern) = m.as_str() {
                            for entry in resolve_glob_members(repo_path, pattern) {
                                let cargo_toml = repo_path.join(&entry).join("Cargo.toml");
                                if cargo_toml.exists() {
                                    let name = std::fs::read_to_string(&cargo_toml).ok()
                                        .and_then(|c| c.parse::<toml::Value>().ok())
                                        .and_then(|v| v.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str()).map(|s| s.to_string()))
                                        .unwrap_or_else(|| entry.clone());
                                    let version = std::fs::read_to_string(&cargo_toml).ok()
                                        .and_then(|c| c.parse::<toml::Value>().ok())
                                        .and_then(|v| v.get("package").and_then(|p| p.get("version")).and_then(|n| n.as_str()).map(|s| s.to_string()));
                                    members.push(crate::types::PackageInfo {
                                        name,
                                        path: entry,
                                        version,
                                        pkg_type: "cargo_crate".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }

    // 3b. Standalone crates: scan common dirs for Cargo.toml even without a workspace
    // This catches repos like sensei where crates/ has Rust packages but no root Cargo.toml
    let cargo_member_paths: std::collections::HashSet<String> = members.iter()
        .filter(|m| m.pkg_type == "cargo_crate")
        .map(|m| m.path.clone())
        .collect();
    for dir_name in &["crates", "rust", "lib", "services"] {
        let dir = repo_path.join(dir_name);
        if dir.is_dir()
            && let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    if !entry.path().is_dir() { continue; }
                    let cargo_toml = entry.path().join("Cargo.toml");
                    if !cargo_toml.exists() { continue; }
                    let rel_path = format!("{}/{}", dir_name, entry.file_name().to_string_lossy());
                    if cargo_member_paths.contains(&rel_path) { continue; } // already found via workspace
                    let name = std::fs::read_to_string(&cargo_toml).ok()
                        .and_then(|c| c.parse::<toml::Value>().ok())
                        .and_then(|v| v.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str()).map(|s| s.to_string()));
                    if let Some(name) = name {
                        let version = std::fs::read_to_string(&cargo_toml).ok()
                            .and_then(|c| c.parse::<toml::Value>().ok())
                            .and_then(|v| v.get("package").and_then(|p| p.get("version")).and_then(|n| n.as_str()).map(|s| s.to_string()));
                        members.push(crate::types::PackageInfo {
                            name,
                            path: rel_path,
                            version,
                            pkg_type: "cargo_crate".to_string(),
                        });
                    }
                }
            }
    }

    // 4. go.work
    if let Ok(content) = std::fs::read_to_string(repo_path.join("go.work")) {
        // Parse go.work: `use ( ./dir1 ./dir2 )`
        let mut in_use = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("use") { in_use = true; continue; }
            if trimmed == ")" { in_use = false; continue; }
            if in_use && !trimmed.is_empty() && !trimmed.starts_with("//") {
                let dir = trimmed.trim_start_matches("./");
                let go_mod = repo_path.join(dir).join("go.mod");
                if go_mod.exists() {
                    let name = std::fs::read_to_string(&go_mod).ok()
                        .and_then(|c| c.lines().next().map(|l| l.trim_start_matches("module ").trim().to_string()))
                        .unwrap_or_else(|| dir.to_string());
                    members.push(crate::types::PackageInfo {
                        name,
                        path: dir.to_string(),
                        version: None,
                        pkg_type: "go_module".to_string(),
                    });
                }
            }
        }
    }

    members
}

/// Extract workspace glob patterns from package.json.
fn extract_workspace_patterns(pkg: &serde_json::Value) -> Vec<String> {
    let ws = pkg.get("workspaces");
    if let Some(arr) = ws.and_then(|w| w.as_array()) {
        return arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
    }
    // Yarn-style: { workspaces: { packages: [...] } }
    if let Some(obj) = ws.and_then(|w| w.as_object())
        && let Some(packages) = obj.get("packages").and_then(|p| p.as_array()) {
            return packages.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
        }
    vec![]
}

/// Resolve a glob pattern like "packages/*" against repo_path,
/// returning relative directory paths that matched.
fn resolve_glob_members(repo_path: &Path, pattern: &str) -> Vec<String> {
    let mut results = Vec::new();
    // Simple glob handling: replace trailing /* or /** with directory listing
    let clean = pattern.trim_end_matches("/**").trim_end_matches("/*").trim_end_matches('/');
    let target_dir = repo_path.join(clean);
    if target_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&target_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !name.starts_with('.') {
                        results.push(format!("{}/{}", clean, name));
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detect_npm_workspaces() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("package.json"), r#"{"name":"monorepo","workspaces":["packages/*"]}"#).unwrap();
        std::fs::create_dir_all(dir.path().join("packages/ui")).unwrap();
        std::fs::write(dir.path().join("packages/ui/package.json"), r#"{"name":"@repo/ui","version":"1.0.0"}"#).unwrap();
        std::fs::create_dir_all(dir.path().join("packages/api")).unwrap();
        std::fs::write(dir.path().join("packages/api/package.json"), r#"{"name":"@repo/api"}"#).unwrap();
        let members = detect_workspace_members(dir.path());
        assert_eq!(members.len(), 2);
        let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"@repo/ui"));
        assert!(names.contains(&"@repo/api"));
        assert_eq!(members[0].pkg_type, "npm_workspace");
    }

    #[test]
    fn detect_cargo_workspace() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = [\"crates/*\"]").unwrap();
        std::fs::create_dir_all(dir.path().join("crates/core")).unwrap();
        std::fs::write(dir.path().join("crates/core/Cargo.toml"), "[package]\nname = \"my-core\"\nversion = \"0.1.0\"").unwrap();
        let members = detect_workspace_members(dir.path());
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name, "my-core");
        assert_eq!(members[0].pkg_type, "cargo_crate");
    }

    #[test]
    fn detect_pnpm_workspace() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("pnpm-workspace.yaml"), "packages:\n  - 'apps/*'\n  - 'packages/*'").unwrap();
        std::fs::create_dir_all(dir.path().join("apps/web")).unwrap();
        std::fs::write(dir.path().join("apps/web/package.json"), r#"{"name":"@test/web"}"#).unwrap();
        std::fs::create_dir_all(dir.path().join("packages/shared")).unwrap();
        std::fs::write(dir.path().join("packages/shared/package.json"), r#"{"name":"@test/shared"}"#).unwrap();
        let members = detect_workspace_members(dir.path());
        assert_eq!(members.len(), 2);
        let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"@test/web"));
        assert!(names.contains(&"@test/shared"));
    }

    #[test]
    fn detect_go_work() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.21\n\nuse (\n\t./cmd\n\t./pkg\n)\n").unwrap();
        std::fs::create_dir_all(dir.path().join("cmd")).unwrap();
        std::fs::write(dir.path().join("cmd/go.mod"), "module github.com/test/cmd\n\ngo 1.21").unwrap();
        std::fs::create_dir_all(dir.path().join("pkg")).unwrap();
        std::fs::write(dir.path().join("pkg/go.mod"), "module github.com/test/pkg\n\ngo 1.21").unwrap();
        let members = detect_workspace_members(dir.path());
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].pkg_type, "go_module");
    }

    #[test]
    fn detect_yarn_style_workspaces() {
        let dir = TempDir::new().unwrap();
        // Yarn-style: workspaces: { packages: [...] }
        std::fs::write(dir.path().join("package.json"),
            r#"{"name":"yarn-mono","workspaces":{"packages":["packages/*"]}}"#
        ).unwrap();
        std::fs::create_dir_all(dir.path().join("packages/lib")).unwrap();
        std::fs::write(dir.path().join("packages/lib/package.json"), r#"{"name":"@yarn/lib"}"#).unwrap();
        let members = detect_workspace_members(dir.path());
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name, "@yarn/lib");
    }

    #[test]
    fn no_workspace_returns_empty() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("package.json"), r#"{"name":"simple-app"}"#).unwrap();
        let members = detect_workspace_members(dir.path());
        assert!(members.is_empty());
    }
}
