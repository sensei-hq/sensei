//! `YamlConfigAdapter` — parses generic YAML config files.
//!
//! YAML config files in this codebase (`pnpm-workspace.yaml`, GitHub Actions
//! workflows, etc.) rarely carry the `homepage`/`repository`/`description`
//! metadata shape that JSON and TOML manifests do. We still implement the
//! trait so the router can route `.yaml` / `.yml` through `ConfigAdapter`
//! dispatch, but the metadata extractors are best-effort and return empty
//! unless the file explicitly carries those top-level keys.

use super::{ConfigAdapter, MetadataLink};

pub struct YamlConfigAdapter;

const LINK_FIELDS: &[&str] = &["homepage", "repository", "documentation"];

impl ConfigAdapter for YamlConfigAdapter {
    fn extensions(&self) -> &[&'static str] {
        &["yaml", "yml"]
    }

    fn extract_metadata_links(&self, content: &str) -> Vec<MetadataLink> {
        let Ok(val) = serde_yaml::from_str::<serde_json::Value>(content) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for &field in LINK_FIELDS {
            if let Some(url) = val.get(field).and_then(|v| v.as_str())
                && url.starts_with("http")
            {
                out.push(MetadataLink { url: url.to_string(), field: static_field(field) });
            }
        }
        out
    }

    fn extract_description(&self, content: &str) -> Option<String> {
        let val = serde_yaml::from_str::<serde_json::Value>(content).ok()?;
        val.get("description")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    fn extract_version(&self, content: &str) -> Option<String> {
        let val = serde_yaml::from_str::<serde_json::Value>(content).ok()?;
        val.get("version").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string())
    }
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
    fn extensions_yaml_and_yml() {
        assert_eq!(YamlConfigAdapter.extensions(), &["yaml", "yml"]);
    }

    #[test]
    fn extract_metadata_links_from_top_level_keys() {
        let src =
            "homepage: https://sensei-hq.com\nrepository: https://github.com/sensei-hq/sensei\n";
        let mut links = YamlConfigAdapter.extract_metadata_links(src);
        links.sort_by(|a, b| a.field.cmp(b.field));
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].field, "homepage");
        assert_eq!(links[1].field, "repository");
    }

    #[test]
    fn extract_metadata_links_empty_for_pnpm_workspace() {
        // pnpm-workspace.yaml only carries a `packages:` list — no metadata.
        let src = "packages:\n  - 'apps/*'\n  - 'packages/*'\n";
        assert!(YamlConfigAdapter.extract_metadata_links(src).is_empty());
    }

    #[test]
    fn extract_metadata_links_empty_for_invalid_yaml() {
        assert!(YamlConfigAdapter.extract_metadata_links(":\n:\n:").is_empty());
    }

    #[test]
    fn extract_description_finds_top_level_field() {
        assert_eq!(
            YamlConfigAdapter.extract_description("description: A YAML config\n"),
            Some("A YAML config".to_string())
        );
    }

    #[test]
    fn extract_version_finds_top_level_field() {
        assert_eq!(
            YamlConfigAdapter.extract_version("version: '1.2.3'\n"),
            Some("1.2.3".to_string())
        );
    }
}
