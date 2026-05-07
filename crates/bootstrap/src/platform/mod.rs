//! Platform abstraction layer for bootstrap operations.
//!
//! Each platform provides a [`PlatformProvider`] that knows how to check
//! and install prerequisites using the native package manager.

pub mod macos;
pub mod windows;

use serde::{Deserialize, Serialize};

use crate::types::ComponentStatus;

/// Host operating system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    MacOS,
    Linux,
    Windows,
}

/// A remedy the UI can present to the user — a title, a shell command, and
/// an optional documentation URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallRemedy {
    pub title: String,
    pub command: String,
    pub url: Option<String>,
}

/// Trait implemented by each platform to provide package-manager and
/// service-management operations.
pub trait PlatformProvider: Send + Sync {
    /// Which platform this provider represents.
    fn platform(&self) -> Platform;

    /// Check whether the platform package manager is installed and return its
    /// version.
    fn check_package_manager(&self) -> ComponentStatus;

    /// Human-readable name of the package manager (e.g. "Homebrew", "winget").
    fn package_manager_name(&self) -> &str;

    /// Install all sensei prerequisites using the platform package manager.
    /// Blocks until installation completes.
    fn install_prerequisites(&self) -> Result<(), String>;

    /// Start a service by logical name (e.g. "postgresql", "ollama", "daemon").
    fn start_service(&self, name: &str) -> Result<ComponentStatus, String>;

    /// Remedy shown when prerequisites are missing.
    fn prereq_install_remedy(&self) -> InstallRemedy;

    /// Remedy shown when the package manager itself is missing.
    fn package_manager_remedy(&self) -> InstallRemedy;
}

/// Detect the current platform and return the appropriate provider.
pub fn detect() -> Box<dyn PlatformProvider> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsProvider::new())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Box::new(macos::MacOSProvider::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_provider() {
        let provider = detect();
        let p = provider.platform();
        assert!(
            matches!(p, Platform::MacOS | Platform::Linux | Platform::Windows),
            "detect() should return a provider with a known platform"
        );
    }

    #[test]
    fn provider_has_package_manager_name() {
        let provider = detect();
        let name = provider.package_manager_name();
        assert!(!name.is_empty(), "package_manager_name() must be non-empty");
    }
}
