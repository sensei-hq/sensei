//! DatabaseResolver — creates the sensei database via `sensei db:create`.

use std::process::Command;
use crate::health::resolver::{Resolver, ResolveOutcome};
use crate::health::types::{ComponentId, Remedy};

pub struct DatabaseResolver {
    pub db_name: String,
}

impl Resolver for DatabaseResolver {
    fn id(&self) -> &'static str { "db_setup" }
    fn resolves(&self) -> &'static [ComponentId] { &[ComponentId::Database] }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        let sensei = match crate::util::which_binary("sensei") {
            Some(p) => p,
            None    => return ResolveOutcome::NeedsHumanAction(missing_cli_remedy()),
        };
        let status = Command::new(sensei).args(["db:create"]).status();
        match status {
            Ok(s) if s.success() => ResolveOutcome::Resolved,
            Ok(s)  => ResolveOutcome::NeedsHumanAction(
                         db_failed_remedy(format!("sensei db:create exited {s}"))),
            Err(e) => ResolveOutcome::NeedsHumanAction(
                         db_failed_remedy(format!("sensei db:create: {e}"))),
        }
    }
}

fn missing_cli_remedy() -> Remedy {
    Remedy {
        message: "The `sensei` CLI is not installed. Install it via Homebrew first.".to_string(),
        script:  "brew install sensei-hq/tap/sensei".to_string(),
        url:     None,
    }
}

fn db_failed_remedy(detail: String) -> Remedy {
    Remedy {
        message: format!("Couldn't set up the database automatically ({detail}). Run it yourself."),
        script:  "sensei db:create".to_string(),
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
}
