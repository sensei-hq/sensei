//! sensei-bootstrap — prerequisite detection, installation, and hardware profiling.
//!
//! This crate has NO database dependencies and NO daemon dependencies.
//! It checks and fixes prerequisites using shell commands and port probes.
//!
//! Consumers: sensei-cli (`sensei doctor`), Tauri desktop app (sidecar commands).

pub mod config;
pub mod database;
pub mod hardware;
pub mod models;
pub mod platform;
pub mod prereq;
pub mod types;
pub mod util;

pub use config::{
    SenseiConfig, SenseiMode, SenseiLocalConfig,
    binary_is_dev, home_dir,
    BREW_PATHS, BREW_TAP, GITHUB_ORG, GITHUB_REPO,
    HOMEBREW_BREWFILE_URL, HOMEBREW_TAP_REPO, HOMEBREW_TAP_URL,
    MARKETPLACE_RAW_URL, MARKETPLACE_REPO,
    OLLAMA_PORT, POSTGRES_PORT,
    DB_POOL_MAX_CONNECTIONS, DB_POOL_ACQUIRE_TIMEOUT_SECS, DB_POOL_IDLE_TIMEOUT_SECS,
};
pub use prereq::{BootstrapReport, GateReport, GateStatus, HumanAction, ProgressEvent};
pub use prereq::engine::{BootstrapContext, BootstrapEngine};
pub use types::*;

/// Return the daemon port for the current mode.
///
/// Delegates to [`SenseiConfig::from_env`] — single source of truth.
pub fn daemon_port() -> u16 {
    SenseiConfig::from_env().daemon_port
}

/// Lazily-initialized config singleton — avoids duplicating the OnceLock
/// pattern in every binary crate (cli, mcp).
pub fn config() -> &'static SenseiConfig {
    use std::sync::OnceLock;
    static CFG: OnceLock<SenseiConfig> = OnceLock::new();
    CFG.get_or_init(SenseiConfig::detect)
}

/// Shorthand for `config().daemon_url()`.
pub fn daemon_url() -> String {
    config().daemon_url()
}

/// Return the platform provider for the current OS.
pub fn provider() -> Box<dyn platform::PlatformProvider> {
    platform::detect()
}

/// Run check_and_fix with a pre-built BootstrapContext (supports test injection).
pub fn check_and_fix_with_context<F>(ctx: BootstrapContext, callback: F) -> BootstrapReport
where
    F: Fn(ProgressEvent) + Send + Sync + 'static,
{
    let engine = BootstrapEngine::new(std::sync::Arc::new(ctx));
    engine.check_and_fix(callback)
}

/// Convenience: run check_and_fix using environment config (no test injection).
pub fn check_and_fix<F>(app_version: &str, callback: F) -> BootstrapReport
where
    F: Fn(ProgressEvent) + Send + Sync + 'static,
{
    let ctx = BootstrapContext::new(
        std::sync::Arc::from(provider()),
        SenseiConfig::from_env(),
        app_version.to_string(),
    );
    check_and_fix_with_context(ctx, callback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_returns_valid_platform() {
        let prov = provider();
        let p = prov.platform();
        assert!(
            matches!(p, platform::Platform::MacOS | platform::Platform::Linux | platform::Platform::Windows),
            "provider() should return a known platform"
        );
    }

    #[test]
    fn check_and_fix_with_context_all_ready_returns_all_ok() {
        use std::sync::Arc;
        use crate::prereq::registry::COMPONENTS;
        use crate::prereq::checker::Checker;
        use crate::prereq::CheckResult;

        struct OkChecker;
        impl Checker for OkChecker { fn check(&self) -> CheckResult { CheckResult::ok("mock") } }

        let mut ctx = BootstrapContext::new(
            Arc::from(provider()),
            SenseiConfig::from_env(),
            "0.1.0".to_string(),
        );
        for spec in COMPONENTS {
            ctx = ctx.with_checker(spec.id, Arc::new(OkChecker));
        }
        let report = check_and_fix_with_context(ctx, |_| {});
        assert!(report.all_ok);
    }
}
