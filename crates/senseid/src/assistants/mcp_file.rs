use std::path::PathBuf;
use super::trait_def::{Assistant, AssistantConfigureOk};
use super::helpers::{home, upsert_sensei_in_json, remove_sensei_from_json};

/// Entry format written to the MCP config file.
pub(crate) enum McpEntryFormat {
    /// `{"command": mcp_cmd, "args": []}` — used by most assistants
    Standard,
    /// `{"type": "local", "command": [mcp_cmd, ""], "enabled": true}` — OpenCode
    OpenCode,
}

/// Generic Assistant that reads/writes a JSON MCP config file.
/// Covers Claude Desktop, Cursor, Windsurf, Zed, Kiro, VS Code, and OpenCode.
pub(crate) struct McpFileAssistant {
    pub id: &'static str,
    pub name: &'static str,
    pub family_id: Option<&'static str>,
    pub family_label: Option<&'static str>,
    pub mcp_key: &'static str,
    pub config_rel: &'static str,
    pub entry_format: McpEntryFormat,
    /// App bundle names to check in /Applications and ~/Applications
    pub app_names: &'static [&'static str],
    /// Binary names to check on PATH
    pub bin_names: &'static [&'static str],
    /// Additional paths relative to $HOME to check for detection
    pub home_paths: &'static [&'static str],
}

impl Assistant for McpFileAssistant {
    fn id(&self) -> &str { self.id }
    fn name(&self) -> &str { self.name }
    fn family(&self) -> &str { self.family_id.unwrap_or(self.id) }
    fn family_name(&self) -> &str { self.family_label.unwrap_or(self.name) }
    fn mcp_key(&self) -> &str { self.mcp_key }
    fn config_path(&self) -> PathBuf { home().join(self.config_rel) }

    fn detect(&self) -> bool {
        let h = home();
        for app in self.app_names {
            if std::path::Path::new("/Applications").join(app).exists()
                || h.join("Applications").join(app).exists()
            {
                return true;
            }
        }
        for bin in self.bin_names {
            if sensei_bootstrap::util::which_binary(bin).is_some() { return true; }
        }
        for path in self.home_paths {
            if h.join(path).exists() { return true; }
        }
        false
    }

    fn configure(&self, mcp_cmd: &str) -> Result<AssistantConfigureOk, String> {
        let entry = match self.entry_format {
            McpEntryFormat::Standard => {
                serde_json::json!({"command": mcp_cmd, "args": []})
            }
            McpEntryFormat::OpenCode => {
                serde_json::json!({"type": "local", "command": [mcp_cmd, ""], "enabled": true})
            }
        };
        upsert_sensei_in_json(&self.config_path(), self.mcp_key, entry)?;
        Ok(AssistantConfigureOk { plugin: false, warnings: vec![] })
    }

    fn remove(&self) -> bool {
        remove_sensei_from_json(&self.config_path(), self.mcp_key)
    }
}
