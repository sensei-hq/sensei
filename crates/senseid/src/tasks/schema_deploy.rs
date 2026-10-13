//! The daemon deploys its own schema when its version changed (#260).
//!
//! Only first install and the desktop app's upgrade flow used to deploy, so a
//! `brew upgrade` (which does not even restart the service) left the new binary
//! running on the old release's schema. At boot, before any worker runs, the
//! version that last deployed the schema is compared with this binary's; when
//! they differ the deploy runs — `sensei_bootstrap::database::deploy`, the same
//! reconcile-and-backup path the installer takes — and the version is recorded
//! only once it succeeded. A failure is said loudly and retried next boot.

use std::future::Future;

use crate::db::pg_store::PgStore;

/// `sensei.config` key holding the version whose schema this database has.
pub const DEPLOYED_VERSION_KEY: &str = "schema_deployed_version";

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The schema is already this version's.
    Current,
    /// The schema was deployed and the version recorded.
    Deployed,
    /// The deploy (or recording it) failed; nothing was recorded.
    Failed(String),
}

/// Deploy when the version recorded under `key` is not `current`. An
/// unreadable record deploys too: it is what an old or broken schema looks
/// like, the very case this exists for.
pub async fn deploy_if_version_changed<F, Fut>(
    pg: &PgStore,
    key: &str,
    current: &str,
    deploy: F,
) -> Outcome
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<(), String>>,
{
    if let Ok(Some(recorded)) = pg.get_config(key).await
        && recorded == current
    {
        return Outcome::Current;
    }
    if let Err(e) = deploy().await {
        return Outcome::Failed(e);
    }
    match pg.set_config(key, current).await {
        Ok(()) => Outcome::Deployed,
        Err(e) => Outcome::Failed(format!("schema deployed, but recording {current} failed: {e}")),
    }
}

/// The boot step: this binary's version, the installer's deploy.
pub async fn on_boot(pg: &PgStore) {
    let version = env!("CARGO_PKG_VERSION");
    let deploy = || async {
        let db = sensei_bootstrap::SenseiConfig::from_env().db_name;
        tokio::task::spawn_blocking(move || sensei_bootstrap::database::deploy(&db, version))
            .await
            .map_err(|e| format!("schema deploy task failed: {e}"))?
    };
    match deploy_if_version_changed(pg, DEPLOYED_VERSION_KEY, version, deploy).await {
        Outcome::Current => {}
        Outcome::Deployed => tracing::info!(version, "startup: schema deployed for this version"),
        Outcome::Failed(e) => tracing::error!(
            version,
            error = %e,
            "startup: SCHEMA DEPLOY FAILED — the daemon runs on the previous schema; retried next start"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    async fn store_and_key() -> (PgStore, String) {
        let pg = PgStore::connect_test().await.unwrap();
        (pg, format!("schema_deploy_test_{}", uuid::Uuid::new_v4()))
    }

    async fn drop_key(pg: &PgStore, key: &str) {
        sqlx_core::query::query("DELETE FROM sensei.config WHERE key = $1")
            .bind(key)
            .execute(pg.pool())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_first_start_deploys_and_records_the_version() {
        let (pg, key) = store_and_key().await;
        let calls = AtomicUsize::new(0);
        let out = deploy_if_version_changed(&pg, &key, "1.2.0", || async {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .await;
        assert_eq!(out, Outcome::Deployed);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(pg.get_config(&key).await.unwrap().as_deref(), Some("1.2.0"));
        drop_key(&pg, &key).await;
    }

    #[tokio::test]
    async fn an_upgrade_deploys_once_and_a_restart_on_it_does_not() {
        let (pg, key) = store_and_key().await;
        pg.set_config(&key, "1.1.0").await.unwrap();
        let calls = AtomicUsize::new(0);
        let deploy = || async {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        };
        assert_eq!(deploy_if_version_changed(&pg, &key, "1.2.0", deploy).await, Outcome::Deployed);
        assert_eq!(deploy_if_version_changed(&pg, &key, "1.2.0", deploy).await, Outcome::Current);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        drop_key(&pg, &key).await;
    }

    #[tokio::test]
    async fn a_failed_deploy_records_nothing_so_the_next_start_retries() {
        let (pg, key) = store_and_key().await;
        pg.set_config(&key, "1.1.0").await.unwrap();
        let out = deploy_if_version_changed(&pg, &key, "1.2.0", || async {
            Err("dbd reconcile failed: boom".to_string())
        })
        .await;
        assert_eq!(out, Outcome::Failed("dbd reconcile failed: boom".into()));
        assert_eq!(pg.get_config(&key).await.unwrap().as_deref(), Some("1.1.0"));
        drop_key(&pg, &key).await;
    }
}
