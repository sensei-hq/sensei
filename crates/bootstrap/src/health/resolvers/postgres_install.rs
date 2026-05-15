//! PostgresInstallResolver — resolves ComponentId::Postgres via
//! `brew install postgresql@17` AND `brew services start postgresql@17`.
//! Install alone leaves the server stopped — every downstream checker
//! (PortChecker for 5432, database setup) would then fail. Both steps are
//! idempotent: brew skips installed formulas and treats "already started"
//! as success.

use crate::health::resolver::{Resolver, ResolveOutcome};
use crate::health::resolvers::brew_helpers::brew_install_and_start_to_outcome;
use crate::health::types::ComponentId;

pub struct PostgresInstallResolver;

const FORMULA: &str = "postgresql@17";
const SERVICE: &str = "postgresql@17";
const TARGETS: &[ComponentId] = &[ComponentId::Postgres];

impl Resolver for PostgresInstallResolver {
    fn id(&self) -> &'static str { "postgres_install" }
    fn resolves(&self) -> &'static [ComponentId] { TARGETS }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        brew_install_and_start_to_outcome(FORMULA, &[], SERVICE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
