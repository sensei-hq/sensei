//! sensei-bootstrap — prerequisite detection, installation, and hardware profiling.
//!
//! This crate has NO database dependencies and NO daemon dependencies.
//! It checks and fixes prerequisites using shell commands and port probes.
//!
//! Consumers: sensei-cli (`sensei doctor`), Tauri desktop app (sidecar commands).

pub mod database;
pub mod hardware;
pub mod models;
pub mod platform;
pub mod types;
pub mod util;

pub use types::*;

/// Default ports for sensei services.
pub const DAEMON_PORT: u16 = 7744;
pub const OLLAMA_PORT: u16 = 11434;
pub const POSTGRES_PORT: u16 = 5432;

/// Return the platform provider for the current OS.
pub fn provider() -> Box<dyn platform::PlatformProvider> {
    platform::detect()
}

/// Run the full bootstrap check — all phases, returns composite result.
pub fn run() -> BootstrapResult {
    let hw = hardware::detect();
    let prov = platform::detect();

    let components = vec![
        // Gate 1: package manager
        prov.check_package_manager(),
        // Gate 2: binaries (cross-platform)
        util::check_binary("postgresql", "postgres", "--version"),
        util::check_binary("ollama", "ollama", "--version"),
        util::check_binary("sensei", "sensei", "--version"),
        // Gate 3: services (port probes)
        util::check_service("postgresql", POSTGRES_PORT),
        util::check_service("ollama", OLLAMA_PORT),
        // Gate 4: database
        database::check(None),
        // Gate 5: daemon service
        util::check_service("daemon", DAEMON_PORT),
    ];

    BootstrapResult::from_checks(components, hw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_does_not_panic() {
        // The full bootstrap check should not panic regardless of system state.
        // Individual components may be failed/missing, but the run should complete.
        let result = run();
        assert!(!result.components.is_empty(), "should check at least one component");
        assert!(result.components.len() >= 5, "should check multiple components");
    }

    #[test]
    fn port_constants() {
        assert_eq!(DAEMON_PORT, 7744);
        assert_eq!(OLLAMA_PORT, 11434);
        assert_eq!(POSTGRES_PORT, 5432);
    }

    #[test]
    fn provider_returns_valid_platform() {
        let prov = provider();
        let p = prov.platform();
        assert!(
            matches!(p, platform::Platform::MacOS | platform::Platform::Linux | platform::Platform::Windows),
            "provider() should return a known platform"
        );
    }
}
