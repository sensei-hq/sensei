//! OllamaInstallResolver — resolves ComponentId::Ollama via
//! `brew install ollama`. Identical structure to PostgresInstallResolver;
//! reuses its remedy builders.

use crate::health::resolver::{Resolver, ResolveOutcome};
use crate::health::resolvers::brew_helpers::{brew_install, BrewError};
use crate::health::resolvers::postgres_install::{
    generic_brew_remedy, homebrew_install_remedy, overwrite_link_remedy, tap_missing_remedy,
};
use crate::health::types::ComponentId;

pub struct OllamaInstallResolver;

const FORMULA: &str = "ollama";
const TARGETS: &[ComponentId] = &[ComponentId::Ollama];

impl Resolver for OllamaInstallResolver {
    fn id(&self) -> &'static str { "ollama_install" }
    fn resolves(&self) -> &'static [ComponentId] { TARGETS }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        match brew_install(FORMULA, &[]) {
            Ok(())                                => ResolveOutcome::Resolved,
            Err(BrewError::BrewNotFound)          => ResolveOutcome::NeedsHumanAction(homebrew_install_remedy()),
            Err(BrewError::LinkConflict { path }) => ResolveOutcome::NeedsHumanAction(overwrite_link_remedy(FORMULA, &path)),
            Err(BrewError::TapMissing)            => ResolveOutcome::NeedsHumanAction(tap_missing_remedy(FORMULA)),
            Err(BrewError::Other(stderr))         => ResolveOutcome::NeedsHumanAction(generic_brew_remedy(FORMULA, &stderr)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_ollama_install() {
        assert_eq!(OllamaInstallResolver.id(), "ollama_install");
    }

    #[test]
    fn resolves_only_ollama() {
        assert_eq!(OllamaInstallResolver.resolves(), &[ComponentId::Ollama]);
    }

    #[test]
    fn does_not_cover_others() {
        let t = OllamaInstallResolver.resolves();
        assert!(!t.contains(&ComponentId::Postgres));
        assert!(!t.contains(&ComponentId::Sensei));
        assert!(!t.contains(&ComponentId::Database));
        assert!(!t.contains(&ComponentId::Daemon));
    }
}
