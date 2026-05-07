//! Pluggable Fixer strategies for prerequisite remediation.

use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use crate::platform::PlatformProvider;
use crate::util;
use super::{FixResult, HumanAction};

/// Pluggable fix strategy.
pub trait Fixer: Send + Sync {
    fn fix(&self) -> Result<FixResult, String>;
    /// If this fixer requires human action before it can proceed, return Some.
    /// The engine stops fix execution and surfaces the action to the user.
    fn human_action(&self) -> Option<HumanAction> { None }
}

/// No-op fixer — always returns Err with the given reason.
/// Use for prerequisites that cannot be auto-installed (e.g. Homebrew itself).
pub struct NoopFixer {
    pub reason: String,
}

impl NoopFixer {
    pub fn new(reason: impl Into<String>) -> Self {
        Self { reason: reason.into() }
    }
}

impl Fixer for NoopFixer {
    fn fix(&self) -> Result<FixResult, String> {
        Err(self.reason.clone())
    }
}

/// Runs `brew install <formula>`.
pub struct BrewFixer {
    pub brew_path: String,
    pub formula: String,
}

impl BrewFixer {
    pub fn new(brew_path: impl Into<String>, formula: impl Into<String>) -> Self {
        Self { brew_path: brew_path.into(), formula: formula.into() }
    }
}

impl Fixer for BrewFixer {
    fn fix(&self) -> Result<FixResult, String> {
        let output = Command::new(&self.brew_path)
            .args(["install", &self.formula])
            .output()
            .map_err(|e| format!("failed to run brew install: {e}"))?;

        if output.status.success() {
            Ok(FixResult::new(format!("brew install {}", self.formula)))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!("brew install {} failed: {stderr}", self.formula))
        }
    }
}

/// Runs `brew upgrade <formula>` first (handles version bump); falls back to
/// `brew install <formula>` for first-time installs. One fixer covers both cases.
pub struct BrewUpgradeFixer {
    pub brew_path: String,
    pub formula: String,
}

impl BrewUpgradeFixer {
    pub fn new(brew_path: impl Into<String>, formula: impl Into<String>) -> Self {
        Self { brew_path: brew_path.into(), formula: formula.into() }
    }
}

impl Fixer for BrewUpgradeFixer {
    fn fix(&self) -> Result<FixResult, String> {
        // Try upgrade first — works when formula is already installed but outdated
        let upgrade = Command::new(&self.brew_path)
            .args(["upgrade", &self.formula])
            .output()
            .map_err(|e| format!("brew upgrade failed to run: {e}"))?;

        if upgrade.status.success() {
            return Ok(FixResult::new(format!("brew upgrade {}", self.formula)));
        }

        // Fall back to install — handles first-time install
        let install = Command::new(&self.brew_path)
            .args(["install", &self.formula])
            .output()
            .map_err(|e| format!("brew install failed to run: {e}"))?;

        if install.status.success() {
            return Ok(FixResult::new(format!("brew install {}", self.formula)));
        }

        let stderr = String::from_utf8_lossy(&install.stderr).trim().to_string();
        Err(format!("brew install {} failed: {stderr}", self.formula))
    }
}

/// Runs `winget install --id <package> -e --silent`.
pub struct WingetFixer {
    pub package: String,
}

impl WingetFixer {
    pub fn new(package: impl Into<String>) -> Self {
        Self { package: package.into() }
    }
}

impl Fixer for WingetFixer {
    fn fix(&self) -> Result<FixResult, String> {
        let output = Command::new("winget")
            .args(["install", "--id", &self.package, "-e", "--silent"])
            .output()
            .map_err(|e| format!("failed to run winget install: {e}"))?;

        if output.status.success() {
            Ok(FixResult::new(format!("winget install {}", self.package)))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!("winget install {} failed: {stderr}", self.package))
        }
    }
}

/// Calls `platform_provider.start_service(name)` then polls the port for up to 30s.
pub struct ServiceStartFixer {
    provider: Arc<dyn PlatformProvider>,
    service_name: String,
    port: u16,
}

impl ServiceStartFixer {
    pub fn new(
        provider: Arc<dyn PlatformProvider>,
        service_name: impl Into<String>,
        port: u16,
    ) -> Self {
        Self { provider, service_name: service_name.into(), port }
    }
}

impl Fixer for ServiceStartFixer {
    fn fix(&self) -> Result<FixResult, String> {
        self.provider
            .start_service(&self.service_name)
            .map_err(|e| format!("failed to start {}: {e}", self.service_name))?;

        // Poll up to 30s for the service to bind its port
        for _ in 0..30 {
            std::thread::sleep(Duration::from_secs(1));
            if util::probe_port(self.port) {
                return Ok(FixResult::new(format!(
                    "started {} on port {}", self.service_name, self.port
                )));
            }
        }
        Err(format!(
            "timed out waiting for {} on port {}", self.service_name, self.port
        ))
    }
}

/// Runs the full database setup pipeline (create db, extensions, dbd deploy).
pub struct DatabaseSetupFixer {
    pub app_version: String,
}

impl DatabaseSetupFixer {
    pub fn new(app_version: impl Into<String>) -> Self {
        Self { app_version: app_version.into() }
    }
}

impl Fixer for DatabaseSetupFixer {
    fn fix(&self) -> Result<FixResult, String> {
        crate::database::setup(&self.app_version).map(|status| {
            FixResult::new(format!(
                "database setup complete: {}",
                status.version.as_deref().unwrap_or("unknown")
            ))
        })
    }
}

/// Fetches the Brewfile from a URL and pipes it to `brew bundle --upgrade --file=-`.
/// This is the single fixer for all binary components — handles both first installs
/// and upgrades in one pass. Idempotent: brew skips already-current formulae.
pub struct BrewBundleFixer {
    pub brew_path:    String,
    pub brewfile_url: String,
}

impl BrewBundleFixer {
    pub fn new(brew_path: impl Into<String>, brewfile_url: impl Into<String>) -> Self {
        Self { brew_path: brew_path.into(), brewfile_url: brewfile_url.into() }
    }
}

impl Fixer for BrewBundleFixer {
    fn fix(&self) -> Result<FixResult, String> {
        // Fetch the Brewfile content from the tap
        let brewfile = reqwest::blocking::get(&self.brewfile_url)
            .map_err(|e| format!("failed to fetch Brewfile from {}: {e}", self.brewfile_url))?
            .error_for_status()
            .map_err(|e| format!("Brewfile URL returned error: {e}"))?
            .text()
            .map_err(|e| format!("failed to read Brewfile response: {e}"))?;

        // Pipe the Brewfile to `brew bundle --upgrade --file=-`
        let mut child = Command::new(&self.brew_path)
            .args(["bundle", "--upgrade", "--file=-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn brew bundle: {e}"))?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(brewfile.as_bytes())
                .map_err(|e| format!("failed to write Brewfile to stdin: {e}"))?;
        }

        let output = child.wait_with_output()
            .map_err(|e| format!("failed to wait for brew bundle: {e}"))?;

        if output.status.success() {
            Ok(FixResult::new("brew bundle --upgrade completed"))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!("brew bundle --upgrade failed: {stderr}"))
        }
    }
}

/// Fixer that cannot auto-fix — surfaces a structured human action instead.
/// Used for: Homebrew missing (cannot auto-install), dev-mode binaries (run `make install-dev`).
pub struct HumanActionFixer {
    pub component_id: &'static str,
    pub title:        &'static str,
    pub command:      &'static str,
    pub url:          Option<&'static str>,
}

impl Fixer for HumanActionFixer {
    fn fix(&self) -> Result<FixResult, String> {
        Err(format!("human action required: {}", self.title))
    }

    fn human_action(&self) -> Option<HumanAction> {
        Some(HumanAction {
            component_id: self.component_id.to_string(),
            title:        self.title.to_string(),
            command:      self.command.to_string(),
            url:          self.url.map(|s| s.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_fixer_returns_err_with_reason() {
        let fixer = NoopFixer::new("manual installation required");
        let result = fixer.fix();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "manual installation required");
    }

    #[test]
    fn brew_fixer_stores_fields() {
        let fixer = BrewFixer::new("/opt/homebrew/bin/brew", "postgresql@17");
        assert_eq!(fixer.formula, "postgresql@17");
        assert_eq!(fixer.brew_path, "/opt/homebrew/bin/brew");
    }

    #[test]
    fn winget_fixer_stores_package() {
        let fixer = WingetFixer::new("PostgreSQL.PostgreSQL");
        assert_eq!(fixer.package, "PostgreSQL.PostgreSQL");
    }

    #[test]
    fn brew_fixer_nonexistent_brew_returns_err() {
        let fixer = BrewFixer::new("/nonexistent/path/to/brew", "postgresql@17");
        let result = fixer.fix();
        assert!(result.is_err(), "should fail when brew binary does not exist");
    }

    #[test]
    fn service_start_fixer_stores_fields() {
        use crate::platform;
        let provider = Arc::from(platform::detect());
        let fixer = ServiceStartFixer::new(provider, "postgresql", 5432);
        assert_eq!(fixer.service_name, "postgresql");
        assert_eq!(fixer.port, 5432);
    }

    #[test]
    fn brew_upgrade_fixer_stores_fields() {
        let fixer = BrewUpgradeFixer::new("/opt/homebrew/bin/brew", "sensei-hq/tap/sensei");
        assert_eq!(fixer.brew_path, "/opt/homebrew/bin/brew");
        assert_eq!(fixer.formula, "sensei-hq/tap/sensei");
    }

    #[test]
    fn brew_upgrade_fixer_nonexistent_brew_returns_err() {
        let fixer = BrewUpgradeFixer::new("/nonexistent/brew", "sensei-hq/tap/sensei");
        let result = fixer.fix();
        assert!(result.is_err(), "should fail when brew binary does not exist");
        let err = result.unwrap_err();
        assert!(
            err.contains("brew upgrade") || err.contains("brew install"),
            "error should mention brew operation, got: {err}"
        );
    }

    #[test]
    fn database_setup_fixer_stores_version() {
        let fixer = DatabaseSetupFixer::new("0.1.0");
        assert_eq!(fixer.app_version, "0.1.0");
    }

    #[test]
    fn brew_bundle_fixer_stores_fields() {
        let f = BrewBundleFixer::new("/opt/homebrew/bin/brew", "https://example.com/Brewfile");
        assert_eq!(f.brew_path, "/opt/homebrew/bin/brew");
        assert_eq!(f.brewfile_url, "https://example.com/Brewfile");
    }

    #[test]
    fn brew_bundle_fixer_nonexistent_brew_returns_err() {
        // Use a URL that will fail quickly (not a real URL, but reqwest will fail)
        let f = BrewBundleFixer::new("/nonexistent/brew", "http://localhost:19999/Brewfile");
        let result = f.fix();
        assert!(result.is_err(), "should fail with unreachable URL or invalid brew path");
    }

    #[test]
    fn human_action_fixer_fix_returns_err() {
        let f = HumanActionFixer {
            component_id: "homebrew",
            title:        "Install Homebrew",
            command:      "/bin/bash install.sh",
            url:          Some("https://brew.sh"),
        };
        assert!(f.fix().is_err(), "HumanActionFixer fix() always returns Err");
    }

    #[test]
    fn human_action_fixer_returns_some_human_action() {
        let f = HumanActionFixer {
            component_id: "homebrew",
            title:        "Install Homebrew",
            command:      "/bin/bash install.sh",
            url:          Some("https://brew.sh"),
        };
        let action = f.human_action();
        assert!(action.is_some());
        let a = action.unwrap();
        assert_eq!(a.component_id, "homebrew");
        assert_eq!(a.command, "/bin/bash install.sh");
        assert_eq!(a.url.as_deref(), Some("https://brew.sh"));
    }

    #[test]
    fn noop_fixer_human_action_returns_none() {
        let f = NoopFixer::new("reason");
        assert!(f.human_action().is_none());
    }
}
