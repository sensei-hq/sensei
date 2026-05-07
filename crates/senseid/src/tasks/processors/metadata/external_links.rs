//! External link extraction from repo docs and config files.

use std::path::Path;
use serde::Serialize;

/// External links found in a repo's docs and config.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ExternalLinksResult {
    pub links: Vec<ExternalLink>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalLink {
    /// The URL.
    pub url: String,
    /// Classified kind: "jira", "confluence", "wiki", "docs", "ci", "hosting", "other".
    pub kind: String,
    /// Human label (from markdown link text, or inferred from URL).
    pub label: Option<String>,
    /// Which file it was found in.
    pub found_in: String,
}

/// Scan specific files in a repo for external links.
///
/// Checks: README.md, CONTRIBUTING.md, .sensei/rules.md, package.json (homepage/bugs/repository),
/// Cargo.toml (homepage/repository), and docs/*.md (first level only).
pub fn scan_external_links(repo_path: &Path) -> ExternalLinksResult {
    let mut links = Vec::new();

    // Markdown files to scan
    let md_files: Vec<String> = [
        "README.md", "readme.md", "CONTRIBUTING.md", "CHANGELOG.md",
        ".sensei/rules.md",
    ]
    .iter()
    .map(|f| f.to_string())
    .chain(list_md_files_in(repo_path, "docs", 1))
    .collect();

    for rel_path in &md_files {
        let abs = repo_path.join(rel_path);
        if let Ok(content) = std::fs::read_to_string(&abs) {
            extract_markdown_links(&content, rel_path, &mut links);
        }
    }

    // package.json
    let pkg = repo_path.join("package.json");
    if let Ok(content) = std::fs::read_to_string(&pkg) {
        extract_package_json_links(&content, "package.json", &mut links);
    }

    // Cargo.toml
    let cargo = repo_path.join("Cargo.toml");
    if let Ok(content) = std::fs::read_to_string(&cargo) {
        extract_toml_links(&content, "Cargo.toml", &mut links);
    }

    // Deduplicate by URL
    links.sort_by(|a, b| a.url.cmp(&b.url));
    links.dedup_by(|a, b| a.url == b.url);

    ExternalLinksResult { links }
}

/// Extract markdown links `[text](url)` and classify them.
fn extract_markdown_links(content: &str, found_in: &str, links: &mut Vec<ExternalLink>) {
    // Match [label](url) patterns
    let re_inline = regex::Regex::new(r"\[([^\]]*)\]\((https?://[^\)]+)\)").unwrap();
    for cap in re_inline.captures_iter(content) {
        let label = cap.get(1).map(|m| m.as_str().to_string());
        let url = cap[2].to_string();
        let kind = classify_url(&url);
        if kind != "skip" {
            links.push(ExternalLink {
                url,
                kind,
                label,
                found_in: found_in.to_string(),
            });
        }
    }

    // Also match bare URLs on their own line
    let re_bare = regex::Regex::new(r"(?m)^(https?://\S+)$").unwrap();
    for cap in re_bare.captures_iter(content) {
        let url = cap[1].to_string();
        let kind = classify_url(&url);
        if kind != "skip" {
            links.push(ExternalLink {
                url,
                kind,
                label: None,
                found_in: found_in.to_string(),
            });
        }
    }
}

/// Extract links from package.json fields.
fn extract_package_json_links(content: &str, found_in: &str, links: &mut Vec<ExternalLink>) {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
        for key in &["homepage", "repository", "bugs"] {
            if let Some(url) = val[key].as_str() {
                if url.starts_with("http") {
                    links.push(ExternalLink {
                        url: url.to_string(),
                        kind: classify_url(url),
                        label: Some(key.to_string()),
                        found_in: found_in.to_string(),
                    });
                }
            }
            // repository can be {type, url}
            if let Some(url) = val[key]["url"].as_str() {
                if url.starts_with("http") {
                    links.push(ExternalLink {
                        url: url.to_string(),
                        kind: classify_url(url),
                        label: Some(key.to_string()),
                        found_in: found_in.to_string(),
                    });
                }
            }
        }
    }
}

/// Extract links from Cargo.toml fields.
fn extract_toml_links(content: &str, found_in: &str, links: &mut Vec<ExternalLink>) {
    // Simple line-based extraction — avoid pulling in a toml parser just for this
    for line in content.lines() {
        let trimmed = line.trim();
        for key in &["homepage", "repository", "documentation"] {
            let prefix = format!("{} = ", key);
            if let Some(rest) = trimmed.strip_prefix(&prefix) {
                let url = rest.trim_matches(|c| c == '"' || c == '\'').to_string();
                if url.starts_with("http") {
                    links.push(ExternalLink {
                        url,
                        kind: classify_url(rest),
                        label: Some(key.to_string()),
                        found_in: found_in.to_string(),
                    });
                }
            }
        }
    }
}

/// Classify a URL by its domain/path.
pub(crate) fn classify_url(url: &str) -> String {
    let lower = url.to_lowercase();

    // Skip common noise
    if lower.contains("shields.io") || lower.contains("badge") || lower.contains("img.shields") {
        return "skip".into();
    }

    if lower.contains("jira") || lower.contains("atlassian.net/browse") {
        return "jira".into();
    }
    if lower.contains("confluence") || lower.contains("atlassian.net/wiki") {
        return "confluence".into();
    }
    if lower.contains("notion.so") || lower.contains("notion.site") {
        return "wiki".into();
    }
    if lower.contains("dbdocs.io") || lower.contains("dbdiagram.io") {
        return "database-docs".into();
    }
    if lower.contains("figma.com") {
        return "design".into();
    }
    if lower.contains("linear.app") {
        return "issues".into();
    }
    if lower.contains("github.com") && lower.contains("/issues") {
        return "issues".into();
    }
    if lower.contains("github.com") && lower.contains("/wiki") {
        return "wiki".into();
    }
    if lower.contains("github.com") && lower.contains("/actions") {
        return "ci".into();
    }
    if lower.contains("docs.") || lower.contains("/docs") || lower.contains("readme.io")
        || lower.contains("gitbook.io") || lower.contains("docusaurus")
    {
        return "docs".into();
    }
    if lower.contains("slack.com") || lower.contains("discord.gg") || lower.contains("discord.com") {
        return "chat".into();
    }
    if lower.contains("vercel.app") || lower.contains("netlify.app") || lower.contains("heroku") {
        return "hosting".into();
    }
    if lower.contains("circleci") || lower.contains("travis-ci") || lower.contains("jenkins") {
        return "ci".into();
    }
    if lower.contains("sentry.io") || lower.contains("datadog") || lower.contains("grafana") {
        return "monitoring".into();
    }
    "other".into()
}

/// List .md files in a directory (non-recursive or up to depth).
pub(crate) fn list_md_files_in(repo_path: &Path, subdir: &str, _max_depth: usize) -> Vec<String> {
    let dir = repo_path.join(subdir);
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                    if ext == "md" || ext == "mdx" {
                        let rel = format!("{}/{}", subdir, entry.file_name().to_string_lossy());
                        files.push(rel);
                    }
                }
            }
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── URL Classification ──────────────────────────────────────────

    #[test]
    fn classify_jira() {
        assert_eq!(classify_url("https://myorg.atlassian.net/browse/PROJ-123"), "jira");
    }

    #[test]
    fn classify_confluence() {
        assert_eq!(classify_url("https://myorg.atlassian.net/wiki/spaces/DEV"), "confluence");
    }

    #[test]
    fn classify_figma() {
        assert_eq!(classify_url("https://figma.com/file/abc123"), "design");
    }

    #[test]
    fn classify_linear() {
        assert_eq!(classify_url("https://linear.app/myorg/issue/PROJ-42"), "issues");
    }

    #[test]
    fn classify_dbdocs() {
        assert_eq!(classify_url("https://dbdocs.io/myorg/schema"), "database-docs");
    }

    #[test]
    fn classify_shields_skipped() {
        assert_eq!(classify_url("https://img.shields.io/badge/foo-bar"), "skip");
    }

    // ── Link Extraction ─────────────────────────────────────────────

    #[test]
    fn extract_markdown_links_finds_inline() {
        let content = "Check our [Jira board](https://myorg.atlassian.net/browse/PROJ) for issues.";
        let mut links = Vec::new();
        extract_markdown_links(content, "README.md", &mut links);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].kind, "jira");
        assert_eq!(links[0].label.as_deref(), Some("Jira board"));
    }

    #[test]
    fn extract_package_json_links_finds_homepage() {
        let content = r#"{"homepage": "https://docs.acme.com", "bugs": {"url": "https://github.com/acme/api/issues"}}"#;
        let mut links = Vec::new();
        extract_package_json_links(content, "package.json", &mut links);
        assert_eq!(links.len(), 2);
    }
}
