//! Windows platform provider backed by winget.

use crate::types::ComponentStatus;
use crate::util;

use super::{InstallRemedy, Platform, PlatformProvider};

/// Windows provider that delegates to winget.
pub struct WindowsProvider;

impl WindowsProvider {
    /// Create a new Windows provider.
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformProvider for WindowsProvider {
    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn check_package_manager(&self) -> ComponentStatus {
        match util::which_binary("winget") {
            Some(_) => ComponentStatus::ready("winget", "built-in"),
            None => ComponentStatus::failed("winget", "not found in PATH"),
        }
    }

    fn package_manager_name(&self) -> &str {
        "winget"
    }

    fn install_prerequisites(&self) -> Result<(), String> {
        Err("Windows prerequisite installation not yet implemented".to_string())
    }

    fn start_service(&self, name: &str) -> Result<ComponentStatus, String> {
        Err(format!(
            "Windows service management not yet implemented for {name}"
        ))
    }

    fn prereq_install_remedy(&self) -> InstallRemedy {
        InstallRemedy {
            title: "Install missing components".to_string(),
            command: "winget install PostgreSQL.PostgreSQL.17 && winget install Ollama.Ollama"
                .to_string(),
            url: None,
        }
    }

    fn package_manager_remedy(&self) -> InstallRemedy {
        InstallRemedy {
            title: "winget is built into Windows".to_string(),
            command: String::new(),
            url: Some(
                "https://learn.microsoft.com/en-us/windows/package-manager/winget/".to_string(),
            ),
        }
    }
}
