use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported AI Coding Platforms
const ACP_SPECS: &[AcpSpec] = &[
    AcpSpec { id: "claude-desktop", name: "Claude Desktop", mcp_key: "mcpServers", config_rel: "Library/Application Support/Claude/claude_desktop_config.json", adapter: AcpAdapter::McpFile },
    AcpSpec { id: "claude-code",    name: "Claude Code",    mcp_key: "mcpServers", config_rel: ".claude/settings.json", adapter: AcpAdapter::ClaudeCode },
    AcpSpec { id: "cursor",         name: "Cursor",         mcp_key: "mcpServers", config_rel: ".cursor/mcp.json",      adapter: AcpAdapter::McpFile },
    AcpSpec { id: "windsurf",       name: "Windsurf",       mcp_key: "mcpServers", config_rel: ".codeium/windsurf/mcp_config.json", adapter: AcpAdapter::McpFile },
    AcpSpec { id: "zed",            name: "Zed",            mcp_key: "mcpServers", config_rel: ".config/zed/settings.json", adapter: AcpAdapter::McpFile },
    AcpSpec { id: "kiro",           name: "Kiro",           mcp_key: "mcpServers", config_rel: ".kiro/settings/mcp.json",   adapter: AcpAdapter::McpFile },
    AcpSpec { id: "opencode",       name: "OpenCode",       mcp_key: "mcp",        config_rel: ".config/opencode/opencode.json", adapter: AcpAdapter::OpenCode },
    AcpSpec { id: "vscode",         name: "VS Code",        mcp_key: "mcpServers", config_rel: ".vscode/mcp.json",      adapter: AcpAdapter::McpFile },
];

struct AcpSpec {
    id: &'static str,
    name: &'static str,
    mcp_key: &'static str,
    config_rel: &'static str,
    adapter: AcpAdapter,
}

/// Each ACP has an adapter that knows how to configure and unconfigure.
#[derive(Clone, Copy)]
enum AcpAdapter {
    /// Claude Code: uses `claude` CLI for plugin/mcp/hooks
    ClaudeCode,
    /// Generic: writes sensei entry to a JSON MCP config file
    McpFile,
    /// OpenCode: different MCP entry format
    OpenCode,
}

impl AcpAdapter {
    fn configure(&self, spec: &AcpSpec, mcp_cmd: &str) -> Result<(), String> {
        match self {
            AcpAdapter::ClaudeCode => adapter_claude_code_configure(mcp_cmd),
            AcpAdapter::McpFile => adapter_mcp_file_configure(spec, mcp_cmd),
            AcpAdapter::OpenCode => adapter_opencode_configure(spec, mcp_cmd),
        }
    }

    fn unconfigure(&self, spec: &AcpSpec) -> bool {
        match self {
            AcpAdapter::ClaudeCode => adapter_claude_code_unconfigure(),
            AcpAdapter::McpFile => adapter_mcp_file_unconfigure(spec),
            AcpAdapter::OpenCode => adapter_mcp_file_unconfigure(spec), // same removal logic
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AcpStatus {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub mcp_configured: bool,
    pub config_path: String,
}

fn home() -> PathBuf { crate::paths::home() }

fn which_exists(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}

// ── Detection ───────────────────────────────────────────────────────────────

pub fn detect() -> Vec<AcpStatus> {
    let h = home();

    let detectors: Vec<(&str, bool)> = vec![
        ("claude-desktop", {
            std::path::Path::new("/Applications/Claude.app").exists()
                || h.join("Applications/Claude.app").exists()
        }),
        ("claude-code", {
            h.join(".claude/settings.json").exists()
                || h.join(".claude/CLAUDE.md").exists()
                || h.join(".claude").exists()
        }),
        ("cursor", {
            std::path::Path::new("/Applications/Cursor.app").exists()
                || h.join("Applications/Cursor.app").exists()
                || which_exists("cursor")
        }),
        ("windsurf", {
            std::path::Path::new("/Applications/Windsurf.app").exists()
                || h.join("Applications/Windsurf.app").exists()
                || which_exists("windsurf")
        }),
        ("zed", {
            std::path::Path::new("/Applications/Zed.app").exists()
                || h.join("Applications/Zed.app").exists()
                || h.join(".config/zed/settings.json").exists()
                || which_exists("zed")
        }),
        ("kiro", {
            std::path::Path::new("/Applications/Kiro.app").exists()
                || h.join("Applications/Kiro.app").exists()
                || which_exists("kiro")
        }),
        ("opencode", {
            which_exists("opencode")
                || h.join(".config/opencode/opencode.json").exists()
                || h.join(".local/bin/opencode").is_file()
        }),
        ("vscode", {
            std::path::Path::new("/Applications/Visual Studio Code.app").exists()
                || h.join("Applications/Visual Studio Code.app").exists()
                || which_exists("code")
        }),
    ];

    let detected: std::collections::HashMap<&str, bool> = detectors.into_iter().collect();

    ACP_SPECS
        .iter()
        .map(|spec| {
            let installed = *detected.get(spec.id).unwrap_or(&false);
            let config_path = h.join(spec.config_rel);
            let mcp_configured = check_mcp_configured(&config_path, spec.mcp_key);
            AcpStatus {
                id: spec.id.to_string(),
                name: spec.name.to_string(),
                installed,
                mcp_configured,
                config_path: config_path.to_string_lossy().into_owned(),
            }
        })
        .collect()
}

fn check_mcp_configured(config_path: &std::path::Path, mcp_key: &str) -> bool {
    if !config_path.exists() { return false; }
    std::fs::read_to_string(config_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .map(|v| v[mcp_key]["sensei"].is_object())
        .unwrap_or(false)
}

// ── Configure ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ConfigureResult {
    pub configured: Vec<String>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
}

pub fn configure(acp_ids: &[String]) -> ConfigureResult {
    let all_status = detect();
    let h = home();
    let mut result = ConfigureResult {
        configured: vec![],
        skipped: vec![],
        errors: vec![],
    };

    let mcp_cmd = match find_mcp_binary() {
        Some(p) => p.to_string_lossy().to_string(),
        None => {
            result.errors.push("sensei-mcp not found on PATH".into());
            return result;
        }
    };

    let targets: Vec<&AcpStatus> = if acp_ids.is_empty() {
        all_status.iter().filter(|s| s.installed).collect()
    } else {
        all_status.iter().filter(|s| acp_ids.contains(&s.id)).collect()
    };

    for status in &targets {
        let spec = match ACP_SPECS.iter().find(|s| s.id == status.id) {
            Some(s) => s,
            None => { result.skipped.push(status.id.clone()); continue; }
        };

        match spec.adapter.configure(spec, &mcp_cmd) {
            Ok(()) => result.configured.push(status.id.clone()),
            Err(e) => result.errors.push(format!("{}: {}", status.id, e)),
        }
    }

    // Save configured ACPs
    let sensei_dir = h.join(".sensei");
    std::fs::create_dir_all(&sensei_dir).ok();
    let config_file = sensei_dir.join("config.json");
    let mut config: serde_json::Value = config_file
        .exists()
        .then(|| std::fs::read_to_string(&config_file).ok())
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));
    config["configured_acps"] = serde_json::json!(
        targets.iter().filter(|s| result.configured.contains(&s.id)).map(|s| &s.id).collect::<Vec<_>>()
    );
    std::fs::write(&config_file, serde_json::to_string_pretty(&config).unwrap()).ok();

    result
}

// ── Unconfigure ─────────────────────────────────────────────────────────────

pub fn unconfigure() -> Vec<String> {
    let mut removed = vec![];

    for spec in ACP_SPECS {
        if spec.adapter.unconfigure(spec) {
            removed.push(spec.id.to_string());
        }
    }

    removed
}

// ── Shared helpers ──────────────────────────────────────────────────────────

fn find_mcp_binary() -> Option<PathBuf> {
    if which_exists("sensei-mcp") {
        return Some(PathBuf::from("sensei-mcp"));
    }
    let h = home();
    let search = [
        h.join(".claude/plugins/sensei/bin/sensei-mcp"),
        h.join(".local/bin/sensei-mcp"),
        PathBuf::from("/opt/homebrew/bin/sensei-mcp"),
        PathBuf::from("/usr/local/bin/sensei-mcp"),
    ];
    search.into_iter().find(|p| p.exists())
}

fn find_marketplace_plugin() -> Option<PathBuf> {
    let h = home();
    let candidates = [
        h.join(".sensei/marketplace"),
        PathBuf::from("/opt/homebrew/share/sensei/marketplace"),
        PathBuf::from("/usr/local/share/sensei/marketplace"),
    ];
    candidates.into_iter().find(|p| p.join(".claude-plugin/plugin.json").exists())
}

fn hooks_dir() -> PathBuf {
    home().join(".claude/plugins/sensei/hooks")
}

/// Read a JSON file, remove "sensei" from the object at `mcp_key`, write back.
fn remove_sensei_from_json(path: &std::path::Path, mcp_key: &str) -> bool {
    if !path.exists() { return false; }
    let s = match std::fs::read_to_string(path) { Ok(s) => s, Err(_) => return false };
    let mut v: serde_json::Value = match serde_json::from_str(&s) { Ok(v) => v, Err(_) => return false };
    if let Some(servers) = v.get_mut(mcp_key).and_then(|s| s.as_object_mut()) {
        if servers.remove("sensei").is_some() {
            std::fs::write(path, serde_json::to_string_pretty(&v).unwrap()).ok();
            return true;
        }
    }
    false
}

/// Write sensei MCP entry into a JSON config file at the given key.
fn upsert_sensei_in_json(
    path: &std::path::Path,
    mcp_key: &str,
    entry: serde_json::Value,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut config: serde_json::Value = path
        .exists()
        .then(|| std::fs::read_to_string(path).ok())
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));

    config
        .as_object_mut()
        .ok_or("invalid config")?
        .entry(mcp_key)
        .or_insert(serde_json::json!({}))
        .as_object_mut()
        .ok_or("invalid mcp section")?
        .insert("sensei".into(), entry);

    std::fs::write(path, serde_json::to_string_pretty(&config).unwrap())
        .map_err(|e| e.to_string())
}

// ── Claude Code adapter ─────────────────────────────────────────────────────

fn adapter_claude_code_configure(mcp_cmd: &str) -> Result<(), String> {
    // 1. Try plugin install (handles agents, hooks, MCP, commands, skills)
    if let Some(plugin_path) = find_marketplace_plugin() {
        let status = std::process::Command::new("claude")
            .args(["plugin", "install", &plugin_path.to_string_lossy()])
            .status();
        match status {
            Ok(s) if s.success() => return Ok(()),
            _ => {} // fall through to manual
        }
    }

    // 2. Try `claude mcp add`
    let mcp_added = std::process::Command::new("claude")
        .args(["mcp", "add", "-t", "stdio", "-s", "user", "sensei", "--", mcp_cmd])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    // 3. Fallback: write ~/.claude.json directly
    if !mcp_added {
        let claude_json = home().join(".claude.json");
        upsert_sensei_in_json(&claude_json, "mcpServers", serde_json::json!({"command": mcp_cmd, "args": []}))?;
    }

    // 4. Hooks
    let hooks = hooks_dir();
    let hooks_str = hooks.to_string_lossy();
    let hooks_file = home().join(".claude/hooks.json");
    let hooks_config = serde_json::json!({"hooks": {
        "SessionStart": [{"matcher": "startup|resume|clear|compact", "hooks": [{"type": "command", "command": format!("{}/run-hook.cmd session-start", hooks_str)}]}],
        "PreToolExecution": [{"matcher": "", "hooks": [{"type": "command", "command": format!("{}/run-hook.cmd pre-tool", hooks_str)}]}],
        "PostToolExecution": [{"matcher": "", "hooks": [{"type": "command", "command": format!("{}/run-hook.cmd post-tool", hooks_str)}]}],
    }});
    std::fs::write(&hooks_file, serde_json::to_string_pretty(&hooks_config).unwrap())
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn adapter_claude_code_unconfigure() -> bool {
    let h = home();
    let mut removed = false;

    // 1. Try `claude plugin uninstall` (mirrors plugin install path)
    //    This removes agents, hooks, MCP, commands, and skills installed by the plugin.
    if std::process::Command::new("claude")
        .args(["plugin", "uninstall", "sensei"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        removed = true;
    }

    // 2. Try `claude mcp remove` (covers non-plugin install path)
    if !removed {
        if std::process::Command::new("claude")
            .args(["mcp", "remove", "-s", "user", "sensei"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            removed = true;
        }
    }

    // 3. Fallback: remove from ~/.claude.json
    if !removed {
        if remove_sensei_from_json(&h.join(".claude.json"), "mcpServers") {
            removed = true;
        }
    }

    // 4. Remove hooks (in case plugin uninstall didn't clean them)
    let hooks_file = h.join(".claude/hooks.json");
    if hooks_file.exists() {
        std::fs::remove_file(&hooks_file).ok();
    }

    removed
}

// ── Generic MCP file adapter ────────────────────────────────────────────────

fn adapter_mcp_file_configure(spec: &AcpSpec, mcp_cmd: &str) -> Result<(), String> {
    let config_path = home().join(spec.config_rel);
    let entry = serde_json::json!({"command": mcp_cmd, "args": []});
    upsert_sensei_in_json(&config_path, spec.mcp_key, entry)
}

fn adapter_mcp_file_unconfigure(spec: &AcpSpec) -> bool {
    let config_path = home().join(spec.config_rel);
    remove_sensei_from_json(&config_path, spec.mcp_key)
}

// ── OpenCode adapter ────────────────────────────────────────────────────────

fn adapter_opencode_configure(spec: &AcpSpec, mcp_cmd: &str) -> Result<(), String> {
    let config_path = home().join(spec.config_rel);
    let entry = serde_json::json!({"type": "local", "command": [mcp_cmd, ""], "enabled": true});
    upsert_sensei_in_json(&config_path, spec.mcp_key, entry)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── upsert_sensei_in_json ───────────────────────────────────────────

    #[test]
    fn upsert_creates_file_from_scratch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        let entry = serde_json::json!({"command": "sensei-mcp", "args": []});

        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["sensei"]["command"], "sensei-mcp");
    }

    #[test]
    fn upsert_preserves_existing_servers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, r#"{"mcpServers":{"svelte":{"command":"npx","args":["-y","svelte-mcp"]}}}"#).unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["sensei"]["command"], "sensei-mcp");
        assert_eq!(content["mcpServers"]["svelte"]["command"], "npx");
    }

    #[test]
    fn upsert_overwrites_existing_sensei_entry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, r#"{"mcpServers":{"sensei":{"command":"old-binary"}}}"#).unwrap();

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["sensei"]["command"], "sensei-mcp");
    }

    #[test]
    fn upsert_creates_nested_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("deep/nested/mcp.json");

        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        assert!(path.exists());
    }

    #[test]
    fn upsert_with_different_mcp_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");

        let entry = serde_json::json!({"type": "local", "command": ["sensei-mcp", ""], "enabled": true});
        upsert_sensei_in_json(&path, "mcp", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcp"]["sensei"]["type"], "local");
        assert_eq!(content["mcp"]["sensei"]["enabled"], true);
    }

    // ── remove_sensei_from_json ─────────────────────────────────────────

    #[test]
    fn remove_from_nonexistent_file_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist.json");
        assert!(!remove_sensei_from_json(&path, "mcpServers"));
    }

    #[test]
    fn remove_when_sensei_not_present_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, r#"{"mcpServers":{"svelte":{"command":"npx"}}}"#).unwrap();

        assert!(!remove_sensei_from_json(&path, "mcpServers"));
        // File unchanged
        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"]["svelte"].is_object());
    }

    #[test]
    fn remove_sensei_preserves_other_servers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, r#"{"mcpServers":{"sensei":{"command":"sensei-mcp"},"svelte":{"command":"npx"}}}"#).unwrap();

        assert!(remove_sensei_from_json(&path, "mcpServers"));

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"]["sensei"].is_null());
        assert_eq!(content["mcpServers"]["svelte"]["command"], "npx");
    }

    #[test]
    fn remove_sensei_only_server() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, r#"{"mcpServers":{"sensei":{"command":"sensei-mcp"}}}"#).unwrap();

        assert!(remove_sensei_from_json(&path, "mcpServers"));

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"]["sensei"].is_null());
        assert!(content["mcpServers"].as_object().unwrap().is_empty());
    }

    #[test]
    fn remove_with_opencode_mcp_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        std::fs::write(&path, r#"{"mcp":{"sensei":{"type":"local","command":["sensei-mcp",""]}}}"#).unwrap();

        assert!(remove_sensei_from_json(&path, "mcp"));
    }

    // ── roundtrip: upsert then remove ───────────────────────────────────

    #[test]
    fn roundtrip_upsert_then_remove() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, r#"{"mcpServers":{"svelte":{"command":"npx"}}}"#).unwrap();

        // Install
        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"]["sensei"].is_object());
        assert!(content["mcpServers"]["svelte"].is_object());

        // Uninstall
        assert!(remove_sensei_from_json(&path, "mcpServers"));

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(content["mcpServers"]["sensei"].is_null());
        assert_eq!(content["mcpServers"]["svelte"]["command"], "npx");
    }

    #[test]
    fn roundtrip_fresh_file_upsert_remove() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");

        // Install into new file
        upsert_sensei_in_json(&path, "mcpServers", serde_json::json!({"command": "sensei-mcp"})).unwrap();
        assert!(path.exists());

        // Uninstall
        assert!(remove_sensei_from_json(&path, "mcpServers"));
    }

    // ── AcpAdapter dispatch ─────────────────────────────────────────────

    #[test]
    fn mcp_file_adapter_configure_creates_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_rel = dir.path().join("test-acp/mcp.json");
        let spec = AcpSpec {
            id: "test-acp",
            name: "Test ACP",
            mcp_key: "mcpServers",
            config_rel: "", // not used — we call the helper directly
            adapter: AcpAdapter::McpFile,
        };

        let entry = serde_json::json!({"command": "sensei-mcp", "args": []});
        upsert_sensei_in_json(&config_rel, spec.mcp_key, entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&config_rel).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["sensei"]["command"], "sensei-mcp");

        // Unconfigure
        assert!(remove_sensei_from_json(&config_rel, spec.mcp_key));
        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&config_rel).unwrap()).unwrap();
        assert!(content["mcpServers"]["sensei"].is_null());
    }

    #[test]
    fn opencode_adapter_uses_different_entry_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");

        let entry = serde_json::json!({"type": "local", "command": ["sensei-mcp", ""], "enabled": true});
        upsert_sensei_in_json(&path, "mcp", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcp"]["sensei"]["type"], "local");
        assert_eq!(content["mcp"]["sensei"]["enabled"], true);
        let cmd = content["mcp"]["sensei"]["command"].as_array().unwrap();
        assert_eq!(cmd[0], "sensei-mcp");
    }

    // ── ACP_SPECS consistency ───────────────────────────────────────────

    #[test]
    fn all_acp_specs_have_unique_ids() {
        let mut ids: Vec<&str> = ACP_SPECS.iter().map(|s| s.id).collect();
        let original_len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "duplicate ACP ids found");
    }

    #[test]
    fn all_acp_specs_have_nonempty_fields() {
        for spec in ACP_SPECS {
            assert!(!spec.id.is_empty(), "empty id");
            assert!(!spec.name.is_empty(), "empty name for {}", spec.id);
            assert!(!spec.mcp_key.is_empty(), "empty mcp_key for {}", spec.id);
            assert!(!spec.config_rel.is_empty(), "empty config_rel for {}", spec.id);
        }
    }

    #[test]
    fn claude_code_spec_uses_claude_code_adapter() {
        let spec = ACP_SPECS.iter().find(|s| s.id == "claude-code").unwrap();
        assert!(matches!(spec.adapter, AcpAdapter::ClaudeCode));
    }

    #[test]
    fn opencode_spec_uses_opencode_adapter() {
        let spec = ACP_SPECS.iter().find(|s| s.id == "opencode").unwrap();
        assert!(matches!(spec.adapter, AcpAdapter::OpenCode));
    }

    #[test]
    fn generic_acps_use_mcp_file_adapter() {
        let generic = ["cursor", "windsurf", "zed", "kiro", "vscode", "claude-desktop"];
        for id in generic {
            let spec = ACP_SPECS.iter().find(|s| s.id == id).expect(id);
            assert!(matches!(spec.adapter, AcpAdapter::McpFile), "{} should use McpFile adapter", id);
        }
    }

    // ── Edge cases ──────────────────────────────────────────────────────

    #[test]
    fn upsert_into_invalid_json_file_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, "not json at all").unwrap();

        // serde_json::from_str fails → falls back to empty object
        let entry = serde_json::json!({"command": "sensei-mcp"});
        upsert_sensei_in_json(&path, "mcpServers", entry).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["sensei"]["command"], "sensei-mcp");
    }

    #[test]
    fn remove_from_invalid_json_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, "not json").unwrap();

        assert!(!remove_sensei_from_json(&path, "mcpServers"));
    }

    #[test]
    fn remove_when_mcp_key_missing_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, r#"{"otherKey": {}}"#).unwrap();

        assert!(!remove_sensei_from_json(&path, "mcpServers"));
    }

    // Note: adapter_claude_code_configure/unconfigure call `claude` CLI which
    // can't be safely mocked in parallel unit tests (requires PATH mutation).
    // The CLI invocation behavior is tested via the integration/e2e path.
    // Here we test the fallback paths that operate on JSON files.

    #[test]
    fn claude_code_unconfigure_fallback_removes_from_claude_json() {
        // Simulate the fallback path: claude CLI not available, remove from JSON
        let dir = tempfile::tempdir().unwrap();
        let claude_json = dir.path().join(".claude.json");
        std::fs::write(&claude_json, r#"{"mcpServers":{"sensei":{"command":"sensei-mcp"},"svelte":{"command":"npx"}}}"#).unwrap();

        // Direct test of the shared helper used by the fallback
        assert!(remove_sensei_from_json(&claude_json, "mcpServers"));

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&claude_json).unwrap()).unwrap();
        assert!(content["mcpServers"]["sensei"].is_null());
        assert_eq!(content["mcpServers"]["svelte"]["command"], "npx");
    }

    #[test]
    fn claude_code_configure_fallback_writes_claude_json() {
        // Simulate the fallback path: claude CLI not available, write JSON directly
        let dir = tempfile::tempdir().unwrap();
        let claude_json = dir.path().join(".claude.json");

        upsert_sensei_in_json(&claude_json, "mcpServers", serde_json::json!({"command": "sensei-mcp", "args": []})).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&claude_json).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["sensei"]["command"], "sensei-mcp");
    }

    #[test]
    fn claude_code_configure_fallback_preserves_existing_claude_json() {
        let dir = tempfile::tempdir().unwrap();
        let claude_json = dir.path().join(".claude.json");
        std::fs::write(&claude_json, r#"{"mcpServers":{"svelte":{"command":"npx"}},"other":"data"}"#).unwrap();

        upsert_sensei_in_json(&claude_json, "mcpServers", serde_json::json!({"command": "sensei-mcp"})).unwrap();

        let content: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&claude_json).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["sensei"]["command"], "sensei-mcp");
        assert_eq!(content["mcpServers"]["svelte"]["command"], "npx");
        assert_eq!(content["other"], "data");
    }
}
