use std::path::PathBuf;
use super::helpers::check_mcp_configured;
use super::AcpStatus;

/// Result of configuring an ACP. `plugin` is true when `claude plugin install` succeeded.
pub(crate) struct AcpConfigureOk {
    pub plugin: bool,
    pub warnings: Vec<String>,
}

/// Each AI Coding Platform implements detect, configure, and remove.
pub(crate) trait Acp {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn mcp_key(&self) -> &str;
    fn config_path(&self) -> PathBuf;
    fn detect(&self) -> bool;
    fn configure(&self, mcp_cmd: &str) -> Result<AcpConfigureOk, String>;
    fn remove(&self) -> bool;

    fn is_configured(&self) -> bool {
        check_mcp_configured(&self.config_path(), self.mcp_key())
    }

    fn status(&self) -> AcpStatus {
        AcpStatus {
            id: self.id().to_string(),
            name: self.name().to_string(),
            installed: self.detect(),
            mcp_configured: self.is_configured(),
            config_path: self.config_path().to_string_lossy().into_owned(),
        }
    }
}
