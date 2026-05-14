//! PostgresInstallResolver — resolves ComponentId::Postgres via
//! `brew install postgresql@17`. Auto-attempts brew; escalates to
//! NeedsHumanAction only when brew itself fails. The dmg / Postgres.app
//! case never reaches this resolver — the binary checker passes first
//! and the orchestrator skips us entirely.

use std::path::Path;

use crate::health::resolver::{Resolver, ResolveOutcome};
use crate::health::resolvers::brew_helpers::{brew_install, BrewError};
use crate::health::types::{ComponentId, Remedy};

pub struct PostgresInstallResolver;

const FORMULA: &str = "postgresql@17";
const TARGETS: &[ComponentId] = &[ComponentId::Postgres];

impl Resolver for PostgresInstallResolver {
    fn id(&self) -> &'static str { "postgres_install" }
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

pub(crate) fn homebrew_install_remedy() -> Remedy {
    Remedy {
        message: "Homebrew isn't installed. Run the script below to install it, then re-check.".to_string(),
        script:  r#"/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)""#.to_string(),
        url:     Some("https://brew.sh".to_string()),
    }
}

pub(crate) fn overwrite_link_remedy(formula: &str, path: &Path) -> Remedy {
    Remedy {
        message: format!(
            "Couldn't link `{formula}` because `{}` already exists. If you installed it elsewhere (e.g. .dmg or Postgres.app) you can keep that and skip this step. To switch to the brew install, run the script below.",
            path.display(),
        ),
        script:  format!("brew link --overwrite {formula}"),
        url:     None,
    }
}

pub(crate) fn tap_missing_remedy(formula: &str) -> Remedy {
    Remedy {
        message: format!("Couldn't find `{formula}`. Run the script below to add the tap, then re-check."),
        script:  format!("brew tap sensei-hq/tap https://github.com/sensei-hq/homebrew-tap && brew install {formula}"),
        url:     None,
    }
}

pub(crate) fn generic_brew_remedy(formula: &str, stderr_tail: &str) -> Remedy {
    Remedy {
        message: format!(
            "Couldn't install `{formula}` automatically. Last brew output was:\n\n```\n{stderr_tail}\n```\n\nRun the script below to retry."
        ),
        script:  format!("brew install {formula}"),
        url:     None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn id_is_postgres_install() {
        assert_eq!(PostgresInstallResolver.id(), "postgres_install");
    }

    #[test]
    fn resolves_only_postgres() {
        assert_eq!(PostgresInstallResolver.resolves(), &[ComponentId::Postgres]);
    }

    #[test]
    fn does_not_cover_others() {
        let t = PostgresInstallResolver.resolves();
        assert!(!t.contains(&ComponentId::Ollama));
        assert!(!t.contains(&ComponentId::Sensei));
        assert!(!t.contains(&ComponentId::Database));
        assert!(!t.contains(&ComponentId::Daemon));
    }

    #[test]
    fn homebrew_install_remedy_contains_install_script_and_url() {
        let r = homebrew_install_remedy();
        assert!(r.script.contains("brew.sh") || r.script.contains("Homebrew/install"));
        assert_eq!(r.url.as_deref(), Some("https://brew.sh"));
    }

    #[test]
    fn overwrite_link_remedy_mentions_formula_and_path() {
        let r = overwrite_link_remedy("postgresql@17", &PathBuf::from("/opt/homebrew/bin/psql"));
        assert!(r.message.contains("postgresql@17"));
        assert!(r.message.contains("/opt/homebrew/bin/psql"));
        assert_eq!(r.script, "brew link --overwrite postgresql@17");
    }

    #[test]
    fn tap_missing_remedy_runs_tap_then_install() {
        let r = tap_missing_remedy("postgresql@17");
        assert!(r.script.contains("brew tap"));
        assert!(r.script.contains("brew install postgresql@17"));
    }

    #[test]
    fn generic_brew_remedy_includes_stderr_tail() {
        let r = generic_brew_remedy("postgresql@17", "some failure detail");
        assert!(r.message.contains("some failure detail"));
        assert_eq!(r.script, "brew install postgresql@17");
    }
}
