//! Repo-level metadata scanners — icon detection, external link extraction, solution matching.
//!
//! Each scanner is a pure function that takes a repo path and returns structured results.
//! Called from process_repo after file discovery, before task enqueuing.

use std::path::Path;
use serde::Serialize;

// ── Icon Scanner ────────────────────────────────────────────────────────────

/// Detected icon/logo for a project.
#[derive(Debug, Clone, Serialize, Default)]
pub struct IconResult {
    /// Relative path to the best icon found (e.g., "assets/logo.svg").
    pub path: Option<String>,
    /// Whether a dark-mode variant exists.
    pub has_dark_variant: bool,
    /// How the icon was found: "exact", "convention", "assets-dir".
    pub source: Option<String>,
}

/// Scan a repo for icon/logo files.
///
/// Search order (first match wins):
/// 1. Exact match at root: `logo.svg`, `icon.svg`, `app-icon.png`, etc.
/// 2. Repo-name match: `{repo-name}.svg`, `{repo-name}.png`
/// 3. Convention directories (1 level deep): `assets/`, `public/`, `static/`, `.github/`, `brand/`, `branding/`, `img/`, `images/`
/// 4. Nested convention dirs (2 levels): `src/assets/`, `src/images/`
/// 5. Framework-specific: `src-tauri/icons/`, `android/app/src/main/res/`, `ios/*/Assets.xcassets/AppIcon.appiconset/`
/// 6. Favicon: `favicon.ico`, `favicon.svg`, `favicon.png`
///
/// For each match, also checks for dark variant (`-dark`, `_dark`, `.dark`).
pub fn scan_icons(repo_path: &Path) -> IconResult {
    let repo_name = repo_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    let icon_names = [
        "logo.svg", "logo.png", "icon.svg", "icon.png",
        "app-icon.svg", "app-icon.png", "Logo.svg", "Logo.png",
        "Icon.svg", "Icon.png",
    ];

    // 1. Root-level exact names
    for name in &icon_names {
        if repo_path.join(name).is_file() {
            return IconResult {
                has_dark_variant: has_dark_variant(repo_path, name),
                path: Some(name.to_string()),
                source: Some("exact".into()),
            };
        }
    }

    // 2. Repo-name match — exact and prefix
    //    Finds: kavach.svg, rokkit.svg (exact)
    //    Also:  rokkit-icon.svg, kavach-logo.png (prefix)
    //    Skips: dark/light variants — those are detected as dark_variant
    if let Some(result) = scan_repo_name_icons(repo_path, &repo_name, "") {
        return result;
    }

    // 3. Convention directories (1 level) — generic names + repo-name prefix
    let dirs = [
        "assets", "public", "static", ".github", "brand", "branding",
        "img", "images", "resources", "res",
    ];
    for dir in &dirs {
        if let Some(result) = scan_dir_for_icons(repo_path, dir, &icon_names) {
            return result;
        }
        if let Some(result) = scan_repo_name_icons(repo_path, &repo_name, dir) {
            return result;
        }
    }

    // 4. Nested convention dirs (2 levels) — generic names + repo-name prefix
    let nested_dirs = [
        "src/assets", "src/images", "src/img", "src/resources",
        "app/assets", "lib/assets", "site/static", "site/assets",
    ];
    for dir in &nested_dirs {
        if let Some(result) = scan_dir_for_icons(repo_path, dir, &icon_names) {
            return result;
        }
        if let Some(result) = scan_repo_name_icons(repo_path, &repo_name, dir) {
            return result;
        }
    }

    // 5. Framework-specific locations
    // Tauri
    let tauri_icons = repo_path.join("src-tauri/icons");
    if tauri_icons.is_dir() {
        // Prefer SVG > PNG > ICO, prefer larger sizes
        for name in &["icon.svg", "icon.png", "512x512.png", "256x256.png", "128x128.png", "icon.ico"] {
            if tauri_icons.join(name).is_file() {
                let full = format!("src-tauri/icons/{}", name);
                return IconResult {
                    has_dark_variant: false,
                    path: Some(full),
                    source: Some("tauri".into()),
                };
            }
        }
    }

    // Electron
    let electron_names = ["build/icon.png", "build/icon.svg", "build/icon.ico"];
    for name in &electron_names {
        if repo_path.join(name).is_file() {
            return IconResult {
                has_dark_variant: false,
                path: Some(name.to_string()),
                source: Some("electron".into()),
            };
        }
    }

    // 6. Favicon
    for name in &["favicon.svg", "favicon.ico", "favicon.png"] {
        if repo_path.join(name).is_file() {
            return IconResult {
                has_dark_variant: false,
                path: Some(name.to_string()),
                source: Some("favicon".into()),
            };
        }
    }
    // Favicon in public/
    for name in &["public/favicon.svg", "public/favicon.ico", "public/favicon.png"] {
        if repo_path.join(name).is_file() {
            return IconResult {
                has_dark_variant: false,
                path: Some(name.to_string()),
                source: Some("favicon".into()),
            };
        }
    }

    IconResult::default()
}

/// Scan a specific directory for icon files. Returns the first match.
fn scan_dir_for_icons(repo_path: &Path, dir: &str, icon_names: &[&str]) -> Option<IconResult> {
    let dir_path = repo_path.join(dir);
    if !dir_path.is_dir() { return None; }
    for name in icon_names {
        let full = format!("{}/{}", dir, name);
        if repo_path.join(&full).is_file() {
            return Some(IconResult {
                has_dark_variant: has_dark_variant(&dir_path, name),
                path: Some(full),
                source: Some("convention-dir".into()),
            });
        }
    }
    None
}

/// Scan for files matching `{repo_name}*.{svg,png}` in a directory.
/// Prefers: exact > icon/logo suffix > any match. Skips dark/light variants.
/// `subdir` is "" for root, or "assets" etc.
fn scan_repo_name_icons(repo_path: &Path, repo_name: &str, subdir: &str) -> Option<IconResult> {
    let scan_dir = if subdir.is_empty() { repo_path.to_path_buf() } else { repo_path.join(subdir) };
    if !scan_dir.is_dir() { return None; }

    let entries = std::fs::read_dir(&scan_dir).ok()?;
    let mut candidates: Vec<(String, String, u8)> = Vec::new(); // (rel_path, filename, priority)

    for entry in entries.flatten() {
        if !entry.path().is_file() { continue; }
        let fname = entry.file_name().to_string_lossy().to_lowercase();

        // Must be an image file starting with repo name
        if !fname.starts_with(repo_name) { continue; }
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "svg" | "png" | "ico") { continue; }

        // Skip dark/light/black/white variants — these are alternates, not primary
        if is_variant_name(&fname, repo_name) { continue; }

        let rel = if subdir.is_empty() {
            entry.file_name().to_string_lossy().to_string()
        } else {
            format!("{}/{}", subdir, entry.file_name().to_string_lossy())
        };

        // Priority: exact match > -icon/-logo suffix > anything else. SVG preferred over PNG.
        let priority = match (fname.as_str(), ext) {
            _ if fname == format!("{}.svg", repo_name) => 0,
            _ if fname == format!("{}.png", repo_name) => 1,
            _ if (fname.contains("-icon") || fname.contains("-logo")) && ext == "svg" => 2,
            _ if (fname.contains("-icon") || fname.contains("-logo")) && ext == "png" => 3,
            _ if ext == "svg" => 4,
            _ if ext == "png" => 5,
            _ => 6,
        };

        candidates.push((rel, entry.file_name().to_string_lossy().to_string(), priority));
    }

    candidates.sort_by_key(|(_, _, p)| *p);
    let (rel, _fname, _) = candidates.into_iter().next()?;

    Some(IconResult {
        has_dark_variant: has_repo_name_dark_variant(&scan_dir, repo_name),
        path: Some(rel),
        source: Some("repo-name".into()),
    })
}

/// Check if a filename is a dark/light/black/white variant (not a primary icon).
fn is_variant_name(fname: &str, repo_name: &str) -> bool {
    let suffix = &fname[repo_name.len()..];
    let variant_markers = ["-dark", "-light", "-black", "-white", "_dark", "_light", "_black", "_white", ".dark", ".light"];
    variant_markers.iter().any(|m| suffix.starts_with(m))
}

/// Check if dark-mode variants exist for repo-name icons.
fn has_repo_name_dark_variant(dir: &Path, repo_name: &str) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_lowercase();
            if fname.starts_with(repo_name) {
                let suffix = &fname[repo_name.len()..];
                if suffix.starts_with("-dark") || suffix.starts_with("_dark") || suffix.starts_with(".dark") {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a dark-mode variant exists alongside a file.
/// Looks for: `logo-dark.svg`, `logo_dark.svg`, `logo.dark.svg`, `dark/logo.svg`.
fn has_dark_variant(dir: &Path, filename: &str) -> bool {
    let stem = Path::new(filename).file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let ext = Path::new(filename).extension().and_then(|e| e.to_str()).unwrap_or("");

    let variants = [
        format!("{}-dark.{}", stem, ext),
        format!("{}_dark.{}", stem, ext),
        format!("{}.dark.{}", stem, ext),
    ];
    for v in &variants {
        if dir.join(v).is_file() {
            return true;
        }
    }
    // Also check dark/ subdirectory
    dir.join("dark").join(filename).is_file()
}

// ── External Link Scanner ───────────────────────────────────────────────────

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
fn classify_url(url: &str) -> String {
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
fn list_md_files_in(repo_path: &Path, subdir: &str, _max_depth: usize) -> Vec<String> {
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

// ── Solution Matcher ────────────────────────────────────────────────────────

/// Suggested solution grouping for discovered repos.
#[derive(Debug, Clone, Serialize)]
pub struct SolutionMatch {
    /// Suggested solution name.
    pub name: String,
    /// How the match was found: "parent-folder", "name-prefix", "subtree".
    pub strategy: String,
    /// Repo IDs that belong to this group.
    pub repo_ids: Vec<String>,
}

/// Given a list of (repo_id, repo_path) pairs, suggest solution groupings.
///
/// Strategies (applied in order, repos can only belong to one group):
/// 1. **Parent folder** — repos sharing an immediate parent directory
///    (e.g., `~/projects/acme/api` and `~/projects/acme/frontend` → "acme")
/// 2. **Name prefix** — repos with a common name stem
///    (e.g., `acme-api`, `acme-ui`, `acme-shared` → "acme")
///
/// Subtree detection is handled separately in process_repo (already exists).
pub fn suggest_solutions(repos: &[(String, String)]) -> Vec<SolutionMatch> {
    let mut assigned: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut matches = Vec::new();

    // Strategy 1: Parent folder grouping
    let parent_groups = group_by_parent_folder(repos);
    for (parent_name, repo_ids) in &parent_groups {
        if repo_ids.len() >= 2 {
            for id in repo_ids {
                assigned.insert(id.clone());
            }
            matches.push(SolutionMatch {
                name: parent_name.clone(),
                strategy: "parent-folder".into(),
                repo_ids: repo_ids.clone(),
            });
        }
    }

    // Strategy 2: Name prefix (only for repos not already grouped)
    let remaining: Vec<(String, String)> = repos.iter()
        .filter(|(id, _)| !assigned.contains(id))
        .cloned()
        .collect();

    let prefix_groups = group_by_name_prefix(&remaining);
    for (prefix, repo_ids) in &prefix_groups {
        if repo_ids.len() >= 2 {
            matches.push(SolutionMatch {
                name: prefix.clone(),
                strategy: "name-prefix".into(),
                repo_ids: repo_ids.clone(),
            });
        }
    }

    matches
}

/// Group repos by their immediate parent directory name.
fn group_by_parent_folder(repos: &[(String, String)]) -> Vec<(String, Vec<String>)> {
    let mut groups: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    for (repo_id, repo_path) in repos {
        if let Some(parent) = Path::new(repo_path).parent() {
            let parent_name = parent.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            // Skip generic parent names
            if !parent_name.is_empty()
                && !matches!(parent_name.as_str(),
                    "Developer" | "dev" | "projects" | "repos" | "src" | "code"
                    | "workspace" | "workspaces" | "home" | "Users" | "Home"
                )
            {
                groups.entry(parent_name).or_default().push(repo_id.clone());
            }
        }
    }

    groups.into_iter().collect()
}

/// Group repos by common name prefix (split on `-`, `_`, `.`).
fn group_by_name_prefix(repos: &[(String, String)]) -> Vec<(String, Vec<String>)> {
    let mut groups: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    for (repo_id, _) in repos {
        let prefix = extract_name_prefix(repo_id);
        if !prefix.is_empty() && prefix != *repo_id {
            groups.entry(prefix).or_default().push(repo_id.clone());
        }
    }

    groups.into_iter().collect()
}

/// Extract the common prefix from a repo name.
/// "acme-api" → "acme", "my-project-frontend" → "my-project"
fn extract_name_prefix(name: &str) -> String {
    // Split on common separators
    let parts: Vec<&str> = name.split(|c| c == '-' || c == '_' || c == '.').collect();
    if parts.len() >= 2 {
        parts[0].to_string()
    } else {
        String::new()
    }
}

// ── Project Summary Extractor ───────────────────────────────────────────────

/// Extracted project summary from README or config files.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ProjectSummary {
    /// One-line description (from package.json description, Cargo.toml description, or README first paragraph).
    pub description: Option<String>,
    /// Inferred project status: "active", "archived", "unmaintained".
    pub status: Option<String>,
}

/// Extract a project summary from common files.
pub fn extract_summary(repo_path: &Path) -> ProjectSummary {
    let mut summary = ProjectSummary::default();

    // Try package.json description
    if let Ok(content) = std::fs::read_to_string(repo_path.join("package.json")) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(desc) = val["description"].as_str() {
                if !desc.is_empty() {
                    summary.description = Some(desc.to_string());
                }
            }
        }
    }

    // Try Cargo.toml description
    if summary.description.is_none() {
        if let Ok(content) = std::fs::read_to_string(repo_path.join("Cargo.toml")) {
            for line in content.lines() {
                if let Some(rest) = line.trim().strip_prefix("description = ") {
                    let desc = rest.trim_matches(|c| c == '"' || c == '\'');
                    if !desc.is_empty() {
                        summary.description = Some(desc.to_string());
                        break;
                    }
                }
            }
        }
    }

    // Try README first non-heading paragraph
    if summary.description.is_none() {
        for name in &["README.md", "readme.md", "README"] {
            if let Ok(content) = std::fs::read_to_string(repo_path.join(name)) {
                summary.description = extract_first_paragraph(&content);
                break;
            }
        }
    }

    // Infer status from signals
    if repo_path.join("DEPRECATED.md").is_file()
        || repo_path.join(".archived").is_file()
    {
        summary.status = Some("archived".into());
    }

    summary
}

/// Extract the first non-heading, non-empty paragraph from markdown.
fn extract_first_paragraph(content: &str) -> Option<String> {
    let mut in_frontmatter = false;
    let mut found_content = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip frontmatter
        if trimmed == "---" {
            in_frontmatter = !in_frontmatter;
            continue;
        }
        if in_frontmatter { continue; }

        // Skip headings, badges, empty lines
        if trimmed.is_empty() {
            if found_content { break; } // end of paragraph
            continue;
        }
        if trimmed.starts_with('#') { continue; }
        if trimmed.starts_with('[') && trimmed.contains("![") { continue; } // badge
        if trimmed.starts_with("![") { continue; } // image
        if trimmed.starts_with('<') { continue; } // HTML

        found_content = true;
        let desc = trimmed.to_string();
        // Cap at 200 chars
        if desc.len() > 200 {
            return Some(format!("{}...", &desc[..197]));
        }
        return Some(desc);
    }
    None
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // ── Icon Scanner ────────────────────────────────────────────────

    #[test]
    fn scan_icons_finds_root_logo_svg() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("logo.svg"), "<svg/>").unwrap();

        let result = scan_icons(dir.path());
        assert_eq!(result.path.as_deref(), Some("logo.svg"));
        assert_eq!(result.source.as_deref(), Some("exact"));
    }

    #[test]
    fn scan_icons_finds_repo_name_match() {
        let dir = tempfile::tempdir().unwrap();
        // Create a dir named "acme-api" to simulate repo name
        let repo = dir.path().join("acme-api");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join("acme-api.svg"), "<svg/>").unwrap();

        let result = scan_icons(&repo);
        assert_eq!(result.path.as_deref(), Some("acme-api.svg"));
        assert_eq!(result.source.as_deref(), Some("repo-name"));
    }

    #[test]
    fn scan_icons_detects_dark_variant() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("logo.svg"), "<svg/>").unwrap();
        fs::write(dir.path().join("logo-dark.svg"), "<svg/>").unwrap();

        let result = scan_icons(dir.path());
        assert!(result.has_dark_variant);
    }

    #[test]
    fn scan_icons_finds_in_assets_dir() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("assets")).unwrap();
        fs::write(dir.path().join("assets/logo.png"), &[0u8]).unwrap();

        let result = scan_icons(dir.path());
        assert_eq!(result.path.as_deref(), Some("assets/logo.png"));
        assert_eq!(result.source.as_deref(), Some("convention-dir"));
    }

    #[test]
    fn scan_icons_returns_default_when_none_found() {
        let dir = tempfile::tempdir().unwrap();
        let result = scan_icons(dir.path());
        assert!(result.path.is_none());
    }

    #[test]
    fn scan_icons_finds_repo_name_prefix_with_dark() {
        // Simulates: rokkit/rokkit.svg + rokkit/rokkit-dark.svg
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("rokkit");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join("rokkit.svg"), "<svg/>").unwrap();
        fs::write(repo.join("rokkit-dark.svg"), "<svg/>").unwrap();
        fs::write(repo.join("rokkit-light.svg"), "<svg/>").unwrap();

        let result = scan_icons(&repo);
        assert_eq!(result.path.as_deref(), Some("rokkit.svg"));
        assert_eq!(result.source.as_deref(), Some("repo-name"));
        assert!(result.has_dark_variant);
    }

    #[test]
    fn scan_icons_finds_repo_name_icon_variant() {
        // Simulates: rokkit/site/static/rokkit-icon.svg
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("rokkit");
        fs::create_dir_all(repo.join("site/static")).unwrap();
        fs::write(repo.join("site/static/rokkit-icon.svg"), "<svg/>").unwrap();
        fs::write(repo.join("site/static/rokkit-dark.svg"), "<svg/>").unwrap();

        let result = scan_icons(&repo);
        assert_eq!(result.path.as_deref(), Some("site/static/rokkit-icon.svg"));
        assert!(result.has_dark_variant);
    }

    #[test]
    fn scan_icons_finds_tauri_icon() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src-tauri/icons")).unwrap();
        fs::write(dir.path().join("src-tauri/icons/icon.png"), &[0u8]).unwrap();
        fs::write(dir.path().join("src-tauri/icons/512x512.png"), &[0u8]).unwrap();

        let result = scan_icons(dir.path());
        assert_eq!(result.path.as_deref(), Some("src-tauri/icons/icon.png"));
        assert_eq!(result.source.as_deref(), Some("tauri"));
    }

    #[test]
    fn scan_icons_skips_dark_variant_as_primary() {
        // Only dark variant exists — should still find it (it's the only option)
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("kavach");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join("kavach-black.svg"), "<svg/>").unwrap();

        let result = scan_icons(&repo);
        // kavach-black is a variant, but if there's no primary, we should still have None
        // because the user would need a proper icon
        assert!(result.path.is_none());
    }

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

    // ── Solution Matcher ────────────────────────────────────────────

    #[test]
    fn suggest_solutions_by_parent_folder() {
        let repos = vec![
            ("api".into(), "/home/dev/acme/api".into()),
            ("frontend".into(), "/home/dev/acme/frontend".into()),
            ("shared".into(), "/home/dev/acme/shared".into()),
            ("blog".into(), "/home/dev/other/blog".into()),
        ];
        let matches = suggest_solutions(&repos);
        let acme = matches.iter().find(|m| m.name == "acme");
        assert!(acme.is_some());
        assert_eq!(acme.unwrap().repo_ids.len(), 3);
        assert_eq!(acme.unwrap().strategy, "parent-folder");
    }

    #[test]
    fn suggest_solutions_skips_generic_parents() {
        let repos = vec![
            ("api".into(), "/home/dev/Developer/api".into()),
            ("ui".into(), "/home/dev/Developer/ui".into()),
        ];
        let matches = suggest_solutions(&repos);
        // "Developer" is a generic name, should not create a solution
        assert!(matches.iter().all(|m| m.strategy != "parent-folder" || m.name != "Developer"));
    }

    #[test]
    fn suggest_solutions_by_name_prefix() {
        let repos = vec![
            ("acme-api".into(), "/a/acme-api".into()),
            ("acme-ui".into(), "/b/acme-ui".into()),
            ("blog".into(), "/c/blog".into()),
        ];
        let matches = suggest_solutions(&repos);
        let acme = matches.iter().find(|m| m.name == "acme");
        assert!(acme.is_some());
        assert_eq!(acme.unwrap().strategy, "name-prefix");
    }

    #[test]
    fn extract_name_prefix_splits_on_dash() {
        assert_eq!(extract_name_prefix("acme-api"), "acme");
        assert_eq!(extract_name_prefix("my-project-frontend"), "my");
    }

    #[test]
    fn extract_name_prefix_returns_empty_for_simple_name() {
        assert_eq!(extract_name_prefix("blog"), "");
    }

    // ── Project Summary ─────────────────────────────────────────────

    #[test]
    fn extract_first_paragraph_skips_frontmatter_and_headings() {
        let content = "---\ntitle: Test\n---\n# My Project\n\nThis is the description.\n\nMore text.";
        assert_eq!(
            extract_first_paragraph(content).as_deref(),
            Some("This is the description.")
        );
    }

    #[test]
    fn extract_first_paragraph_skips_badges() {
        let content = "# Title\n\n[![badge](https://img.shields.io/foo)]\n\nReal description here.";
        assert_eq!(
            extract_first_paragraph(content).as_deref(),
            Some("Real description here.")
        );
    }

    #[test]
    fn extract_summary_from_package_json() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"name": "acme", "description": "The Acme platform"}"#,
        ).unwrap();

        let summary = extract_summary(dir.path());
        assert_eq!(summary.description.as_deref(), Some("The Acme platform"));
    }

    #[test]
    fn extract_summary_detects_archived() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("DEPRECATED.md"), "This project is deprecated.").unwrap();

        let summary = extract_summary(dir.path());
        assert_eq!(summary.status.as_deref(), Some("archived"));
    }
}
