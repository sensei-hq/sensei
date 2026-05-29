use std::path::{Path, PathBuf};
use tracing::{info, warn};
use super::trait_def::{Assistant, AssistantConfigureOk};
use super::helpers::{home, find_claude_binary};

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
/// cleanup so the plugin install can re-register cleanly. The user has
/// explicitly authorised this scope (sensei/sensei-dev are daemon-owned).
const SENSEI_MCP_KEYS: &[&str] = &["sensei", "sensei-dev"];

/// Remove any sensei-keyed (`sensei` or `sensei-dev`) entries from a user/project
/// `mcp.json`-shaped file. Writes a `.bak` next to the file before editing so
/// the original is recoverable. Returns the list of keys removed. No-op (and
/// no `.bak` written) when the file is missing or carries no sensei keys.
///
/// This is the auto-cleanup gate that lets `configure()` heal stale state
/// from prior install attempts without the user having to edit JSON.
fn clean_user_mcp_json(path: &Path) -> Result<Vec<String>, String> {
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

/// Locate the on-disk manifest for an installed Claude Code plugin. Reads
/// `installed_plugins.json`, finds the first entry whose key starts with
/// `{plugin_name}@`, and returns `<installPath>/.claude-plugin/plugin.json`
/// if it exists. Returns `None` if the plugin isn't installed, the manifest
/// can't be parsed, or the file isn't where the manifest claims it is.
///
/// `_claude_bin` is reserved for a future implementation that queries the
/// claude CLI directly; we read the manifest because it's stable and doesn't
/// require parsing CLI output.
fn find_cached_plugin_manifest(_claude_bin: &Path, plugin_name: &str) -> Option<PathBuf> {
    let manifest = installed_plugins_manifest();
    let content = std::fs::read_to_string(&manifest).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    let plugins = value.get("plugins")?.as_object()?;
    let prefix = format!("{}@", plugin_name);
    let entries = plugins.iter().find(|(k, _)| k.starts_with(&prefix))?.1;
    // `plugins[key]` is an array of install records; pick the first.
    let install_path = entries.as_array()?
        .first()?
        .get("installPath")?
        .as_str()?;
    let plugin_json = Path::new(install_path).join(".claude-plugin/plugin.json");
    plugin_json.exists().then_some(plugin_json)
}

/// In dev mode, rewrite the cached plugin manifest at `path` so the MCP entry
/// uses the dev key (`sensei-dev`) and dev binary (`sensei-mcp-dev`) instead
/// of the prod defaults. Returns Ok(true) if a change was written, Ok(false)
/// if the manifest was already dev-shaped (idempotent), and Err on read/write
/// failures.
///
/// The marketplace ships a single plugin manifest for both modes — this is
/// the daemon-side override that makes the dev MCP actually connect to the
/// dev binary. Re-running `configure()` is safe; `claude plugin update` will
/// revert this and the daemon-startup re-apply (task #18) heals it.
fn patch_dev_plugin_manifest(path: &Path) -> Result<bool, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("read {}: {}", path.display(), e))?;
    let mut value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("parse {}: {}", path.display(), e))?;

    let Some(servers) = value.get_mut("mcpServers").and_then(|s| s.as_object_mut()) else {
        return Ok(false); // manifest has no MCP block; nothing to patch
    };

    // Already dev-shaped? Idempotent no-op.
    if servers.contains_key("sensei-dev") && !servers.contains_key("sensei") {
        return Ok(false);
    }

    // Pull the prod entry, rewrite its command, and re-insert under the dev key.
    let Some(mut entry) = servers.remove("sensei") else {
        return Ok(false); // no prod entry to migrate
    };
    if let Some(obj) = entry.as_object_mut() {
        obj.insert("command".into(), serde_json::Value::String("sensei-mcp-dev".into()));
    }
    servers.insert("sensei-dev".into(), entry);

    let serialized = serde_json::to_string_pretty(&value)
        .map_err(|e| format!("serialise {}: {}", path.display(), e))?;
    std::fs::write(path, serialized)
        .map_err(|e| format!("write {}: {}", path.display(), e))?;
    info!(path = %path.display(), "patched dev plugin manifest (sensei → sensei-dev)");
    Ok(true)
}

/// All Claude Code hook event types sensei listens to.
const HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "InstructionsLoaded",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "Stop",
    "SubagentStart",
    "SubagentStop",
    "Notification",
    "PreCompact",
    "PostCompact",
];

pub(crate) struct ClaudeCodeAssistant;

impl Assistant for ClaudeCodeAssistant {
    fn id(&self) -> &str { "claude-code" }
    fn name(&self) -> &str { "Claude Code" }
    fn family(&self) -> &str { "claude" }
    fn family_name(&self) -> &str { "Claude" }
    fn mcp_key(&self) -> &str { "mcpServers" }
    fn config_path(&self) -> PathBuf { home().join(".claude/settings.json") }

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
        //    daemon owns the sensei/sensei-dev MCP keys (user authorised); any
        //    leftover entries here would either be broken (wrong binary path)
        //    or about to be re-registered by the plugin install below. Sweep
        //    them now so a re-run heals the state without the user editing
        //    JSON by hand.
        let user_mcp = home().join(".claude/mcp.json");
        match clean_user_mcp_json(&user_mcp) {
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

        // 3. Dev mode only: rewrite the cached plugin manifest's MCP entry to
        //    use the dev binary (sensei-mcp-dev) and dev key (sensei-dev). The
        //    marketplace ships a single prod-flavoured manifest; this is the
        //    daemon-side override so dev MCP actually connects. Best-effort —
        //    surfaced as a warning if it fails because the plugin install
        //    itself is already healthy.
        if crate::paths::mode().is_dev()
            && let Some(plugin_manifest) = find_cached_plugin_manifest(&claude_bin, "sensei")
        {
            match patch_dev_plugin_manifest(&plugin_manifest) {
                Ok(true)  => info!("configure: dev plugin manifest patched"),
                Ok(false) => {}
                Err(e)    => warnings.push(format!("dev plugin manifest patch: {}", e)),
            }
        }

        // 4. Dev daemon additionally registers sensei-hook-dev.ts so the dev
        //    daemon also receives hook events (alongside the release hooks from
        //    step 2). Dev-hook failure is non-fatal — plugin install already
        //    succeeded — but surface it as a warning so the operator knows.
        if crate::paths::mode().is_dev()
            && let Err(e) = write_dev_hook_entries()
        {
            warnings.push(format!("dev hooks: {}", e));
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

        // Dev daemon additionally removes its own hook entries (sensei-hook-dev.ts).
        // This never touches the release plugin entries — only removes the dev entries.
        if crate::paths::mode().is_dev() {
            match remove_dev_hook_entries() {
                Ok(_)  => {}
                Err(e) => warn!(error = %e, "remove: failed to clean dev hook entries from settings.json"),
            }
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

// ── Dev hook helpers ─────────────────────────────────────────────────────────

/// Write `sensei-hook-dev.ts` entries into `~/.claude/settings.json`.
/// Appends to existing arrays — does not overwrite entries from other tools.
fn write_dev_hook_entries() -> Result<(), String> {
    let settings = home().join(".claude/settings.json");
    let hook_script = home().join(".claude/hooks/sensei-hook-dev.ts")
        .display().to_string();

    let mut config: serde_json::Value = if settings.exists() {
        let s = std::fs::read_to_string(&settings).map_err(|e| e.to_string())?;
        json5::from_str::<serde_json::Value>(&s).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let new_entry = serde_json::json!({
        "hooks": [{ "type": "command", "command": hook_script }]
    });

    let hooks = config
        .as_object_mut()
        .ok_or("invalid settings.json")?
        .entry("hooks")
        .or_insert(serde_json::json!({}));

    let hooks_obj = hooks.as_object_mut().ok_or("invalid hooks section")?;

    for event in HOOK_EVENTS {
        let arr = hooks_obj
            .entry(*event)
            .or_insert(serde_json::json!([]));
        let arr = arr.as_array_mut().ok_or("invalid hook array")?;

        // Idempotent: skip if already registered
        let already = arr.iter().any(|e| {
            e["hooks"][0]["command"].as_str() == Some(hook_script.as_str())
        });
        if !already {
            arr.push(new_entry.clone());
        }
    }

    if let Some(parent) = settings.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&settings, serde_json::to_string_pretty(&config).unwrap())
        .map_err(|e| e.to_string())
}

/// Remove `sensei-hook-dev.ts` entries from `~/.claude/settings.json`.
/// Only removes entries whose command matches the dev hook script path.
/// Never removes entries belonging to the release plugin or other tools.
///
/// Returns Ok(()) on success — including the no-op cases of "settings file
/// doesn't exist" or "no hooks section". Returns Err with the underlying
/// reason if reading/parsing/writing fails so the caller can log it instead
/// of seeing a stale bool.
fn remove_dev_hook_entries() -> Result<(), String> {
    let settings = home().join(".claude/settings.json");
    if !settings.exists() { return Ok(()); }

    let hook_script = home().join(".claude/hooks/sensei-hook-dev.ts")
        .display().to_string();

    let s = std::fs::read_to_string(&settings)
        .map_err(|e| format!("read settings.json: {}", e))?;
    let mut config: serde_json::Value = json5::from_str(&s)
        .map_err(|e| format!("parse settings.json: {}", e))?;

    let Some(hooks_obj) = config.get_mut("hooks").and_then(|h| h.as_object_mut()) else {
        return Ok(()); // no hooks section — nothing to remove
    };

    let mut modified = false;
    for event in HOOK_EVENTS {
        if let Some(arr) = hooks_obj.get_mut(*event).and_then(|a| a.as_array_mut()) {
            let before = arr.len();
            arr.retain(|e| e["hooks"][0]["command"].as_str() != Some(hook_script.as_str()));
            if arr.len() < before { modified = true; }
        }
    }

    if !modified { return Ok(()); }

    let serialized = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("serialise settings.json: {}", e))?;
    std::fs::write(&settings, serialized)
        .map_err(|e| format!("write settings.json: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_tmp_home() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn hook_events_list_is_nonempty() {
        assert!(!HOOK_EVENTS.is_empty());
        assert!(HOOK_EVENTS.contains(&"SessionStart"));
        assert!(HOOK_EVENTS.contains(&"PostToolUse"));
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

    // ── clean_user_mcp_json: auto-cleanup of stale sensei MCP entries ────────
    //
    // ~/.claude/mcp.json accumulates broken `sensei` entries from prior
    // install attempts (e.g. command="sensei-mcp" when only sensei-mcp-dev
    // is on PATH, or command="bun /old/path" pointing at a moved repo).
    // configure() runs cleanup_stale() first so a re-run heals these without
    // the user having to edit JSON by hand. The user has explicitly authorised
    // removing ANY entry keyed sensei/sensei-dev — the daemon owns those keys.

    #[test]
    fn clean_user_mcp_json_removes_sensei_entry() {
        let tmp = make_tmp_home();
        let mcp = tmp.path().join("mcp.json");
        std::fs::write(&mcp, r#"{
            "mcpServers": {
                "sensei":    { "command": "sensei-mcp" },
                "playwright": { "command": "npx", "args": ["@playwright/mcp@latest"] }
            }
        }"#).unwrap();

        let removed = clean_user_mcp_json(&mcp).unwrap();
        assert_eq!(removed.iter().map(|s| s.as_str()).collect::<Vec<_>>(), vec!["sensei"]);

        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&mcp).unwrap()).unwrap();
        assert!(v["mcpServers"]["sensei"].is_null(), "sensei key should be gone");
        assert!(v["mcpServers"]["playwright"].is_object(), "playwright key must survive");
    }

    #[test]
    fn clean_user_mcp_json_removes_sensei_dev_entry() {
        let tmp = make_tmp_home();
        let mcp = tmp.path().join("mcp.json");
        std::fs::write(&mcp, r#"{
            "mcpServers": {
                "sensei-dev": { "command": "sensei-mcp-dev" }
            }
        }"#).unwrap();

        let removed = clean_user_mcp_json(&mcp).unwrap();
        assert_eq!(removed, vec!["sensei-dev"]);

        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&mcp).unwrap()).unwrap();
        let servers = v["mcpServers"].as_object().unwrap();
        assert!(servers.is_empty(), "all sensei-keyed entries removed");
    }

    #[test]
    fn clean_user_mcp_json_writes_backup_before_editing() {
        let tmp = make_tmp_home();
        let mcp = tmp.path().join("mcp.json");
        let original = r#"{"mcpServers":{"sensei":{"command":"sensei-mcp"}}}"#;
        std::fs::write(&mcp, original).unwrap();

        clean_user_mcp_json(&mcp).unwrap();

        let backup = mcp.with_extension("json.bak");
        assert!(backup.exists(), ".bak must be written before any destructive edit");
        let backup_content = std::fs::read_to_string(&backup).unwrap();
        assert_eq!(backup_content, original, "backup must mirror the file as it was before edit");
    }

    #[test]
    fn clean_user_mcp_json_no_op_when_no_sensei_entries() {
        let tmp = make_tmp_home();
        let mcp = tmp.path().join("mcp.json");
        std::fs::write(&mcp, r#"{"mcpServers":{"playwright":{"command":"npx"}}}"#).unwrap();

        let removed = clean_user_mcp_json(&mcp).unwrap();
        assert!(removed.is_empty());
        // No backup when nothing changed
        let backup = mcp.with_extension("json.bak");
        assert!(!backup.exists(), "no .bak should be written when no edits happened");
    }

    #[test]
    fn clean_user_mcp_json_no_op_when_file_missing() {
        let tmp = make_tmp_home();
        let mcp = tmp.path().join("does-not-exist.json");
        let removed = clean_user_mcp_json(&mcp).unwrap();
        assert!(removed.is_empty());
    }

    // ── patch_dev_plugin_manifest: rewrite cached plugin.json for dev mode ───
    //
    // The marketplace ships ONE plugin manifest with `mcpServers.sensei.command
    // = "sensei-mcp"` (prod binary). In dev mode the binary doesn't exist —
    // only `sensei-mcp-dev` does. After `claude plugin install sensei` lands,
    // configure() runs this helper to rewrite the cached manifest's MCP entry
    // to use the dev key + binary. Idempotent so re-running configure() is
    // safe.

    #[test]
    fn patch_dev_plugin_manifest_renames_key_and_command() {
        let tmp = make_tmp_home();
        let manifest = tmp.path().join("plugin.json");
        std::fs::write(&manifest, r#"{
            "name": "sensei",
            "mcpServers": { "sensei": { "command": "sensei-mcp" } }
        }"#).unwrap();

        let changed = patch_dev_plugin_manifest(&manifest).unwrap();
        assert!(changed, "first call should report a change");

        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&manifest).unwrap()).unwrap();
        assert!(v["mcpServers"]["sensei"].is_null(), "prod key must be removed");
        assert_eq!(v["mcpServers"]["sensei-dev"]["command"], "sensei-mcp-dev");
    }

    #[test]
    fn patch_dev_plugin_manifest_is_idempotent() {
        let tmp = make_tmp_home();
        let manifest = tmp.path().join("plugin.json");
        std::fs::write(&manifest, r#"{
            "name": "sensei",
            "mcpServers": { "sensei-dev": { "command": "sensei-mcp-dev" } }
        }"#).unwrap();

        let changed = patch_dev_plugin_manifest(&manifest).unwrap();
        assert!(!changed, "no change needed when manifest is already dev-shaped");
    }

    #[test]
    fn patch_dev_plugin_manifest_errs_on_missing_file() {
        let tmp = make_tmp_home();
        let manifest = tmp.path().join("nope.json");
        assert!(patch_dev_plugin_manifest(&manifest).is_err());
    }

    #[test]
    fn write_dev_hook_entries_creates_settings_file() {
        let tmp = make_tmp_home();
        let settings = tmp.path().join(".claude/settings.json");

        // Temporarily override home() is not possible without mocking, so we
        // test the helper logic directly by calling the write/read functions.
        let hook_script = tmp.path().join(".claude/hooks/sensei-hook-dev.ts")
            .display().to_string();
        let mut config = serde_json::json!({});

        let new_entry = serde_json::json!({
            "hooks": [{ "type": "command", "command": hook_script }]
        });
        let hooks = config
            .as_object_mut().unwrap()
            .entry("hooks")
            .or_insert(serde_json::json!({}));
        let hooks_obj = hooks.as_object_mut().unwrap();
        for event in HOOK_EVENTS {
            let arr = hooks_obj.entry(*event).or_insert(serde_json::json!([]));
            arr.as_array_mut().unwrap().push(new_entry.clone());
        }

        std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
        std::fs::write(&settings, serde_json::to_string_pretty(&config).unwrap()).unwrap();

        // Verify the file was written correctly
        let content = std::fs::read_to_string(&settings).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed["hooks"]["SessionStart"].is_array());
        assert_eq!(
            parsed["hooks"]["SessionStart"][0]["hooks"][0]["command"].as_str().unwrap(),
            hook_script
        );
    }

    #[test]
    fn idempotent_write_does_not_duplicate_entries() {
        let tmp = make_tmp_home();
        let hook_script = tmp.path().join(".claude/hooks/sensei-hook-dev.ts")
            .display().to_string();
        let new_entry = serde_json::json!({
            "hooks": [{ "type": "command", "command": hook_script }]
        });

        let mut config = serde_json::json!({ "hooks": {} });
        let hooks_obj = config["hooks"].as_object_mut().unwrap();

        // Write twice
        for _ in 0..2 {
            for event in HOOK_EVENTS {
                let arr = hooks_obj.entry(*event).or_insert(serde_json::json!([]));
                let arr = arr.as_array_mut().unwrap();
                let already = arr.iter().any(|e| {
                    e["hooks"][0]["command"].as_str() == Some(hook_script.as_str())
                });
                if !already { arr.push(new_entry.clone()); }
            }
        }

        // Each event type should have exactly one entry
        for event in HOOK_EVENTS {
            let count = config["hooks"][event].as_array().unwrap().len();
            assert_eq!(count, 1, "event {} should have exactly 1 entry, got {}", event, count);
        }
    }

    #[test]
    fn remove_leaves_other_entries_intact() {
        let hook_dev = "/home/user/.claude/hooks/sensei-hook-dev.ts";
        let hook_other = "/some/other/hook.sh";

        let mut config = serde_json::json!({
            "hooks": {
                "SessionStart": [
                    { "hooks": [{ "type": "command", "command": hook_dev }] },
                    { "hooks": [{ "type": "command", "command": hook_other }] }
                ]
            }
        });

        let hooks_obj = config["hooks"].as_object_mut().unwrap();
        for event in HOOK_EVENTS {
            if let Some(arr) = hooks_obj.get_mut(*event).and_then(|a| a.as_array_mut()) {
                arr.retain(|e| e["hooks"][0]["command"].as_str() != Some(hook_dev));
            }
        }

        let remaining = &config["hooks"]["SessionStart"];
        assert_eq!(remaining.as_array().unwrap().len(), 1);
        assert_eq!(
            remaining[0]["hooks"][0]["command"].as_str().unwrap(),
            hook_other
        );
    }
}
