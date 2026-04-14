use crate::db::Store;

/// Fetch library content from URL (async part).
pub async fn fetch_lib_url(url: &str) -> Result<String, String> {
    fetch_url(url).await
}

/// Index pre-fetched library content (sync part — safe to hold mutex).
pub fn index_lib_content(
    store: &Store,
    lib_name: &str,
    url: &str,
    content: &str,
    version: Option<&str>,
) -> Result<LibIndexResult, String> {
    let source_type = detect_source_type(url, content);

    let docs = match source_type.as_str() {
        "llms-txt" => parse_llms_txt(&content, lib_name),
        "markdown" => parse_markdown(&content, lib_name, url),
        _ => parse_markdown(&content, lib_name, url),
    };

    let now = chrono::Utc::now().to_rfc3339();
    let mut indexed = 0u32;

    for doc in &docs {
        let id = format!("lib:{}:{}", lib_name, doc.title.replace(' ', "_").to_lowercase());
        store.upsert_lib_doc(
            &id, lib_name, &doc.title, Some(url), &doc.summary,
            Some(&doc.content), &source_type, doc.component.as_deref(), &now,
        ).map_err(|e| e.to_string())?;
        indexed += 1;
    }

    // Update lib_meta
    store.upsert_lib_meta(lib_name, &source_type, Some(url), version, &now)
        .map_err(|e| e.to_string())?;

    Ok(LibIndexResult {
        lib_name: lib_name.to_string(),
        docs_indexed: indexed,
        source_type,
        version: version.map(|v| v.to_string()),
    })
}

/// Fetch and store dependency versions from a project's manifest files.
pub fn extract_dep_versions(
    store: &Store,
    repo_id: &str,
    repo_path: &str,
) -> Result<Vec<DepVersion>, String> {
    let repo = std::path::Path::new(repo_path);
    let mut deps = Vec::new();

    // package.json
    if let Ok(content) = std::fs::read_to_string(repo.join("package.json")) {
        if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
            for section in &["dependencies", "devDependencies", "peerDependencies"] {
                if let Some(obj) = pkg.get(section).and_then(|v| v.as_object()) {
                    for (name, ver) in obj {
                        let version = ver.as_str().unwrap_or("*").to_string();
                        deps.push(DepVersion {
                            lib_name: name.clone(),
                            version: clean_version(&version),
                            raw_version: version,
                            source: "package.json".into(),
                            dev: *section != "dependencies",
                        });
                    }
                }
            }
        }
    }

    // Cargo.toml
    if let Ok(content) = std::fs::read_to_string(repo.join("Cargo.toml")) {
        if let Ok(cargo) = content.parse::<toml::Value>() {
            for section in &["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(obj) = cargo.get(section).and_then(|v| v.as_table()) {
                    for (name, ver) in obj {
                        let version = match ver {
                            toml::Value::String(s) => s.clone(),
                            toml::Value::Table(t) => t.get("version")
                                .and_then(|v| v.as_str())
                                .unwrap_or("*").to_string(),
                            _ => "*".into(),
                        };
                        deps.push(DepVersion {
                            lib_name: name.clone(),
                            version: clean_version(&version),
                            raw_version: version,
                            source: "Cargo.toml".into(),
                            dev: *section != "dependencies",
                        });
                    }
                }
            }
        }
    }

    // pyproject.toml
    if let Ok(content) = std::fs::read_to_string(repo.join("pyproject.toml")) {
        if let Ok(pyp) = content.parse::<toml::Value>() {
            if let Some(deps_arr) = pyp.get("project").and_then(|v| v.get("dependencies")).and_then(|v| v.as_array()) {
                for dep in deps_arr {
                    if let Some(s) = dep.as_str() {
                        let (name, ver) = parse_pep508(s);
                        deps.push(DepVersion {
                            lib_name: name,
                            version: ver.clone(),
                            raw_version: ver,
                            source: "pyproject.toml".into(),
                            dev: false,
                        });
                    }
                }
            }
        }
    }

    // Store versions in lib_meta
    let now = chrono::Utc::now().to_rfc3339();
    for dep in &deps {
        store.upsert_lib_meta(&dep.lib_name, "dependency", None, Some(&dep.version), &now).ok();
    }

    Ok(deps)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LibIndexResult {
    pub lib_name: String,
    pub docs_indexed: u32,
    pub source_type: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DepVersion {
    pub lib_name: String,
    pub version: String,
    pub raw_version: String,
    pub source: String,
    pub dev: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LibDoc {
    pub id: String,
    pub title: String,
    pub url: String,
    pub summary: String,
    pub content: Option<String>,
    pub source_type: String,
    pub component: String,
    pub indexed_at: String,
}

struct ParsedDoc {
    title: String,
    summary: String,
    content: String,
    component: Option<String>,
}

/// Fetch a URL. Public so MCP handler can use it.
pub async fn fetch_lib_url_with_timeout(url: &str, timeout_secs: u64) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client.get(url).send().await.map_err(|e| format!("Fetch failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("Read body: {}", e))
}

async fn fetch_url(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client.get(url).send().await.map_err(|e| format!("Fetch failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("Read body: {}", e))
}

fn detect_source_type(url: &str, content: &str) -> String {
    if url.ends_with("llms.txt") || url.ends_with("llms-full.txt") {
        return "llms-txt".into();
    }
    if url.contains("raw.githubusercontent.com") || url.ends_with(".md") {
        return "markdown".into();
    }
    if content.starts_with("# ") || content.contains("\n## ") {
        return "markdown".into();
    }
    "text".into()
}

/// Parse llms.txt format — sections delimited by `# heading` with content.
fn parse_llms_txt(content: &str, lib_name: &str) -> Vec<ParsedDoc> {
    let mut docs = Vec::new();
    let mut current_title = String::new();
    let mut current_content = String::new();

    for line in content.lines() {
        if line.starts_with("# ") {
            // Flush previous section
            if !current_title.is_empty() && !current_content.trim().is_empty() {
                let summary = current_content.lines().take(3).collect::<Vec<_>>().join(" ");
                docs.push(ParsedDoc {
                    title: current_title.clone(),
                    summary: summary.chars().take(200).collect(),
                    content: current_content.trim().to_string(),
                    component: None,
                });
            }
            current_title = line[2..].trim().to_string();
            current_content.clear();
        } else if line.starts_with("## ") {
            // Sub-section becomes component
            if !current_title.is_empty() && !current_content.trim().is_empty() {
                let summary = current_content.lines().take(3).collect::<Vec<_>>().join(" ");
                docs.push(ParsedDoc {
                    title: current_title.clone(),
                    summary: summary.chars().take(200).collect(),
                    content: current_content.trim().to_string(),
                    component: None,
                });
            }
            current_title = line[3..].trim().to_string();
            current_content.clear();
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Flush last section
    if !current_title.is_empty() && !current_content.trim().is_empty() {
        let summary = current_content.lines().take(3).collect::<Vec<_>>().join(" ");
        docs.push(ParsedDoc {
            title: current_title,
            summary: summary.chars().take(200).collect(),
            content: current_content.trim().to_string(),
            component: None,
        });
    }

    if docs.is_empty() {
        // Treat entire content as one doc
        let summary = content.lines().take(3).collect::<Vec<_>>().join(" ");
        docs.push(ParsedDoc {
            title: lib_name.to_string(),
            summary: summary.chars().take(200).collect(),
            content: content.to_string(),
            component: None,
        });
    }

    docs
}

fn parse_markdown(content: &str, lib_name: &str, _url: &str) -> Vec<ParsedDoc> {
    // Same as llms_txt for now — split by headings
    parse_llms_txt(content, lib_name)
}

fn clean_version(v: &str) -> String {
    v.trim_start_matches('^')
        .trim_start_matches('~')
        .trim_start_matches(">=")
        .trim_start_matches("<=")
        .trim_start_matches('>')
        .trim_start_matches('<')
        .trim_start_matches('=')
        .trim()
        .to_string()
}

fn parse_pep508(spec: &str) -> (String, String) {
    // "requests>=2.28" → ("requests", "2.28")
    let parts: Vec<&str> = spec.splitn(2, |c: char| c == '>' || c == '<' || c == '=' || c == '!' || c == '[').collect();
    let name = parts[0].trim().to_string();
    let version = if parts.len() > 1 {
        clean_version(parts[1].trim_start_matches('='))
    } else {
        "*".into()
    };
    (name, version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_llms_txt_sections() {
        let content = "# Overview\nThis is a test lib.\n\n## Components\nButton, Card\n\n# API\nSome API docs.";
        let docs = parse_llms_txt(content, "test-lib");
        assert!(docs.len() >= 2);
        assert_eq!(docs[0].title, "Overview");
    }

    #[test]
    fn clean_semver() {
        assert_eq!(clean_version("^1.2.3"), "1.2.3");
        assert_eq!(clean_version("~0.8"), "0.8");
        assert_eq!(clean_version(">=2.0"), "2.0");
    }

    #[test]
    fn parse_pep508_spec() {
        let (name, ver) = parse_pep508("requests>=2.28");
        assert_eq!(name, "requests");
        assert_eq!(ver, "2.28");

        let (name, ver) = parse_pep508("flask");
        assert_eq!(name, "flask");
        assert_eq!(ver, "*");
    }

    #[test]
    fn detect_llms_txt() {
        assert_eq!(detect_source_type("https://example.com/llms.txt", ""), "llms-txt");
        assert_eq!(detect_source_type("https://example.com/README.md", "# Hi"), "markdown");
    }
}
