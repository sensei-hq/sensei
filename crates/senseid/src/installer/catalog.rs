//! Catalog fetch/cache, marketplace version tracking, item download.

use std::fs;
use std::path::Path;

use super::{cache_dir, sensei_dir, Catalog, MARKETPLACE_CATALOG, MARKETPLACE_REPO};

// ── Catalog fetch/cache ──────────────────────────────────────────────────────

/// Fetch the marketplace catalog. Uses cache if version matches.
pub fn fetch_catalog() -> Result<Catalog, String> {
    let cache = cache_dir();
    let cached_path = cache.join(MARKETPLACE_CATALOG);

    // Check cache
    if cached_path.exists()
        && let Ok(content) = fs::read_to_string(&cached_path)
            && let Ok(catalog) = serde_json::from_str::<Catalog>(&content) {
                let cached_ver = catalog.version.as_deref().unwrap_or("");
                let saved_ver = load_marketplace_version();
                if !cached_ver.is_empty() && cached_ver == saved_ver {
                    return Ok(catalog);
                }
            }

    // Download fresh
    let url = format!("{}/{}", MARKETPLACE_REPO, MARKETPLACE_CATALOG);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(&url).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = resp.text().map_err(|e| e.to_string())?;

    // Cache
    fs::create_dir_all(&cache).ok();
    fs::write(&cached_path, &text).ok();

    let catalog: Catalog = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    save_marketplace_version(catalog.version.as_deref().unwrap_or(""));
    Ok(catalog)
}

/// Download a single item from the marketplace and cache it.
pub(super) fn load_or_download(cache: &Path, path: &str) -> Result<String, String> {
    let cached = cache.join(path);
    if cached.exists() {
        return fs::read_to_string(&cached).map_err(|e| e.to_string());
    }

    let url = format!("{}/{}", MARKETPLACE_REPO, path);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(&url).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("{}: HTTP {}", path, resp.status()));
    }
    let text = resp.text().map_err(|e| e.to_string())?;

    if let Some(parent) = cached.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&cached, &text).ok();
    Ok(text)
}

// ── Marketplace version tracking via SenseiLocalConfig ───────────────────────

pub(super) fn load_marketplace_version() -> String {
    let dir = sensei_dir();
    sensei_bootstrap::SenseiLocalConfig::load(&dir)
        .marketplace_version
        .unwrap_or_default()
}

pub(super) fn save_marketplace_version(version: &str) {
    let dir = sensei_dir();
    let mut cfg = sensei_bootstrap::SenseiLocalConfig::load(&dir);
    cfg.marketplace_version = Some(version.to_string());
    cfg.save(&dir).ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::CatalogItem;
    use std::fs;

    // ── Catalog deserialization ────────────────────────────────────────

    #[test]
    fn catalog_deserializes_minimal() {
        let json = r#"{"items": []}"#;
        let cat: Catalog = serde_json::from_str(json).unwrap();
        assert!(cat.version.is_none());
        assert!(cat.items.is_empty());
    }

    #[test]
    fn catalog_deserializes_full() {
        let json = r#"{
            "version": "2.0.0",
            "items": [{
                "name": "review",
                "kind": "skill",
                "description": "Code review",
                "scope": "global",
                "path": "skills/review.md",
                "recommended_for": ["claude-code"],
                "stage": ["review"]
            }]
        }"#;
        let cat: Catalog = serde_json::from_str(json).unwrap();
        assert_eq!(cat.version.as_deref(), Some("2.0.0"));
        assert_eq!(cat.items.len(), 1);
        assert_eq!(cat.items[0].name, "review");
        assert_eq!(cat.items[0].kind, "skill");
        assert_eq!(cat.items[0].description, "Code review");
        assert_eq!(cat.items[0].scope, "global");
        assert_eq!(cat.items[0].path, "skills/review.md");
        assert_eq!(cat.items[0].recommended_for, vec!["claude-code"]);
        assert_eq!(cat.items[0].stage, vec!["review"]);
    }

    #[test]
    fn catalog_item_defaults_for_missing_fields() {
        let json = r#"{"items": [{"name": "test", "kind": "command"}]}"#;
        let cat: Catalog = serde_json::from_str(json).unwrap();
        let item = &cat.items[0];
        assert_eq!(item.description, "");
        assert_eq!(item.scope, "");
        assert_eq!(item.path, "");
        assert!(item.recommended_for.is_empty());
        assert!(item.stage.is_empty());
    }

    #[test]
    fn catalog_item_serializes_round_trip() {
        let item = CatalogItem {
            name: "review".into(),
            kind: "skill".into(),
            description: "Code review".into(),
            scope: "global".into(),
            path: "skills/review.md".into(),
            recommended_for: vec!["claude-code".into()],
            stage: vec!["review".into()],
        };
        let json = serde_json::to_string(&item).unwrap();
        let back: CatalogItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "review");
        assert_eq!(back.kind, "skill");
    }

    // ── load_or_download (cache-hit path) ─────────────────────────────

    #[test]
    fn load_or_download_returns_cached_content() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path();

        // Pre-populate cache
        let item_dir = cache.join("skills");
        fs::create_dir_all(&item_dir).unwrap();
        fs::write(item_dir.join("review.md"), "# Cached review skill").unwrap();

        let content = load_or_download(cache, "skills/review.md").unwrap();
        assert_eq!(content, "# Cached review skill");
    }
}
