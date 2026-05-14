//! macOS (and Linux-via-Homebrew) PlatformProvider.
//!
//! This impl provides ONLY platform-specific knowledge:
//!   - which package manager (homebrew)
//!   - the platform-specific checker for each dependency id
//!   - the resolvers available on this platform
//!   - the default remedy
//!
//! The shared orchestration (`check` / `resolve`) is inherited from the
//! PlatformProvider trait's default methods.
//!
//! IMPORTANT: `checker_for` uses exhaustive `match id { ... }` (no `_ =>`).
//! Adding a new ComponentId is a compile error here until handled.

use crate::config::{POSTGRES_PORT, OLLAMA_PORT, SenseiConfig};
use crate::health::checker::Checker;
use crate::health::checkers::{BinaryChecker, PortChecker, AndChecker, PostgresDatabaseChecker};
use crate::health::provider::PlatformProvider;
use crate::health::resolver::Resolver;
use crate::health::resolvers::{
    DatabaseResolver, DaemonStartResolver,
    PostgresInstallResolver, OllamaInstallResolver, SenseiInstallResolver,
};
use crate::health::types::{ComponentId, PackageManagerId, Platform, Remedy};

pub struct MacOSProvider;

impl PlatformProvider for MacOSProvider {
    fn platform(&self) -> Platform {
        if cfg!(target_os = "linux") { Platform::Linux } else { Platform::Macos }
    }

    fn package_manager_id(&self) -> PackageManagerId { PackageManagerId::Homebrew }

    fn package_manager_checker(&self) -> Box<dyn Checker> {
        Box::new(BinaryChecker::with_version("brew", "--version"))
    }

    fn checker_for(&self, id: ComponentId) -> Box<dyn Checker> {
        match id {
            ComponentId::Postgres => Box::new(AndChecker(vec![
                Box::new(BinaryChecker::with_version("pg_isready", "--version")),
                Box::new(PortChecker::new("postgres", POSTGRES_PORT)),
            ])),
            ComponentId::Ollama => Box::new(AndChecker(vec![
                Box::new(BinaryChecker::with_version("ollama", "--version")),
                Box::new(PortChecker::new("ollama", OLLAMA_PORT)),
            ])),
            ComponentId::Sensei => {
                let cfg = SenseiConfig::from_env();
                Box::new(AndChecker(vec![
                    Box::new(BinaryChecker::new(cfg.sensei_binary())),
                    Box::new(BinaryChecker::new(cfg.senseid_binary())),
                    Box::new(BinaryChecker::new(cfg.sensei_mcp_binary())),
                ]))
            },
            ComponentId::Database => Box::new(PostgresDatabaseChecker {
                db_name: SenseiConfig::from_env().db_name,
            }),
            ComponentId::Daemon => Box::new(PortChecker::new(
                "daemon", SenseiConfig::from_env().daemon_port,
            )),
        }
    }

    fn resolvers(&self) -> Vec<Box<dyn Resolver>> {
        vec![
            Box::new(PostgresInstallResolver),
            Box::new(OllamaInstallResolver),
            Box::new(SenseiInstallResolver),
            Box::new(DatabaseResolver { db_name: SenseiConfig::from_env().db_name }),
            Box::new(DaemonStartResolver),
        ]
    }

    fn default_remedy(&self) -> Remedy {
        Remedy {
            message: "Some components need attention. Run the script below to (re)install sensei; missing prerequisites will be installed when the daemon next runs its health check.".to_string(),
            script:  SenseiConfig::from_env().brew_install_script(),
            url:     None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_is_macos_or_linux() {
        let p = MacOSProvider;
        let plat = p.platform();
        assert!(matches!(plat, Platform::Macos | Platform::Linux));
    }

    #[test]
    fn package_manager_is_homebrew() {
        assert_eq!(MacOSProvider.package_manager_id(), PackageManagerId::Homebrew);
    }

    #[test]
    fn builds_a_checker_for_every_component_id() {
        let p = MacOSProvider;
        // Exhaustive over the 5 ComponentId variants — if a new one is added,
        // checker_for's match arm fails to compile.
        let _ = p.checker_for(ComponentId::Postgres);
        let _ = p.checker_for(ComponentId::Ollama);
        let _ = p.checker_for(ComponentId::Sensei);
        let _ = p.checker_for(ComponentId::Database);
        let _ = p.checker_for(ComponentId::Daemon);
    }

    #[test]
    fn provides_five_resolvers() {
        let r = MacOSProvider.resolvers();
        assert_eq!(r.len(), 5);
        let ids: Vec<&'static str> = r.iter().map(|r| r.id()).collect();
        assert!(ids.contains(&"postgres_install"));
        assert!(ids.contains(&"ollama_install"));
        assert!(ids.contains(&"sensei_install"));
        assert!(ids.contains(&"db_setup"));
        assert!(ids.contains(&"daemon_start"));
    }

    #[test]
    fn each_brew_install_resolver_covers_one_component() {
        let r = MacOSProvider.resolvers();
        let by_id = |id: &str| -> &'static [ComponentId] {
            r.iter().find(|r| r.id() == id).expect("present").resolves()
        };
        assert_eq!(by_id("postgres_install"), &[ComponentId::Postgres]);
        assert_eq!(by_id("ollama_install"), &[ComponentId::Ollama]);
        assert_eq!(by_id("sensei_install"), &[ComponentId::Sensei]);
    }

    #[cfg(feature = "dev")]
    #[test]
    fn default_remedy_uses_dev_formula_and_head_in_dev_mode() {
        let r = MacOSProvider.default_remedy();
        assert!(r.script.contains("--HEAD"));
        assert!(r.script.contains("sensei-hq/tap/sensei-dev"));
    }

    #[cfg(not(feature = "dev"))]
    #[test]
    fn default_remedy_uses_prod_formula_no_head() {
        let r = MacOSProvider.default_remedy();
        assert!(!r.script.contains("--HEAD"));
        assert!(r.script.contains("sensei-hq/tap/sensei"));
        assert!(!r.script.contains("sensei-dev"));
    }

    #[test]
    fn sensei_checker_failure_detail_mentions_current_mode_binary() {
        use crate::config::SenseiConfig;
        use crate::health::types::ComponentStatus;
        let p = MacOSProvider;
        let c = p.checker_for(ComponentId::Sensei);
        let outcome = c.check();
        if matches!(outcome.status, ComponentStatus::Failed) {
            let detail = outcome.detail.unwrap_or_default();
            let cfg = SenseiConfig::from_env();
            let expected = [cfg.sensei_binary(), cfg.senseid_binary(), cfg.sensei_mcp_binary()];
            assert!(
                expected.iter().any(|n| detail.contains(n)),
                "expected detail '{detail}' to mention one of {expected:?}"
            );
        }
    }
}
