//! Installer — marketplace fetch/cache, hook/skill/command installation.
//!
//! Both CLI and desktop delegate here via HTTP endpoints.
//! Binary copying is the only step that stays in the CLI (needs to happen
//! before the daemon is available).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const MARKETPLACE_REPO: &str =
    "https://raw.githubusercontent.com/mizukisu/sensei-marketplace/main";
const MARKETPLACE_CATALOG: &str = "catalog.json";

// ── Paths ────────────────────────────────────────────────────────────────────

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
}
fn sensei_dir() -> PathBuf {
    home().join(".sensei")
}
fn plugin_dir() -> PathBuf {
    home().join(".claude/plugins/sensei")
}
fn cache_dir() -> PathBuf {
    sensei_dir().join("cache/marketplace")
}

// ── Catalog types ────────────────────────────────────────────────────────────

#[derive(Deserialize, Clone)]
pub struct Catalog {
    pub version: Option<String>,
    pub items: Vec<CatalogItem>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct CatalogItem {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub recommended_for: Vec<String>,
    #[serde(default)]
    pub stage: Vec<String>,
}

// ── Install result ───────────────────────────────────────────────────────────

#[derive(Serialize, Default)]
pub struct InstallResult {
    pub hooks_installed: u32,
    pub skills_installed: u32,
    pub commands_installed: u32,
    pub acps_configured: Vec<String>,
    pub errors: Vec<String>,
    pub marketplace_version: String,
}

// ── Full install ─────────────────────────────────────────────────────────────

/// Run the full install: hooks, marketplace (skills/commands), ACP config.
/// Binary copying is NOT included — CLI handles that before daemon starts.
pub fn install(acps: &[String], scope: &str) -> InstallResult {
    let mut result = InstallResult::default();

    // 1. Install hooks
    match install_hooks() {
        Ok(n) => result.hooks_installed = n,
        Err(e) => result.errors.push(format!("hooks: {}", e)),
    }

    // 2. Fetch & install marketplace items (skills, commands)
    match install_marketplace(scope, acps) {
        Ok((skills, commands, version)) => {
            result.skills_installed = skills;
            result.commands_installed = commands;
            result.marketplace_version = version;
        }
        Err(e) => result.errors.push(format!("marketplace: {}", e)),
    }

    // 3. Configure ACPs
    let acp_result = crate::acp::configure(acps);
    result.acps_configured = acp_result.configured;
    result.errors.extend(acp_result.errors);

    result
}

// ── Hook installation ────────────────────────────────────────────────────────

/// Install hook scripts (public for direct endpoint use).
pub fn install_hooks_only() -> Result<u32, String> {
    install_hooks()
}

/// Install hook scripts from embedded marketplace content.
fn install_hooks() -> Result<u32, String> {
    let hooks_dir = plugin_dir().join("hooks");
    fs::create_dir_all(&hooks_dir).map_err(|e| e.to_string())?;

    let hooks: &[(&str, &str)] = &[
        ("session-start", include_str!("../../../marketplace/hooks/session-start")),
        ("pre-tool", include_str!("../../../marketplace/hooks/pre-tool")),
        ("post-tool", include_str!("../../../marketplace/hooks/post-tool")),
        ("run-hook.cmd", include_str!("../../../marketplace/hooks/run-hook.cmd")),
    ];

    let mut count = 0u32;
    for (name, content) in hooks {
        let path = hooks_dir.join(name);
        fs::write(&path, content).map_err(|e| format!("{}: {}", name, e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).ok();
        }
        count += 1;
    }
    Ok(count)
}

// ── Marketplace ──────────────────────────────────────────────────────────────

/// Fetch catalog, download items, install skills & commands.
/// Returns (skills_count, commands_count, version).
fn install_marketplace(
    scope: &str,
    acps: &[String],
) -> Result<(u32, u32, String), String> {
    let catalog = fetch_catalog()?;
    let version = catalog.version.clone().unwrap_or_default();

    let supports_skills = acps.iter().any(|a| a == "claude-code");
    let items: Vec<&CatalogItem> = catalog
        .items
        .iter()
        .filter(|i| scope == "all" || i.scope == scope || i.scope == "global")
        .filter(|i| match i.kind.as_str() {
            "skill" | "command" => supports_skills,
            _ => false,
        })
        .collect();

    let h = home();
    let cache = cache_dir();
    let mut skills = 0u32;
    let mut commands = 0u32;

    for item in &items {
        let content = load_or_download(&cache, &item.path)?;

        let dest = match item.kind.as_str() {
            "skill" => {
                skills += 1;
                h.join(".claude/skills").join(format!("{}.md", item.name))
            }
            "command" => {
                commands += 1;
                h.join(".claude/commands").join(format!("{}.md", item.name))
            }
            _ => continue,
        };
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&dest, &content).map_err(|e| e.to_string())?;
    }

    // Save version to ~/.sensei/config.json
    save_marketplace_version(&version);

    Ok((skills, commands, version))
}

// ── Catalog fetch/cache ──────────────────────────────────────────────────────

/// Fetch the marketplace catalog. Uses cache if version matches.
pub fn fetch_catalog() -> Result<Catalog, String> {
    let cache = cache_dir();
    let cached_path = cache.join(MARKETPLACE_CATALOG);

    // Check cache
    if cached_path.exists() {
        if let Ok(content) = fs::read_to_string(&cached_path) {
            if let Ok(catalog) = serde_json::from_str::<Catalog>(&content) {
                let cached_ver = catalog.version.as_deref().unwrap_or("");
                let saved_ver = load_marketplace_version();
                if !cached_ver.is_empty() && cached_ver == saved_ver {
                    return Ok(catalog);
                }
            }
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
fn load_or_download(cache: &Path, path: &str) -> Result<String, String> {
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

// ── Marketplace version tracking via ~/.sensei/config.json ───────────────────

fn load_marketplace_version() -> String {
    let config_file = sensei_dir().join("config.json");
    config_file
        .exists()
        .then(|| fs::read_to_string(&config_file).ok())
        .flatten()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["marketplace_version"].as_str().map(String::from))
        .unwrap_or_default()
}

fn save_marketplace_version(version: &str) {
    let config_file = sensei_dir().join("config.json");
    fs::create_dir_all(sensei_dir()).ok();
    let mut config: serde_json::Value = config_file
        .exists()
        .then(|| fs::read_to_string(&config_file).ok())
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    config["marketplace_version"] = serde_json::json!(version);
    fs::write(&config_file, serde_json::to_string_pretty(&config).unwrap()).ok();
}

// ── Uninstall ────────────────────────────────────────────────────────────────

#[derive(Serialize, Default)]
pub struct UninstallResult {
    pub acps_removed: Vec<String>,
    pub hooks_removed: bool,
    pub skills_removed: u32,
    pub plugin_removed: bool,
    pub cache_cleared: bool,
}

/// Full uninstall: remove MCP config from ACPs, hooks, skills, plugin dir, cache.
pub fn uninstall() -> UninstallResult {
    let mut result = UninstallResult::default();
    let h = home();

    // 1. Remove ACP configurations
    result.acps_removed = crate::acp::unconfigure();

    // 2. Remove plugin directory (hooks + binaries)
    let plugin = plugin_dir();
    if plugin.exists() {
        fs::remove_dir_all(&plugin).ok();
        result.plugin_removed = true;
        result.hooks_removed = true;
    }

    // 3. Remove installed skills
    let skills_dir = h.join(".claude/skills");
    if skills_dir.exists() {
        if let Ok(entries) = fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().is_some_and(|e| e == "md") {
                    fs::remove_file(entry.path()).ok();
                    result.skills_removed += 1;
                }
            }
        }
    }

    // 4. Remove installed commands
    let commands_dir = h.join(".claude/commands");
    if commands_dir.exists() {
        if let Ok(entries) = fs::read_dir(&commands_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().is_some_and(|e| e == "md") {
                    fs::remove_file(entry.path()).ok();
                }
            }
        }
    }

    // 5. Clear marketplace cache
    let cache = cache_dir();
    if cache.exists() {
        fs::remove_dir_all(&cache).ok();
        result.cache_cleared = true;
    }

    // 6. Clear config
    let config_file = sensei_dir().join("config.json");
    if config_file.exists() {
        fs::remove_file(&config_file).ok();
    }

    result
}

// ── Individual item install (for desktop UI) ─────────────────────────────────

/// Install a single marketplace item by name.
pub fn install_item(name: &str, kind: &str) -> Result<String, String> {
    let catalog = fetch_catalog()?;
    let item = catalog
        .items
        .iter()
        .find(|i| i.name == name && i.kind == kind)
        .ok_or_else(|| format!("{} '{}' not found in catalog", kind, name))?;

    let cache = cache_dir();
    let content = load_or_download(&cache, &item.path)?;
    let h = home();

    let dest = match kind {
        "skill" => h.join(".claude/skills").join(format!("{}.md", name)),
        "command" => h.join(".claude/commands").join(format!("{}.md", name)),
        _ => return Err(format!("unsupported kind: {}", kind)),
    };

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&dest, &content).map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().into_owned())
}

/// Remove a single installed item.
pub fn uninstall_item(name: &str, kind: &str) -> Result<(), String> {
    let h = home();
    let path = match kind {
        "skill" => h.join(".claude/skills").join(format!("{}.md", name)),
        "command" => h.join(".claude/commands").join(format!("{}.md", name)),
        _ => return Err(format!("unsupported kind: {}", kind)),
    };
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// List installed items (skills + commands).
pub fn list_installed() -> Vec<InstalledItem> {
    let h = home();
    let mut items = vec![];

    for (kind, dir) in &[("skill", ".claude/skills"), ("command", ".claude/commands")] {
        let dir = h.join(dir);
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    items.push(InstalledItem {
                        name,
                        kind: kind.to_string(),
                        path: path.to_string_lossy().into_owned(),
                    });
                }
            }
        }
    }

    items
}

#[derive(Serialize)]
pub struct InstalledItem {
    pub name: String,
    pub kind: String,
    pub path: String,
}
