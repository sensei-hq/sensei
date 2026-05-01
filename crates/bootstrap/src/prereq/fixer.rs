//! Pluggable Fixer strategies for prerequisite remediation.

use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use crate::platform::PlatformProvider;
use crate::util;
use super::FixResult;

/// Pluggable fix strategy.
pub trait Fixer: Send + Sync {
    fn fix(&self) -> Result<FixResult, String>;
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

/// Runs the full database setup pipeline (create db, extensions, migrations).
pub struct DatabaseSetupFixer;

impl Fixer for DatabaseSetupFixer {
    fn fix(&self) -> Result<FixResult, String> {
        crate::database::setup(None).map(|status| {
            FixResult::new(format!(
                "database setup complete: {}",
                status.version.as_deref().unwrap_or("unknown")
            ))
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
}
