//! Windows PlatformProvider — Phase 1b stub.
//! Real winget-based checkers and resolvers come in Phase 2.

use crate::health::checker::{Checker, CheckOutcome};
use crate::health::provider::PlatformProvider;
use crate::health::resolver::Resolver;
use crate::health::types::{ComponentId, PackageManagerId, Platform, Remedy};

pub struct WindowsProvider;

struct UnsupportedChecker(&'static str);
impl Checker for UnsupportedChecker {
    fn check(&self) -> CheckOutcome {
        CheckOutcome::failed(format!("{}: Windows support not yet implemented", self.0))
    }
}

impl PlatformProvider for WindowsProvider {
    fn platform(&self) -> Platform { Platform::Windows }

    fn package_manager_id(&self) -> PackageManagerId { PackageManagerId::Winget }

    fn package_manager_checker(&self) -> Box<dyn Checker> {
        Box::new(UnsupportedChecker("winget"))
    }

    fn checker_for(&self, id: ComponentId, _retry: bool) -> Box<dyn Checker> {
        Box::new(UnsupportedChecker(match id {
            ComponentId::Postgres => "postgres",
            ComponentId::Ollama   => "ollama",
            ComponentId::Sensei   => "sensei",
            ComponentId::Database => "database",
            ComponentId::Daemon   => "daemon",
        }))
    }

    fn resolvers(&self) -> Vec<Box<dyn Resolver>> { vec![] }

    fn default_remedy(&self) -> Remedy {
        Remedy {
            message: "Windows support is coming. For now, install components manually.".to_string(),
            script:  "# windows install steps TBD".to_string(),
            url:     None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::types::ComponentStatus;

    #[test]
    fn platform_is_windows() {
        assert_eq!(WindowsProvider.platform(), Platform::Windows);
    }

    #[test]
    fn package_manager_is_winget() {
        assert_eq!(WindowsProvider.package_manager_id(), PackageManagerId::Winget);
    }

    #[test]
    fn package_manager_checker_reports_unsupported() {
        let o = WindowsProvider.package_manager_checker().check();
        assert!(matches!(o.status, ComponentStatus::Failed));
        assert!(o.detail.as_deref().unwrap().contains("winget"));
        assert!(o.detail.as_deref().unwrap().contains("not yet implemented"));
    }

    #[test]
    fn checker_for_every_component_reports_unsupported() {
        let p = WindowsProvider;
        for id in [ComponentId::Postgres, ComponentId::Ollama, ComponentId::Sensei,
                   ComponentId::Database, ComponentId::Daemon] {
            let o = p.checker_for(id, false).check();
            assert!(matches!(o.status, ComponentStatus::Failed));
            assert!(o.detail.as_deref().unwrap().contains("Windows support not yet implemented"));
        }
    }

    #[test]
    fn no_resolvers_yet() {
        assert!(WindowsProvider.resolvers().is_empty());
    }

    #[test]
    fn default_remedy_has_message_and_script() {
        let r = WindowsProvider.default_remedy();
        assert!(!r.message.is_empty());
        assert!(!r.script.is_empty());
    }
}
