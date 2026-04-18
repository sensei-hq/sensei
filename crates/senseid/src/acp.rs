use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported AI Coding Platforms
const ACP_SPECS: &[AcpSpec] = &[
    AcpSpec { id: "claude-desktop", name: "Claude Desktop", mcp_key: "mcpServers", config_rel: "Library/Application Support/Claude/claude_desktop_config.json" },
    AcpSpec { id: "claude-code",    name: "Claude Code",    mcp_key: "mcpServers", config_rel: ".claude/settings.json" },
    AcpSpec { id: "cursor",         name: "Cursor",         mcp_key: "mcpServers", config_rel: ".cursor/mcp.json" },
    AcpSpec { id: "windsurf",       name: "Windsurf",       mcp_key: "mcpServers", config_rel: ".codeium/windsurf/mcp_config.json" },
    AcpSpec { id: "zed",            name: "Zed",            mcp_key: "mcpServers", config_rel: ".config/zed/settings.json" },
    AcpSpec { id: "kiro",           name: "Kiro",           mcp_key: "mcpServers", config_rel: ".kiro/settings/mcp.json" },
    AcpSpec { id: "opencode",       name: "OpenCode",       mcp_key: "mcp",        config_rel: ".config/opencode/opencode.json" },
    AcpSpec { id: "vscode",         name: "VS Code",        mcp_key: "mcpServers", config_rel: ".vscode/mcp.json" },
];

struct AcpSpec {
    id: &'static str,
    name: &'static str,
    mcp_key: &'static str,
    config_rel: &'static str,
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

/// Check if a binary is on PATH (no shell-out, safe from injection).
fn which_exists(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}

/// Detect which ACPs are installed on this machine.
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

/// Check if sensei MCP is configured in an ACP's config file.
fn check_mcp_configured(config_path: &std::path::Path, mcp_key: &str) -> bool {
    if !config_path.exists() {
        return false;
    }
    std::fs::read_to_string(config_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .map(|v| v[mcp_key]["sensei"].is_object())
        .unwrap_or(false)
}

/// Find the sensei-mcp binary.
fn find_mcp_binary() -> Option<PathBuf> {
    let h = home();
    // Prefer plugin dir (installed by sensei install)
    let plugin_bin = h.join(".claude/plugins/sensei/bin/sensei-mcp");
    if plugin_bin.exists() {
        return Some(plugin_bin);
    }
    // Search common bin dirs
    let search = [
        h.join(".bun/bin/sensei-mcp"),
        h.join(".local/bin/sensei-mcp"),
        PathBuf::from("/opt/homebrew/bin/sensei-mcp"),
        PathBuf::from("/usr/local/bin/sensei-mcp"),
    ];
    search.into_iter().find(|p| p.exists())
}

/// Find the hooks directory.
fn hooks_dir() -> PathBuf {
    home().join(".claude/plugins/sensei/hooks")
}

#[derive(Debug, Serialize)]
pub struct ConfigureResult {
    pub configured: Vec<String>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
}

/// Configure MCP for the given ACPs (or all detected ACPs if empty).
pub fn configure(acp_ids: &[String]) -> ConfigureResult {
    let all_status = detect();
    let h = home();
    let mut result = ConfigureResult {
        configured: vec![],
        skipped: vec![],
        errors: vec![],
    };

    let mcp_bin = match find_mcp_binary() {
        Some(p) => p,
        None => {
            result.errors.push("sensei-mcp binary not found — run sensei install first".into());
            return result;
        }
    };
    let mcp_cmd = mcp_bin.to_string_lossy().to_string();

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
        let config_path = h.join(spec.config_rel);

        // Special handling for Claude Code — also configure hooks
        if status.id == "claude-code" {
            match configure_claude_code(&mcp_cmd) {
                Ok(()) => result.configured.push(status.id.clone()),
                Err(e) => result.errors.push(format!("{}: {}", status.id, e)),
            }
            continue;
        }

        match configure_mcp_file(&config_path, spec.mcp_key, &mcp_cmd, &status.id) {
            Ok(()) => result.configured.push(status.id.clone()),
            Err(e) => result.errors.push(format!("{}: {}", status.id, e)),
        }
    }

    // Save configured ACPs to ~/.sensei/config.json
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

/// Configure Claude Code — MCP server + hooks.
fn configure_claude_code(mcp_cmd: &str) -> Result<(), String> {
    let h = home();

    // 1. MCP in ~/.claude.json (Claude Code's global MCP config)
    let claude_json = h.join(".claude.json");
    let mut config: serde_json::Value = claude_json
        .exists()
        .then(|| std::fs::read_to_string(&claude_json).ok())
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));

    config
        .as_object_mut()
        .ok_or("invalid claude.json")?
        .entry("mcpServers")
        .or_insert(serde_json::json!({}))
        .as_object_mut()
        .ok_or("invalid mcpServers")?
        .insert(
            "sensei".into(),
            serde_json::json!({ "command": mcp_cmd, "args": [] }),
        );
    std::fs::write(&claude_json, serde_json::to_string_pretty(&config).unwrap())
        .map_err(|e| e.to_string())?;

    // 2. Hooks in ~/.claude/hooks.json
    let hooks = hooks_dir();
    let hooks_str = hooks.to_string_lossy();
    let hooks_file = h.join(".claude/hooks.json");
    let hooks_config = serde_json::json!({ "hooks": {
        "SessionStart": [{"matcher": "startup|resume|clear|compact", "hooks": [{"type": "command", "command": format!("{}/run-hook.cmd session-start", hooks_str)}]}],
        "PreToolExecution": [{"matcher": "", "hooks": [{"type": "command", "command": format!("{}/run-hook.cmd pre-tool", hooks_str)}]}],
        "PostToolExecution": [{"matcher": "", "hooks": [{"type": "command", "command": format!("{}/run-hook.cmd post-tool", hooks_str)}]}],
    }});
    std::fs::write(&hooks_file, serde_json::to_string_pretty(&hooks_config).unwrap())
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Write MCP entry to a generic ACP config file.
fn configure_mcp_file(
    config_path: &std::path::Path,
    mcp_key: &str,
    mcp_cmd: &str,
    acp_id: &str,
) -> Result<(), String> {
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut config: serde_json::Value = config_path
        .exists()
        .then(|| std::fs::read_to_string(config_path).ok())
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));

    let entry = if acp_id == "opencode" {
        serde_json::json!({
            "type": "local",
            "command": [mcp_cmd, ""],
            "enabled": true
        })
    } else {
        serde_json::json!({ "command": mcp_cmd, "args": [] })
    };

    config
        .as_object_mut()
        .ok_or("invalid config")?
        .entry(mcp_key)
        .or_insert(serde_json::json!({}))
        .as_object_mut()
        .ok_or("invalid mcp section")?
        .insert("sensei".into(), entry);

    std::fs::write(config_path, serde_json::to_string_pretty(&config).unwrap())
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Remove sensei MCP from all known ACP configs.
pub fn unconfigure() -> Vec<String> {
    let h = home();
    let mut removed = vec![];

    for spec in ACP_SPECS {
        let config_path = h.join(spec.config_rel);
        if !config_path.exists() {
            continue;
        }
        if let Ok(s) = std::fs::read_to_string(&config_path) {
            if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&s) {
                if let Some(servers) = config
                    .get_mut(spec.mcp_key)
                    .and_then(|s| s.as_object_mut())
                {
                    if servers.remove("sensei").is_some() {
                        std::fs::write(
                            &config_path,
                            serde_json::to_string_pretty(&config).unwrap(),
                        )
                        .ok();
                        removed.push(spec.id.to_string());
                    }
                }
            }
        }
    }

    // Also remove Claude Code hooks
    let hooks_file = h.join(".claude/hooks.json");
    if hooks_file.exists() {
        std::fs::remove_file(&hooks_file).ok();
    }

    // Also remove MCP from ~/.claude.json
    let claude_json = h.join(".claude.json");
    if claude_json.exists() {
        if let Ok(s) = std::fs::read_to_string(&claude_json) {
            if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&s) {
                if let Some(servers) = config
                    .get_mut("mcpServers")
                    .and_then(|s| s.as_object_mut())
                {
                    servers.remove("sensei");
                }
                std::fs::write(&claude_json, serde_json::to_string_pretty(&config).unwrap())
                    .ok();
            }
        }
    }

    removed
}
