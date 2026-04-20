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

// ── Paths (delegated to crate::paths) ────────────────────────────────────────

fn home() -> PathBuf { crate::paths::home() }
fn sensei_dir() -> PathBuf { crate::paths::sensei_dir() }
fn plugin_dir() -> PathBuf { crate::paths::plugin_dir() }
fn cache_dir() -> PathBuf { crate::paths::cache_dir() }

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

// ── Uninstall request/result ────────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct UninstallRequest {
    #[serde(default = "default_scope")]
    pub scope: String,
    #[serde(default)]
    pub projects: Vec<String>,
}
fn default_scope() -> String { "all".into() }

#[derive(Serialize, Default)]
pub struct UninstallResult {
    pub acps_removed: Vec<String>,
    pub hooks_removed: bool,
    pub skills_removed: u32,
    pub commands_removed: u32,
    pub agents_removed: u32,
    pub plugin_removed: bool,
    pub cache_cleared: bool,
    pub projects_cleaned: Vec<String>,
    pub errors: Vec<String>,
}

/// Scoped uninstall — handles user scope, project scope, or both.
pub fn uninstall(req: &UninstallRequest) -> UninstallResult {
    let mut result = UninstallResult::default();
    let do_user = req.scope == "all" || req.scope == "user";
    let do_project = req.scope == "all" || req.scope == "project";

    if do_user {
        uninstall_user_scope(&mut result);
    }

    if do_project {
        for path in &req.projects {
            uninstall_project_scope(path, &mut result);
        }
    }

    result
}

/// Legacy uninstall — backward compat for old API callers.
pub fn uninstall_legacy() -> UninstallResult {
    let req = UninstallRequest { scope: "user".into(), projects: vec![] };
    uninstall(&req)
}

/// User scope: remove ACP configs, global commands/skills, cache, config.
fn uninstall_user_scope(result: &mut UninstallResult) {
    let h = home();

    // 1. Remove ACP configurations (each adapter handles its own cleanup)
    result.acps_removed = crate::acp::unconfigure();

    // 2. Remove legacy plugin directory (hooks + binaries)
    let plugin = plugin_dir();
    if plugin.exists() {
        fs::remove_dir_all(&plugin).ok();
        result.plugin_removed = true;
        result.hooks_removed = true;
    }

    // 3. Remove global skills (user-scoped, in ~/.claude/skills/)
    let skills_dir = h.join(".claude/skills");
    result.skills_removed += remove_md_files_in(&skills_dir);

    // 4. Remove global commands (user-scoped, in ~/.claude/commands/)
    let commands_dir = h.join(".claude/commands");
    result.commands_removed += remove_md_files_in(&commands_dir);

    // 5. Remove global agents (user-scoped, in ~/.claude/agents/)
    let agents_dir = h.join(".claude/agents");
    result.agents_removed += remove_md_files_in(&agents_dir);

    // 6. Clear marketplace cache
    let cache = cache_dir();
    if cache.exists() {
        fs::remove_dir_all(&cache).ok();
        result.cache_cleared = true;
    }

    // 7. Clear ~/.sensei/ (config, cache, indexes, projects registry)
    let sd = sensei_dir();
    if sd.exists() {
        fs::remove_dir_all(&sd).ok();
    }
}

/// Project scope: remove .sensei/, .claude/{commands,skills,agents}, sensei from .mcp.json.
fn uninstall_project_scope(project_path: &str, result: &mut UninstallResult) {
    let root = std::path::PathBuf::from(project_path);
    if !root.exists() { return; }

    // .sensei/ directory
    let sensei = root.join(".sensei");
    if sensei.exists() {
        fs::remove_dir_all(&sensei).ok();
    }

    // .claude/commands/, .claude/skills/, .claude/agents/
    for subdir in &["commands", "skills", "agents"] {
        let dir = root.join(".claude").join(subdir);
        if dir.exists() {
            let count = remove_md_files_in(&dir);
            match *subdir {
                "skills" => result.skills_removed += count,
                "commands" => result.commands_removed += count,
                "agents" => result.agents_removed += count,
                _ => {}
            }
        }
    }

    // Remove sensei entry from .mcp.json (preserve other servers)
    let mcp_file = root.join(".mcp.json");
    if mcp_file.exists() {
        if let Ok(content) = fs::read_to_string(&mcp_file) {
            if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(servers) = config
                    .get_mut("mcpServers")
                    .and_then(|s| s.as_object_mut())
                {
                    if servers.remove("sensei").is_some() {
                        if servers.is_empty() {
                            fs::remove_file(&mcp_file).ok();
                        } else {
                            fs::write(&mcp_file, serde_json::to_string_pretty(&config).unwrap()).ok();
                        }
                    }
                }
            }
        }
    }

    result.projects_cleaned.push(project_path.to_string());
}

/// Remove all .md files in a directory. Removes the directory if empty afterward.
fn remove_md_files_in(dir: &std::path::Path) -> u32 {
    if !dir.exists() { return 0; }
    let mut count = 0u32;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|e| e == "md") {
                fs::remove_file(entry.path()).ok();
                count += 1;
            }
        }
    }
    if fs::read_dir(dir).map(|mut d| d.next().is_none()).unwrap_or(true) {
        fs::remove_dir(dir).ok();
    }
    count
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // ── remove_md_files_in ─────────────────────────────────────────────

    #[test]
    fn remove_md_files_in_nonexistent_dir_returns_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("does_not_exist");
        assert_eq!(remove_md_files_in(&missing), 0);
    }

    #[test]
    fn remove_md_files_in_empty_dir_returns_zero_and_removes_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("empty");
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(remove_md_files_in(&dir), 0);
        // Empty dir should be removed
        assert!(!dir.exists());
    }

    #[test]
    fn remove_md_files_in_removes_only_md_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("skills");
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("one.md"), "# skill one").unwrap();
        fs::write(dir.join("two.md"), "# skill two").unwrap();
        fs::write(dir.join("keep.txt"), "not markdown").unwrap();

        let count = remove_md_files_in(&dir);
        assert_eq!(count, 2);
        assert!(!dir.join("one.md").exists());
        assert!(!dir.join("two.md").exists());
        // Non-md file is preserved
        assert!(dir.join("keep.txt").exists());
        // Dir is NOT removed because it still has files
        assert!(dir.exists());
    }

    #[test]
    fn remove_md_files_in_cleans_empty_dir_after_removing_all_md() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("commands");
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("only.md"), "# only md file").unwrap();

        let count = remove_md_files_in(&dir);
        assert_eq!(count, 1);
        // Dir should be removed because it's now empty
        assert!(!dir.exists());
    }

    // ── uninstall_project_scope ────────────────────────────────────────

    #[test]
    fn uninstall_project_scope_nonexistent_path_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("no_such_project");
        let mut result = UninstallResult::default();
        uninstall_project_scope(missing.to_str().unwrap(), &mut result);
        // Should not be added to projects_cleaned because path doesn't exist
        assert!(result.projects_cleaned.is_empty());
    }

    #[test]
    fn uninstall_project_scope_removes_sensei_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("my_project");
        let sensei = project.join(".sensei");
        fs::create_dir_all(sensei.join("indexes")).unwrap();
        fs::write(sensei.join("config.json"), "{}").unwrap();

        let mut result = UninstallResult::default();
        uninstall_project_scope(project.to_str().unwrap(), &mut result);

        assert!(!sensei.exists());
        assert_eq!(result.projects_cleaned, vec![project.to_str().unwrap()]);
    }

    #[test]
    fn uninstall_project_scope_removes_claude_subdirs_md_files() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let claude = project.join(".claude");

        // Create skills with 2 md files
        let skills_dir = claude.join("skills");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(skills_dir.join("a.md"), "skill a").unwrap();
        fs::write(skills_dir.join("b.md"), "skill b").unwrap();

        // Create commands with 1 md file + 1 non-md
        let cmds_dir = claude.join("commands");
        fs::create_dir_all(&cmds_dir).unwrap();
        fs::write(cmds_dir.join("c.md"), "command c").unwrap();
        fs::write(cmds_dir.join("keep.txt"), "keep").unwrap();

        // Create agents with 1 md file (agents count is not tracked)
        let agents_dir = claude.join("agents");
        fs::create_dir_all(&agents_dir).unwrap();
        fs::write(agents_dir.join("d.md"), "agent d").unwrap();

        let mut result = UninstallResult::default();
        uninstall_project_scope(project.to_str().unwrap(), &mut result);

        assert_eq!(result.skills_removed, 2);
        assert_eq!(result.commands_removed, 1);
        // Non-md file is preserved, so commands dir still exists
        assert!(cmds_dir.join("keep.txt").exists());
        // Skills dir should be cleaned up (was only md files)
        assert!(!skills_dir.exists());
        // Agents dir should be cleaned up
        assert!(!agents_dir.exists());
    }

    #[test]
    fn uninstall_project_scope_removes_sensei_from_mcp_json() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        fs::create_dir_all(&project).unwrap();

        let mcp_file = project.join(".mcp.json");
        let config = serde_json::json!({
            "mcpServers": {
                "sensei": {"command": "sensei-mcp", "args": []},
                "other": {"command": "other-mcp", "args": []}
            }
        });
        fs::write(&mcp_file, serde_json::to_string_pretty(&config).unwrap()).unwrap();

        let mut result = UninstallResult::default();
        uninstall_project_scope(project.to_str().unwrap(), &mut result);

        // File should still exist with 'other' server preserved
        assert!(mcp_file.exists());
        let content: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&mcp_file).unwrap()).unwrap();
        assert!(content["mcpServers"]["sensei"].is_null());
        assert_eq!(content["mcpServers"]["other"]["command"], "other-mcp");
    }

    #[test]
    fn uninstall_project_scope_deletes_mcp_json_when_sensei_is_only_server() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        fs::create_dir_all(&project).unwrap();

        let mcp_file = project.join(".mcp.json");
        let config = serde_json::json!({
            "mcpServers": {
                "sensei": {"command": "sensei-mcp"}
            }
        });
        fs::write(&mcp_file, serde_json::to_string_pretty(&config).unwrap()).unwrap();

        let mut result = UninstallResult::default();
        uninstall_project_scope(project.to_str().unwrap(), &mut result);

        // File should be deleted when sensei was the only server
        assert!(!mcp_file.exists());
    }

    #[test]
    fn uninstall_project_scope_preserves_mcp_json_without_sensei_key() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        fs::create_dir_all(&project).unwrap();

        let mcp_file = project.join(".mcp.json");
        let original = serde_json::json!({
            "mcpServers": {
                "other": {"command": "other-mcp"}
            }
        });
        let original_str = serde_json::to_string_pretty(&original).unwrap();
        fs::write(&mcp_file, &original_str).unwrap();

        let mut result = UninstallResult::default();
        uninstall_project_scope(project.to_str().unwrap(), &mut result);

        // File should be untouched — no sensei key to remove
        assert!(mcp_file.exists());
        let content = fs::read_to_string(&mcp_file).unwrap();
        assert_eq!(content, original_str);
    }

    // ── uninstall (integration-level) ─────────────────────────────────

    #[test]
    fn uninstall_project_scope_via_uninstall_fn() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let sensei = project.join(".sensei");
        fs::create_dir_all(&sensei).unwrap();
        fs::write(sensei.join("config.json"), "{}").unwrap();

        let req = UninstallRequest {
            scope: "project".into(),
            projects: vec![project.to_str().unwrap().to_string()],
        };
        let result = uninstall(&req);

        assert!(!sensei.exists());
        assert_eq!(result.projects_cleaned.len(), 1);
    }

    #[test]
    fn uninstall_with_scope_project_does_not_touch_user() {
        // With scope="project", hooks_removed and cache_cleared should be false
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        fs::create_dir_all(&project).unwrap();

        let req = UninstallRequest {
            scope: "project".into(),
            projects: vec![project.to_str().unwrap().to_string()],
        };
        let result = uninstall(&req);

        assert!(!result.hooks_removed);
        assert!(!result.cache_cleared);
        assert!(!result.plugin_removed);
    }

    #[test]
    fn uninstall_multiple_projects() {
        let tmp = tempfile::tempdir().unwrap();

        let p1 = tmp.path().join("proj1");
        let p2 = tmp.path().join("proj2");
        fs::create_dir_all(p1.join(".sensei")).unwrap();
        fs::create_dir_all(p2.join(".sensei")).unwrap();

        let req = UninstallRequest {
            scope: "project".into(),
            projects: vec![
                p1.to_str().unwrap().to_string(),
                p2.to_str().unwrap().to_string(),
            ],
        };
        let result = uninstall(&req);

        assert!(!p1.join(".sensei").exists());
        assert!(!p2.join(".sensei").exists());
        assert_eq!(result.projects_cleaned.len(), 2);
    }

    // ── uninstall_legacy ──────────────────────────────────────────────

    #[test]
    fn uninstall_legacy_returns_result_with_user_scope() {
        // Mainly verifies it doesn't panic and uses user scope
        // (project-specific assertions can't be validated without real home)
        let result = uninstall_legacy();
        // projects_cleaned is always empty because legacy has no projects
        assert!(result.projects_cleaned.is_empty());
    }

    // ── UninstallRequest deserialization ───────────────────────────────

    #[test]
    fn uninstall_request_default_scope_is_all() {
        let req: UninstallRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(req.scope, "all");
        assert!(req.projects.is_empty());
    }

    #[test]
    fn uninstall_request_parses_full_json() {
        let json = r#"{"scope": "project", "projects": ["/tmp/a", "/tmp/b"]}"#;
        let req: UninstallRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.scope, "project");
        assert_eq!(req.projects, vec!["/tmp/a", "/tmp/b"]);
    }

    // ── InstallResult serialization ───────────────────────────────────

    #[test]
    fn install_result_default_is_empty() {
        let result = InstallResult::default();
        assert_eq!(result.hooks_installed, 0);
        assert_eq!(result.skills_installed, 0);
        assert_eq!(result.commands_installed, 0);
        assert!(result.acps_configured.is_empty());
        assert!(result.errors.is_empty());
        assert!(result.marketplace_version.is_empty());
    }

    #[test]
    fn install_result_serializes_to_json() {
        let result = InstallResult {
            hooks_installed: 4,
            skills_installed: 3,
            commands_installed: 2,
            acps_configured: vec!["claude-code".into()],
            errors: vec![],
            marketplace_version: "1.0.0".into(),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["hooks_installed"], 4);
        assert_eq!(json["skills_installed"], 3);
        assert_eq!(json["commands_installed"], 2);
        assert_eq!(json["marketplace_version"], "1.0.0");
    }

    // ── UninstallResult serialization ─────────────────────────────────

    #[test]
    fn uninstall_result_default_is_empty() {
        let result = UninstallResult::default();
        assert!(!result.hooks_removed);
        assert!(!result.plugin_removed);
        assert!(!result.cache_cleared);
        assert_eq!(result.skills_removed, 0);
        assert_eq!(result.commands_removed, 0);
        assert_eq!(result.agents_removed, 0);
        assert!(result.acps_removed.is_empty());
        assert!(result.projects_cleaned.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn uninstall_result_serializes_to_json() {
        let result = UninstallResult {
            acps_removed: vec!["claude-code".into()],
            hooks_removed: true,
            skills_removed: 5,
            commands_removed: 3,
            agents_removed: 8,
            plugin_removed: true,
            cache_cleared: true,
            projects_cleaned: vec!["/tmp/proj".into()],
            errors: vec!["some error".into()],
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["hooks_removed"], true);
        assert_eq!(json["skills_removed"], 5);
        assert_eq!(json["commands_removed"], 3);
        assert_eq!(json["agents_removed"], 8);
        assert_eq!(json["plugin_removed"], true);
        assert_eq!(json["cache_cleared"], true);
        assert_eq!(json["projects_cleaned"][0], "/tmp/proj");
        assert_eq!(json["errors"][0], "some error");
    }

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

    // ── InstalledItem serialization ───────────────────────────────────

    #[test]
    fn installed_item_serializes() {
        let item = InstalledItem {
            name: "review".into(),
            kind: "skill".into(),
            path: "/home/user/.claude/skills/review.md".into(),
        };
        let json: serde_json::Value = serde_json::to_value(&item).unwrap();
        assert_eq!(json["name"], "review");
        assert_eq!(json["kind"], "skill");
        assert!(json["path"].as_str().unwrap().ends_with("review.md"));
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

    // ── install_hooks (writes to plugin_dir — tests via real dirs) ────

    #[test]
    fn install_hooks_creates_hook_files() {
        // install_hooks uses plugin_dir() which goes to real home;
        // we can still verify it returns count and doesn't error.
        // This test exercises the real path but is safe (idempotent writes).
        let result = install_hooks();
        assert!(result.is_ok());
        let count = result.unwrap();
        assert_eq!(count, 4); // session-start, pre-tool, post-tool, run-hook.cmd
    }

    #[cfg(unix)]
    #[test]
    fn install_hooks_sets_executable_permissions() {
        use std::os::unix::fs::PermissionsExt;

        // Run install first
        install_hooks().unwrap();

        let hooks_dir = plugin_dir().join("hooks");
        for name in &["session-start", "pre-tool", "post-tool"] {
            let path = hooks_dir.join(name);
            assert!(path.exists(), "hook {} should exist", name);
            let perms = fs::metadata(&path).unwrap().permissions();
            let mode = perms.mode() & 0o777;
            assert_eq!(mode, 0o755, "hook {} should be executable (0o755)", name);
        }
    }

    // ── uninstall_project_scope edge cases ────────────────────────────

    #[test]
    fn uninstall_project_scope_handles_invalid_mcp_json() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        fs::create_dir_all(&project).unwrap();

        // Write invalid JSON
        let mcp_file = project.join(".mcp.json");
        fs::write(&mcp_file, "not valid json!!!").unwrap();

        let mut result = UninstallResult::default();
        uninstall_project_scope(project.to_str().unwrap(), &mut result);

        // Should not panic, file should still exist (couldn't parse it)
        assert!(mcp_file.exists());
        assert_eq!(result.projects_cleaned, vec![project.to_str().unwrap()]);
    }

    #[test]
    fn uninstall_project_scope_handles_mcp_json_without_mcp_servers() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        fs::create_dir_all(&project).unwrap();

        let mcp_file = project.join(".mcp.json");
        fs::write(&mcp_file, r#"{"other_key": "value"}"#).unwrap();

        let mut result = UninstallResult::default();
        uninstall_project_scope(project.to_str().unwrap(), &mut result);

        // File untouched, no panic
        assert!(mcp_file.exists());
        let content = fs::read_to_string(&mcp_file).unwrap();
        assert!(content.contains("other_key"));
    }

    #[test]
    fn uninstall_project_scope_full_cleanup() {
        // Test a project with .sensei, all .claude subdirs, and .mcp.json
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");

        // .sensei/
        fs::create_dir_all(project.join(".sensei/indexes")).unwrap();
        fs::write(project.join(".sensei/config.json"), "{}").unwrap();

        // .claude/skills/, .claude/commands/, .claude/agents/
        fs::create_dir_all(project.join(".claude/skills")).unwrap();
        fs::write(project.join(".claude/skills/s1.md"), "s1").unwrap();
        fs::write(project.join(".claude/skills/s2.md"), "s2").unwrap();
        fs::create_dir_all(project.join(".claude/commands")).unwrap();
        fs::write(project.join(".claude/commands/c1.md"), "c1").unwrap();
        fs::create_dir_all(project.join(".claude/agents")).unwrap();
        fs::write(project.join(".claude/agents/a1.md"), "a1").unwrap();

        // .mcp.json with sensei as sole server
        fs::write(
            project.join(".mcp.json"),
            r#"{"mcpServers":{"sensei":{"command":"sensei-mcp"}}}"#,
        )
        .unwrap();

        let mut result = UninstallResult::default();
        uninstall_project_scope(project.to_str().unwrap(), &mut result);

        assert!(!project.join(".sensei").exists());
        assert!(!project.join(".claude/skills").exists());
        assert!(!project.join(".claude/commands").exists());
        assert!(!project.join(".claude/agents").exists());
        assert!(!project.join(".mcp.json").exists());
        assert_eq!(result.skills_removed, 2);
        assert_eq!(result.commands_removed, 1);
        assert_eq!(result.agents_removed, 1);
        assert_eq!(result.projects_cleaned, vec![project.to_str().unwrap()]);
    }

    // ── remove_md_files_in edge cases ─────────────────────────────────

    #[test]
    fn remove_md_files_in_ignores_subdirectories() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("skills");
        fs::create_dir_all(dir.join("subdir")).unwrap();
        fs::write(dir.join("a.md"), "a").unwrap();
        fs::write(dir.join("subdir/b.md"), "b").unwrap();

        let count = remove_md_files_in(&dir);
        // Only top-level .md files are removed
        assert_eq!(count, 1);
        assert!(!dir.join("a.md").exists());
        // Subdirectory and its files are untouched
        assert!(dir.join("subdir/b.md").exists());
    }

    #[test]
    fn remove_md_files_in_handles_mixed_extensions() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("commands");
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("file.md"), "md").unwrap();
        fs::write(dir.join("file.txt"), "txt").unwrap();
        fs::write(dir.join("file.json"), "json").unwrap();
        fs::write(dir.join("file.md.bak"), "bak").unwrap();
        fs::write(dir.join("no_ext"), "none").unwrap();

        let count = remove_md_files_in(&dir);
        assert_eq!(count, 1);
        assert!(!dir.join("file.md").exists());
        assert!(dir.join("file.txt").exists());
        assert!(dir.join("file.json").exists());
        assert!(dir.join("file.md.bak").exists());
        assert!(dir.join("no_ext").exists());
    }

    // ── Accumulation across multiple project uninstalls ───────────────

    #[test]
    fn uninstall_accumulates_counts_across_projects() {
        let tmp = tempfile::tempdir().unwrap();

        // Project 1: 2 skills, 1 command
        let p1 = tmp.path().join("p1");
        fs::create_dir_all(p1.join(".claude/skills")).unwrap();
        fs::write(p1.join(".claude/skills/a.md"), "a").unwrap();
        fs::write(p1.join(".claude/skills/b.md"), "b").unwrap();
        fs::create_dir_all(p1.join(".claude/commands")).unwrap();
        fs::write(p1.join(".claude/commands/c.md"), "c").unwrap();

        // Project 2: 1 skill, 2 commands
        let p2 = tmp.path().join("p2");
        fs::create_dir_all(p2.join(".claude/skills")).unwrap();
        fs::write(p2.join(".claude/skills/d.md"), "d").unwrap();
        fs::create_dir_all(p2.join(".claude/commands")).unwrap();
        fs::write(p2.join(".claude/commands/e.md"), "e").unwrap();
        fs::write(p2.join(".claude/commands/f.md"), "f").unwrap();

        let req = UninstallRequest {
            scope: "project".into(),
            projects: vec![
                p1.to_str().unwrap().to_string(),
                p2.to_str().unwrap().to_string(),
            ],
        };
        let result = uninstall(&req);

        assert_eq!(result.skills_removed, 3);   // 2 + 1
        assert_eq!(result.commands_removed, 3);  // 1 + 2
        assert_eq!(result.projects_cleaned.len(), 2);
    }

    // ── default_scope helper ──────────────────────────────────────────

    #[test]
    fn default_scope_returns_all() {
        assert_eq!(default_scope(), "all");
    }
}
