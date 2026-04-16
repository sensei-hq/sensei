use std::path::Path;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub file_type: String,
    pub path: String,
    pub data: serde_json::Value,
}

/// Detect the technology stack from config files in a repo.
#[allow(dead_code)]
pub fn detect_stack(repo_path: &Path) -> Vec<String> {
    let mut stack = Vec::new();

    if repo_path.join("Cargo.toml").exists() { stack.push("rust".into()); }
    if repo_path.join("go.mod").exists() { stack.push("go".into()); }
    if repo_path.join("Package.swift").exists() { stack.push("swift".into()); }
    if repo_path.join("pubspec.yaml").exists() { stack.push("dart".into()); }
    if repo_path.join("Gemfile").exists() { stack.push("ruby".into()); }
    if repo_path.join("pom.xml").exists() || repo_path.join("build.gradle").exists() {
        stack.push("java".into());
    }
    if repo_path.join("pyproject.toml").exists() || repo_path.join("requirements.txt").exists() {
        stack.push("python".into());
    }

    if repo_path.join("package.json").exists() {
        if let Ok(content) = std::fs::read_to_string(repo_path.join("package.json")) {
            let frameworks = [
                ("@sveltejs/kit", "sveltekit"),
                ("svelte", "svelte"),
                ("next", "nextjs"),
                ("react", "react"),
                ("nuxt", "nuxtjs"),
                ("vue", "vue"),
                ("solid-js", "solid"),
                ("@tauri-apps/api", "tauri"),
                ("electron", "electron"),
                ("@angular/core", "angular"),
                ("hono", "hono"),
                ("express", "express"),
            ];
            let mut found = false;
            for (key, label) in &frameworks {
                if content.contains(&format!("\"{}\"", key)) {
                    stack.push(label.to_string());
                    found = true;
                    break;
                }
            }
            if !found { stack.push("node".into()); }
        }
    }

    stack
}

/// Detect and parse config files in a repo.
#[allow(dead_code)]
pub fn detect_config_files(repo_path: &Path) -> Vec<ConfigFile> {
    let mut configs = Vec::new();

    // package.json
    if let Ok(content) = std::fs::read_to_string(repo_path.join("package.json")) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            let extracted = serde_json::json!({
                "name": json.get("name"),
                "version": json.get("version"),
                "type": json.get("type"),
                "scripts": json.get("scripts"),
                "dependencies": extract_dep_names(&json, "dependencies"),
                "devDependencies": extract_dep_names(&json, "devDependencies"),
                "workspaces": json.get("workspaces"),
            });
            configs.push(ConfigFile {
                file_type: "package.json".into(),
                path: "package.json".into(),
                data: extracted,
            });
        }
    }

    // Cargo.toml
    if let Ok(content) = std::fs::read_to_string(repo_path.join("Cargo.toml")) {
        if let Ok(toml_val) = content.parse::<toml::Value>() {
            let extracted = serde_json::json!({
                "name": toml_val.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str()),
                "version": toml_val.get("package").and_then(|p| p.get("version")).and_then(|v| v.as_str()),
                "edition": toml_val.get("package").and_then(|p| p.get("edition")).and_then(|e| e.as_str()),
                "hasWorkspace": toml_val.get("workspace").is_some(),
            });
            configs.push(ConfigFile {
                file_type: "cargo.toml".into(),
                path: "Cargo.toml".into(),
                data: extracted,
            });
        }
    }

    // pyproject.toml
    if let Ok(content) = std::fs::read_to_string(repo_path.join("pyproject.toml")) {
        if let Ok(toml_val) = content.parse::<toml::Value>() {
            let name = toml_val.get("project").and_then(|p| p.get("name")).and_then(|n| n.as_str())
                .or_else(|| toml_val.get("tool").and_then(|t| t.get("poetry")).and_then(|p| p.get("name")).and_then(|n| n.as_str()));
            configs.push(ConfigFile {
                file_type: "pyproject.toml".into(),
                path: "pyproject.toml".into(),
                data: serde_json::json!({ "name": name }),
            });
        }
    }

    // Dockerfile
    if repo_path.join("Dockerfile").exists() {
        if let Ok(content) = std::fs::read_to_string(repo_path.join("Dockerfile")) {
            let from = content.lines()
                .find(|l| l.starts_with("FROM "))
                .map(|l| l.trim_start_matches("FROM ").split_whitespace().next().unwrap_or(""))
                .unwrap_or("");
            let expose: Vec<&str> = content.lines()
                .filter(|l| l.starts_with("EXPOSE "))
                .map(|l| l.trim_start_matches("EXPOSE ").trim())
                .collect();
            configs.push(ConfigFile {
                file_type: "dockerfile".into(),
                path: "Dockerfile".into(),
                data: serde_json::json!({ "from": from, "expose": expose }),
            });
        }
    }

    // docker-compose.yml
    for name in &["docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"] {
        if let Ok(content) = std::fs::read_to_string(repo_path.join(name)) {
            if let Ok(yaml) = serde_yaml::from_str::<serde_json::Value>(&content) {
                let services: Vec<String> = yaml.get("services")
                    .and_then(|s| s.as_object())
                    .map(|m| m.keys().cloned().collect())
                    .unwrap_or_default();
                configs.push(ConfigFile {
                    file_type: "docker-compose".into(),
                    path: name.to_string(),
                    data: serde_json::json!({ "services": services }),
                });
            }
            break;
        }
    }

    configs
}

/// Detect workspace members / sub-packages in a monorepo.
/// Returns (name, relative_path, pkg_type) for each discovered member.
pub fn detect_workspace_members(repo_path: &Path) -> Vec<crate::types::PackageInfo> {
    let mut members = Vec::new();

    // 1. package.json workspaces
    if let Ok(content) = std::fs::read_to_string(repo_path.join("package.json")) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
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
    }

    // 2. pnpm-workspace.yaml
    if members.is_empty() {
        if let Ok(content) = std::fs::read_to_string(repo_path.join("pnpm-workspace.yaml")) {
            if let Ok(yaml) = serde_yaml::from_str::<serde_json::Value>(&content) {
                if let Some(packages) = yaml.get("packages").and_then(|p| p.as_array()) {
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
            }
        }
    }

    // 3. Cargo.toml workspace members
    if let Ok(content) = std::fs::read_to_string(repo_path.join("Cargo.toml")) {
        if let Ok(toml_val) = content.parse::<toml::Value>() {
            if let Some(workspace) = toml_val.get("workspace") {
                if let Some(ws_members) = workspace.get("members").and_then(|m| m.as_array()) {
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
        if dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
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
    if let Some(obj) = ws.and_then(|w| w.as_object()) {
        if let Some(packages) = obj.get("packages").and_then(|p| p.as_array()) {
            return packages.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
        }
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

fn extract_dep_names(pkg: &serde_json::Value, field: &str) -> Vec<String> {
    pkg.get(field)
        .and_then(|d| d.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detect_node_stack() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("package.json"), r#"{"name":"test","dependencies":{"express":"4"}}"#).unwrap();
        let stack = detect_stack(dir.path());
        assert!(stack.contains(&"express".to_string()));
    }

    #[test]
    fn detect_rust_stack() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"foo\"\nversion = \"0.1.0\"\nedition = \"2024\"").unwrap();
        let stack = detect_stack(dir.path());
        assert!(stack.contains(&"rust".to_string()));
    }

    #[test]
    fn detect_python_stack() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("requirements.txt"), "flask\nrequests").unwrap();
        let stack = detect_stack(dir.path());
        assert!(stack.contains(&"python".to_string()));
    }

    #[test]
    fn parse_package_json() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("package.json"),
            r#"{"name":"my-app","version":"1.0.0","dependencies":{"hono":"4","zod":"3"},"scripts":{"dev":"bun run dev"}}"#
        ).unwrap();
        let configs = detect_config_files(dir.path());
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].file_type, "package.json");
        let deps = configs[0].data["dependencies"].as_array().unwrap();
        assert!(deps.contains(&serde_json::json!("hono")));
        assert!(deps.contains(&serde_json::json!("zod")));
    }

    #[test]
    fn parse_cargo_toml() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"),
            "[package]\nname = \"senseid\"\nversion = \"0.1.0\"\nedition = \"2024\""
        ).unwrap();
        let configs = detect_config_files(dir.path());
        assert_eq!(configs[0].file_type, "cargo.toml");
        assert_eq!(configs[0].data["name"], "senseid");
    }

    #[test]
    fn parse_dockerfile() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("Dockerfile"), "FROM node:20-alpine\nEXPOSE 3000\nCMD [\"node\", \"index.js\"]").unwrap();
        let configs = detect_config_files(dir.path());
        let dc = configs.iter().find(|c| c.file_type == "dockerfile").unwrap();
        assert_eq!(dc.data["from"], "node:20-alpine");
        assert_eq!(dc.data["expose"][0], "3000");
    }

    #[test]
    fn parse_docker_compose() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("docker-compose.yml"),
            "services:\n  api:\n    image: node:20\n  db:\n    image: postgres:16"
        ).unwrap();
        let configs = detect_config_files(dir.path());
        let dc = configs.iter().find(|c| c.file_type == "docker-compose").unwrap();
        let services = dc.data["services"].as_array().unwrap();
        assert!(services.contains(&serde_json::json!("api")));
        assert!(services.contains(&serde_json::json!("db")));
    }

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
    fn empty_dir_returns_nothing() {
        let dir = TempDir::new().unwrap();
        assert!(detect_stack(dir.path()).is_empty());
        assert!(detect_config_files(dir.path()).is_empty());
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

    #[test]
    fn e2e_sensei_repo_detection() {
        // Test against the actual sensei repo
        let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap().parent().unwrap().to_path_buf();
        if !repo.join("package.json").exists() { return; }

        let stack = detect_stack(&repo);
        assert!(!stack.is_empty(), "sensei repo should have a detected stack");

        let configs = detect_config_files(&repo);
        assert!(!configs.is_empty(), "sensei repo should have config files");
        let pkg = configs.iter().find(|c| c.file_type == "package.json").unwrap();
        assert!(pkg.data["name"].as_str().is_some());
    }
}
