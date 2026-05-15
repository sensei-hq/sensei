//! DatabaseResolver — create the sensei database, install pgvector, deploy
//! the schema via dbd-core. Restored to call into the `database` module after
//! the previous version shipped a phantom `sensei db:create` CLI subcommand
//! that doesn't exist (lost when `_legacy/database.rs` was deleted).

use crate::database;
use crate::health::resolver::{Resolver, ResolveOutcome};
use crate::health::types::{ComponentId, Remedy};

/// Bootstrap crate's Cargo.toml version. The `make bump` flow keeps this in
/// sync with the workspace `VERSION` file, so it matches the running app.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct DatabaseResolver {
    pub db_name: String,
}

impl Resolver for DatabaseResolver {
    fn id(&self) -> &'static str { "db_setup" }
    fn resolves(&self) -> &'static [ComponentId] { &[ComponentId::Database] }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        match database::setup(&self.db_name, APP_VERSION) {
            Ok(()) => ResolveOutcome::Resolved,
            Err(e) => ResolveOutcome::NeedsHumanAction(db_failed_remedy(&self.db_name, e)),
        }
    }
}

fn db_failed_remedy(db_name: &str, detail: String) -> Remedy {
    Remedy {
        message: format!(
            "Couldn't set up the database automatically ({detail}). Make sure PostgreSQL is running, then create the database and the pgvector extension manually."
        ),
        // Minimum manual fallback. Schema deploy still needs dbd-core; the
        // user is referred to the docs for that.
        script: format!(
            "createdb {db_name} && psql -d {db_name} -c 'CREATE EXTENSION IF NOT EXISTS vector'"
        ),
        url: None,
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
    fn failed_remedy_uses_db_name_in_script() {
        let r = db_failed_remedy("sensei_dev", "bang".to_string());
        assert!(r.script.contains("sensei_dev"));
        assert!(r.script.contains("CREATE EXTENSION"));
        assert!(r.message.contains("bang"));
    }

    #[test]
    fn app_version_matches_cargo_pkg_version() {
        assert_eq!(APP_VERSION, env!("CARGO_PKG_VERSION"));
    }
}
