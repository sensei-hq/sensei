//! Full install, hook installation, and individual item install/remove/list.

use std::fs;

use super::{
    cache_dir, home, plugin_dir, InstalledItem, InstallResult,
    catalog::fetch_catalog,
    marketplace::install_marketplace,
};

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
        Ok((skills, commands, stale_cmds, stale_skills, version)) => {
            result.skills_installed = skills;
            result.commands_installed = commands;
            result.stale_commands_removed = stale_cmds;
            result.stale_skills_removed = stale_skills;
            result.marketplace_version = version;
        }
        Err(e) => result.errors.push(format!("marketplace: {}", e)),
    }

    // 3. Configure ACPs
    let acp_result = crate::assistants::configure(acps);
    result.acps_configured = acp_result.configured;
    result.errors.extend(acp_result.errors);

    result
}

// ── Hook installation ────────────────────────────────────────────────────────

/// Install hook scripts (public for direct endpoint use).
pub fn install_hooks_only() -> Result<u32, String> {
    install_hooks()
}

/// Hook file names to install from the marketplace repo.
const HOOK_FILES: &[&str] = &[
    "session-start",
    "user-prompt",
    "pre-compact",
    "pre-tool",
    "post-tool",
    "run-hook.cmd",
];

/// Install hook scripts by downloading from the marketplace GitHub repo.
fn install_hooks() -> Result<u32, String> {
    let hooks_dir = plugin_dir().join("hooks");
    fs::create_dir_all(&hooks_dir).map_err(|e| e.to_string())?;

    let cache = cache_dir();
    let mut count = 0u32;

    for name in HOOK_FILES {
        let repo_path = format!("plugins/sensei/hooks/{}", name);
        let content = super::catalog::load_or_download(&cache, &repo_path)?;
        let path = hooks_dir.join(name);
        fs::write(&path, &content).map_err(|e| format!("{}: {}", name, e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).ok();
        }
        count += 1;
    }
    Ok(count)
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
    let content = super::catalog::load_or_download(&cache, &item.path)?;
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
pub fn remove_item(name: &str, kind: &str) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── InstallResult serialization ───────────────────────────────────

    #[test]
    fn install_result_default_is_empty() {
        let result = InstallResult::default();
        assert_eq!(result.hooks_installed, 0);
        assert_eq!(result.skills_installed, 0);
        assert_eq!(result.commands_installed, 0);
        assert_eq!(result.stale_commands_removed, 0);
        assert_eq!(result.stale_skills_removed, 0);
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
            stale_commands_removed: 1,
            stale_skills_removed: 0,
            acps_configured: vec!["claude-code".into()],
            errors: vec![],
            marketplace_version: "1.0.0".into(),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["hooks_installed"], 4);
        assert_eq!(json["skills_installed"], 3);
        assert_eq!(json["commands_installed"], 2);
        assert_eq!(json["stale_commands_removed"], 1);
        assert_eq!(json["marketplace_version"], "1.0.0");
    }

    // ── install_hooks (requires network — downloads from GitHub) ────

    #[test]
    fn hook_file_list_is_complete() {
        assert_eq!(HOOK_FILES.len(), 6);
        assert!(HOOK_FILES.contains(&"session-start"));
        assert!(HOOK_FILES.contains(&"user-prompt"));
        assert!(HOOK_FILES.contains(&"pre-compact"));
        assert!(HOOK_FILES.contains(&"pre-tool"));
        assert!(HOOK_FILES.contains(&"post-tool"));
        assert!(HOOK_FILES.contains(&"run-hook.cmd"));
    }

    #[test]
    #[ignore] // requires network access — run with: cargo test -- --ignored
    fn install_hooks_creates_hook_files() {
        let result = install_hooks();
        assert!(result.is_ok());
        let count = result.unwrap();
        assert_eq!(count, 6);
    }

    #[cfg(unix)]
    #[test]
    #[ignore] // requires network access
    fn install_hooks_sets_executable_permissions() {
        use std::os::unix::fs::PermissionsExt;

        install_hooks().unwrap();

        let hooks_dir = plugin_dir().join("hooks");
        for name in &["session-start", "user-prompt", "pre-compact", "pre-tool", "post-tool"] {
            let path = hooks_dir.join(name);
            assert!(path.exists(), "hook {} should exist", name);
            let perms = fs::metadata(&path).unwrap().permissions();
            let mode = perms.mode() & 0o777;
            assert_eq!(mode, 0o755, "hook {} should be executable (0o755)", name);
        }
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
}
