//! DatabaseResolver — creates the sensei database via the sensei CLI
//! (`sensei db:create` in prod, `sensei-dev db:create` in dev). Mode is
//! driven by [`SenseiConfig::sensei_binary`].

use std::process::Command;
use crate::config::SenseiConfig;
use crate::health::resolver::{Resolver, ResolveOutcome};
use crate::health::types::{ComponentId, Remedy};

pub struct DatabaseResolver {
    pub db_name: String,
}

impl Resolver for DatabaseResolver {
    fn id(&self) -> &'static str { "db_setup" }
    fn resolves(&self) -> &'static [ComponentId] { &[ComponentId::Database] }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        let bin = SenseiConfig::from_env().sensei_binary();
        let sensei = match crate::util::which_binary(bin) {
            Some(p) => p,
            None    => return ResolveOutcome::NeedsHumanAction(missing_cli_remedy(bin)),
        };
        let status = Command::new(sensei).args(["db:create"]).status();
        match status {
            Ok(s) if s.success() => ResolveOutcome::Resolved,
            Ok(s)  => ResolveOutcome::NeedsHumanAction(
                         db_failed_remedy(bin, format!("{bin} db:create exited {s}"))),
            Err(e) => ResolveOutcome::NeedsHumanAction(
                         db_failed_remedy(bin, format!("{bin} db:create: {e}"))),
        }
    }
}

fn missing_cli_remedy(bin: &str) -> Remedy {
    Remedy {
        message: format!("The `{bin}` CLI is not installed. Install it via Homebrew first."),
        script:  SenseiConfig::from_env().brew_install_script(),
        url:     None,
    }
}

fn db_failed_remedy(bin: &str, detail: String) -> Remedy {
    Remedy {
        message: format!("Couldn't set up the database automatically ({detail}). Run it yourself."),
        script:  format!("{bin} db:create"),
        url:     None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_db_setup() {
        let r = DatabaseResolver { db_name: "test".to_string() };
        assert_eq!(r.id(), "db_setup");
    }

    #[test]
    fn covers_database_only() {
        let r = DatabaseResolver { db_name: "test".to_string() };
        assert_eq!(r.resolves(), &[ComponentId::Database]);
    }

    #[test]
    fn missing_cli_remedy_names_current_mode_binary() {
        let bin = SenseiConfig::from_env().sensei_binary();
        let r = missing_cli_remedy(bin);
        assert!(r.message.contains(bin), "message '{}' should mention '{bin}'", r.message);
    }

    #[test]
    fn db_failed_remedy_script_uses_current_mode_binary() {
        let bin = SenseiConfig::from_env().sensei_binary();
        let r = db_failed_remedy(bin, "test".to_string());
        assert_eq!(r.script, format!("{bin} db:create"));
    }
}
