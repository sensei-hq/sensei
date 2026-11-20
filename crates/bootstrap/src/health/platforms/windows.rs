//! Windows PlatformProvider (#505).
//!
//! Postgres is the `sensei-pg` Docker container, checked through the Docker
//! CLI and engine; winget installs Ollama and Git for Windows; the database
//! check first needs the credentials in `~/.sensei/.env`. Git for Windows is
//! part of the sensei component: sensei's hooks run under its `sh`.
//!
//! `checker_for` matches exhaustively, so a new ComponentId will not compile
//! until it is handled here.

use super::{DAEMON_PORT_TIMEOUT, OLLAMA_PORT_TIMEOUT, POSTGRES_PORT_TIMEOUT, port};
use crate::config::{OLLAMA_PORT, POSTGRES_PORT, SenseiConfig};
use crate::health::checker::Checker;
use crate::health::checkers::binary::BinaryChecker;
use crate::health::checkers::composite::AndChecker;
use crate::health::checkers::postgres_db::PostgresDatabaseChecker;
use crate::health::checkers::windows::{
    CredentialsChecker, DockerPostgresChecker, PosixShellChecker,
};
use crate::health::provider::PlatformProvider;
use crate::health::resolver::Resolver;
use crate::health::resolvers::daemon_start::DaemonStartResolver;
use crate::health::resolvers::db_setup::DatabaseResolver;
use crate::health::resolvers::windows::{
    CredentialedDatabaseResolver, DockerPostgresResolver, OLLAMA_PACKAGE, SenseiWindowsResolver,
    WingetInstallResolver,
};
use crate::health::types::{ComponentId, PackageManagerId, Platform, Remedy};

pub struct WindowsProvider;

impl PlatformProvider for WindowsProvider {
    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn package_manager_id(&self) -> PackageManagerId {
        PackageManagerId::Winget
    }

    fn package_manager_checker(&self) -> Box<dyn Checker> {
        Box::new(BinaryChecker::with_version("winget", "--version"))
    }

    fn checker_for(&self, id: ComponentId, retry: bool) -> Box<dyn Checker> {
        match id {
            ComponentId::Postgres => Box::new(AndChecker {
                checkers: vec![
                    Box::new(DockerPostgresChecker { run: crate::pg_container::docker }),
                    port("postgres", POSTGRES_PORT, retry, POSTGRES_PORT_TIMEOUT),
                ],
            }),
            ComponentId::Ollama => port("ollama", OLLAMA_PORT, retry, OLLAMA_PORT_TIMEOUT),
            ComponentId::Sensei => {
                let cfg = SenseiConfig::from_env();
                Box::new(AndChecker {
                    checkers: vec![
                        Box::new(BinaryChecker::new(cfg.sensei_binary())),
                        Box::new(BinaryChecker::new(cfg.senseid_binary())),
                        Box::new(BinaryChecker::new(cfg.sensei_mcp_binary())),
                        Box::new(PosixShellChecker),
                    ],
                })
            }
            ComponentId::Database => Box::new(AndChecker {
                checkers: vec![
                    Box::new(CredentialsChecker { env_file: crate::credentials::env_file() }),
                    Box::new(PostgresDatabaseChecker { db_name: SenseiConfig::from_env().db_name }),
                ],
            }),
            ComponentId::Daemon => {
                port("daemon", SenseiConfig::from_env().daemon_port, retry, DAEMON_PORT_TIMEOUT)
            }
        }
    }

    fn resolvers(&self) -> Vec<Box<dyn Resolver>> {
        vec![
            Box::new(DockerPostgresResolver { run: crate::pg_container::docker }),
            Box::new(WingetInstallResolver {
                id: "ollama_winget",
                component: &[ComponentId::Ollama],
                package: OLLAMA_PACKAGE,
                name: "Ollama",
            }),
            Box::new(SenseiWindowsResolver),
            Box::new(CredentialedDatabaseResolver {
                inner: DatabaseResolver { db_name: SenseiConfig::from_env().db_name },
            }),
            Box::new(DaemonStartResolver),
        ]
    }

    fn default_remedy(&self) -> Remedy {
        Remedy {
            message: "Some components need attention. Run `sensei doctor --fix` to install or start what it can; each remaining component lists its own step.".to_string(),
            script: "sensei doctor --fix".to_string(),
            url: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn platform_is_windows() {
        assert_eq!(WindowsProvider.platform(), Platform::Windows);
    }

    #[test]
    fn package_manager_is_winget() {
        assert_eq!(WindowsProvider.package_manager_id(), PackageManagerId::Winget);
    }

    #[test]
    fn builds_a_checker_for_every_component_id() {
        for retry in [false, true] {
            for id in [
                ComponentId::Postgres,
                ComponentId::Ollama,
                ComponentId::Sensei,
                ComponentId::Database,
                ComponentId::Daemon,
            ] {
                let _ = WindowsProvider.checker_for(id, retry);
            }
        }
    }

    #[test]
    fn every_component_has_exactly_one_resolver() {
        let mut seen = HashSet::new();
        for r in WindowsProvider.resolvers() {
            for c in r.resolves() {
                assert!(seen.insert(format!("{c:?}")), "{c:?} has two resolvers");
            }
        }
        assert_eq!(seen.len(), 5, "resolved: {seen:?}");
    }

    #[test]
    fn ollama_is_installed_with_winget() {
        let r = WindowsProvider
            .resolvers()
            .into_iter()
            .find(|r| r.resolves() == [ComponentId::Ollama])
            .unwrap();
        assert!(r.fallback_remedy().script.starts_with("winget install --id Ollama.Ollama -e"));
    }

    #[test]
    fn default_remedy_points_at_doctor_fix() {
        assert_eq!(WindowsProvider.default_remedy().script, "sensei doctor --fix");
    }
}
