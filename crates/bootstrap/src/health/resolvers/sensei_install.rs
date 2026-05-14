//! SenseiInstallResolver — resolves ComponentId::Sensei via
//! `brew install [--HEAD] sensei-hq/tap/sensei[-dev]`. The dev/prod
//! formula split branches on `SenseiConfig::is_dev()` (compile-time
//! feature flag).

use crate::config::SenseiConfig;
use crate::health::resolver::{Resolver, ResolveOutcome};
use crate::health::resolvers::brew_helpers::brew_install_to_outcome;
use crate::health::types::ComponentId;

pub struct SenseiInstallResolver;

const TARGETS: &[ComponentId] = &[ComponentId::Sensei];

fn formula_and_args() -> (&'static str, &'static [&'static str]) {
    SenseiConfig::from_env().sensei_tap_install_args()
}

impl Resolver for SenseiInstallResolver {
    fn id(&self) -> &'static str { "sensei_install" }
    fn resolves(&self) -> &'static [ComponentId] { TARGETS }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        let (formula, args) = formula_and_args();
        brew_install_to_outcome(formula, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_sensei_install() {
        assert_eq!(SenseiInstallResolver.id(), "sensei_install");
    }

    #[test]
    fn resolves_only_sensei() {
        assert_eq!(SenseiInstallResolver.resolves(), &[ComponentId::Sensei]);
    }

    #[test]
    fn does_not_cover_others() {
        let t = SenseiInstallResolver.resolves();
        assert!(!t.contains(&ComponentId::Postgres));
        assert!(!t.contains(&ComponentId::Ollama));
        assert!(!t.contains(&ComponentId::Database));
        assert!(!t.contains(&ComponentId::Daemon));
    }

    #[cfg(feature = "dev")]
    #[test]
    fn dev_build_uses_head_arg_and_dev_formula() {
        let (formula, args) = formula_and_args();
        assert_eq!(formula, "sensei-hq/tap/sensei-dev");
        assert_eq!(args, &["--HEAD"]);
    }

    #[cfg(not(feature = "dev"))]
    #[test]
    fn prod_build_uses_stable_tap_no_args() {
        let (formula, args) = formula_and_args();
        assert_eq!(formula, "sensei-hq/tap/sensei");
        assert_eq!(args, &[] as &[&str]);
    }
}
