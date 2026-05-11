//! Project summary extraction from README and config files.

use std::path::Path;
use serde::Serialize;

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
    if let Ok(content) = std::fs::read_to_string(repo_path.join("package.json"))
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&content)
            && let Some(desc) = val["description"].as_str()
                && !desc.is_empty() {
                    summary.description = Some(desc.to_string());
                }

    // Try Cargo.toml description
    if summary.description.is_none()
        && let Ok(content) = std::fs::read_to_string(repo_path.join("Cargo.toml")) {
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
pub(crate) fn extract_first_paragraph(content: &str) -> Option<String> {
    let mut in_frontmatter = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip frontmatter
        if trimmed == "---" {
            in_frontmatter = !in_frontmatter;
            continue;
        }
        if in_frontmatter { continue; }

        // Skip headings, badges, empty lines
        if trimmed.is_empty() { continue; }
        if trimmed.starts_with('#') { continue; }
        if trimmed.starts_with('[') && trimmed.contains("![") { continue; } // badge
        if trimmed.starts_with("![") { continue; } // image
        if trimmed.starts_with('<') { continue; } // HTML

        let desc = trimmed.to_string();
        // Cap at 200 chars
        if desc.len() > 200 {
            return Some(format!("{}...", &desc[..197]));
        }
        return Some(desc);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
