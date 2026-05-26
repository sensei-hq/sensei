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

    fn fallback_remedy(&self) -> Remedy {
        db_failed_remedy(&self.db_name, "post-resolve check still failing".to_string())
    }
}

fn db_failed_remedy(db_name: &str, detail: String) -> Remedy {
    Remedy {
        message: format!(
            "Couldn't set up the database automatically ({detail}). Make sure PostgreSQL is running, then create the database manually. The pgvector extension and schema are deployed by `dbd` on the next run."
        ),
        script: format!("createdb {db_name}"),
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
        assert_eq!(r.script, "createdb sensei_dev");
        assert!(r.message.contains("bang"));
        assert!(r.message.contains("dbd"),
            "remedy message should explain that dbd handles the extension + schema");
    }

    #[test]
    fn app_version_matches_cargo_pkg_version() {
        assert_eq!(APP_VERSION, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn fallback_remedy_carries_db_name_and_manual_script() {
        let r = DatabaseResolver { db_name: "sensei_dev".to_string() }.fallback_remedy();
        assert_eq!(r.script, "createdb sensei_dev");
    }
}
