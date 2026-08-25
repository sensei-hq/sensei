//! `TomlConfigAdapter` — parses generic TOML config files.
//!
//! Recognises the two common project-section conventions: Cargo's
//! `[package]` and pyproject's `[project]`. Description / version /
//! `homepage` / `repository` / `documentation` live inside these sections.

use super::{ConfigAdapter, MetadataLink};

pub struct TomlConfigAdapter;

/// Well-known link fields the adapter looks for. Both `[package]` (Cargo) and
/// `[project]` (pyproject / PEP 621) use these keys.
const LINK_FIELDS: &[&str] = &["homepage", "repository", "documentation"];

impl ConfigAdapter for TomlConfigAdapter {
    fn extensions(&self) -> &[&'static str] {
        &["toml"]
    }

    fn extract_metadata_links(&self, content: &str) -> Vec<MetadataLink> {
        let Ok(val) = content.parse::<toml::Value>() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for section in project_sections(&val) {
            for &field in LINK_FIELDS {
                if let Some(url) = section.get(field).and_then(|v| v.as_str())
                    && url.starts_with("http")
                {
                    out.push(MetadataLink { url: url.to_string(), field: static_field(field) });
                }
                // pyproject stores multiple URLs under `[project.urls]` — pick
                // up `homepage` / `documentation` / `repository` from there
                // when the top-level key isn't set.
                if let Some(urls) = section.get("urls").and_then(|v| v.as_table())
                    && let Some(url) = urls.get(field).and_then(|v| v.as_str())
                    && url.starts_with("http")
                {
                    out.push(MetadataLink { url: url.to_string(), field: static_field(field) });
                }
            }
        }
        out
    }

    fn extract_description(&self, content: &str) -> Option<String> {
        let val = content.parse::<toml::Value>().ok()?;
        project_sections(&val).into_iter().find_map(|section| {
            section
                .get("description")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
        })
    }

    fn extract_version(&self, content: &str) -> Option<String> {
        let val = content.parse::<toml::Value>().ok()?;
        project_sections(&val).into_iter().find_map(|section| {
            section
                .get("version")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
        })
    }
}

/// Return references to the recognised project sections in preference order
/// (`[package]` first, then `[project]`). A file may declare both — Cargo's
/// `[package]` wins for a Cargo.toml, and pyproject's `[project]` wins for a
/// pyproject.toml.
fn project_sections(val: &toml::Value) -> Vec<&toml::value::Table> {
    let mut out = Vec::new();
    if let Some(pkg) = val.get("package").and_then(|v| v.as_table()) {
        out.push(pkg);
    }
    if let Some(proj) = val.get("project").and_then(|v| v.as_table()) {
        out.push(proj);
    }
    out
}

fn static_field(field: &str) -> &'static str {
    match field {
        "homepage" => "homepage",
        "repository" => "repository",
        "documentation" => "documentation",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_toml() {
        assert_eq!(TomlConfigAdapter.extensions(), &["toml"]);
    }

    #[test]
    fn extract_metadata_links_from_cargo_package_section() {
        let src = r#"
            [package]
            name = "sensei"
            homepage = "https://sensei-hq.com"
            repository = "https://github.com/sensei-hq/sensei"
            documentation = "https://docs.sensei-hq.com"
        "#;
        let mut links = TomlConfigAdapter.extract_metadata_links(src);
        links.sort_by(|a, b| a.field.cmp(b.field));
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].field, "documentation");
        assert_eq!(links[1].field, "homepage");
        assert_eq!(links[2].field, "repository");
    }

    #[test]
    fn extract_metadata_links_from_pyproject_urls_section() {
        // pyproject uses `[project.urls]` for multiple URLs
        let src = r#"
            [project]
            name = "py"

            [project.urls]
            homepage = "https://py.example"
            documentation = "https://docs.py.example"
        "#;
        let mut links = TomlConfigAdapter.extract_metadata_links(src);
        links.sort_by(|a, b| a.field.cmp(b.field));
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].field, "documentation");
        assert_eq!(links[1].field, "homepage");
    }

    #[test]
    fn extract_metadata_links_ignores_non_http_urls() {
        let src = r#"
            [package]
            homepage = "./readme.md"
            repository = "git@github.com:x/y"
        "#;
        assert!(TomlConfigAdapter.extract_metadata_links(src).is_empty());
    }

    #[test]
    fn extract_metadata_links_empty_for_invalid_toml() {
        assert!(TomlConfigAdapter.extract_metadata_links("not toml [").is_empty());
    }

    #[test]
    fn extract_description_from_cargo_package() {
        let src = "[package]\nname = \"x\"\ndescription = \"A crate\"";
        assert_eq!(TomlConfigAdapter.extract_description(src), Some("A crate".to_string()));
    }

    #[test]
    fn extract_description_from_pyproject_project() {
        let src = "[project]\nname = \"y\"\ndescription = \"A python pkg\"";
        assert_eq!(TomlConfigAdapter.extract_description(src), Some("A python pkg".to_string()));
    }

    #[test]
    fn extract_description_ignores_empty_string() {
        let src = "[package]\ndescription = \"\"";
        assert_eq!(TomlConfigAdapter.extract_description(src), None);
    }

    #[test]
    fn extract_version_from_cargo_package() {
        let src = "[package]\nversion = \"0.2.23\"";
        assert_eq!(TomlConfigAdapter.extract_version(src), Some("0.2.23".to_string()));
    }

    #[test]
    fn extract_version_none_when_absent() {
        assert_eq!(TomlConfigAdapter.extract_version("[package]\nname = \"x\""), None);
    }
}
