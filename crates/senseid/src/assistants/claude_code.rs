use std::path::{Path, PathBuf};
use tracing::{info, warn};
use super::trait_def::{Assistant, AssistantConfigureOk};
use super::helpers::{home, find_claude_binary};
use super::AssistantPart;

use crate::paths::MARKETPLACE_REPO as SENSEI_MARKETPLACE_REPO;

/// Pure helper: read `installed_plugins.json` and confirm that a plugin
/// `{plugin_name}@*` is recorded. Returns false on any read/parse failure
/// or when the entry is absent. Extracted so configure() can re-use it as
/// the post-install verification gate (don't trust `claude plugin install`'s
/// exit code alone — confirm the manifest actually has the plugin).
fn verify_plugin_installed(manifest_path: &Path, plugin_name: &str) -> bool {
    let Some(content) = std::fs::read_to_string(manifest_path).ok() else { return false };
    let Some(value) = serde_json::from_str::<serde_json::Value>(&content).ok() else { return false };
    let Some(plugins) = value.get("plugins").and_then(|p| p.as_object()) else { return false };
    let prefix = format!("{}@", plugin_name);
    plugins.keys().any(|k| k.starts_with(&prefix))
}

/// Path to Claude Code's `installed_plugins.json` (under the user's home).
fn installed_plugins_manifest() -> PathBuf {
    home().join(".claude/plugins/installed_plugins.json")
}

/// MCP server registry keys the daemon owns. Any entry under these keys in a
/// user/project MCP config is presumed to be sensei's and gets removed during
/// cleanup so the plugin install can re-register cleanly.
const SENSEI_MCP_KEYS: &[&str] = &["sensei"];

/// Filenames (basename match) the daemon recognises as legacy sensei hook
/// dispatchers — installed by previous versions of the daemon directly into
/// `~/.claude/settings.json` before the plugin migration. These are the ONLY
/// commands `clean_legacy_sensei_hooks` will strip; anything else with
/// "sensei" in the path is left alone so user-authored hooks don't get
/// removed by mistake.
const LEGACY_SENSEI_HOOK_BASENAMES: &[&str] = &["sensei-hook.ts", "sensei-hook-dev.ts"];

/// Remove any sensei-keyed entries from a user/project `mcp.json`-shaped file.
/// Writes a `.bak` next to the file before editing so the original is
/// recoverable. Returns the list of keys removed. No-op (and no `.bak`
/// written) when the file is missing or carries no sensei keys.
///
/// Path-agnostic — works for `~/.claude/mcp.json` (user scope) and
/// `<project>/.mcp.json` (project scope) alike.
fn clean_sensei_from_mcp_file(path: &Path) -> Result<Vec<String>, String> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let original = std::fs::read_to_string(path)
        .map_err(|e| format!("read {}: {}", path.display(), e))?;
    let mut value: serde_json::Value = json5::from_str(&original)
        .map_err(|e| format!("parse {}: {}", path.display(), e))?;

    let Some(servers) = value.get_mut("mcpServers").and_then(|s| s.as_object_mut()) else {
        return Ok(vec![]);
    };

    let mut removed = Vec::new();
    for key in SENSEI_MCP_KEYS {
        if servers.remove(*key).is_some() {
            removed.push((*key).to_string());
        }
    }
    if removed.is_empty() {
        return Ok(vec![]);
    }

    // Backup BEFORE the destructive write. If anything below fails the user
    // still has the original on disk at `<path>.bak`.
    let backup = path.with_extension(
        path.extension().and_then(|e| e.to_str()).map(|e| format!("{}.bak", e))
            .unwrap_or_else(|| "bak".into()),
    );
    std::fs::write(&backup, &original)
        .map_err(|e| format!("write backup {}: {}", backup.display(), e))?;

    let serialized = serde_json::to_string_pretty(&value)
        .map_err(|e| format!("serialise {}: {}", path.display(), e))?;
    std::fs::write(path, serialized)
        .map_err(|e| format!("write {}: {}", path.display(), e))?;

    info!(path = %path.display(), removed = ?removed, "cleaned stale sensei mcp entries");
    Ok(removed)
}

/// Strip hook entries whose command basename matches a known legacy sensei
/// hook dispatcher (`sensei-hook.ts`, `sensei-hook-dev.ts`) from a Claude
/// Code `settings.json`-shaped file. These were installed by versions of
/// the daemon that wrote directly into user settings before the plugin
/// migration; they survive `claude plugin uninstall sensei` because the
/// plugin uninstaller only touches `installed_plugins.json`.
///
/// Behaviour mirrors `clean_sensei_from_mcp_file`: writes a `.bak` before
/// any destructive edit, no-op (no `.bak`) when nothing to strip, returns
/// the list of hook event names where at least one matcher group was
/// modified or removed.
fn clean_legacy_sensei_hooks(settings_path: &Path) -> Result<Vec<String>, String> {
    if !settings_path.exists() {
        return Ok(vec![]);
    }
    let original = std::fs::read_to_string(settings_path)
        .map_err(|e| format!("read {}: {}", settings_path.display(), e))?;
    let mut value: serde_json::Value = json5::from_str(&original)
        .map_err(|e| format!("parse {}: {}", settings_path.display(), e))?;

    let Some(hooks_obj) = value.get_mut("hooks").and_then(|h| h.as_object_mut()) else {
        return Ok(vec![]);
    };

    let mut stripped_events: Vec<String> = Vec::new();
    let event_names: Vec<String> = hooks_obj.keys().cloned().collect();
    for event_name in event_names {
        let Some(groups) = hooks_obj.get_mut(&event_name).and_then(|g| g.as_array_mut()) else {
            continue;
        };
        let mut modified = false;
        groups.retain_mut(|group| {
            let Some(group_obj) = group.as_object_mut() else { return true; };
            let Some(inner_hooks) = group_obj.get_mut("hooks").and_then(|h| h.as_array_mut()) else {
                return true;
            };
            let before = inner_hooks.len();
            inner_hooks.retain(|hook| {
                let cmd = hook.get("command").and_then(|c| c.as_str()).unwrap_or("");
                let basename = std::path::Path::new(cmd)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                !LEGACY_SENSEI_HOOK_BASENAMES.contains(&basename)
            });
            if inner_hooks.len() != before { modified = true; }
            // Drop the matcher group when its only hooks were legacy entries.
            !inner_hooks.is_empty()
        });
        if modified {
            stripped_events.push(event_name.clone());
        }
        if groups.is_empty() {
            hooks_obj.remove(&event_name);
        }
    }

    if stripped_events.is_empty() {
        return Ok(vec![]);
    }

    let backup = settings_path.with_extension(
        settings_path.extension().and_then(|e| e.to_str()).map(|e| format!("{}.bak", e))
            .unwrap_or_else(|| "bak".into()),
    );
    std::fs::write(&backup, &original)
        .map_err(|e| format!("write backup {}: {}", backup.display(), e))?;

    let serialized = serde_json::to_string_pretty(&value)
        .map_err(|e| format!("serialise {}: {}", settings_path.display(), e))?;
    std::fs::write(settings_path, serialized)
        .map_err(|e| format!("write {}: {}", settings_path.display(), e))?;

    info!(path = %settings_path.display(), events = ?stripped_events, "cleaned legacy sensei hook entries");
    Ok(stripped_events)
}

/// Delete any legacy sensei hook script files left under `<home>/.claude/hooks/`.
/// Returns the paths actually removed. No-op when the directory or files
/// are missing. Failing to remove a single file does not abort — the rest
/// of remove() should still run.
fn delete_legacy_sensei_hook_files(home_dir: &Path) -> Vec<PathBuf> {
    let mut deleted = Vec::new();
    let hooks_dir = home_dir.join(".claude/hooks");
    for basename in LEGACY_SENSEI_HOOK_BASENAMES {
        let p = hooks_dir.join(basename);
        if p.exists() && std::fs::remove_file(&p).is_ok() {
            deleted.push(p);
        }
    }
    deleted
}

/// Read the daemon's tracked project list from `~/.sensei/projects.json`.
/// The file is a JSON array of absolute path strings; missing file,
/// malformed JSON, or wrong-shaped data all return an empty Vec rather
/// than an error — the remove flow must not abort on a corrupted index.
fn read_tracked_projects(projects_json: &Path) -> Vec<PathBuf> {
    let Ok(s) = std::fs::read_to_string(projects_json) else { return vec![] };
    let Ok(paths) = serde_json::from_str::<Vec<String>>(&s) else { return vec![] };
    paths.into_iter().map(PathBuf::from).collect()
}

pub(crate) struct ClaudeCodeAssistant;

impl Assistant for ClaudeCodeAssistant {
    fn id(&self) -> &str { "claude-code" }
    fn name(&self) -> &str { "Claude Code" }
    fn family(&self) -> &str { "claude" }
    fn family_name(&self) -> &str { "Claude" }
    fn mcp_key(&self) -> &str { "mcpServers" }
    fn config_path(&self) -> PathBuf { home().join(".claude/settings.json") }

    /// Claude Code's plugin install lands five capability areas at once:
    /// plugins (the manifest), skills, commands, agents, and the bundled
    /// sensei MCP entry. They transition together — `claude plugin install`
    /// has no per-part progress — but exposing them as separate chips makes
    /// the wizard reflect what the install actually registered. MCP is on
    /// the list because skills/commands/agents all invoke sensei *through*
    /// MCP; the wizard's Instruments stage handles third-party MCPs only.
    fn parts(&self) -> Vec<AssistantPart> {
        vec![
            AssistantPart { id: "plugins".into(),  label: "plugins".into() },
            AssistantPart { id: "skills".into(),   label: "skills".into() },
            AssistantPart { id: "commands".into(), label: "commands".into() },
            AssistantPart { id: "agents".into(),   label: "agents".into() },
            AssistantPart { id: "mcp".into(),      label: "mcp server".into() },
        ]
    }

    fn detect(&self) -> bool {
        let h = home();
        h.join(".claude/settings.json").exists()
            || h.join(".claude/CLAUDE.md").exists()
            || h.join(".claude").exists()
    }

    /// Claude Code integrates via `claude plugin install sensei`, which records
    /// installed plugins in `~/.claude/plugins/installed_plugins.json` under
    /// the key `sensei@sensei-marketplace`. The settings.json MCP block is not
    /// touched (the plugin bundles MCP, skills, commands, agents, and hooks
    /// together), so the default `check_mcp_in_config` check would always
    /// return false. Override here to look at the plugin manifest.
    fn is_configured(&self) -> bool {
        verify_plugin_installed(&installed_plugins_manifest(), "sensei")
    }

    fn configure(&self, _mcp_cmd: &str) -> Result<AssistantConfigureOk, String> {
        let claude_bin = find_claude_binary()
            .ok_or_else(|| "claude binary not found on PATH".to_string())?;

        let mut warnings = Vec::new();

        // 0. Auto-cleanup of stale state from prior install attempts. The
        //    daemon owns the sensei MCP key (user authorised); any leftover
        //    entry here would either be broken (wrong binary path) or about
        //    to be re-registered by the plugin install below. Sweep it now
        //    so a re-run heals the state without the user editing JSON by
        //    hand.
        let user_mcp = home().join(".claude/mcp.json");
        match clean_sensei_from_mcp_file(&user_mcp) {
            Ok(removed) if !removed.is_empty() => {
                info!(removed = ?removed, "configure: cleaned stale sensei entries from ~/.claude/mcp.json");
            }
            Ok(_)  => {}
            Err(e) => warnings.push(format!("cleanup ~/.claude/mcp.json: {}", e)),
        }

        // 1. Register marketplace. `claude plugin marketplace add` returns
        //    non-zero if the marketplace is already registered — that's not an
        //    error for us. Probe with `marketplace list` first instead of
        //    sniffing stderr for the word "already", which is fragile across
        //    Claude Code releases.
        if !marketplace_registered(&claude_bin, "sensei-marketplace") {
            let add_out = std::process::Command::new(&claude_bin)
                .args(["plugin", "marketplace", "add", SENSEI_MARKETPLACE_REPO, "--scope", "user"])
                .output()
                .map_err(|e| format!("marketplace add: spawn failed: {}", e))?;
            log_subprocess("claude plugin marketplace add", &add_out);
            if !add_out.status.success() {
                return Err(format!("marketplace add: {}", combined_output(&add_out)));
            }
        }

        // 2. Install plugin (skills, commands, agents, marketplace hooks, MCP).
        let install_out = std::process::Command::new(&claude_bin)
            .args(["plugin", "install", "sensei", "--scope", "user"])
            .output()
            .map_err(|e| format!("plugin install: spawn failed: {}", e))?;
        log_subprocess("claude plugin install sensei", &install_out);
        if !install_out.status.success() {
            return Err(format!("plugin install: {}", combined_output(&install_out)));
        }

        // 2b. Post-condition gate. `claude plugin install` has been observed to
        //     return success without actually writing the plugin into the
        //     manifest (network blip, race with another install, silent skip).
        //     Verify by reading installed_plugins.json — if the entry isn't
        //     there, fail loudly with the captured command output so the
        //     operator sees *something* instead of getting a half-installed
        //     state that pretends to be healthy.
        let manifest = installed_plugins_manifest();
        if !verify_plugin_installed(&manifest, "sensei") {
            return Err(format!(
                "plugin install: returned success but {} does not record sensei. \
                 Command output:\n{}",
                manifest.display(),
                combined_output(&install_out),
            ));
        }

        Ok(AssistantConfigureOk { plugin: true, warnings })
    }

    fn remove(&self) -> bool {
        let claude_bin = match find_claude_binary() {
            Some(b) => b,
            None => {
                warn!("remove: claude binary not on PATH; cannot uninstall sensei plugin");
                return false;
            }
        };

        // Uninstall marketplace plugin (skills, commands, agents, marketplace hooks, MCP).
        let uninstall = std::process::Command::new(&claude_bin)
            .args(["plugin", "uninstall", "sensei"])
            .output();
        let plugin_removed = match &uninstall {
            Ok(out) => {
                log_subprocess("claude plugin uninstall sensei", out);
                out.status.success()
            }
            Err(e) => {
                warn!(error = %e, "remove: failed to spawn claude plugin uninstall");
                false
            }
        };

        // Also remove the marketplace registration. Log the result instead of
        // letting `let _ = ...` swallow it silently — if this fails, the
        // marketplace will still appear in `claude plugin marketplace list`
        // even though the plugin is gone, which is confusing on the next
        // install attempt.
        match std::process::Command::new(&claude_bin)
            .args(["plugin", "marketplace", "remove", "sensei-marketplace"])
            .output()
        {
            Ok(out) => log_subprocess("claude plugin marketplace remove sensei-marketplace", &out),
            Err(e) => warn!(error = %e, "remove: failed to spawn claude plugin marketplace remove"),
        }

        // Strip the sensei MCP entry from every place we might have written it.
        // `claude plugin uninstall` only touches `installed_plugins.json`; the
        // plugin's bundled MCP comes back to life on the next install via the
        // plugin manifest, so the actual leak risk is stale entries from
        // pre-plugin installs (`claude mcp add sensei …`) that survive at
        // user scope and at each registered project's `.mcp.json`.
        let home_dir = home();
        let user_mcp = home_dir.join(".claude/mcp.json");
        if let Ok(removed) = clean_sensei_from_mcp_file(&user_mcp)
            && !removed.is_empty()
        {
            info!(removed = ?removed, path = %user_mcp.display(),
                "remove: cleaned sensei from user mcp.json");
        }
        let sensei_dir = crate::paths::sensei_dir();
        for project in read_tracked_projects(&sensei_dir.join("projects.json")) {
            let project_mcp = project.join(".mcp.json");
            if let Ok(removed) = clean_sensei_from_mcp_file(&project_mcp)
                && !removed.is_empty()
            {
                info!(removed = ?removed, path = %project_mcp.display(),
                    "remove: cleaned sensei from project .mcp.json");
            }
        }

        // Scrub legacy `sensei-hook[-dev].ts` references from settings.json,
        // then delete the dispatcher files themselves. These predate the
        // plugin migration and otherwise keep firing on every Claude session.
        let settings = home_dir.join(".claude/settings.json");
        if let Ok(stripped) = clean_legacy_sensei_hooks(&settings)
            && !stripped.is_empty()
        {
            info!(events = ?stripped, "remove: cleaned legacy sensei hook entries from settings.json");
        }
        let deleted = delete_legacy_sensei_hook_files(&home_dir);
        if !deleted.is_empty() {
            info!(files = ?deleted, "remove: deleted legacy sensei hook dispatcher files");
        }

        plugin_removed
    }
}

/// True if `claude plugin marketplace list` has an entry whose name matches
/// `marketplace_name`. Probes the registry before issuing `marketplace add`
/// so a re-run of `configure()` is idempotent without sniffing stderr.
fn marketplace_registered(claude_bin: &Path, marketplace_name: &str) -> bool {
    let Ok(out) = std::process::Command::new(claude_bin)
        .args(["plugin", "marketplace", "list"])
        .output()
    else { return false };
    if !out.status.success() { return false }
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout.lines().any(|l| l.contains(marketplace_name))
}

/// Format both streams of a finished subprocess in a copy-pasteable shape.
/// Includes the exit code so an empty-output failure ("exit code 1, no output")
/// is still actionable rather than blank.
fn combined_output(out: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = stdout.trim();
    let stderr = stderr.trim();
    let code = out.status.code()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "?".into());
    match (stdout.is_empty(), stderr.is_empty()) {
        (true,  true)  => format!("exit={} (no output)", code),
        (true,  false) => format!("exit={} stderr: {}", code, stderr),
        (false, true)  => format!("exit={} stdout: {}", code, stdout),
        (false, false) => format!("exit={} stdout: {} | stderr: {}", code, stdout, stderr),
    }
}

/// Emit tracing events for a subprocess invocation so failed installs leave
/// a breadcrumb in the daemon log. info on success, warn on non-zero exit.
fn log_subprocess(label: &str, out: &std::process::Output) {
    if out.status.success() {
        info!(label = %label, exit = out.status.code().unwrap_or(0), "subprocess ok");
    } else {
        warn!(
            label = %label,
            exit = out.status.code().unwrap_or(-1),
            stdout = %String::from_utf8_lossy(&out.stdout).trim(),
            stderr = %String::from_utf8_lossy(&out.stderr).trim(),
            "subprocess failed",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_tmp_home() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    // ── verify_plugin_installed: the post-condition gate ─────────────────────
    //
    // `claude plugin install` can exit 0 without actually installing the plugin
    // (network hiccup, race with another invocation, silent skip). Our previous
    // configure() trusted the exit code blindly and wrote dev-hook entries
    // assuming success — that's how Jerry's machine ended up with hooks in
    // settings.json but no sensei plugin registered. The fix is to read
    // installed_plugins.json after the install and confirm the plugin is
    // actually there before declaring success.

    #[test]
    fn verify_plugin_installed_true_when_sensei_recorded() {
        let tmp = make_tmp_home();
        let manifest = tmp.path().join("installed_plugins.json");
        std::fs::write(&manifest, r#"{
            "version": 2,
            "plugins": {
                "feature-dev@claude-plugins-official": [],
                "sensei@sensei-marketplace": [
                    { "scope": "user", "installPath": "/foo", "version": "0.2.13" }
                ]
            }
        }"#).unwrap();
        assert!(verify_plugin_installed(&manifest, "sensei"));
    }

    #[test]
    fn verify_plugin_installed_false_when_manifest_missing() {
        let tmp = make_tmp_home();
        let manifest = tmp.path().join("nope.json");
        assert!(!verify_plugin_installed(&manifest, "sensei"));
    }

    #[test]
    fn verify_plugin_installed_false_when_no_plugins_key() {
        let tmp = make_tmp_home();
        let manifest = tmp.path().join("installed_plugins.json");
        std::fs::write(&manifest, r#"{ "version": 2 }"#).unwrap();
        assert!(!verify_plugin_installed(&manifest, "sensei"));
    }

    #[test]
    fn verify_plugin_installed_false_when_sensei_absent() {
        let tmp = make_tmp_home();
        let manifest = tmp.path().join("installed_plugins.json");
        std::fs::write(&manifest, r#"{
            "version": 2,
            "plugins": {
                "feature-dev@claude-plugins-official": [],
                "playwright@claude-plugins-official": []
            }
        }"#).unwrap();
        assert!(!verify_plugin_installed(&manifest, "sensei"));
    }

    #[test]
    fn verify_plugin_installed_false_when_json_malformed() {
        let tmp = make_tmp_home();
        let manifest = tmp.path().join("installed_plugins.json");
        std::fs::write(&manifest, "{ not really json").unwrap();
        assert!(!verify_plugin_installed(&manifest, "sensei"));
    }

    #[test]
    fn verify_plugin_installed_matches_on_at_prefix_not_substring() {
        // A plugin called "not-sensei@..." must not be confused with sensei.
        let tmp = make_tmp_home();
        let manifest = tmp.path().join("installed_plugins.json");
        std::fs::write(&manifest, r#"{
            "version": 2,
            "plugins": {
                "not-sensei@some-marketplace": []
            }
        }"#).unwrap();
        assert!(!verify_plugin_installed(&manifest, "sensei"));
    }

    // ── clean_sensei_from_mcp_file: auto-cleanup of stale sensei MCP entries ────────
    //
    // ~/.claude/mcp.json accumulates broken `sensei` entries from prior
    // install attempts (e.g. command="bun /old/path" pointing at a moved
    // repo). configure() runs cleanup_stale() first so a re-run heals these
    // without the user having to edit JSON by hand. The daemon owns the
    // `sensei` MCP key, so removing any entry under that key is authorised.

    #[test]
    fn clean_sensei_from_mcp_file_removes_sensei_entry() {
        let tmp = make_tmp_home();
        let mcp = tmp.path().join("mcp.json");
        std::fs::write(&mcp, r#"{
            "mcpServers": {
                "sensei":    { "command": "sensei-mcp" },
                "playwright": { "command": "npx", "args": ["@playwright/mcp@latest"] }
            }
        }"#).unwrap();

        let removed = clean_sensei_from_mcp_file(&mcp).unwrap();
        assert_eq!(removed.iter().map(|s| s.as_str()).collect::<Vec<_>>(), vec!["sensei"]);

        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&mcp).unwrap()).unwrap();
        assert!(v["mcpServers"]["sensei"].is_null(), "sensei key should be gone");
        assert!(v["mcpServers"]["playwright"].is_object(), "playwright key must survive");
    }

    #[test]
    fn clean_sensei_from_mcp_file_writes_backup_before_editing() {
        let tmp = make_tmp_home();
        let mcp = tmp.path().join("mcp.json");
        let original = r#"{"mcpServers":{"sensei":{"command":"sensei-mcp"}}}"#;
        std::fs::write(&mcp, original).unwrap();

        clean_sensei_from_mcp_file(&mcp).unwrap();

        let backup = mcp.with_extension("json.bak");
        assert!(backup.exists(), ".bak must be written before any destructive edit");
        let backup_content = std::fs::read_to_string(&backup).unwrap();
        assert_eq!(backup_content, original, "backup must mirror the file as it was before edit");
    }

    #[test]
    fn clean_sensei_from_mcp_file_no_op_when_no_sensei_entries() {
        let tmp = make_tmp_home();
        let mcp = tmp.path().join("mcp.json");
        std::fs::write(&mcp, r#"{"mcpServers":{"playwright":{"command":"npx"}}}"#).unwrap();

        let removed = clean_sensei_from_mcp_file(&mcp).unwrap();
        assert!(removed.is_empty());
        // No backup when nothing changed
        let backup = mcp.with_extension("json.bak");
        assert!(!backup.exists(), "no .bak should be written when no edits happened");
    }

    #[test]
    fn clean_sensei_from_mcp_file_no_op_when_file_missing() {
        let tmp = make_tmp_home();
        let mcp = tmp.path().join("does-not-exist.json");
        let removed = clean_sensei_from_mcp_file(&mcp).unwrap();
        assert!(removed.is_empty());
    }

    // ── clean_legacy_sensei_hooks: scrub pre-plugin sensei-hook[-dev].ts entries
    //
    // Older daemon versions installed hooks directly into ~/.claude/settings.json
    // pointing at ~/.claude/hooks/sensei-hook(-dev).ts. `claude plugin uninstall`
    // doesn't touch settings.json, so those entries kept firing on every Claude
    // session after the plugin migration. These tests pin the cleanup behaviour.

    #[test]
    fn clean_legacy_sensei_hooks_strips_dev_ts_entries() {
        let tmp = make_tmp_home();
        let settings = tmp.path().join("settings.json");
        std::fs::write(&settings, r#"{
            "hooks": {
                "SessionStart": [{
                    "hooks": [
                        { "type": "command", "command": "/Users/Jerry/.claude/hooks/sensei-hook-dev.ts" }
                    ]
                }],
                "PreToolUse": [{
                    "hooks": [
                        { "type": "command", "command": "/some/other/hook.sh" }
                    ]
                }]
            }
        }"#).unwrap();

        let stripped = clean_legacy_sensei_hooks(&settings).unwrap();
        assert_eq!(stripped, vec!["SessionStart".to_string()]);

        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        assert!(v["hooks"]["SessionStart"].is_null(),
            "SessionStart should be removed entirely (had only legacy hook)");
        assert!(v["hooks"]["PreToolUse"].is_array(),
            "PreToolUse should survive (no legacy entries)");
    }

    #[test]
    fn clean_legacy_sensei_hooks_strips_release_ts_too() {
        let tmp = make_tmp_home();
        let settings = tmp.path().join("settings.json");
        std::fs::write(&settings, r#"{
            "hooks": {
                "PostToolUse": [{
                    "hooks": [
                        { "type": "command", "command": "/Users/Jerry/.claude/hooks/sensei-hook.ts" }
                    ]
                }]
            }
        }"#).unwrap();

        let stripped = clean_legacy_sensei_hooks(&settings).unwrap();
        assert_eq!(stripped, vec!["PostToolUse".to_string()]);
    }

    #[test]
    fn clean_legacy_sensei_hooks_preserves_unrelated_hooks_in_same_group() {
        // A matcher group with both a legacy entry and a user-authored hook
        // should keep the user hook and drop only the legacy one.
        let tmp = make_tmp_home();
        let settings = tmp.path().join("settings.json");
        std::fs::write(&settings, r#"{
            "hooks": {
                "SessionStart": [{
                    "hooks": [
                        { "type": "command", "command": "/Users/Jerry/.claude/hooks/sensei-hook-dev.ts" },
                        { "type": "command", "command": "/Users/Jerry/.claude/hooks/my-custom.sh" }
                    ]
                }]
            }
        }"#).unwrap();

        let stripped = clean_legacy_sensei_hooks(&settings).unwrap();
        assert_eq!(stripped, vec!["SessionStart".to_string()]);

        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        let groups = v["hooks"]["SessionStart"].as_array().expect("SessionStart group survives");
        assert_eq!(groups.len(), 1);
        let inner = groups[0]["hooks"].as_array().unwrap();
        assert_eq!(inner.len(), 1, "user hook survives");
        assert_eq!(inner[0]["command"], "/Users/Jerry/.claude/hooks/my-custom.sh");
    }

    #[test]
    fn clean_legacy_sensei_hooks_no_op_when_no_legacy_entries() {
        let tmp = make_tmp_home();
        let settings = tmp.path().join("settings.json");
        let original = r#"{"hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":"/x/y.sh"}]}]}}"#;
        std::fs::write(&settings, original).unwrap();

        let stripped = clean_legacy_sensei_hooks(&settings).unwrap();
        assert!(stripped.is_empty());
        let backup = settings.with_extension("json.bak");
        assert!(!backup.exists(),
            "no .bak should be written when nothing was stripped");
    }

    #[test]
    fn clean_legacy_sensei_hooks_no_op_when_file_missing() {
        let tmp = make_tmp_home();
        let settings = tmp.path().join("nope.json");
        let stripped = clean_legacy_sensei_hooks(&settings).unwrap();
        assert!(stripped.is_empty());
    }

    #[test]
    fn clean_legacy_sensei_hooks_writes_bak_before_destructive_write() {
        let tmp = make_tmp_home();
        let settings = tmp.path().join("settings.json");
        let original = r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"/x/sensei-hook-dev.ts"}]}]}}"#;
        std::fs::write(&settings, original).unwrap();

        clean_legacy_sensei_hooks(&settings).unwrap();

        let backup = settings.with_extension("json.bak");
        assert!(backup.exists(), ".bak must exist after destructive edit");
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), original,
            "backup must mirror original");
    }

    #[test]
    fn clean_legacy_sensei_hooks_leaves_user_named_sensei_hooks_alone() {
        // A user-authored hook with "sensei" anywhere in the path but a
        // basename that doesn't exactly match one of the legacy filenames
        // must NOT be removed — basename match is the only signal.
        let tmp = make_tmp_home();
        let settings = tmp.path().join("settings.json");
        std::fs::write(&settings, r#"{
            "hooks": {
                "SessionStart": [{
                    "hooks": [
                        { "type": "command", "command": "/Users/Jerry/.claude/hooks/my-sensei-helper.ts" }
                    ]
                }]
            }
        }"#).unwrap();

        let stripped = clean_legacy_sensei_hooks(&settings).unwrap();
        assert!(stripped.is_empty(),
            "user-named files containing 'sensei' must not be touched");
    }

    // ── delete_legacy_sensei_hook_files ────────────────────────────────────

    #[test]
    fn delete_legacy_sensei_hook_files_removes_both_variants() {
        let tmp = make_tmp_home();
        let hooks_dir = tmp.path().join(".claude/hooks");
        std::fs::create_dir_all(&hooks_dir).unwrap();
        std::fs::write(hooks_dir.join("sensei-hook-dev.ts"), "x").unwrap();
        std::fs::write(hooks_dir.join("sensei-hook.ts"), "x").unwrap();
        std::fs::write(hooks_dir.join("user-keepme.ts"), "x").unwrap();

        let deleted = delete_legacy_sensei_hook_files(tmp.path());
        assert_eq!(deleted.len(), 2);
        assert!(!hooks_dir.join("sensei-hook-dev.ts").exists());
        assert!(!hooks_dir.join("sensei-hook.ts").exists());
        assert!(hooks_dir.join("user-keepme.ts").exists(),
            "unrelated files must survive");
    }

    #[test]
    fn delete_legacy_sensei_hook_files_no_op_when_dir_missing() {
        let tmp = make_tmp_home();
        let deleted = delete_legacy_sensei_hook_files(tmp.path());
        assert!(deleted.is_empty());
    }

    // ── read_tracked_projects ──────────────────────────────────────────────

    #[test]
    fn read_tracked_projects_parses_string_array() {
        let tmp = make_tmp_home();
        let f = tmp.path().join("projects.json");
        std::fs::write(&f, r#"["/path/one","/path/two"]"#).unwrap();
        let paths = read_tracked_projects(&f);
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], PathBuf::from("/path/one"));
        assert_eq!(paths[1], PathBuf::from("/path/two"));
    }

    #[test]
    fn read_tracked_projects_empty_when_file_missing() {
        let tmp = make_tmp_home();
        let f = tmp.path().join("nope.json");
        assert!(read_tracked_projects(&f).is_empty());
    }

    #[test]
    fn read_tracked_projects_empty_when_wrong_shape() {
        // projects.json is a root array; reject object shapes silently so
        // a corrupted index doesn't abort remove().
        let tmp = make_tmp_home();
        let f = tmp.path().join("bad.json");
        std::fs::write(&f, r#"{"projects":["/x"]}"#).unwrap();
        assert!(read_tracked_projects(&f).is_empty());
    }

    #[test]
    fn read_tracked_projects_empty_when_malformed_json() {
        let tmp = make_tmp_home();
        let f = tmp.path().join("bad.json");
        std::fs::write(&f, "{ not even json").unwrap();
        assert!(read_tracked_projects(&f).is_empty());
    }

}
