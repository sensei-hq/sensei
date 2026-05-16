//! PostgresInstallResolver — resolves ComponentId::Postgres via
//! `brew install postgresql@17` AND `brew services start postgresql@17`.
//! Install alone leaves the server stopped — every downstream checker
//! (PortChecker for 5432, database setup) would then fail. Both steps are
//! idempotent: brew skips installed formulas and treats "already started"
//! as success.

use crate::health::resolver::{Resolver, ResolveOutcome};
use crate::health::resolvers::service_cascade::{resolve_service_cascade, ServiceCascadeSpec};
use crate::health::types::{ComponentId, Remedy};

pub struct PostgresInstallResolver;

const FORMULA: &str = "postgresql@17";
const SERVICE: &str = "postgresql@17";
const TARGETS: &[ComponentId] = &[ComponentId::Postgres];

/// `direct_launcher: None` — postgres `pg_ctl start` needs a data directory
/// and superuser, which we can't reliably guess. Stage 2 is skipped; if
/// stage 1 fails the cascade goes straight to brew install.
const SPEC: ServiceCascadeSpec = ServiceCascadeSpec {
    formula: FORMULA,
    service: SERVICE,
    install_args: &[],
    direct_launcher: None,
};

impl Resolver for PostgresInstallResolver {
    fn id(&self) -> &'static str { "postgres_install" }
    fn resolves(&self) -> &'static [ComponentId] { TARGETS }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        resolve_service_cascade(&SPEC)
    }

    fn fallback_remedy(&self) -> Remedy { postgres_fallback_remedy() }
}

/// Most likely cause of a post-resolve check still failing: `postgresql@17`
/// is keg-only, so `pg_isready`/`psql` aren't symlinked onto PATH even after
/// a successful install. Force-link, then restart the service.
fn postgres_fallback_remedy() -> Remedy {
    Remedy {
        message: format!(
            "PostgreSQL didn't come up after install. The `{FORMULA}` formula is keg-only, so its CLI tools (psql, pg_isready) need to be force-linked. Run the script below."
        ),
        script: format!("brew link --force --overwrite {FORMULA} && brew services restart {SERVICE}"),
        url: None,
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

    #[test]
    fn fallback_remedy_addresses_keg_only_link() {
        let r = PostgresInstallResolver.fallback_remedy();
        assert!(r.script.contains("brew link --force"));
        assert!(r.script.contains(FORMULA));
        assert!(r.script.contains("brew services restart"));
        assert!(r.message.contains("keg-only"));
    }
}
