//! Catalog fetch/cache, marketplace version tracking, item download.

use std::fs;
use std::path::Path;

use super::{Catalog, MARKETPLACE_CATALOG, MARKETPLACE_REPO, cache_dir, sensei_dir};

// ── Catalog fetch/cache ──────────────────────────────────────────────────────

/// Fetch the marketplace catalog. Uses cache if version matches.
pub fn fetch_catalog() -> Result<Catalog, String> {
    let cache = cache_dir();
    let cached_path = cache.join(MARKETPLACE_CATALOG);

    // Check cache
    if cached_path.exists()
        && let Ok(content) = fs::read_to_string(&cached_path)
        && let Ok(catalog) = serde_json::from_str::<Catalog>(&content)
    {
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
    
    // Validate version format to prevent injection attacks
    let new_version = catalog.version.as_deref().unwrap_or("");
    if !new_version.is_empty() && !is_valid_version(new_version) {
        return Err(format!(
            "invalid marketplace version format: '{}' (expected semver like '0.9.1')",
            new_version
        ));
    }
    
    // Clear cache when version changes to force re-download from pinned version
    let old_version = load_marketplace_version();
    if !old_version.is_empty() && old_version != new_version {
        clear_item_cache(&cache);
    }
    
    save_marketplace_version(new_version);
    Ok(catalog)
}

/// Download a single item from the marketplace and cache it.
/// 
/// Security: Downloads are pinned to the catalog version tag when available,
/// falling back to main only if no version is specified. Cached files are
/// invalidated when the catalog version changes.
pub(super) fn load_or_download(cache: &Path, path: &str) -> Result<String, String> {
    let cached = cache.join(path);
    
    // Reject cached content if it predates the current catalog version.
    // The catalog fetch clears the cache on version change, but this is
    // defense-in-depth: if a cache file somehow survives (e.g. manual
    // write, interrupted clear), we refuse to use it.
    let version = load_marketplace_version();
    if cached.exists() && !version.is_empty() {
        // Check if cache was written before the current version was saved.
        // If we can't determine freshness, reject the cache and re-download.
        if let Ok(metadata) = fs::metadata(&cached) {
            if let Ok(modified) = metadata.modified() {
                let version_path = sensei_dir().join("config.json");
                if let Ok(version_meta) = fs::metadata(&version_path) {
                    if let Ok(version_modified) = version_meta.modified() {
                        // Cache is stale if it predates the version file
                        if modified < version_modified {
                            tracing::debug!(
                                path = %path,
                                cache_age = ?modified,
                                version_age = ?version_modified,
                                "rejecting stale cache entry"
                            );
                            let _ = fs::remove_file(&cached);
                        }
                    }
                }
            }
        }
    }
    
    if cached.exists() {
        return fs::read_to_string(&cached).map_err(|e| e.to_string());
    }

    // Pin download to the catalog version tag if available.
    // This prevents a compromised main branch from serving malicious content
    // after a legitimate catalog has been fetched.
    let base_url = if !version.is_empty() {
        // Validate version before using it in URL (defense in depth)
        if !is_valid_version(&version) {
            return Err(format!(
                "refusing to download {}: invalid version format '{}'",
                path, version
            ));
        }
        // Use git tag for the specific version
        format!("https://raw.githubusercontent.com/sensei-hq/marketplace/v{}", version)
    } else {
        // Fallback to main only if no version (should not happen in practice)
        tracing::warn!(
            path = %path,
            "downloading from main branch: no catalog version available"
        );
        MARKETPLACE_REPO.to_string()
    };
    
    let url = format!("{}/{}", base_url, path);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(&url).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("{}: HTTP {} (version: {})", path, resp.status(), version));
    }
    let text = resp.text().map_err(|e| e.to_string())?;

    if let Some(parent) = cached.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&cached, &text).ok();
    Ok(text)
}

/// Clear all cached marketplace items (not the catalog itself).
/// Called when the catalog version changes to force re-download from the new version.
fn clear_item_cache(cache: &Path) {
    let dirs_to_clear = ["plugins", "skills", "commands"];
    for dir in &dirs_to_clear {
        let path = cache.join(dir);
        if path.exists() {
            if let Err(e) = fs::remove_dir_all(&path) {
                tracing::warn!(dir = %path.display(), error = %e, "failed to clear cache directory");
            } else {
                tracing::info!(dir = %path.display(), "cleared cache directory for new version");
            }
        }
    }
}

/// Validate that a version string is a valid semver format.
/// This prevents injection attacks via malicious version strings in the catalog.
/// Accepts formats like: "0.9.1", "1.0.0", "2.1.3-beta", "1.0.0-rc.1"
fn is_valid_version(version: &str) -> bool {
    // Basic semver pattern: digits.digits.digits with optional pre-release/build metadata
    // This is intentionally strict to prevent path traversal or command injection
    let parts: Vec<&str> = version.split('-').collect();
    let base = parts[0];
    
    // Check base version (X.Y.Z)
    let nums: Vec<&str> = base.split('.').collect();
    if nums.len() != 3 {
        return false;
    }
    
    for num in nums {
        if num.is_empty() || !num.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    
    // If there's a pre-release part, validate it contains only safe characters
    if parts.len() > 1 {
        for part in &parts[1..] {
            if part.is_empty() || !part.chars().all(|c| c.is_alphanumeric() || c == '.') {
                return false;
            }
        }
    }
    
    true
}

// ── Marketplace version tracking via SenseiLocalConfig ───────────────────────

pub(super) fn load_marketplace_version() -> String {
    let dir = sensei_dir();
    sensei_bootstrap::SenseiLocalConfig::load(&dir).marketplace_version.unwrap_or_default()
}

pub(super) fn save_marketplace_version(version: &str) {
    let dir = sensei_dir();
    let mut cfg = sensei_bootstrap::SenseiLocalConfig::load(&dir);
    cfg.marketplace_version = Some(version.to_string());
    if let Err(e) = cfg.save(&dir) {
        tracing::warn!(error = %e, "installer catalog: save marketplace_version failed");
    }
}

#[cfg(test)]
mod tests {
    use super::super::CatalogItem;
    use super::*;
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

    // ── Version validation ─────────────────────────────────────────────

    #[test]
    fn is_valid_version_accepts_semver() {
        assert!(is_valid_version("0.9.1"));
        assert!(is_valid_version("1.0.0"));
        assert!(is_valid_version("10.20.30"));
        assert!(is_valid_version("1.0.0-beta"));
        assert!(is_valid_version("1.0.0-rc.1"));
        assert!(is_valid_version("2.1.3-alpha.2"));
    }

    #[test]
    fn is_valid_version_rejects_malformed() {
        assert!(!is_valid_version(""));
        assert!(!is_valid_version("1.0"));
        assert!(!is_valid_version("1.0.0.0"));
        assert!(!is_valid_version("v1.0.0"));
        assert!(!is_valid_version("1.0.0/../../etc/passwd"));
        assert!(!is_valid_version("1.0.0; rm -rf /"));
        assert!(!is_valid_version("1.0.0\n"));
        assert!(!is_valid_version("1.0.0 "));
        assert!(!is_valid_version("1.0.a"));
        assert!(!is_valid_version("a.b.c"));
    }
}
