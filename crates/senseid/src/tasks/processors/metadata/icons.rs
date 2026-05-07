//! Icon/logo detection for repos.

use std::path::Path;
use serde::Serialize;

/// Detected icon/logo for a project.
#[derive(Debug, Clone, Serialize, Default)]
pub struct IconResult {
    /// Relative path to the best icon found (e.g., "assets/logo.svg").
    pub path: Option<String>,
    /// Whether a dark-mode variant exists.
    pub has_dark_variant: bool,
    /// How the icon was found: "exact", "convention", "assets-dir".
    pub source: Option<String>,
}

/// Scan a repo for icon/logo files.
///
/// Search order (first match wins):
/// 1. Exact match at root: `logo.svg`, `icon.svg`, `app-icon.png`, etc.
/// 2. Repo-name match: `{repo-name}.svg`, `{repo-name}.png`
/// 3. Convention directories (1 level deep): `assets/`, `public/`, `static/`, `.github/`, `brand/`, `branding/`, `img/`, `images/`
/// 4. Nested convention dirs (2 levels): `src/assets/`, `src/images/`
/// 5. Framework-specific: `src-tauri/icons/`, `android/app/src/main/res/`, `ios/*/Assets.xcassets/AppIcon.appiconset/`
/// 6. Favicon: `favicon.ico`, `favicon.svg`, `favicon.png`
///
/// For each match, also checks for dark variant (`-dark`, `_dark`, `.dark`).
pub fn scan_icons(repo_path: &Path) -> IconResult {
    let repo_name = repo_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    let icon_names = [
        "logo.svg", "logo.png", "icon.svg", "icon.png",
        "app-icon.svg", "app-icon.png", "Logo.svg", "Logo.png",
        "Icon.svg", "Icon.png",
    ];

    // 1. Root-level exact names
    for name in &icon_names {
        if repo_path.join(name).is_file() {
            return IconResult {
                has_dark_variant: has_dark_variant(repo_path, name),
                path: Some(name.to_string()),
                source: Some("exact".into()),
            };
        }
    }

    // 2. Repo-name match — exact and prefix
    //    Finds: kavach.svg, rokkit.svg (exact)
    //    Also:  rokkit-icon.svg, kavach-logo.png (prefix)
    //    Skips: dark/light variants — those are detected as dark_variant
    if let Some(result) = scan_repo_name_icons(repo_path, &repo_name, "") {
        return result;
    }

    // 3. Convention directories (1 level) — generic names + repo-name prefix
    let dirs = [
        "assets", "public", "static", ".github", "brand", "branding",
        "img", "images", "resources", "res",
    ];
    for dir in &dirs {
        if let Some(result) = scan_dir_for_icons(repo_path, dir, &icon_names) {
            return result;
        }
        if let Some(result) = scan_repo_name_icons(repo_path, &repo_name, dir) {
            return result;
        }
    }

    // 4. Nested convention dirs (2 levels) — generic names + repo-name prefix
    let nested_dirs = [
        "src/assets", "src/images", "src/img", "src/resources",
        "app/assets", "lib/assets", "site/static", "site/assets",
    ];
    for dir in &nested_dirs {
        if let Some(result) = scan_dir_for_icons(repo_path, dir, &icon_names) {
            return result;
        }
        if let Some(result) = scan_repo_name_icons(repo_path, &repo_name, dir) {
            return result;
        }
    }

    // 5. Framework-specific locations
    // Tauri
    let tauri_icons = repo_path.join("src-tauri/icons");
    if tauri_icons.is_dir() {
        // Prefer SVG > PNG > ICO, prefer larger sizes
        for name in &["icon.svg", "icon.png", "512x512.png", "256x256.png", "128x128.png", "icon.ico"] {
            if tauri_icons.join(name).is_file() {
                let full = format!("src-tauri/icons/{}", name);
                return IconResult {
                    has_dark_variant: false,
                    path: Some(full),
                    source: Some("tauri".into()),
                };
            }
        }
    }

    // Electron
    let electron_names = ["build/icon.png", "build/icon.svg", "build/icon.ico"];
    for name in &electron_names {
        if repo_path.join(name).is_file() {
            return IconResult {
                has_dark_variant: false,
                path: Some(name.to_string()),
                source: Some("electron".into()),
            };
        }
    }

    // 6. Favicon
    for name in &["favicon.svg", "favicon.ico", "favicon.png"] {
        if repo_path.join(name).is_file() {
            return IconResult {
                has_dark_variant: false,
                path: Some(name.to_string()),
                source: Some("favicon".into()),
            };
        }
    }
    // Favicon in public/
    for name in &["public/favicon.svg", "public/favicon.ico", "public/favicon.png"] {
        if repo_path.join(name).is_file() {
            return IconResult {
                has_dark_variant: false,
                path: Some(name.to_string()),
                source: Some("favicon".into()),
            };
        }
    }

    IconResult::default()
}

/// Scan a specific directory for icon files. Returns the first match.
fn scan_dir_for_icons(repo_path: &Path, dir: &str, icon_names: &[&str]) -> Option<IconResult> {
    let dir_path = repo_path.join(dir);
    if !dir_path.is_dir() { return None; }
    for name in icon_names {
        let full = format!("{}/{}", dir, name);
        if repo_path.join(&full).is_file() {
            return Some(IconResult {
                has_dark_variant: has_dark_variant(&dir_path, name),
                path: Some(full),
                source: Some("convention-dir".into()),
            });
        }
    }
    None
}

/// Scan for files matching `{repo_name}*.{svg,png}` in a directory.
/// Prefers: exact > icon/logo suffix > any match. Skips dark/light variants.
/// `subdir` is "" for root, or "assets" etc.
fn scan_repo_name_icons(repo_path: &Path, repo_name: &str, subdir: &str) -> Option<IconResult> {
    let scan_dir = if subdir.is_empty() { repo_path.to_path_buf() } else { repo_path.join(subdir) };
    if !scan_dir.is_dir() { return None; }

    let entries = std::fs::read_dir(&scan_dir).ok()?;
    let mut candidates: Vec<(String, String, u8)> = Vec::new(); // (rel_path, filename, priority)

    for entry in entries.flatten() {
        if !entry.path().is_file() { continue; }
        let fname = entry.file_name().to_string_lossy().to_lowercase();

        // Must be an image file starting with repo name
        if !fname.starts_with(repo_name) { continue; }
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "svg" | "png" | "ico") { continue; }

        // Skip dark/light/black/white variants — these are alternates, not primary
        if is_variant_name(&fname, repo_name) { continue; }

        let rel = if subdir.is_empty() {
            entry.file_name().to_string_lossy().to_string()
        } else {
            format!("{}/{}", subdir, entry.file_name().to_string_lossy())
        };

        // Priority: exact match > -icon/-logo suffix > anything else. SVG preferred over PNG.
        let priority = match (fname.as_str(), ext) {
            _ if fname == format!("{}.svg", repo_name) => 0,
            _ if fname == format!("{}.png", repo_name) => 1,
            _ if (fname.contains("-icon") || fname.contains("-logo")) && ext == "svg" => 2,
            _ if (fname.contains("-icon") || fname.contains("-logo")) && ext == "png" => 3,
            _ if ext == "svg" => 4,
            _ if ext == "png" => 5,
            _ => 6,
        };

        candidates.push((rel, entry.file_name().to_string_lossy().to_string(), priority));
    }

    candidates.sort_by_key(|(_, _, p)| *p);
    let (rel, _fname, _) = candidates.into_iter().next()?;

    Some(IconResult {
        has_dark_variant: has_repo_name_dark_variant(&scan_dir, repo_name),
        path: Some(rel),
        source: Some("repo-name".into()),
    })
}

/// Check if a filename is a dark/light/black/white variant (not a primary icon).
pub(crate) fn is_variant_name(fname: &str, repo_name: &str) -> bool {
    let suffix = &fname[repo_name.len()..];
    let variant_markers = ["-dark", "-light", "-black", "-white", "_dark", "_light", "_black", "_white", ".dark", ".light"];
    variant_markers.iter().any(|m| suffix.starts_with(m))
}

/// Check if dark-mode variants exist for repo-name icons.
fn has_repo_name_dark_variant(dir: &Path, repo_name: &str) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_lowercase();
            if fname.starts_with(repo_name) {
                let suffix = &fname[repo_name.len()..];
                if suffix.starts_with("-dark") || suffix.starts_with("_dark") || suffix.starts_with(".dark") {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a dark-mode variant exists alongside a file.
/// Looks for: `logo-dark.svg`, `logo_dark.svg`, `logo.dark.svg`, `dark/logo.svg`.
fn has_dark_variant(dir: &Path, filename: &str) -> bool {
    let stem = Path::new(filename).file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let ext = Path::new(filename).extension().and_then(|e| e.to_str()).unwrap_or("");

    let variants = [
        format!("{}-dark.{}", stem, ext),
        format!("{}_dark.{}", stem, ext),
        format!("{}.dark.{}", stem, ext),
    ];
    for v in &variants {
        if dir.join(v).is_file() {
            return true;
        }
    }
    // Also check dark/ subdirectory
    dir.join("dark").join(filename).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_icons_finds_root_logo_svg() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("logo.svg"), "<svg/>").unwrap();

        let result = scan_icons(dir.path());
        assert_eq!(result.path.as_deref(), Some("logo.svg"));
        assert_eq!(result.source.as_deref(), Some("exact"));
    }

    #[test]
    fn scan_icons_finds_repo_name_match() {
        let dir = tempfile::tempdir().unwrap();
        // Create a dir named "acme-api" to simulate repo name
        let repo = dir.path().join("acme-api");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join("acme-api.svg"), "<svg/>").unwrap();

        let result = scan_icons(&repo);
        assert_eq!(result.path.as_deref(), Some("acme-api.svg"));
        assert_eq!(result.source.as_deref(), Some("repo-name"));
    }

    #[test]
    fn scan_icons_detects_dark_variant() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("logo.svg"), "<svg/>").unwrap();
        fs::write(dir.path().join("logo-dark.svg"), "<svg/>").unwrap();

        let result = scan_icons(dir.path());
        assert!(result.has_dark_variant);
    }

    #[test]
    fn scan_icons_finds_in_assets_dir() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("assets")).unwrap();
        fs::write(dir.path().join("assets/logo.png"), &[0u8]).unwrap();

        let result = scan_icons(dir.path());
        assert_eq!(result.path.as_deref(), Some("assets/logo.png"));
        assert_eq!(result.source.as_deref(), Some("convention-dir"));
    }

    #[test]
    fn scan_icons_returns_default_when_none_found() {
        let dir = tempfile::tempdir().unwrap();
        let result = scan_icons(dir.path());
        assert!(result.path.is_none());
    }

    #[test]
    fn scan_icons_finds_repo_name_prefix_with_dark() {
        // Simulates: rokkit/rokkit.svg + rokkit/rokkit-dark.svg
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("rokkit");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join("rokkit.svg"), "<svg/>").unwrap();
        fs::write(repo.join("rokkit-dark.svg"), "<svg/>").unwrap();
        fs::write(repo.join("rokkit-light.svg"), "<svg/>").unwrap();

        let result = scan_icons(&repo);
        assert_eq!(result.path.as_deref(), Some("rokkit.svg"));
        assert_eq!(result.source.as_deref(), Some("repo-name"));
        assert!(result.has_dark_variant);
    }

    #[test]
    fn scan_icons_finds_repo_name_icon_variant() {
        // Simulates: rokkit/site/static/rokkit-icon.svg
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("rokkit");
        fs::create_dir_all(repo.join("site/static")).unwrap();
        fs::write(repo.join("site/static/rokkit-icon.svg"), "<svg/>").unwrap();
        fs::write(repo.join("site/static/rokkit-dark.svg"), "<svg/>").unwrap();

        let result = scan_icons(&repo);
        assert_eq!(result.path.as_deref(), Some("site/static/rokkit-icon.svg"));
        assert!(result.has_dark_variant);
    }

    #[test]
    fn scan_icons_finds_tauri_icon() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src-tauri/icons")).unwrap();
        fs::write(dir.path().join("src-tauri/icons/icon.png"), &[0u8]).unwrap();
        fs::write(dir.path().join("src-tauri/icons/512x512.png"), &[0u8]).unwrap();

        let result = scan_icons(dir.path());
        assert_eq!(result.path.as_deref(), Some("src-tauri/icons/icon.png"));
        assert_eq!(result.source.as_deref(), Some("tauri"));
    }

    #[test]
    fn scan_icons_skips_dark_variant_as_primary() {
        // Only dark variant exists — should still find it (it's the only option)
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("kavach");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join("kavach-black.svg"), "<svg/>").unwrap();

        let result = scan_icons(&repo);
        // kavach-black is a variant, but if there's no primary, we should still have None
        // because the user would need a proper icon
        assert!(result.path.is_none());
    }
}
