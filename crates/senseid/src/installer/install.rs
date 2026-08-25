//! Full install, hook installation, and individual item install/remove/list.

use std::fs;

use super::{
    InstallResult, InstalledItem, cache_dir, catalog::fetch_catalog, home,
    marketplace::install_marketplace, plugin_dir,
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

    // 3. Configure ACPs. CLI path — no SSE consumer here, so pass None
    //    and skip the broadcast.
    let acp_result = crate::assistants::configure(acps, None);
    result.acps_configured = acp_result.configured;
    result.errors.extend(acp_result.errors);
    result.warnings.extend(acp_result.warnings);

    result
}

// ── Hook installation ────────────────────────────────────────────────────────

/// Install hook scripts (public for direct endpoint use).
pub fn install_hooks_only() -> Result<u32, String> {
    install_hooks()
}

/// Hook file names to install from the marketplace repo.
const HOOK_FILES: &[&str] =
    &["session-start", "user-prompt", "pre-compact", "pre-tool", "post-tool", "run-hook.cmd"];

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
            if let Err(e) = fs::set_permissions(&path, fs::Permissions::from_mode(0o755)) {
                tracing::warn!(hook = %name, error = %e, "failed to set executable permissions on hook script");
            }
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

    if let Some(parent) = dest.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        tracing::warn!(dir = %parent.display(), error = %e, "failed to create parent dir for installed item");
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

/// List installed items — skills + commands, enabled or disabled.
///
/// Live files live in `~/.claude/<kind>s/*.md`; disabled files sit in
/// the sibling `disabled/*.md` subfolder. Both are scanned so the
/// Settings UI can show the same row with a toggle regardless of state.
pub fn list_installed() -> Vec<InstalledItem> {
    let h = home();
    let mut items = vec![];

    for (kind, dir) in &[("skill", ".claude/skills"), ("command", ".claude/commands")] {
        let live_dir = h.join(dir);
        // Live entries — anything ending in .md directly under <kind>s/.
        if let Ok(entries) = fs::read_dir(&live_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && path.extension().is_some_and(|e| e == "md")
                    && let Some(name) = path.file_stem().and_then(|s| s.to_str())
                {
                    items.push(InstalledItem {
                        name: name.to_string(),
                        kind: kind.to_string(),
                        path: path.to_string_lossy().into_owned(),
                        enabled: true,
                    });
                }
            }
        }
        // Disabled entries — anything ending in .md under <kind>s/disabled/.
        let disabled_dir = live_dir.join("disabled");
        if let Ok(entries) = fs::read_dir(&disabled_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && path.extension().is_some_and(|e| e == "md")
                    && let Some(name) = path.file_stem().and_then(|s| s.to_str())
                {
                    items.push(InstalledItem {
                        name: name.to_string(),
                        kind: kind.to_string(),
                        path: path.to_string_lossy().into_owned(),
                        enabled: false,
                    });
                }
            }
        }
    }

    items
}

/// Enable or disable an installed skill / command by moving the .md
/// file between `~/.claude/<kind>s/` and its `disabled/` sibling.
/// Returns Ok(true) when a move happened, Ok(false) when the item was
/// already in the target state, and Err with a clear message on I/O
/// failure or unknown kind.
///
/// Claude Code doesn't scan the `disabled/` folder, so a moved file
/// stops being active as soon as the rename lands.
pub fn set_item_enabled(name: &str, kind: &str, enabled: bool) -> Result<bool, String> {
    let subdir = match kind {
        "skill" => ".claude/skills",
        "command" => ".claude/commands",
        other => return Err(format!("unknown kind: {other} (expected 'skill' or 'command')")),
    };
    let live_dir = home().join(subdir);
    let disabled_dir = live_dir.join("disabled");
    let file_name = format!("{name}.md");
    let live_path = live_dir.join(&file_name);
    let disabled_path = disabled_dir.join(&file_name);

    // Ambiguous state — same-named file in both folders — comes first.
    // Probably a mv failed half-way or someone touched the tree by hand;
    // refusing to toggle is safer than picking a side.
    if live_path.exists() && disabled_path.exists() {
        return Err(format!(
            "{kind} '{name}' exists in both live and disabled folders — resolve manually before toggling"
        ));
    }

    // Idempotency + find the source.
    let (source, dest, dest_dir) = match (live_path.exists(), disabled_path.exists(), enabled) {
        (true, _, true) => return Ok(false),  // already enabled
        (_, true, false) => return Ok(false), // already disabled
        (true, false, false) => (&live_path, &disabled_path, &disabled_dir), // enable → disable
        (false, true, true) => (&disabled_path, &live_path, &live_dir), // disable → enable
        (false, false, _) => return Err(format!("{kind} '{name}' not found")), // unknown
    };

    // Ensure the destination directory exists (first-time disable
    // creates `disabled/`; a fresh home may lack it).
    if !dest_dir.exists() {
        std::fs::create_dir_all(dest_dir).map_err(|e| format!("create dir {dest_dir:?}: {e}"))?;
    }
    std::fs::rename(source, dest).map_err(|e| format!("rename {source:?} → {dest:?}: {e}"))?;
    Ok(true)
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
            warnings: vec!["claude-code: dev hooks: settings.json read-only".into()],
            marketplace_version: "1.0.0".into(),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["hooks_installed"], 4);
        assert_eq!(json["skills_installed"], 3);
        assert_eq!(json["commands_installed"], 2);
        assert_eq!(json["stale_commands_removed"], 1);
        assert_eq!(json["marketplace_version"], "1.0.0");
        // warnings must survive the round-trip — that's the field this fix adds.
        assert_eq!(json["warnings"][0], "claude-code: dev hooks: settings.json read-only");
        assert!(json["errors"].as_array().unwrap().is_empty());
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
            enabled: true,
        };
        let json: serde_json::Value = serde_json::to_value(&item).unwrap();
        assert_eq!(json["name"], "review");
        assert_eq!(json["kind"], "skill");
        assert!(json["path"].as_str().unwrap().ends_with("review.md"));
        assert_eq!(json["enabled"], true);
    }

    #[test]
    fn set_item_enabled_rejects_unknown_kind() {
        let err = set_item_enabled("foo", "agent", true).expect_err("agents not covered here");
        assert!(err.contains("unknown kind"), "expected clear kind error, got: {err}");
    }

    // The filesystem-mutation path — live↔disabled/ rename — needs a
    // scratch $HOME, which the process shares with every other parallel
    // test and can't be mutated safely from a `#[test]`. It's covered
    // instead by the live smoke against a running daemon (PUT
    // /api/install/installed/{name}/enabled + `ls` before/after), which
    // this commit records in the message.
}
