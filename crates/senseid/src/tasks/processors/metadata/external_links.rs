//! External link extraction from repo docs and config files.

use serde::Serialize;
use std::path::Path;

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
    let md_files: Vec<String> =
        ["README.md", "readme.md", "CONTRIBUTING.md", "CHANGELOG.md", ".sensei/rules.md"]
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

    // Well-known manifests — delegate link extraction to the format-aware
    // ConfigAdapter (JSON / TOML). Each ManifestAdapter registers its
    // filename; we pick the format adapter by the file's extension and let
    // it pull `homepage` / `repository` / `bugs` / `documentation` fields.
    for filename in ["package.json", "Cargo.toml", "pyproject.toml"] {
        let path = repo_path.join(filename);
        let Ok(content) = std::fs::read_to_string(&path) else { continue };
        let ext = std::path::Path::new(filename).extension().and_then(|e| e.to_str()).unwrap_or("");
        let Some(adapter) = crate::adapters::config::config_adapter_for_ext(ext) else { continue };
        for meta in adapter.extract_metadata_links(&content) {
            let external = meta.into_external(filename);
            if external.kind != "skip" {
                links.push(external);
            }
        }
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
            links.push(ExternalLink { url, kind, label, found_in: found_in.to_string() });
        }
    }

    // Also match bare URLs on their own line
    let re_bare = regex::Regex::new(r"(?m)^(https?://\S+)$").unwrap();
    for cap in re_bare.captures_iter(content) {
        let url = cap[1].to_string();
        let kind = classify_url(&url);
        if kind != "skip" {
            links.push(ExternalLink { url, kind, label: None, found_in: found_in.to_string() });
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
    if lower.contains("docs.")
        || lower.contains("/docs")
        || lower.contains("readme.io")
        || lower.contains("gitbook.io")
        || lower.contains("docusaurus")
    {
        return "docs".into();
    }
    if lower.contains("slack.com") || lower.contains("discord.gg") || lower.contains("discord.com")
    {
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
            if entry.path().is_file()
                && let Some(ext) = entry.path().extension().and_then(|e| e.to_str())
                && (ext == "md" || ext == "mdx")
            {
                let rel = format!("{}/{}", subdir, entry.file_name().to_string_lossy());
                files.push(rel);
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
    fn scan_external_links_pulls_package_json_homepage_and_bugs() {
        // End-to-end: writes a package.json into a tempdir, calls the public
        // scan_external_links entry, verifies the ConfigAdapter dispatch picks
        // up the homepage + bugs fields (plain + object forms).
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"homepage": "https://docs.acme.com", "bugs": {"url": "https://github.com/acme/api/issues"}}"#,
        ).unwrap();
        let result = scan_external_links(dir.path());
        assert_eq!(result.links.len(), 2);
        let urls: Vec<&str> = result.links.iter().map(|l| l.url.as_str()).collect();
        assert!(urls.contains(&"https://docs.acme.com"));
        assert!(urls.contains(&"https://github.com/acme/api/issues"));
    }

    #[test]
    fn scan_external_links_pulls_cargo_toml_homepage_and_repository() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"x\"\nhomepage = \"https://sensei-hq.com\"\nrepository = \"https://github.com/sensei-hq/sensei\"\n",
        ).unwrap();
        let result = scan_external_links(dir.path());
        let urls: Vec<&str> = result.links.iter().map(|l| l.url.as_str()).collect();
        assert!(urls.contains(&"https://sensei-hq.com"));
        assert!(urls.contains(&"https://github.com/sensei-hq/sensei"));
    }

    #[test]
    fn scan_external_links_pulls_pyproject_urls() {
        // New: pyproject.toml [project.urls] now feeds the external-link
        // pass because the TomlConfigAdapter also reads the pyproject shape.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"y\"\n\n[project.urls]\nhomepage = \"https://py.example\"\n",
        )
        .unwrap();
        let result = scan_external_links(dir.path());
        let urls: Vec<&str> = result.links.iter().map(|l| l.url.as_str()).collect();
        assert!(urls.contains(&"https://py.example"));
    }
}
