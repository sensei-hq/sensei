//! `ComposerManifestAdapter` — `composer.json` (#89).
//!
//! Composer is PHP's package manager. `composer.json` is straight JSON with
//! `require` / `require-dev` maps of `"vendor/name": "^1.2.3"` entries.

use super::{ManifestAdapter, ParsedManifest};
use crate::indexer::lib_indexer::DepVersion;

pub struct ComposerManifestAdapter;

impl ManifestAdapter for ComposerManifestAdapter {
    fn manifest_filenames(&self) -> &[&'static str] {
        &["composer.json"]
    }

    fn ecosystem(&self) -> &'static str {
        "composer"
    }

    fn parse_dependencies(&self, content: &str) -> Vec<DepVersion> {
        let Ok(json) = serde_json::from_str::<serde_json::Value>(content) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (section, dev) in [("require", false), ("require-dev", true)] {
            let Some(obj) = json.get(section).and_then(|v| v.as_object()) else { continue };
            for (name, version) in obj {
                // Composer includes meta-requires like "php" or "ext-mbstring"
                // in `require` — those are runtime constraints, not
                // packagist libraries. Skip anything without a `vendor/`
                // slash so we only surface real libraries.
                if !name.contains('/') {
                    continue;
                }
                let ver = version.as_str().unwrap_or("*").to_string();
                out.push(DepVersion {
                    lib_name: name.clone(),
                    version: ver.clone(),
                    raw_version: ver,
                    source: "composer.json".into(),
                    dev,
                    local_source: None,
                });
            }
        }
        out
    }

    fn is_workspace_root(&self, _content: &str) -> bool {
        false
    }

    fn parse_manifest(&self, content: &str) -> ParsedManifest {
        let Ok(json) = serde_json::from_str::<serde_json::Value>(content) else {
            return ParsedManifest::default();
        };
        ParsedManifest {
            name: json.get("name").and_then(|v| v.as_str()).map(String::from),
            version: json.get("version").and_then(|v| v.as_str()).map(String::from),
            description: json.get("description").and_then(|v| v.as_str()).map(String::from),
        }
    }

    fn stack_labels(&self, _content: &str) -> Vec<&'static str> {
        vec!["php"]
    }

    /// Read `composer.json.scripts` — Composer's script runner is a first-
    /// class table like npm's. Each entry becomes a `composer <name>`
    /// invocation with the shared category classifier.
    fn parse_commands(&self, content: &str) -> Vec<super::DiscoveredCommand> {
        let Ok(json) = serde_json::from_str::<serde_json::Value>(content) else {
            return Vec::new();
        };
        let Some(scripts) = json.get("scripts").and_then(|v| v.as_object()) else {
            return Vec::new();
        };
        scripts
            .iter()
            .filter_map(|(name, _)| {
                if name.is_empty() {
                    return None;
                }
                Some(super::DiscoveredCommand {
                    raw_name: name.clone(),
                    command_line: format!("composer {name}"),
                    category: super::command_category::categorise(name),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecosystem_and_filenames() {
        let a = ComposerManifestAdapter;
        assert_eq!(a.ecosystem(), "composer");
        assert_eq!(a.manifest_filenames(), &["composer.json"]);
    }

    #[test]
    fn parse_manifest_extracts_name_version_description() {
        let src = r#"{
            "name": "acme/widget",
            "version": "2.1.0",
            "description": "A widget."
        }"#;
        let p = ComposerManifestAdapter.parse_manifest(src);
        assert_eq!(p.name.as_deref(), Some("acme/widget"));
        assert_eq!(p.version.as_deref(), Some("2.1.0"));
        assert_eq!(p.description.as_deref(), Some("A widget."));
    }

    #[test]
    fn parse_dependencies_reads_require_and_require_dev() {
        let src = r#"{
            "require": {
                "php": "^8.1",
                "ext-mbstring": "*",
                "symfony/console": "^7.0",
                "monolog/monolog": "^3.5"
            },
            "require-dev": {
                "phpunit/phpunit": "^10.0"
            }
        }"#;
        let deps = ComposerManifestAdapter.parse_dependencies(src);
        // php + ext-mbstring don't have vendor/ — skipped.
        assert_eq!(deps.len(), 3);
        let by = |n: &str| deps.iter().find(|d| d.lib_name == n).unwrap();
        assert_eq!(by("symfony/console").version, "^7.0");
        assert!(!by("symfony/console").dev);
        assert!(by("phpunit/phpunit").dev);
    }

    #[test]
    fn parse_dependencies_returns_empty_on_invalid_json() {
        assert!(ComposerManifestAdapter.parse_dependencies("{ broken").is_empty());
    }
}
