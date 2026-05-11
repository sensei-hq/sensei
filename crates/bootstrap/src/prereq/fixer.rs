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

/// Starts the sensei daemon.
///
/// Strategy:
/// - Production: delegates to `provider.start_service("senseid")` which runs
///   `brew services start sensei-hq/tap/sensei`. Falls back to direct binary
///   invocation if brew services fails or is unavailable.
/// - Dev mode: spawns `senseid-dev start --port` directly (not a brew service).
///
/// After start, polls the daemon port for up to 30 s.
pub struct DaemonFixer {
    provider: Arc<dyn PlatformProvider>,
    port: u16,
    is_dev: bool,
    binary_name: &'static str,
}

impl DaemonFixer {
    pub fn new(
        provider: Arc<dyn PlatformProvider>,
        port: u16,
        is_dev: bool,
        binary_name: &'static str,
    ) -> Self {
        Self { provider, port, is_dev, binary_name }
    }

    fn poll_port(&self, started_via: &str) -> Result<FixResult, String> {
        for _ in 0..30 {
            std::thread::sleep(Duration::from_secs(1));
            if util::probe_port(self.port) {
                return Ok(FixResult::new(format!(
                    "daemon started via {started_via} on port {}",
                    self.port
                )));
            }
        }
        Err(format!("timed out waiting for daemon on port {}", self.port))
    }
}

impl Fixer for DaemonFixer {
    fn fix(&self) -> Result<FixResult, String> {
        // Production: use `brew services start` via the platform provider
        // (same code path as postgresql and ollama — brew services logic lives in macos.rs)
        if !self.is_dev && self.provider.start_service("senseid").is_ok() {
            return self.poll_port("brew services");
        }

        // Dev mode or brew services unavailable/failed: start binary directly
        let binary = util::which_binary(self.binary_name)
            .ok_or_else(|| format!("{} not found in PATH", self.binary_name))?;

        let output = Command::new(&binary)
            .args(["start", "--port", &self.port.to_string()])
            .env("PATH", util::enrich_path())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .map_err(|e| format!("failed to start {}: {e}", self.binary_name))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(format!("{} start failed: {stderr}", self.binary_name));
        }

        self.poll_port(self.binary_name)
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
    fn service_start_fixer_stores_fields() {
        use crate::platform;
        let provider = Arc::from(platform::detect());
        let fixer = ServiceStartFixer::new(provider, "postgresql", 5432);
        assert_eq!(fixer.service_name, "postgresql");
        assert_eq!(fixer.port, 5432);
    }

    #[test]
    fn database_setup_fixer_stores_version() {
        let fixer = DatabaseSetupFixer::new("0.1.0");
        assert_eq!(fixer.app_version, "0.1.0");
    }

    #[test]
    fn daemon_fixer_stores_fields() {
        use crate::platform;
        let provider = Arc::from(platform::detect());
        let fixer = DaemonFixer::new(provider, 7744, false, "senseid");
        assert_eq!(fixer.port, 7744);
        assert!(!fixer.is_dev);
        assert_eq!(fixer.binary_name, "senseid");
    }

    /// Mock platform provider for DaemonFixer tests.
    struct MockProvider { start_ok: bool }

    impl crate::platform::PlatformProvider for MockProvider {
        fn platform(&self) -> crate::platform::Platform { crate::platform::Platform::MacOS }
        fn check_package_manager(&self) -> crate::types::ComponentStatus {
            crate::types::ComponentStatus::ready("homebrew", "4.0")
        }
        fn package_manager_name(&self) -> &str { "Homebrew" }
        fn start_service(&self, _name: &str) -> Result<crate::types::ComponentStatus, String> {
            if self.start_ok {
                Ok(crate::types::ComponentStatus {
                    name:    "senseid".into(),
                    state:   crate::types::ComponentState::Starting,
                    version: None,
                    detail:  None,
                })
            } else {
                Err("mock: brew services failed".into())
            }
        }
        fn prereq_install_remedy(&self) -> crate::platform::InstallRemedy {
            crate::platform::InstallRemedy { title: "t".into(), command: "c".into(), url: None }
        }
        fn package_manager_remedy(&self) -> crate::platform::InstallRemedy {
            crate::platform::InstallRemedy { title: "t".into(), command: "c".into(), url: None }
        }
    }

    #[test]
    fn daemon_fixer_brew_fails_binary_not_found_returns_err() {
        // MockProvider.start_service returns Err → DaemonFixer falls back to binary.
        // "senseid" is unlikely to be in PATH during unit tests → Err "not found in PATH".
        if crate::util::which_binary("senseid").is_some() {
            return; // senseid installed — skip; binary path would run, not test error path
        }
        let provider: Arc<dyn crate::platform::PlatformProvider> =
            Arc::new(MockProvider { start_ok: false });
        let fixer = DaemonFixer::new(provider, 19995, false, "senseid");
        let result = fixer.fix();
        assert!(result.is_err(), "should Err when brew fails and binary not in PATH");
        assert!(
            result.unwrap_err().contains("not found in PATH"),
            "error should mention binary not found"
        );
    }

    #[test]
    fn daemon_fixer_in_dev_mode_binary_not_found_returns_err() {
        // In dev mode DaemonFixer skips brew services and goes straight to binary.
        // If senseid-dev isn't installed, it should return Err immediately.
        if crate::util::which_binary("senseid-dev").is_some() {
            return; // binary present — test would go to port-polling path, skip
        }
        let provider: Arc<dyn crate::platform::PlatformProvider> =
            Arc::new(MockProvider { start_ok: true });
        let fixer = DaemonFixer::new(provider, 19994, true, "senseid-dev");
        let result = fixer.fix();
        assert!(result.is_err(), "should Err in dev mode when senseid-dev not in PATH");
        let err = result.unwrap_err();
        assert!(
            err.contains("not found in PATH"),
            "error should name the missing binary, got: {err}"
        );
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
