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
    // The "X not installed" message comes from `crate::util::command_for`
    // and means the resolver couldn't find one of psql / createdb /
    // pg_isready in PATH or the EXTRA_BIN_DIRS fallback (Homebrew
    // prefixes, ~/.local/bin). In that case the existing `createdb`
    // remedy is actively misleading — the daemon's environment can't
    // see postgres binaries, but the user's shell almost certainly
    // can, so they run `createdb`, it succeeds, and the resolver still
    // fails on the next pass. Surface the actual root cause and a
    // remedy that fixes it (link the keg-only formula, or install
    // postgres if it's genuinely missing).
    if detail.contains(" not installed") {
        return Remedy {
            message: format!(
                "Postgres CLI tools are not visible to the daemon ({detail}). Install PostgreSQL or, if you already have a keg-only version, link it (e.g. `brew link --force postgresql@17`) so psql/createdb land in /opt/homebrew/bin."
            ),
            script: "brew install postgresql@17 && brew link --force postgresql@17".to_string(),
            url: None,
        };
    }
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

    #[test]
    fn failed_remedy_for_missing_pg_binary_points_at_install_link_rather_than_createdb() {
        // When `database::setup` errors out because psql/createdb
        // aren't on PATH (the daemon-environment-PATH-too-narrow case),
        // the "createdb sensei" remedy is actively misleading: the
        // user's shell can usually find psql, so they run the script,
        // it succeeds, and the resolver still fails next pass. The
        // remedy should instead point at the actual cause — link or
        // install postgresql so the daemon can see it.
        let r = db_failed_remedy("sensei_dev", "psql not installed".to_string());
        assert!(r.script.contains("brew install postgresql"));
        assert!(r.script.contains("brew link"));
        assert!(r.message.contains("not visible to the daemon"));
        // The DB-name-specific `createdb` script should NOT appear here.
        assert!(
            !r.script.contains("createdb"),
            "missing-binary remedy should not suggest createdb (which would run from the user's shell and confuse the loop): {}",
            r.script,
        );
    }
}
