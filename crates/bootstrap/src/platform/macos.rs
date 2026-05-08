//! macOS / Linux platform provider backed by Homebrew.

use std::path::Path;
use std::process::Command;

use crate::types::{ComponentState, ComponentStatus};
use crate::util;

use super::{InstallRemedy, Platform, PlatformProvider};

/// macOS / Linux provider that delegates to Homebrew.
pub struct MacOSProvider {
    brew_path: Option<String>,
}

impl Default for MacOSProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MacOSProvider {
    /// Create a new provider, auto-detecting the Homebrew binary location.
    pub fn new() -> Self {
        let brew_path = crate::config::BREW_PATHS
            .iter()
            .find(|p| Path::new(p).exists())
            .map(|p| p.to_string());
        Self { brew_path }
    }

    /// Return the resolved path to the `brew` binary, if found.
    pub fn brew_path(&self) -> Option<&str> {
        self.brew_path.as_deref()
    }

    /// Map a logical service name to the Homebrew formula.
    fn formula_for(name: &str) -> &'static str {
        match name {
            "postgresql" | "postgres" => "postgresql@17",
            "ollama" => "ollama",
            "daemon" | "senseid" => "sensei-hq/tap/sensei",
            _ => Box::leak(name.to_string().into_boxed_str()), // uncommon path
        }
    }
}

impl PlatformProvider for MacOSProvider {
    fn platform(&self) -> Platform {
        #[cfg(target_os = "macos")]
        {
            Platform::MacOS
        }
        #[cfg(target_os = "linux")]
        {
            Platform::Linux
        }
        #[cfg(target_os = "windows")]
        {
            Platform::Windows
        }
    }

    fn check_package_manager(&self) -> ComponentStatus {
        let brew = match &self.brew_path {
            Some(p) => p.as_str(),
            None => {
                return ComponentStatus::missing("homebrew");
            }
        };

        let output = match Command::new(brew).arg("--version").output() {
            Ok(o) => o,
            Err(e) => {
                return ComponentStatus::failed("homebrew", &format!("could not run brew: {e}"));
            }
        };

        if !output.status.success() {
            return ComponentStatus::failed("homebrew", "brew --version returned non-zero");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Typical output: "Homebrew 4.4.2"
        let version = stdout
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("unknown");

        ComponentStatus::ready("homebrew", version)
    }

    fn package_manager_name(&self) -> &str {
        "Homebrew"
    }

    fn start_service(&self, name: &str) -> Result<ComponentStatus, String> {
        let formula = Self::formula_for(name);
        let is_daemon = matches!(name, "daemon" | "senseid");

        if let Some(brew) = &self.brew_path {
            let output = Command::new(brew)
                .args(["services", "start", formula])
                .output()
                .map_err(|e| format!("failed to run brew services start: {e}"))?;

            if output.status.success() {
                return Ok(ComponentStatus {
                    name: name.to_string(),
                    state: ComponentState::Starting,
                    version: None,
                    detail: Some(format!("started via brew services ({})", formula)),
                });
            }

            // For daemon, fall back to direct binary start
            if is_daemon {
                return util::start_daemon(crate::daemon_port());
            }

            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("brew services start {formula} failed: {stderr}"));
        }

        // No brew — only the daemon has a fallback
        if is_daemon {
            return util::start_daemon(crate::daemon_port());
        }

        Err(format!(
            "Homebrew not found — cannot start service {name}"
        ))
    }

    fn prereq_install_remedy(&self) -> InstallRemedy {
        InstallRemedy {
            title: "Install missing components".to_string(),
            command: format!(
                "curl -fsSL {} | brew bundle --file=-",
                crate::config::HOMEBREW_BREWFILE_URL
            ),
            url: Some(crate::config::HOMEBREW_TAP_URL.to_string()),
        }
    }

    fn package_manager_remedy(&self) -> InstallRemedy {
        InstallRemedy {
            title: "Install Homebrew".to_string(),
            command: "/bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
                .to_string(),
            url: Some("https://brew.sh".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_reports_platform() {
        let provider = MacOSProvider::new();
        let p = provider.platform();
        assert!(
            matches!(p, Platform::MacOS | Platform::Linux),
            "MacOSProvider should report MacOS or Linux, got {:?}",
            p
        );
    }

    #[test]
    fn check_package_manager_returns_status() {
        let provider = MacOSProvider::new();
        let status = provider.check_package_manager();
        assert_eq!(status.name, "homebrew");
    }

    #[test]
    fn package_manager_name_is_homebrew() {
        let provider = MacOSProvider::new();
        assert_eq!(provider.package_manager_name(), "Homebrew");
    }

    #[test]
    fn prereq_remedy_contains_brewfile_url() {
        let provider = MacOSProvider::new();
        let remedy = provider.prereq_install_remedy();
        assert!(
            remedy.command.contains("homebrew-tap"),
            "prereq remedy command should reference homebrew-tap"
        );
        assert!(
            remedy.url.as_deref().unwrap().contains("homebrew-tap"),
            "prereq remedy url should reference homebrew-tap"
        );
    }

    #[test]
    fn package_manager_remedy_contains_brew_sh() {
        let provider = MacOSProvider::new();
        let remedy = provider.package_manager_remedy();
        assert!(
            remedy.url.as_deref().unwrap().contains("brew.sh"),
            "package manager remedy url should reference brew.sh"
        );
    }
}
