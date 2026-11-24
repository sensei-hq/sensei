//! PostgresInstallResolver — resolves ComponentId::Postgres via
//! `brew install postgresql@17` AND `brew services start postgresql@17`,
//! then `brew install pgvector` (the schema's `vector` extension).
//! Install alone leaves the server stopped — every downstream checker
//! (PortChecker for 5432, database setup) would then fail. Both steps are
//! idempotent: brew skips installed formulas and treats "already started"
//! as success.

use crate::health::resolver::{ResolveOutcome, Resolver};
use crate::health::resolvers::brew_helpers::brew_install_to_outcome;
use crate::health::resolvers::service_cascade::{ServiceCascadeSpec, resolve_service_cascade};
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
    fn id(&self) -> &'static str {
        "postgres_install"
    }
    fn resolves(&self) -> &'static [ComponentId] {
        TARGETS
    }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        then_install_extensions(resolve_service_cascade(&SPEC), |f| brew_install_to_outcome(f, &[]))
    }

    fn fallback_remedy(&self) -> Remedy {
        postgres_fallback_remedy()
    }
}

/// Extensions the sensei schema needs that Homebrew ships as their own
/// formulas — `postgresql@17` alone has no pgvector, so `CREATE EXTENSION
/// vector` in the database step would fail on a fresh machine.
const EXTENSION_FORMULAS: &[&str] = &["pgvector"];

/// Once postgres is up, install each extension formula (idempotent: brew
/// skips an installed one). A cascade that did not bring postgres up keeps
/// its own remedy; the first extension that fails to install returns its.
fn then_install_extensions(
    cascade: ResolveOutcome,
    install: impl Fn(&str) -> ResolveOutcome,
) -> ResolveOutcome {
    if !matches!(cascade, ResolveOutcome::Resolved) {
        return cascade;
    }
    for formula in EXTENSION_FORMULAS {
        if let failed @ ResolveOutcome::NeedsHumanAction(_) = install(formula) {
            return failed;
        }
    }
    ResolveOutcome::Resolved
}

/// Most likely cause of a post-resolve check still failing: `postgresql@17`
/// is keg-only, so `pg_isready`/`psql` aren't symlinked onto PATH even after
/// a successful install. Force-link, then restart the service.
fn postgres_fallback_remedy() -> Remedy {
    Remedy {
        message: format!(
            "PostgreSQL didn't come up after install. The `{FORMULA}` formula is keg-only, so its CLI tools (psql, pg_isready) need to be force-linked. Run the script below."
        ),
        script: format!(
            "brew link --force --overwrite {FORMULA} && brew services restart {SERVICE}"
        ),
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
    fn a_running_postgres_then_gets_pgvector_installed() {
        let installed = std::cell::RefCell::new(Vec::new());
        let out = then_install_extensions(ResolveOutcome::Resolved, |f| {
            installed.borrow_mut().push(f.to_string());
            ResolveOutcome::Resolved
        });
        assert!(matches!(out, ResolveOutcome::Resolved));
        assert_eq!(installed.into_inner(), ["pgvector"]);
    }

    #[test]
    fn a_failed_cascade_installs_no_extension_and_keeps_its_remedy() {
        let remedy = Remedy { message: "m".into(), script: "cascade".into(), url: None };
        let out = then_install_extensions(ResolveOutcome::NeedsHumanAction(remedy), |_| {
            panic!("no extension install when postgres did not come up")
        });
        let ResolveOutcome::NeedsHumanAction(r) = out else { panic!("expected a remedy") };
        assert_eq!(r.script, "cascade");
    }

    #[test]
    fn a_failed_pgvector_install_surfaces_its_remedy() {
        let remedy =
            Remedy { message: "m".into(), script: "brew install pgvector".into(), url: None };
        let out = then_install_extensions(ResolveOutcome::Resolved, |_| {
            ResolveOutcome::NeedsHumanAction(remedy.clone())
        });
        let ResolveOutcome::NeedsHumanAction(r) = out else { panic!("expected a remedy") };
        assert_eq!(r.script, "brew install pgvector");
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
