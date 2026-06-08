//! README frontmatter parser — derives repo identity (organization, project,
//! team, role, stack, summary, tagline, icon) from the YAML frontmatter at the
//! top of a repo's root README. Read-only: scanning never writes frontmatter
//! back — that is an opt-in sync gated by a preference.

use std::path::Path;

/// Parsed identity from a repo's README frontmatter. Every field is optional;
/// absent fields fall back to best-guess elsewhere in the scanner.
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize)]
pub struct Frontmatter {
    pub organization: Option<String>,
    pub client: Option<String>,
    pub project: Option<String>,
    pub team: Option<String>,
    pub role: Option<String>,
    pub stack: Vec<String>,
    pub summary: Option<String>,
    pub tagline: Option<String>,
    /// Default/light icon. A repo-relative path (e.g. `sensei.svg`) or a URL.
    pub icon: Option<String>,
    /// Optional dark-mode icon variant (`icon_dark:` in frontmatter).
    pub icon_dark: Option<String>,
}

/// Parse YAML frontmatter from the top of a markdown document. Returns `None`
/// when there is no `---`-fenced block at the very start, the block is unclosed,
/// or the YAML does not parse.
pub fn parse_frontmatter(content: &str) -> Option<Frontmatter> {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let mut lines = content.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut yaml = String::new();
    let mut closed = false;
    for line in lines {
        if line.trim() == "---" {
            closed = true;
            break;
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    if !closed {
        return None;
    }

    let v: serde_json::Value = serde_yaml::from_str(&yaml).ok()?;

    let str_field = |k: &str| {
        v.get(k)
            .and_then(|x| x.as_str())
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
    };
    let stack = match v.get("stack") {
        Some(serde_json::Value::Array(a)) => a
            .iter()
            .filter_map(|e| e.as_str())
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect(),
        Some(serde_json::Value::String(s)) => s
            .split(',')
            .map(|t| t.trim().to_lowercase())
            .filter(|t| !t.is_empty())
            .collect(),
        _ => Vec::new(),
    };

    Some(Frontmatter {
        organization: str_field("organization"),
        client: str_field("client"),
        project: str_field("project"),
        team: str_field("team"),
        role: str_field("role"),
        stack,
        summary: str_field("summary"),
        tagline: str_field("tagline"),
        icon: str_field("icon"),
        icon_dark: str_field("icon_dark"),
    })
}

/// Classify an icon reference as a remote URL (vs a repo-relative path) so the
/// consumer knows whether to load it directly or resolve it against the repo.
pub fn icon_is_url(path: &str) -> bool {
    let p = path.trim();
    p.starts_with("http://") || p.starts_with("https://") || p.starts_with("//") || p.starts_with("data:")
}

/// Map a free-text frontmatter `role` to a canonical `folder_role` enum value,
/// accepting common synonyms. Returns `None` for values that aren't generic
/// folder roles (e.g. project-specific `marketplace`, or structural `monorepo`)
/// — those are kept only as a `role:<raw>` project tag by the caller.
pub fn folder_role_from_frontmatter(role: &str) -> Option<&'static str> {
    match role.trim().to_ascii_lowercase().as_str() {
        "backend" | "server" | "api" | "service" | "daemon" => Some("backend"),
        "frontend" | "ui" | "client" => Some("frontend"),
        "library" | "lib" | "package" | "sdk" => Some("library"),
        "docs" | "doc" | "documentation" => Some("docs"),
        "infra" | "infrastructure" | "ops" | "devops" | "ci" => Some("infra"),
        "website" | "site" | "web" | "marketing" => Some("website"),
        "desktop" | "app" => Some("desktop"),
        "mobile" | "ios" | "android" => Some("mobile"),
        "config" | "configuration" | "settings" => Some("config"),
        "packaging" | "tap" | "homebrew" | "brew" | "formula" | "distribution" => Some("packaging"),
        // monorepo (structural), marketplace/extensions (project-specific, rare),
        // and anything unrecognised → no enum role; kept as a tag.
        _ => None,
    }
}

/// Normalize a name into a slug (lowercase, non-alphanumeric runs collapsed to
/// a single `-`, trimmed). Used as the unique key for a namespace within a scope.
pub fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = false;
    for c in s.trim().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !out.is_empty() && !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

/// Read and parse frontmatter from a repo's root README, if present.
pub fn read_frontmatter(repo_path: &Path) -> Option<Frontmatter> {
    for name in &["README.md", "readme.md", "Readme.md", "README"] {
        if let Ok(content) = std::fs::read_to_string(repo_path.join(name)) {
            return parse_frontmatter(&content);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_frontmatter() {
        let md = "---\norganization: Sensei HQ\nproject: sensei\nrole: monorepo\nstack: [rust, SvelteKit]\nsummary: A one-line summary.\ntagline: A tagline\nicon: sensei.svg\n---\n\n# Sensei\nbody";
        let fm = parse_frontmatter(md).unwrap();
        assert_eq!(fm.organization.as_deref(), Some("Sensei HQ"));
        assert_eq!(fm.project.as_deref(), Some("sensei"));
        assert_eq!(fm.role.as_deref(), Some("monorepo"));
        assert_eq!(fm.stack, vec!["rust", "sveltekit"]);
        assert_eq!(fm.summary.as_deref(), Some("A one-line summary."));
        assert_eq!(fm.tagline.as_deref(), Some("A tagline"));
        assert_eq!(fm.icon.as_deref(), Some("sensei.svg"));
    }

    #[test]
    fn none_without_frontmatter() {
        assert!(parse_frontmatter("# Title\n\nNo frontmatter here.").is_none());
    }

    #[test]
    fn none_on_unclosed_block() {
        assert!(parse_frontmatter("---\nproject: x\n# never closed").is_none());
    }

    #[test]
    fn stack_accepts_comma_string() {
        let fm = parse_frontmatter("---\nstack: rust, TypeScript\n---\n").unwrap();
        assert_eq!(fm.stack, vec!["rust", "typescript"]);
    }

    #[test]
    fn icon_variants_and_url_classification() {
        let fm = parse_frontmatter("---\nicon: logo.svg\nicon_dark: logo-dark.svg\n---\n").unwrap();
        assert_eq!(fm.icon.as_deref(), Some("logo.svg"));
        assert_eq!(fm.icon_dark.as_deref(), Some("logo-dark.svg"));
        assert!(!icon_is_url("logo.svg"));
        assert!(!icon_is_url("assets/icon.png"));
        assert!(icon_is_url("https://cdn.example.com/i.svg"));
        assert!(icon_is_url("//cdn.example.com/i.svg"));
        assert!(icon_is_url("data:image/svg+xml;base64,AAAA"));
    }

    #[test]
    fn folder_role_maps_synonyms_and_skips_specific() {
        assert_eq!(folder_role_from_frontmatter("desktop"), Some("desktop"));
        assert_eq!(folder_role_from_frontmatter("App"), Some("desktop"));
        assert_eq!(folder_role_from_frontmatter("website"), Some("website"));
        assert_eq!(folder_role_from_frontmatter("homebrew"), Some("packaging"));
        assert_eq!(folder_role_from_frontmatter("  Library "), Some("library"));
        assert_eq!(folder_role_from_frontmatter("mobile"), Some("mobile"));
        // project-specific / structural → no enum role (kept as a tag)
        assert_eq!(folder_role_from_frontmatter("monorepo"), None);
        assert_eq!(folder_role_from_frontmatter("marketplace"), None);
        assert_eq!(folder_role_from_frontmatter("whatever"), None);
    }

    #[test]
    fn slugify_normalizes() {
        assert_eq!(slugify("Sensei HQ"), "sensei-hq");
        assert_eq!(slugify("rust"), "rust");
        assert_eq!(slugify("  A / B  C "), "a-b-c");
        assert_eq!(slugify("C++"), "c");
    }

    #[test]
    fn partial_frontmatter_leaves_others_unset() {
        let fm = parse_frontmatter("---\nproject: solo\n---\n").unwrap();
        assert_eq!(fm.project.as_deref(), Some("solo"));
        assert!(fm.organization.is_none());
        assert!(fm.stack.is_empty());
    }
}
