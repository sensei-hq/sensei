//! `ConfigAdapter` trait + JSON / TOML / YAML impls.
//!
//! Sibling to `ManifestAdapter` — where `ManifestAdapter` knows the schema of
//! a specific ecosystem (npm, cargo, pyproject, go), `ConfigAdapter` knows a
//! file *format*. The router uses it to decide whether a `.json` / `.toml` /
//! `.yaml` / `.yml` file is a config file at all, and the summary /
//! external-links passes use it to extract format-generic metadata (links,
//! description, version) from any config file.
//!
//! Manifest-specific extraction (Cargo's `[package]`, npm's `"exports"`, etc.)
//! still lives on `ManifestAdapter` — that's the ecosystem shape, not the
//! file format. `ConfigAdapter` handles the fields any well-known JSON/TOML
//! config might carry: `homepage`, `repository`, `bugs`, `documentation`,
//! `description`, `version`.

use crate::tasks::processors::metadata::external_links::{ExternalLink, classify_url};

mod json;
mod toml_adapter;
mod yaml;

/// Adapter for a config file *format* (JSON / TOML / YAML).
///
/// `extract_description` and `extract_version` are part of the planned trait
/// surface — the `summary.rs` pass currently uses `ManifestAdapter.parse_manifest`
/// for the shapes it knows, and will fall back to these format-generic
/// extractors as more file types get scanned. Each impl is unit-tested so the
/// methods aren't truly dead; they're just waiting on their non-test caller.
#[allow(dead_code)]
pub trait ConfigAdapter: Send + Sync {
    /// Extensions this adapter handles (lowercase, no dot).
    fn extensions(&self) -> &[&'static str];

    /// Extract external links from well-known metadata fields: `homepage`,
    /// `repository` (plain string or `{"url": …}` object), `bugs`,
    /// `documentation`. Returns links tagged with the source field as
    /// `label`. `found_in` is filled by the caller.
    fn extract_metadata_links(&self, content: &str) -> Vec<MetadataLink>;

    /// Extract a `description` field if present at the top level (JSON) or in
    /// the recognised project section (TOML `[package]` / `[project]`).
    fn extract_description(&self, content: &str) -> Option<String>;

    /// Extract a `version` field if present at the top level (JSON) or in the
    /// recognised project section (TOML `[package]` / `[project]`).
    fn extract_version(&self, content: &str) -> Option<String>;
}

/// A metadata link extracted by a `ConfigAdapter`. The `found_in` field is
/// filled in by the caller since the adapter only sees the file content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataLink {
    pub url: String,
    /// The source field that produced this link (e.g. `homepage`).
    pub field: &'static str,
}

impl MetadataLink {
    /// Promote this link to a full `ExternalLink` with the caller's context.
    pub fn into_external(self, found_in: &str) -> ExternalLink {
        ExternalLink {
            kind: classify_url(&self.url),
            url: self.url,
            label: Some(self.field.to_string()),
            found_in: found_in.to_string(),
        }
    }
}

/// Pick the config adapter for a file extension.
pub fn config_adapter_for_ext(ext: &str) -> Option<&'static dyn ConfigAdapter> {
    for a in registered_adapters() {
        if a.extensions().contains(&ext) {
            return Some(*a);
        }
    }
    None
}

/// All registered ConfigAdapter impls. Add new formats here.
pub fn registered_adapters() -> &'static [&'static dyn ConfigAdapter] {
    &[&json::JsonConfigAdapter, &toml_adapter::TomlConfigAdapter, &yaml::YamlConfigAdapter]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_json_for_json_extension() {
        let a = config_adapter_for_ext("json").unwrap();
        assert!(a.extensions().contains(&"json"));
    }

    #[test]
    fn dispatch_json_for_jsonl_extension() {
        let a = config_adapter_for_ext("jsonl").unwrap();
        assert!(a.extensions().contains(&"jsonl"));
    }

    #[test]
    fn dispatch_toml_for_toml_extension() {
        let a = config_adapter_for_ext("toml").unwrap();
        assert!(a.extensions().contains(&"toml"));
    }

    #[test]
    fn dispatch_yaml_for_yaml_and_yml() {
        assert!(config_adapter_for_ext("yaml").is_some());
        assert!(config_adapter_for_ext("yml").is_some());
    }

    #[test]
    fn dispatch_none_for_unknown_extension() {
        assert!(config_adapter_for_ext("rs").is_none());
        assert!(config_adapter_for_ext("").is_none());
    }

    #[test]
    fn metadata_link_promotion_populates_found_in_and_kind() {
        let link = MetadataLink {
            url: "https://myorg.atlassian.net/browse/PROJ".to_string(),
            field: "homepage",
        };
        let ext = link.into_external("package.json");
        assert_eq!(ext.kind, "jira");
        assert_eq!(ext.label.as_deref(), Some("homepage"));
        assert_eq!(ext.found_in, "package.json");
    }
}
