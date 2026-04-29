//! sensei-bootstrap — prerequisite detection, installation, and hardware profiling.
//!
//! This crate has NO database dependencies and NO daemon dependencies.
//! It checks and fixes prerequisites using shell commands and port probes.
//!
//! Consumers: sensei-cli (`sensei doctor`), Tauri desktop app (sidecar commands).

pub mod database;
pub mod hardware;
pub mod homebrew;
pub mod models;
pub mod platform;
pub mod service;
pub mod types;
pub mod util;

pub use types::*;

/// Default ports for sensei services.
pub const DAEMON_PORT: u16 = 7744;
pub const OLLAMA_PORT: u16 = 11434;
pub const POSTGRES_PORT: u16 = 5432;

/// Run the full bootstrap check — all phases, returns composite result.
pub fn run() -> BootstrapResult {
    let hw = hardware::detect();

    let components = vec![
        homebrew::check(),
        homebrew::check_binary("postgresql", "postgres", "--version"),
        homebrew::check_binary("ollama", "ollama", "--version"),
        homebrew::check_binary("sensei", "sensei", "--version"),
        service::check("postgresql", POSTGRES_PORT),
        service::check("ollama", OLLAMA_PORT),
        database::check(None),
        service::check("daemon", DAEMON_PORT),
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
}
