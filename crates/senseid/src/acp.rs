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

    // 1. Try `claude mcp remove`
    if std::process::Command::new("claude")
        .args(["mcp", "remove", "-s", "user", "sensei"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        removed = true;
    }

    // 2. Fallback: remove from ~/.claude.json
    if !removed {
        if remove_sensei_from_json(&h.join(".claude.json"), "mcpServers") {
            removed = true;
        }
    }

    // 3. Remove hooks
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
