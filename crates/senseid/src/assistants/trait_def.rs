use std::path::PathBuf;
use super::helpers::check_mcp_in_config;
use super::AssistantStatus;

/// Result of configuring an Assistant. `plugin` is true when `claude plugin install` succeeded.
pub(crate) struct AssistantConfigureOk {
    pub plugin: bool,
    pub warnings: Vec<String>,
}

/// Each Assistant implements detect, configure, and remove.
pub(crate) trait Assistant {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn mcp_key(&self) -> &str;
    fn config_path(&self) -> PathBuf;
    fn detect(&self) -> bool;
    fn configure(&self, mcp_cmd: &str) -> Result<AssistantConfigureOk, String>;
    fn remove(&self) -> bool;

    /// Family ID for UI grouping. Assistants in the same family show as one card.
    /// Default: same as id (each Assistant is its own family).
    fn family(&self) -> &str { self.id() }

    /// Display name for the family (used when grouped).
    fn family_name(&self) -> &str { self.name() }

    /// Default integration check — looks for the sensei MCP entry in the
    /// assistant's config file. Override for assistants that integrate via
    /// a different mechanism (Claude Code uses `claude plugin install` and
    /// records membership in `~/.claude/plugins/installed_plugins.json`).
    fn is_configured(&self) -> bool {
        check_mcp_in_config(&self.config_path(), self.mcp_key())
    }

    fn status(&self) -> AssistantStatus {
        AssistantStatus {
            id: self.id().to_string(),
            name: self.name().to_string(),
            family: self.family().to_string(),
            installed: self.detect(),
            configured: self.is_configured(),
            config_path: self.config_path().to_string_lossy().into_owned(),
        }
    }
}
