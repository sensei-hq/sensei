//! BrewBundleResolver — installs postgres@16, ollama, and the sensei
//! binaries via a single `brew bundle` invocation. Covers THREE
//! dependencies; runs once even when all three are failed.

use std::process::Command;
use crate::config::SenseiConfig;
use crate::health::resolver::{Resolver, ResolveOutcome};
use crate::health::types::{ComponentId, Remedy};

pub struct BrewBundleResolver;

const TARGETS: &[ComponentId] = &[
    ComponentId::Postgres,
    ComponentId::Ollama,
    ComponentId::Sensei,
];

impl Resolver for BrewBundleResolver {
    fn id(&self) -> &'static str { "brew_bundle" }
    fn resolves(&self) -> &'static [ComponentId] { TARGETS }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        // Always run the full bundle — it's idempotent. Brew skips items already installed.
        let brew = match crate::util::which_binary("brew") {
            Some(p) => p,
            None    => return ResolveOutcome::NeedsHumanAction(homebrew_install_remedy()),
        };
        let cfg = SenseiConfig::from_env();
        let status = Command::new(brew)
            .args(["bundle", &format!("--file={}", cfg.brewfile_url())])
            .status();
        match status {
            Ok(s) if s.success() => ResolveOutcome::Resolved,
            Ok(s)  => ResolveOutcome::NeedsHumanAction(
                         bundle_failed_remedy(format!("brew bundle exited {s}"))),
            Err(e) => ResolveOutcome::NeedsHumanAction(
                         bundle_failed_remedy(format!("brew bundle: {e}"))),
        }
    }
}

fn homebrew_install_remedy() -> Remedy {
    Remedy {
        message: "Homebrew isn't installed. Run the script below to install it, then re-check.".to_string(),
        script:  r#"/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)""#.to_string(),
        url:     Some("https://brew.sh".to_string()),
    }
}

fn bundle_failed_remedy(detail: String) -> Remedy {
    Remedy {
        message: format!("Couldn't complete `brew bundle` automatically ({detail}). Run it yourself."),
        script:  SenseiConfig::from_env().brew_bundle_script(),
        url:     None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_brew_bundle() {
        assert_eq!(BrewBundleResolver.id(), "brew_bundle");
    }

    #[test]
    fn covers_postgres_ollama_sensei() {
        let t = BrewBundleResolver.resolves();
        assert_eq!(t.len(), 3);
        assert!(t.contains(&ComponentId::Postgres));
        assert!(t.contains(&ComponentId::Ollama));
        assert!(t.contains(&ComponentId::Sensei));
    }

    #[test]
    fn does_not_cover_database_or_daemon() {
        let t = BrewBundleResolver.resolves();
        assert!(!t.contains(&ComponentId::Database));
        assert!(!t.contains(&ComponentId::Daemon));
    }
}
