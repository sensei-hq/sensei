//! Project summary extraction from README and config files.

use serde::Serialize;
use std::path::Path;

/// Extracted project summary from README or config files.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ProjectSummary {
    /// One-line description (from package.json description, Cargo.toml description, or README first paragraph).
    pub description: Option<String>,
    /// Inferred project status: "active", "archived", "unmaintained".
    pub status: Option<String>,
}

/// Extract a project summary from common files.
///
/// Description precedence: registered manifests (package.json → Cargo.toml →
/// pyproject.toml → go.mod) via each `ManifestAdapter.parse_manifest`, then a
/// README first-paragraph fallback. Adding a new ecosystem here is one
/// adapter registration, not a new branch.
pub fn extract_summary(repo_path: &Path) -> ProjectSummary {
    let mut summary = ProjectSummary::default();

    for adapter in crate::adapters::manifest::registered_adapters() {
        if summary.description.is_some() {
            break;
        }
        for filename in adapter.manifest_filenames() {
            let Ok(content) = std::fs::read_to_string(repo_path.join(filename)) else { continue };
            let parsed = adapter.parse_manifest(&content);
            if let Some(desc) = parsed.description.filter(|s| !s.is_empty()) {
                summary.description = Some(desc);
                break;
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
    if repo_path.join("DEPRECATED.md").is_file() || repo_path.join(".archived").is_file() {
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
        if in_frontmatter {
            continue;
        }

        // Skip headings, badges, empty lines
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.contains("![") {
            continue;
        } // badge
        if trimmed.starts_with("![") {
            continue;
        } // image
        if trimmed.starts_with('<') {
            continue;
        } // HTML

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
        let content =
            "---\ntitle: Test\n---\n# My Project\n\nThis is the description.\n\nMore text.";
        assert_eq!(extract_first_paragraph(content).as_deref(), Some("This is the description."));
    }

    #[test]
    fn extract_first_paragraph_skips_badges() {
        let content = "# Title\n\n[![badge](https://img.shields.io/foo)]\n\nReal description here.";
        assert_eq!(extract_first_paragraph(content).as_deref(), Some("Real description here."));
    }

    #[test]
    fn extract_summary_from_package_json() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"name": "acme", "description": "The Acme platform"}"#,
        )
        .unwrap();

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

    #[test]
    fn extract_summary_from_cargo_toml_when_no_package_json() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"c\"\ndescription = \"A Rust thing\"",
        )
        .unwrap();
        let summary = extract_summary(dir.path());
        assert_eq!(summary.description.as_deref(), Some("A Rust thing"));
    }

    #[test]
    fn extract_summary_from_pyproject_when_no_other_manifest() {
        // pyproject.toml → description now flows through the adapter registry
        // (previously not covered).
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"y\"\ndescription = \"A python thing\"\n",
        )
        .unwrap();
        let summary = extract_summary(dir.path());
        assert_eq!(summary.description.as_deref(), Some("A python thing"));
    }
}
