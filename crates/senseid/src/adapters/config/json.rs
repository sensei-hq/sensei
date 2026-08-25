//! `JsonConfigAdapter` — parses generic JSON config files.

use super::{ConfigAdapter, MetadataLink};

pub struct JsonConfigAdapter;

/// Well-known link fields the adapter looks for. Ordered so `homepage` appears
/// before `repository` / `bugs` in the emitted link list for stable output.
const LINK_FIELDS: &[&str] = &["homepage", "repository", "bugs", "documentation"];

impl ConfigAdapter for JsonConfigAdapter {
    fn extensions(&self) -> &[&'static str] {
        &["json", "jsonl"]
    }

    fn extract_metadata_links(&self, content: &str) -> Vec<MetadataLink> {
        let Ok(val) = serde_json::from_str::<serde_json::Value>(content) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for &field in LINK_FIELDS {
            // Plain-string form: `"homepage": "https://..."`
            if let Some(url) = val.get(field).and_then(|v| v.as_str())
                && url.starts_with("http")
            {
                out.push(MetadataLink { url: url.to_string(), field: static_field(field) });
            }
            // Object form (npm `repository`, `bugs`): `{"type":"git","url":"..."}`
            if let Some(url) = val.get(field).and_then(|v| v.get("url")).and_then(|v| v.as_str())
                && url.starts_with("http")
            {
                out.push(MetadataLink { url: url.to_string(), field: static_field(field) });
            }
        }
        out
    }

    fn extract_description(&self, content: &str) -> Option<String> {
        let val = serde_json::from_str::<serde_json::Value>(content).ok()?;
        val.get("description")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    fn extract_version(&self, content: &str) -> Option<String> {
        let val = serde_json::from_str::<serde_json::Value>(content).ok()?;
        val.get("version").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string())
    }
}

/// Map one of the four supported LINK_FIELDS strings back to a `&'static str`
/// so `MetadataLink.field` can stay static-lifetime.
fn static_field(field: &str) -> &'static str {
    match field {
        "homepage" => "homepage",
        "repository" => "repository",
        "bugs" => "bugs",
        "documentation" => "documentation",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_json_and_jsonl() {
        assert_eq!(JsonConfigAdapter.extensions(), &["json", "jsonl"]);
    }

    #[test]
    fn extract_metadata_links_plain_and_object_forms() {
        let content = r#"{
            "homepage": "https://docs.acme.com",
            "bugs": { "url": "https://github.com/acme/api/issues" },
            "repository": "https://github.com/acme/api"
        }"#;
        let mut links = JsonConfigAdapter.extract_metadata_links(content);
        links.sort_by(|a, b| a.field.cmp(b.field).then(a.url.cmp(&b.url)));
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].field, "bugs");
        assert_eq!(links[1].field, "homepage");
        assert_eq!(links[2].field, "repository");
    }

    #[test]
    fn extract_metadata_links_ignores_non_http_urls() {
        // A `git://` or relative URL shouldn't count as an external link.
        let content = r#"{"repository": "git://github.com/x/y", "homepage": "./readme"}"#;
        assert!(JsonConfigAdapter.extract_metadata_links(content).is_empty());
    }

    #[test]
    fn extract_metadata_links_returns_empty_for_invalid_json() {
        assert!(JsonConfigAdapter.extract_metadata_links("{ not json").is_empty());
    }

    #[test]
    fn extract_description_finds_top_level_field() {
        assert_eq!(
            JsonConfigAdapter.extract_description(r#"{"description":"A test package"}"#),
            Some("A test package".to_string())
        );
    }

    #[test]
    fn extract_description_ignores_empty_string() {
        assert_eq!(JsonConfigAdapter.extract_description(r#"{"description":""}"#), None);
    }

    #[test]
    fn extract_version_finds_top_level_field() {
        assert_eq!(
            JsonConfigAdapter.extract_version(r#"{"version":"1.2.3"}"#),
            Some("1.2.3".to_string())
        );
    }

    #[test]
    fn extract_version_none_for_missing_field() {
        assert_eq!(JsonConfigAdapter.extract_version(r#"{"name":"x"}"#), None);
    }
}
