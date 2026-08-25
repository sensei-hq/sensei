use super::AssistantPart;
use super::helpers::{find_claude_binary, home};
use super::trait_def::{Assistant, AssistantConfigureOk};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use crate::assistants::{AdapterCheck, CheckStatus};
use crate::paths::MARKETPLACE_REPO as SENSEI_MARKETPLACE_REPO;

/// Pure helper: read `installed_plugins.json` and confirm that a plugin
/// `{plugin_name}@*` is recorded. Returns false on any read/parse failure
/// or when the entry is absent. Extracted so configure() can re-use it as
/// the post-install verification gate (don't trust `claude plugin install`'s
/// exit code alone — confirm the manifest actually has the plugin).
fn verify_plugin_installed(manifest_path: &Path, plugin_name: &str) -> bool {
    let Some(content) = std::fs::read_to_string(manifest_path).ok() else { return false };
    let Some(value) = serde_json::from_str::<serde_json::Value>(&content).ok() else {
        return false;
    };
    let Some(plugins) = value.get("plugins").and_then(|p| p.as_object()) else { return false };
    let prefix = format!("{}@", plugin_name);
    plugins.keys().any(|k| k.starts_with(&prefix))
}

/// settings.json → enabledPlugins["sensei@sensei-marketplace"] == true.
pub(super) fn check_enabled(settings_path: &Path) -> AdapterCheck {
    let id = "enabled";
    let label = "plugin enabled";
    let Some(content) = std::fs::read_to_string(settings_path).ok() else {
        return AdapterCheck::new(
            id,
            label,
            CheckStatus::Fail,
            Some("settings.json missing".into()),
        );
    };
    let Some(v) = json5::from_str::<serde_json::Value>(&content).ok() else {
        return AdapterCheck::new(
            id,
            label,
            CheckStatus::Unknown,
            Some("settings.json unparseable".into()),
        );
    };
    let enabled = v
        .get("enabledPlugins")
        .and_then(|m| m.get("sensei@sensei-marketplace"))
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    if enabled {
        AdapterCheck::new(id, label, CheckStatus::Ok, None)
    } else {
        AdapterCheck::new(
            id,
            label,
            CheckStatus::Fail,
            Some("enabledPlugins[sensei@sensei-marketplace] != true".into()),
        )
    }
}

/// settings.json → extraKnownMarketplaces["sensei-marketplace"] present.
pub(super) fn check_marketplace(settings_path: &Path) -> AdapterCheck {
    let id = "marketplace";
    let label = "marketplace registered";
    let Some(content) = std::fs::read_to_string(settings_path).ok() else {
        return AdapterCheck::new(
            id,
            label,
            CheckStatus::Fail,
            Some("settings.json missing".into()),
        );
    };
    let Some(v) = json5::from_str::<serde_json::Value>(&content).ok() else {
        return AdapterCheck::new(
            id,
            label,
            CheckStatus::Unknown,
            Some("settings.json unparseable".into()),
        );
    };
    let present =
        v.get("extraKnownMarketplaces").and_then(|m| m.get("sensei-marketplace")).is_some();
    if present {
        AdapterCheck::new(id, label, CheckStatus::Ok, None)
    } else {
        AdapterCheck::new(
            id,
            label,
            CheckStatus::Fail,
            Some("sensei-marketplace not registered".into()),
        )
    }
}

/// installed_plugins.json records sensei (reuses verify_plugin_installed).
pub(super) fn check_plugin(manifest_path: &Path) -> AdapterCheck {
    if verify_plugin_installed(manifest_path, "sensei") {
        AdapterCheck::new("plugin", "plugin installed", CheckStatus::Ok, None)
    } else {
        AdapterCheck::new(
            "plugin",
            "plugin installed",
            CheckStatus::Fail,
            Some("sensei not recorded in installed_plugins.json".into()),
        )
    }
}

/// The plugin's installPath exists and its `.claude-plugin/plugin.json` declares
/// both PreToolUse and PostToolUse hooks. The sensei plugin registers all hooks via
/// the plugin manifest's `hooks` field (it ships no separate hooks/hooks.json), so
/// this reads the same manifest Claude Code loads — otherwise a healthy install
/// false-positive-fails. `install_path` is read from installed_plugins.json.
pub(super) fn check_hooks(install_path: Option<&Path>) -> AdapterCheck {
    let id = "hooks";
    let label = "hooks registered";
    let Some(dir) = install_path else {
        return AdapterCheck::new(
            id,
            label,
            CheckStatus::Fail,
            Some("no plugin installPath recorded".into()),
        );
    };
    if !dir.exists() {
        return AdapterCheck::new(
            id,
            label,
            CheckStatus::Fail,
            Some(format!("installPath missing: {}", dir.display())),
        );
    }
    let manifest = dir.join(".claude-plugin/plugin.json");
    let Some(content) = std::fs::read_to_string(&manifest).ok() else {
        return AdapterCheck::new(
            id,
            label,
            CheckStatus::Fail,
            Some(".claude-plugin/plugin.json missing".into()),
        );
    };
    let Some(v) = json5::from_str::<serde_json::Value>(&content).ok() else {
        return AdapterCheck::new(
            id,
            label,
            CheckStatus::Unknown,
            Some("plugin.json unparseable".into()),
        );
    };
    let hooks = v.get("hooks");
    let has = |evt: &str| hooks.and_then(|h| h.get(evt)).is_some();
    if has("PreToolUse") && has("PostToolUse") {
        AdapterCheck::new(id, label, CheckStatus::Ok, None)
    } else {
        AdapterCheck::new(
            id,
            label,
            CheckStatus::Fail,
            Some("PreToolUse/PostToolUse not declared in plugin.json hooks".into()),
        )
    }
}

/// Read the recorded installPath for `sensei@*` from installed_plugins.json.
pub(super) fn plugin_install_path(manifest_path: &Path) -> Option<PathBuf> {
    let content = std::fs::read_to_string(manifest_path).ok()?;
    let v = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    let plugins = v.get("plugins")?.as_object()?;
    let (_k, entries) = plugins.iter().find(|(k, _)| k.starts_with("sensei@"))?;
    let first = entries.as_array()?.first()?;
    first.get("installPath").and_then(|p| p.as_str()).map(PathBuf::from)
}

/// Read the recorded `version` for `sensei@*` from installed_plugins.json.
/// Mirrors [`plugin_install_path`] but reads the sibling `version` field.
/// Purely diagnostic for `upgrade()` — the post-condition gate is presence
/// (via `verify_plugin_installed`); this just lets the daemon log which
/// version the update landed so an operator can confirm it advanced.
fn plugin_recorded_version(manifest_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(manifest_path).ok()?;
    let v = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    let plugins = v.get("plugins")?.as_object()?;
    let (_k, entries) = plugins.iter().find(|(k, _)| k.starts_with("sensei@"))?;
    let first = entries.as_array()?.first()?;
    first.get("version").and_then(|p| p.as_str()).map(String::from)
}

/// argv for `claude plugin update sensei@sensei-marketplace`. The plugin MUST
/// be referenced with its marketplace qualifier — a bare `sensei` is rejected
/// by Claude Code with "Plugin not found" (verified live: `claude plugin update
/// sensei` fails, `…sensei@sensei-marketplace` succeeds). This matches the
/// `sensei@sensei-marketplace` key used everywhere else in this file
/// (enabledPlugins, installed manifest). Extracted so the exact invocation can
/// be asserted in a unit test without executing it — running the real `claude
/// plugin update` would mutate the installed plugin.
fn plugin_update_args() -> [&'static str; 3] {
    ["plugin", "update", "sensei@sensei-marketplace"]
}

/// Path to Claude Code's `installed_plugins.json` (under the user's home).
fn installed_plugins_manifest() -> PathBuf {
    home().join(".claude/plugins/installed_plugins.json")
}

// ── in-use marker (keep-alive) ───────────────────────────────────────────────
//
// Claude Code's periodic plugin "in-use sweep" prunes any plugin whose
// `<install>/.in_use/<pid>` markers are all stale (no live owning process). The
// daemon installs the plugin out-of-band, so the only marker is the short-lived
// `claude plugin install` process — once it exits the sweep reaps the plugin
// (anthropics/claude-code#69626). We refresh a marker for the long-lived daemon
// process so the sweep always sees a live owner and never prunes it.

/// The exact compact JSON Claude Code writes per marker: `{"pid":N,"procStart":"..."}`.
fn in_use_marker_json(pid: u32, proc_start: &str) -> String {
    format!(r#"{{"pid":{pid},"procStart":"{proc_start}"}}"#)
}

/// Trim `ps -o lstart=` output (right-padded) to the bare asctime string.
/// `None` when empty (process gone / ps failed).
fn clean_lstart(raw: &str) -> Option<String> {
    let t = raw.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// `pid`'s start time formatted exactly as Claude stores it in `.in_use`
/// markers: UTC asctime (`%a %b %e %H:%M:%S %Y`). Obtained via `TZ=UTC ps -o
/// lstart=` so it is byte-identical to what Claude derives for the same pid.
fn proc_start_utc(pid: u32) -> Option<String> {
    let out = std::process::Command::new("ps")
        .env("TZ", "UTC")
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    clean_lstart(&String::from_utf8_lossy(&out.stdout))
}

/// Write/refresh `<install_path>/.in_use/<pid>` so Claude Code's sweep treats
/// the plugin as live.
fn write_in_use_marker(install_path: &Path, pid: u32, proc_start: &str) -> std::io::Result<()> {
    let dir = install_path.join(".in_use");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(pid.to_string()), in_use_marker_json(pid, proc_start))
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
    let original =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    let mut value: serde_json::Value =
        json5::from_str(&original).map_err(|e| format!("parse {}: {}", path.display(), e))?;

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
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| format!("{}.bak", e))
            .unwrap_or_else(|| "bak".into()),
    );
    std::fs::write(&backup, &original)
        .map_err(|e| format!("write backup {}: {}", backup.display(), e))?;

    let serialized = serde_json::to_string_pretty(&value)
        .map_err(|e| format!("serialise {}: {}", path.display(), e))?;
    std::fs::write(path, serialized).map_err(|e| format!("write {}: {}", path.display(), e))?;

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
            let Some(group_obj) = group.as_object_mut() else {
                return true;
            };
            let Some(inner_hooks) = group_obj.get_mut("hooks").and_then(|h| h.as_array_mut())
            else {
                return true;
            };
            let before = inner_hooks.len();
            inner_hooks.retain(|hook| {
                let cmd = hook.get("command").and_then(|c| c.as_str()).unwrap_or("");
                let basename =
                    std::path::Path::new(cmd).file_name().and_then(|n| n.to_str()).unwrap_or("");
                !LEGACY_SENSEI_HOOK_BASENAMES.contains(&basename)
            });
            if inner_hooks.len() != before {
                modified = true;
            }
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
        settings_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!("{}.bak", e))
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

pub(crate) struct ClaudeCodeAssistant;

impl Assistant for ClaudeCodeAssistant {
    fn id(&self) -> &str {
        "claude-code"
    }
    fn name(&self) -> &str {
        "Claude Code"
    }
    fn family(&self) -> &str {
        "claude"
    }
    fn family_name(&self) -> &str {
        "Claude"
    }
    fn mcp_key(&self) -> &str {
        "mcpServers"
    }
    fn config_path(&self) -> PathBuf {
        home().join(".claude/settings.json")
    }

    /// Claude Code's plugin install lands five capability areas at once:
    /// plugins (the manifest), skills, commands, agents, and the bundled
    /// sensei MCP entry. They transition together — `claude plugin install`
    /// has no per-part progress — but exposing them as separate chips makes
    /// the wizard reflect what the install actually registered. MCP is on
    /// the list because skills/commands/agents all invoke sensei *through*
    /// MCP; the wizard's Instruments stage handles third-party MCPs only.
    fn parts(&self) -> Vec<AssistantPart> {
        vec![
            AssistantPart { id: "plugins".into(), label: "plugins".into() },
            AssistantPart { id: "skills".into(), label: "skills".into() },
            AssistantPart { id: "commands".into(), label: "commands".into() },
            AssistantPart { id: "agents".into(), label: "agents".into() },
            AssistantPart { id: "mcp".into(), label: "mcp server".into() },
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

    fn config_health(&self) -> Vec<AdapterCheck> {
        let settings = self.config_path(); // ~/.claude/settings.json
        let manifest = installed_plugins_manifest(); // ~/.claude/plugins/installed_plugins.json
        let install_path = plugin_install_path(&manifest);
        vec![
            check_marketplace(&settings),
            check_plugin(&manifest),
            check_enabled(&settings),
            check_hooks(install_path.as_deref()),
        ]
    }

    /// Refresh the in-use marker for the long-lived daemon process so Claude
    /// Code's plugin sweep keeps the out-of-band install alive
    /// (anthropics/claude-code#69626). No-op if the plugin isn't installed
    /// (the next resolve reinstalls it, then a later tick marks it).
    fn keep_alive(&self) {
        let Some(install_path) = plugin_install_path(&installed_plugins_manifest()) else { return };
        let pid = std::process::id();
        let Some(proc_start) = proc_start_utc(pid) else {
            warn!(
                "keep_alive: could not read daemon process start time; in-use marker not refreshed"
            );
            return;
        };
        if let Err(e) = write_in_use_marker(&install_path, pid, &proc_start) {
            warn!(error = %e, "keep_alive: failed to write plugin in-use marker");
        }
    }

    fn configure(&self, _mcp_cmd: &str) -> Result<AssistantConfigureOk, String> {
        let claude_bin =
            find_claude_binary().ok_or_else(|| "claude binary not found on PATH".to_string())?;

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
            Ok(_) => {}
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

    /// Advance the out-of-band sensei plugin to match a freshly upgraded sensei
    /// binary via `claude plugin update sensei`. Claude Code's plugin is a
    /// copied-out install; a new sensei binary otherwise leaves the old
    /// plugin/MCP behind (the class of bug this fixes). Mirrors `configure()`'s
    /// discipline: don't trust the subprocess exit code — re-read
    /// `installed_plugins.json` and confirm sensei is still recorded before
    /// declaring success.
    fn upgrade(&self) -> Result<AssistantConfigureOk, String> {
        let claude_bin =
            find_claude_binary().ok_or_else(|| "claude binary not found on PATH".to_string())?;

        let update_out = std::process::Command::new(&claude_bin)
            .args(plugin_update_args())
            .output()
            .map_err(|e| format!("plugin update: spawn failed: {}", e))?;
        log_subprocess("claude plugin update sensei@sensei-marketplace", &update_out);
        if !update_out.status.success() {
            return Err(format!("plugin update: {}", combined_output(&update_out)));
        }

        // Post-condition gate — same reused check as configure(). `claude
        // plugin update` can exit 0 without leaving a valid recorded install
        // (partial update, race); confirm the manifest still records sensei
        // before we call the upgrade a success.
        let manifest = installed_plugins_manifest();
        if !verify_plugin_installed(&manifest, "sensei") {
            return Err(format!(
                "plugin update: returned success but {} does not record sensei. \
                 Command output:\n{}",
                manifest.display(),
                combined_output(&update_out),
            ));
        }

        // Diagnostic breadcrumb: log which version the update landed so an
        // operator can confirm it advanced. Presence (above) is the gate.
        if let Some(v) = plugin_recorded_version(&manifest) {
            info!(version = %v, "claude plugin update sensei@sensei-marketplace: manifest records sensei@{}", v);
        }

        Ok(AssistantConfigureOk { plugin: true, warnings: Vec::new() })
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
    let Ok(out) =
        std::process::Command::new(claude_bin).args(["plugin", "marketplace", "list"]).output()
    else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
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
    let code = out.status.code().map(|c| c.to_string()).unwrap_or_else(|| "?".into());
    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => format!("exit={} (no output)", code),
        (true, false) => format!("exit={} stderr: {}", code, stderr),
        (false, true) => format!("exit={} stdout: {}", code, stdout),
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

    // ── in-use marker (keep-alive against Claude Code's plugin sweep) ─────────
    #[test]
    fn in_use_marker_json_matches_claude_compact_format() {
        assert_eq!(
            in_use_marker_json(86189, "Thu Jun 18 14:15:10 2026"),
            r#"{"pid":86189,"procStart":"Thu Jun 18 14:15:10 2026"}"#
        );
    }

    #[test]
    fn clean_lstart_trims_padding_and_rejects_empty() {
        // `ps -o lstart=` right-pads its output; the marker must store the bare string.
        assert_eq!(
            clean_lstart("Thu Jun 18 14:15:10 2026    \n").as_deref(),
            Some("Thu Jun 18 14:15:10 2026")
        );
        assert_eq!(clean_lstart("   "), None);
        assert_eq!(clean_lstart(""), None);
    }

    #[test]
    fn write_in_use_marker_creates_marker_with_exact_contents() {
        let tmp = make_tmp_home();
        let install = tmp.path().join("plugin");
        std::fs::create_dir_all(&install).unwrap();
        write_in_use_marker(&install, 4242, "Thu Jun 18 14:15:10 2026").unwrap();
        let marker = install.join(".in_use/4242");
        assert!(marker.exists(), "marker file not written");
        assert_eq!(
            std::fs::read_to_string(&marker).unwrap(),
            r#"{"pid":4242,"procStart":"Thu Jun 18 14:15:10 2026"}"#
        );
    }

    #[test]
    fn proc_start_utc_returns_asctime_for_current_process() {
        // Live: our own pid exists, so `ps` yields a 5-token asctime
        // (Wday Mon Day HH:MM:SS Year) with no surrounding whitespace.
        let s = proc_start_utc(std::process::id()).expect("own process start time");
        assert_eq!(s, s.trim(), "must be trimmed");
        let toks: Vec<&str> = s.split_whitespace().collect();
        assert_eq!(toks.len(), 5, "expected 5-token asctime, got {s:?}");
        assert_eq!(toks[4].len(), 4, "year should be 4 digits, got {s:?}");
        assert!(toks[4].chars().all(|c| c.is_ascii_digit()), "year not numeric: {s:?}");
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
        std::fs::write(
            &manifest,
            r#"{
            "version": 2,
            "plugins": {
                "feature-dev@claude-plugins-official": [],
                "sensei@sensei-marketplace": [
                    { "scope": "user", "installPath": "/foo", "version": "0.2.13" }
                ]
            }
        }"#,
        )
        .unwrap();
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
        std::fs::write(
            &manifest,
            r#"{
            "version": 2,
            "plugins": {
                "feature-dev@claude-plugins-official": [],
                "playwright@claude-plugins-official": []
            }
        }"#,
        )
        .unwrap();
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
        std::fs::write(
            &manifest,
            r#"{
            "version": 2,
            "plugins": {
                "not-sensei@some-marketplace": []
            }
        }"#,
        )
        .unwrap();
        assert!(!verify_plugin_installed(&manifest, "sensei"));
    }

    // ── clean_sensei_from_mcp_file: auto-cleanup of a stale sensei MCP entry ────────
    //
    // A single stale `sensei` entry can linger in ~/.claude/mcp.json from the
    // pre-plugin era — a path-based `command` (e.g. "bun /old/path") pointing at
    // a moved repo. The MCP key is fixed (`sensei`), so a re-install overwrites
    // it rather than accumulating duplicates; this just removes the dead entry
    // up front. configure() calls clean_sensei_from_mcp_file() first so a re-run
    // heals it without the user editing JSON by hand. The daemon owns the
    // `sensei` MCP key, so removing any entry under that key is authorised.

    #[test]
    fn clean_sensei_from_mcp_file_removes_sensei_entry() {
        let tmp = make_tmp_home();
        let mcp = tmp.path().join("mcp.json");
        std::fs::write(
            &mcp,
            r#"{
            "mcpServers": {
                "sensei":    { "command": "sensei-mcp" },
                "playwright": { "command": "npx", "args": ["@playwright/mcp@latest"] }
            }
        }"#,
        )
        .unwrap();

        let removed = clean_sensei_from_mcp_file(&mcp).unwrap();
        assert_eq!(removed.iter().map(|s| s.as_str()).collect::<Vec<_>>(), vec!["sensei"]);

        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&mcp).unwrap()).unwrap();
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
        assert!(
            v["hooks"]["SessionStart"].is_null(),
            "SessionStart should be removed entirely (had only legacy hook)"
        );
        assert!(
            v["hooks"]["PreToolUse"].is_array(),
            "PreToolUse should survive (no legacy entries)"
        );
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
        let original =
            r#"{"hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":"/x/y.sh"}]}]}}"#;
        std::fs::write(&settings, original).unwrap();

        let stripped = clean_legacy_sensei_hooks(&settings).unwrap();
        assert!(stripped.is_empty());
        let backup = settings.with_extension("json.bak");
        assert!(!backup.exists(), "no .bak should be written when nothing was stripped");
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
        assert_eq!(
            std::fs::read_to_string(&backup).unwrap(),
            original,
            "backup must mirror original"
        );
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
        assert!(stripped.is_empty(), "user-named files containing 'sensei' must not be touched");
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
        assert!(hooks_dir.join("user-keepme.ts").exists(), "unrelated files must survive");
    }

    #[test]
    fn delete_legacy_sensei_hook_files_no_op_when_dir_missing() {
        let tmp = make_tmp_home();
        let deleted = delete_legacy_sensei_hook_files(tmp.path());
        assert!(deleted.is_empty());
    }

    // ── config probes ──────────────────────────────────────────────────
    #[test]
    fn check_enabled_true_when_flag_set() {
        let tmp = make_tmp_home();
        let s = tmp.path().join("settings.json");
        std::fs::write(&s, r#"{"enabledPlugins":{"sensei@sensei-marketplace":true}}"#).unwrap();
        assert_eq!(check_enabled(&s).status, CheckStatus::Ok);
    }
    #[test]
    fn check_enabled_fail_when_false_or_missing() {
        let tmp = make_tmp_home();
        let s = tmp.path().join("settings.json");
        std::fs::write(&s, r#"{"enabledPlugins":{"sensei@sensei-marketplace":false}}"#).unwrap();
        assert_eq!(check_enabled(&s).status, CheckStatus::Fail);
        let s2 = tmp.path().join("missing.json");
        assert_eq!(check_enabled(&s2).status, CheckStatus::Fail);
    }
    #[test]
    fn check_enabled_unknown_when_malformed() {
        let tmp = make_tmp_home();
        let s = tmp.path().join("settings.json");
        // json5 is lenient (accepts `{ not json` partially), so use content no
        // parser will accept.
        std::fs::write(&s, "}{").unwrap();
        assert_eq!(check_enabled(&s).status, CheckStatus::Unknown);
    }
    #[test]
    fn check_marketplace_ok_when_registered() {
        let tmp = make_tmp_home();
        let s = tmp.path().join("settings.json");
        std::fs::write(&s, r#"{"extraKnownMarketplaces":{"sensei-marketplace":{"source":{}}}}"#)
            .unwrap();
        assert_eq!(check_marketplace(&s).status, CheckStatus::Ok);
    }
    #[test]
    fn check_marketplace_fail_when_absent() {
        let tmp = make_tmp_home();
        let s = tmp.path().join("settings.json");
        std::fs::write(&s, r#"{"extraKnownMarketplaces":{}}"#).unwrap();
        assert_eq!(check_marketplace(&s).status, CheckStatus::Fail);
    }
    #[test]
    fn check_marketplace_fail_when_file_missing() {
        let tmp = make_tmp_home();
        assert_eq!(check_marketplace(&tmp.path().join("missing.json")).status, CheckStatus::Fail);
    }
    #[test]
    fn check_plugin_reuses_verify() {
        let tmp = make_tmp_home();
        let m = tmp.path().join("installed_plugins.json");
        std::fs::write(&m, r#"{"plugins":{"sensei@sensei-marketplace":[{"installPath":"/x"}]}}"#)
            .unwrap();
        assert_eq!(check_plugin(&m).status, CheckStatus::Ok);
    }
    #[test]
    fn check_plugin_fail_when_absent() {
        // Wrapping of verify_plugin_installed's false result into Fail.
        let tmp = make_tmp_home();
        let m = tmp.path().join("installed_plugins.json");
        std::fs::write(&m, r#"{"plugins":{"other@mp":[]}}"#).unwrap();
        assert_eq!(check_plugin(&m).status, CheckStatus::Fail);
    }
    #[test]
    fn plugin_install_path_reads_first_entry() {
        let tmp = make_tmp_home();
        let m = tmp.path().join("installed_plugins.json");
        std::fs::write(
            &m,
            r#"{"plugins":{"sensei@sensei-marketplace":[{"installPath":"/foo/bar"}]}}"#,
        )
        .unwrap();
        assert_eq!(plugin_install_path(&m), Some(PathBuf::from("/foo/bar")));
    }
    // ── upgrade: command construction + version readback ─────────────────────
    //
    // The real `claude plugin update sensei@sensei-marketplace` is a side effect
    // (mutates the installed plugin), so it is NOT executed here. The testable
    // parts mirror the install path: the exact argv, and the
    // installed_plugins.json readback that gates/annotates success.

    #[test]
    fn plugin_update_args_target_marketplace_qualified_plugin() {
        // Lock the exact invocation. The marketplace qualifier is REQUIRED — a
        // bare `sensei` is rejected by Claude Code with "Plugin not found"
        // (verified live), so dropping the `@sensei-marketplace` suffix would
        // make every automatic upgrade silently fail.
        assert_eq!(plugin_update_args(), ["plugin", "update", "sensei@sensei-marketplace"]);
    }

    #[test]
    fn upgrade_reuses_verify_plugin_installed_as_gate() {
        // upgrade()'s post-condition gate is the same verify_plugin_installed
        // check configure() uses. Exercise the gate directly on a manifest that
        // records sensei (post-update happy path) vs one that doesn't.
        let tmp = make_tmp_home();
        let manifest = tmp.path().join("installed_plugins.json");
        std::fs::write(
            &manifest,
            r#"{"plugins":{"sensei@sensei-marketplace":[
            {"scope":"user","installPath":"/foo","version":"0.2.14"}
        ]}}"#,
        )
        .unwrap();
        assert!(
            verify_plugin_installed(&manifest, "sensei"),
            "gate must pass when the update left sensei recorded"
        );

        let empty = tmp.path().join("empty.json");
        std::fs::write(&empty, r#"{"plugins":{}}"#).unwrap();
        assert!(
            !verify_plugin_installed(&empty, "sensei"),
            "gate must fail when the update left no sensei record"
        );
    }

    #[test]
    fn plugin_recorded_version_reads_version() {
        let tmp = make_tmp_home();
        let m = tmp.path().join("installed_plugins.json");
        std::fs::write(
            &m,
            r#"{"plugins":{"sensei@sensei-marketplace":[
            {"installPath":"/foo","version":"0.2.14"}
        ]}}"#,
        )
        .unwrap();
        assert_eq!(plugin_recorded_version(&m).as_deref(), Some("0.2.14"));
    }

    #[test]
    fn plugin_recorded_version_none_when_field_or_plugin_absent() {
        let tmp = make_tmp_home();
        let no_field = tmp.path().join("no_field.json");
        std::fs::write(
            &no_field,
            r#"{"plugins":{"sensei@sensei-marketplace":[{"installPath":"/foo"}]}}"#,
        )
        .unwrap();
        assert_eq!(plugin_recorded_version(&no_field), None);

        let no_plugin = tmp.path().join("no_plugin.json");
        std::fs::write(&no_plugin, r#"{"plugins":{"other@mp":[]}}"#).unwrap();
        assert_eq!(plugin_recorded_version(&no_plugin), None);

        let missing = tmp.path().join("missing.json");
        assert_eq!(plugin_recorded_version(&missing), None);
    }

    #[test]
    fn plugin_install_path_none_on_empty_entries() {
        // Plugin key present but no install entry recorded → None (caller treats
        // this the same as "not installed", which is the right escalation).
        let tmp = make_tmp_home();
        let m = tmp.path().join("installed_plugins.json");
        std::fs::write(&m, r#"{"plugins":{"sensei@sensei-marketplace":[]}}"#).unwrap();
        assert_eq!(plugin_install_path(&m), None);
    }
    #[test]
    fn check_hooks_ok_when_declared_in_plugin_manifest() {
        // The real sensei plugin declares hooks in `.claude-plugin/plugin.json`'s
        // `hooks` field and ships NO hooks/hooks.json. The check must read the same
        // manifest Claude Code loads, or it false-positive-fails on a healthy install.
        let tmp = make_tmp_home();
        let dir = tmp.path().join("plugin");
        std::fs::create_dir_all(dir.join(".claude-plugin")).unwrap();
        std::fs::write(
            dir.join(".claude-plugin/plugin.json"),
            r#"{"hooks":{"PreToolUse":[{}],"PostToolUse":[{}],"SessionStart":[{}]}}"#,
        )
        .unwrap();
        assert_eq!(check_hooks(Some(&dir)).status, CheckStatus::Ok);
    }
    #[test]
    fn check_hooks_fail_when_path_missing() {
        assert_eq!(check_hooks(None).status, CheckStatus::Fail);
        let tmp = make_tmp_home();
        assert_eq!(check_hooks(Some(&tmp.path().join("nope"))).status, CheckStatus::Fail);
    }
    #[test]
    fn check_hooks_fail_when_events_absent() {
        let tmp = make_tmp_home();
        let dir = tmp.path().join("plugin");
        std::fs::create_dir_all(dir.join(".claude-plugin")).unwrap();
        std::fs::write(
            dir.join(".claude-plugin/plugin.json"),
            r#"{"hooks":{"SessionStart":[{}]}}"#,
        )
        .unwrap();
        assert_eq!(check_hooks(Some(&dir)).status, CheckStatus::Fail);
    }
}
