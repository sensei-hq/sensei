use std::path::PathBuf;
use super::trait_def::{Acp, AcpConfigureOk};
use super::helpers::{home, find_claude_binary};

const SENSEI_MARKETPLACE_REPO: &str = "mizukisu/sensei-marketplace";

pub(crate) struct ClaudeCodeAcp;

impl Acp for ClaudeCodeAcp {
    fn id(&self) -> &str { "claude-code" }
    fn name(&self) -> &str { "Claude Code" }
    fn mcp_key(&self) -> &str { "mcpServers" }
    fn config_path(&self) -> PathBuf { home().join(".claude/settings.json") }

    fn detect(&self) -> bool {
        let h = home();
        h.join(".claude/settings.json").exists()
            || h.join(".claude/CLAUDE.md").exists()
            || h.join(".claude").exists()
    }

    fn configure(&self, _mcp_cmd: &str) -> Result<AcpConfigureOk, String> {
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

        // 2. Install plugin (handles commands, skills, hooks, MCP)
        let install_out = std::process::Command::new(&claude_bin)
            .args(["plugin", "install", "sensei", "--scope", "user"])
            .output()
            .map_err(|e| format!("plugin install: {}", e))?;

        if !install_out.status.success() {
            let err = String::from_utf8_lossy(&install_out.stderr).trim().to_string();
            return Err(format!("plugin install: {}", err));
        }

        Ok(AcpConfigureOk { plugin: true, warnings: vec![] })
    }

    fn remove(&self) -> bool {
        let claude_bin = match find_claude_binary() {
            Some(b) => b,
            None => return false,
        };

        let plugin_removed = std::process::Command::new(&claude_bin)
            .args(["plugin", "uninstall", "sensei"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        // Also remove the marketplace registration
        let _ = std::process::Command::new(&claude_bin)
            .args(["plugin", "marketplace", "remove", "sensei-marketplace"])
            .output();

        plugin_removed
    }
}
