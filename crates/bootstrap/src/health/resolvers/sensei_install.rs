//! SenseiInstallResolver — resolves ComponentId::Sensei via
//! `brew install [--HEAD] sensei-hq/tap/sensei[-dev]`. The dev/prod
//! formula split branches on `SenseiConfig::is_dev()` (compile-time
//! feature flag).

use crate::config::SenseiConfig;
use crate::health::resolver::{Resolver, ResolveOutcome};
use crate::health::resolvers::brew_helpers::{brew_install, BrewError};
use crate::health::resolvers::postgres_install::{
    generic_brew_remedy, homebrew_install_remedy, overwrite_link_remedy, tap_missing_remedy,
};
use crate::health::types::ComponentId;

pub struct SenseiInstallResolver;

const TARGETS: &[ComponentId] = &[ComponentId::Sensei];

const PROD_FORMULA: &str = "sensei-hq/tap/sensei";
const DEV_FORMULA:  &str = "sensei-hq/tap/sensei-dev";

fn formula_and_args() -> (&'static str, &'static [&'static str]) {
    if SenseiConfig::from_env().is_dev() {
        (DEV_FORMULA, &["--HEAD"])
    } else {
        (PROD_FORMULA, &[])
    }
}

impl Resolver for SenseiInstallResolver {
    fn id(&self) -> &'static str { "sensei_install" }
    fn resolves(&self) -> &'static [ComponentId] { TARGETS }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        let (formula, args) = formula_and_args();
        match brew_install(formula, args) {
            Ok(())                                => ResolveOutcome::Resolved,
            Err(BrewError::BrewNotFound)          => ResolveOutcome::NeedsHumanAction(homebrew_install_remedy()),
            Err(BrewError::LinkConflict { path }) => ResolveOutcome::NeedsHumanAction(overwrite_link_remedy(formula, &path)),
            Err(BrewError::TapMissing)            => ResolveOutcome::NeedsHumanAction(tap_missing_remedy(formula)),
            Err(BrewError::Other(stderr))         => ResolveOutcome::NeedsHumanAction(generic_brew_remedy(formula, &stderr)),
        }
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
