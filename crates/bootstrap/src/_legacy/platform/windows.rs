//! Windows platform provider backed by winget.

use crate::types::ComponentStatus;
use crate::util;

use super::{InstallRemedy, Platform, PlatformProvider};

// ── Platform-specific utilities ───────────────────────────────────────────────

/// No additional PATH directories needed on Windows beyond the defaults.
pub const EXTRA_PATHS: &[&str] = &[];

/// PATH separator character on Windows.
pub const PATH_SEPARATOR: char = ';';

/// Build a semicolon-separated PATH string on Windows.
/// Currently a no-op (no extra directories to inject), but kept symmetric with
/// the macOS implementation for future extension.
pub fn enrich_path() -> String {
    std::env::var("PATH").unwrap_or_default()
}

/// Detect GPU on Windows. Not yet implemented.
pub fn detect_gpu() -> Option<String> {
    None
}

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
