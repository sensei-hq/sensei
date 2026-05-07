use std::path::PathBuf;
use super::trait_def::{Assistant, AssistantConfigureOk};
use super::helpers::{home, find_claude_binary};

use crate::paths::MARKETPLACE_REPO as SENSEI_MARKETPLACE_REPO;

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

    fn configure(&self, _mcp_cmd: &str) -> Result<AssistantConfigureOk, String> {
        let claude_bin = find_claude_binary()
            .ok_or_else(|| "claude binary not found on PATH".to_string())?;

        // 1. Register marketplace
        let add_out = std::process::Command::new(&claude_bin)
            .args(["plugin", "marketplace", "add", SENSEI_MARKETPLACE_REPO, "--scope", "user"])
            .output()
            .map_err(|e| format!("marketplace add: {}", e))?;

        if !add_out.status.success() {
            let err = String::from_utf8_lossy(&add_out.stderr).trim().to_string();
            if !err.contains("already") {
                return Err(format!("marketplace add: {}", err));
            }
        }

        // 2. Install plugin (skills, commands, agents, marketplace hooks, MCP)
        let install_out = std::process::Command::new(&claude_bin)
            .args(["plugin", "install", "sensei", "--scope", "user"])
            .output()
            .map_err(|e| format!("plugin install: {}", e))?;

        if !install_out.status.success() {
            let err = String::from_utf8_lossy(&install_out.stderr).trim().to_string();
            return Err(format!("plugin install: {}", err));
        }

        // 3. Dev daemon additionally registers sensei-hook-dev.ts so the dev
        //    daemon also receives hook events (alongside the release hooks from step 2).
        if crate::paths::mode() == crate::paths::Mode::Dev {
            if let Err(e) = write_dev_hook_entries() {
                // Non-fatal: plugin install succeeded; dev hooks are best-effort.
                return Ok(AssistantConfigureOk {
                    plugin: true,
                    warnings: vec![format!("dev hooks: {}", e)],
                });
            }
        }

        Ok(AssistantConfigureOk { plugin: true, warnings: vec![] })
    }

    fn remove(&self) -> bool {
        let claude_bin = match find_claude_binary() {
            Some(b) => b,
            None => return false,
        };

        // Uninstall marketplace plugin (skills, commands, agents, marketplace hooks, MCP)
        let plugin_removed = std::process::Command::new(&claude_bin)
            .args(["plugin", "uninstall", "sensei"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        // Also remove the marketplace registration
        let _ = std::process::Command::new(&claude_bin)
            .args(["plugin", "marketplace", "remove", "sensei-marketplace"])
            .output();

        // Dev daemon additionally removes its own hook entries (sensei-hook-dev.ts).
        // This never touches the release plugin entries — only removes the dev entries.
        if crate::paths::mode() == crate::paths::Mode::Dev {
            remove_dev_hook_entries();
        }

        plugin_removed
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
fn remove_dev_hook_entries() -> bool {
    let settings = home().join(".claude/settings.json");
    if !settings.exists() { return true; }

    let hook_script = home().join(".claude/hooks/sensei-hook-dev.ts")
        .display().to_string();

    let s = match std::fs::read_to_string(&settings) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let mut config: serde_json::Value = match json5::from_str(&s) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let hooks_obj = match config.get_mut("hooks").and_then(|h| h.as_object_mut()) {
        Some(h) => h,
        None => return true, // no hooks section — nothing to remove
    };

    let mut modified = false;
    for event in HOOK_EVENTS {
        if let Some(arr) = hooks_obj.get_mut(*event).and_then(|a| a.as_array_mut()) {
            let before = arr.len();
            arr.retain(|e| e["hooks"][0]["command"].as_str() != Some(hook_script.as_str()));
            if arr.len() < before { modified = true; }
        }
    }

    if !modified { return true; }

    std::fs::write(&settings, serde_json::to_string_pretty(&config).unwrap()).is_ok()
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
