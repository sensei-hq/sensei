//! Platform-aware factory — builds Prerequisite lists for each bootstrap phase.
//!
//! Adding a new prerequisite: implement the Prerequisite trait (or use GenericPrerequisite)
//! and register it here. Adding a new platform: add a match arm in each factory function.

use std::sync::Arc;
use crate::{POSTGRES_PORT, OLLAMA_PORT, DAEMON_PORT};
use crate::platform::{Platform, PlatformProvider};
use super::{GateKind, Prerequisite};
use super::checker::{BinaryChecker, PortChecker, DatabaseChecker};
use super::fixer::{BrewFixer, WingetFixer, NoopFixer, ServiceStartFixer, DatabaseSetupFixer, Fixer};
use super::generic::GenericPrerequisite;

/// Find Homebrew binary in well-known locations.
pub fn detect_brew_path() -> Option<String> {
    ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(|s| s.to_string())
}

/// Phase 1 — install binaries: postgresql, ollama, sensei CLI.
pub fn install_prerequisites(provider: Arc<dyn PlatformProvider>) -> Vec<Box<dyn Prerequisite>> {
    match provider.platform() {
        Platform::MacOS | Platform::Linux => {
            let brew = detect_brew_path();
            let mk_fixer = |formula: &str| -> Box<dyn Fixer> {
                match &brew {
                    Some(p) => Box::new(BrewFixer::new(p, formula)),
                    None => Box::new(NoopFixer::new("Homebrew not found — install Homebrew first")),
                }
            };
            vec![
                Box::new(GenericPrerequisite::new(
                    "postgresql", "PostgreSQL",
                    Box::new(BinaryChecker::new("postgres", "--version")),
                    mk_fixer("postgresql@17"),
                    GateKind::Install, None,
                )),
                Box::new(GenericPrerequisite::new(
                    "ollama", "Ollama",
                    Box::new(BinaryChecker::new("ollama", "--version")),
                    mk_fixer("ollama"),
                    GateKind::Install, None,
                )),
                Box::new(GenericPrerequisite::new(
                    "sensei", "Sensei CLI",
                    Box::new(BinaryChecker::new("sensei", "--version")),
                    mk_fixer("sensei-hq/tap/sensei"),
                    GateKind::Install, None,
                )),
            ]
        }
        Platform::Windows => vec![
            Box::new(GenericPrerequisite::new(
                "postgresql", "PostgreSQL",
                Box::new(BinaryChecker::new("postgres", "--version")),
                Box::new(WingetFixer::new("PostgreSQL.PostgreSQL")),
                GateKind::Install, None,
            )),
            Box::new(GenericPrerequisite::new(
                "ollama", "Ollama",
                Box::new(BinaryChecker::new("ollama", "--version")),
                Box::new(WingetFixer::new("Ollama.Ollama")),
                GateKind::Install, None,
            )),
            Box::new(GenericPrerequisite::new(
                "sensei", "Sensei CLI",
                Box::new(BinaryChecker::new("sensei", "--version")),
                Box::new(NoopFixer::new("Download sensei from sensei.so/download")),
                GateKind::Install, None,
            )),
        ],
    }
}

/// Phase 2 — start services: postgresql, ollama, daemon.
pub fn start_services(provider: Arc<dyn PlatformProvider>) -> Vec<Box<dyn Prerequisite>> {
    vec![
        Box::new(GenericPrerequisite::new(
            "postgresql", "PostgreSQL",
            Box::new(PortChecker::new("postgresql", POSTGRES_PORT)),
            Box::new(ServiceStartFixer::new(provider.clone(), "postgresql", POSTGRES_PORT)),
            GateKind::Service, None,
        )),
        Box::new(GenericPrerequisite::new(
            "ollama", "Ollama",
            Box::new(PortChecker::new("ollama", OLLAMA_PORT)),
            Box::new(ServiceStartFixer::new(provider.clone(), "ollama", OLLAMA_PORT)),
            GateKind::Service, None,
        )),
        Box::new(GenericPrerequisite::new(
            "daemon", "Sensei Daemon",
            Box::new(PortChecker::new("daemon", DAEMON_PORT)),
            Box::new(ServiceStartFixer::new(provider, "daemon", DAEMON_PORT)),
            GateKind::Service, None,
        )),
    ]
}

/// Phase 3 — database setup.
pub fn setup_database(app_version: &str) -> Vec<Box<dyn Prerequisite>> {
    vec![
        Box::new(GenericPrerequisite::new(
            "database", "Sensei Database",
            Box::new(DatabaseChecker),
            Box::new(DatabaseSetupFixer::new(app_version)),
            GateKind::Install, None,
        )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform;

    #[test]
    fn install_prerequisites_returns_three_gates() {
        let provider = Arc::from(platform::detect());
        let prereqs = install_prerequisites(provider);
        assert_eq!(prereqs.len(), 3);
        assert_eq!(prereqs[0].id(), "postgresql");
        assert_eq!(prereqs[1].id(), "ollama");
        assert_eq!(prereqs[2].id(), "sensei");
    }

    #[test]
    fn start_services_returns_three_gates() {
        let provider = Arc::from(platform::detect());
        let prereqs = start_services(provider);
        assert_eq!(prereqs.len(), 3);
        assert_eq!(prereqs[0].id(), "postgresql");
        assert_eq!(prereqs[1].id(), "ollama");
        assert_eq!(prereqs[2].id(), "daemon");
    }

    #[test]
    fn setup_database_returns_one_gate() {
        let prereqs = setup_database("0.1.0");
        assert_eq!(prereqs.len(), 1);
        assert_eq!(prereqs[0].id(), "database");
    }

    #[test]
    fn install_prereqs_all_have_install_kind() {
        let provider = Arc::from(platform::detect());
        for p in install_prerequisites(provider) {
            assert_eq!(p.gate_kind(), GateKind::Install, "{} should be Install kind", p.id());
        }
    }

    #[test]
    fn service_prereqs_all_have_service_kind() {
        let provider = Arc::from(platform::detect());
        for p in start_services(provider) {
            assert_eq!(p.gate_kind(), GateKind::Service, "{} should be Service kind", p.id());
        }
    }

    #[test]
    fn detect_brew_path_does_not_panic() {
        let _ = detect_brew_path(); // Some on macOS with brew, None otherwise
    }
}
