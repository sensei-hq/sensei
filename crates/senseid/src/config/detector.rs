use std::path::Path;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub file_type: String,
    pub path: String,
    pub data: serde_json::Value,
}

/// Detect the technology stack from config files in a repo.
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
    fn empty_dir_returns_nothing() {
        let dir = TempDir::new().unwrap();
        assert!(detect_stack(dir.path()).is_empty());
        assert!(detect_config_files(dir.path()).is_empty());
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
