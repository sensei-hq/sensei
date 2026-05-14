//! macOS / Linux platform provider backed by Homebrew.

use std::path::Path;
use std::process::Command;

use crate::types::{ComponentState, ComponentStatus};

use super::{InstallRemedy, Platform, PlatformProvider};

// ── Platform-specific utilities ───────────────────────────────────────────────

/// Well-known binary directories on macOS/Linux (Homebrew + system).
/// Used to supplement PATH when launched from an .app bundle.
pub const EXTRA_PATHS: &[&str] = &[
    "/opt/homebrew/bin",
    "/opt/homebrew/sbin",
    "/usr/local/bin",
    "/usr/local/sbin",
];

/// PATH separator character on Unix.
pub const PATH_SEPARATOR: char = ':';

/// Build a colon-separated PATH string that includes the well-known directories.
pub fn enrich_path() -> String {
    let current = std::env::var("PATH").unwrap_or_default();
    let mut parts: Vec<&str> = current.split(PATH_SEPARATOR).collect();
    for extra in EXTRA_PATHS {
        if !parts.contains(extra) {
            parts.push(extra);
        }
    }
    parts.join(&PATH_SEPARATOR.to_string())
}

/// Detect GPU on macOS (via sysctl) or Linux (via lspci).
pub fn detect_gpu() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
            .ok()?;
        let brand = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if brand.contains("Apple") {
            return Some(brand);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("lspci").output().ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if line.contains("VGA") || line.contains("3D") {
                return Some(line.trim().to_string());
            }
        }
    }

    None
}

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
    fn formula_for(name: &str) -> std::borrow::Cow<'static, str> {
        match name {
            "postgresql" | "postgres" => std::borrow::Cow::Borrowed("postgresql@17"),
            "ollama" => std::borrow::Cow::Borrowed("ollama"),
            "daemon" | "senseid" => std::borrow::Cow::Borrowed(crate::config::BREW_TAP),
            _ => std::borrow::Cow::Owned(name.to_string()),
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

        let brew = self.brew_path.as_deref()
            .ok_or_else(|| format!("Homebrew not found — cannot start service {name}"))?;

        let output = Command::new(brew)
            .args(["services", "start", &*formula])
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

        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("brew services start {formula} failed: {stderr}"))
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

    // ── formula_for ───────────────────────────────────────────────────────────

    #[test]
    fn formula_for_known_services() {
        assert_eq!(MacOSProvider::formula_for("postgresql"), "postgresql@17");
        assert_eq!(MacOSProvider::formula_for("postgres"),   "postgresql@17");
        assert_eq!(MacOSProvider::formula_for("ollama"),     "ollama");
        assert_eq!(MacOSProvider::formula_for("daemon"),     crate::config::BREW_TAP);
        assert_eq!(MacOSProvider::formula_for("senseid"),    crate::config::BREW_TAP);
    }

    #[test]
    fn formula_for_unknown_name_falls_through() {
        let name = "my-custom-service";
        assert_eq!(MacOSProvider::formula_for(name), name);
    }

    // ── check_package_manager edge cases ──────────────────────────────────────

    #[test]
    fn check_package_manager_no_brew_path_returns_missing() {
        let provider = MacOSProvider { brew_path: None };
        let status = provider.check_package_manager();
        assert_eq!(status.name, "homebrew");
        assert!(
            status.is_failed(),
            "no brew path should report failed/missing, got: {:?}", status.state
        );
    }

    #[test]
    fn check_package_manager_nonexistent_brew_returns_failed() {
        let provider = MacOSProvider { brew_path: Some("/nonexistent/path/to/brew".to_string()) };
        let status = provider.check_package_manager();
        assert_eq!(status.name, "homebrew");
        assert!(status.is_failed(), "nonexistent brew binary should report failed");
    }

    // ── start_service edge cases ───────────────────────────────────────────────

    #[test]
    fn start_service_no_brew_returns_err() {
        let provider = MacOSProvider { brew_path: None };
        let result = provider.start_service("postgresql");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Homebrew not found"),
            "error should mention Homebrew not found, got: {err}"
        );
    }

    #[test]
    fn start_service_nonexistent_brew_returns_err() {
        let provider = MacOSProvider { brew_path: Some("/nonexistent/brew".to_string()) };
        let result = provider.start_service("postgresql");
        assert!(result.is_err(), "nonexistent brew binary should fail to start service");
    }

    #[test]
    fn start_service_no_brew_for_senseid_returns_err() {
        // The daemon uses formula_for("senseid") → BREW_TAP; no brew → Err, no binary fallback
        // (binary fallback is DaemonFixer's responsibility, not start_service)
        let provider = MacOSProvider { brew_path: None };
        let result = provider.start_service("senseid");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Homebrew not found"));
    }
}
