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

    /// dbd#112: on dbd-core 0.27.8 a `now()` call resolved to a `now` view,
    /// so a design shaped like relay's — a `default now()` beside a `now`
    /// view that reads the table — reported a cyclic dependency (the
    /// relay-schema B1 gate). The scratch tree below reproduces that shape:
    /// the load half (`Design::report`, the same loader `deploy_to` opens
    /// with) must come back clean, and the daemon's own `deploy_to` must
    /// land it on a scratch database the test creates and drops.
    ///
    /// Red on 0.27.8 at both halves: the load half reports the cycle and
    /// `deploy_to` (which opens with the same loader) gates on it too —
    /// the report assert fires first, then the deploy assert would. Green
    /// on 0.27.9 both ways (L4 probe, dbd-pin impl r1 F1).
    #[tokio::test]
    async fn deploy_accepts_a_view_named_like_a_function() {
        use dbd_core::Design;
        use std::process::Command;

        let base = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
            format!("postgresql://localhost:{}/sensei_test", sensei_bootstrap::POSTGRES_PORT)
        });
        let host = base.rsplit_once('/').map(|(h, _)| h).unwrap_or(base.as_str());
        let admin_url = format!("{host}/postgres");
        let name = format!("sensei_test_dbd112_{}", std::process::id());
        let db_url = format!("{host}/{name}");
        let run = |url: &str, sql: &str| {
            Command::new("psql").arg(url).arg("-X").arg("-q").arg("-c").arg(sql).status().unwrap()
        };
        assert!(run(&admin_url, &format!("DROP DATABASE IF EXISTS \"{name}\"")).success());
        assert!(run(&admin_url, &format!("CREATE DATABASE \"{name}\"")).success());

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(
            root.join("design.yaml"),
            "project:\n  name: dbd112\nsource:\n  dialect: postgresql\nschemas:\n  - dbd112\n",
        )
        .unwrap();
        let table_dir = root.join("ddl/table/dbd112");
        let view_dir = root.join("ddl/view/dbd112");
        std::fs::create_dir_all(&table_dir).unwrap();
        std::fs::create_dir_all(&view_dir).unwrap();
        std::fs::write(
            table_dir.join("widgets.ddl"),
            "set search_path to dbd112;\n\ncreate table if not exists dbd112.widgets (\n  id text primary key,\n  created_at timestamptz not null default now()\n);\n",
        )
        .unwrap();
        // The view reads the table, closing the 0.27.8 cycle: widgets
        // →(now() default)→ now →(select)→ widgets — the relay B1 shape.
        std::fs::write(
            view_dir.join("now.ddl"),
            "create or replace view dbd112.now as select id, created_at from dbd112.widgets;\n",
        )
        .unwrap();

        let mut design =
            Design::from_config_with_dir(&root.join("design.yaml"), "prod", Some(root)).unwrap();
        let report = design.report(None, None);
        assert!(
            report.issues.is_empty(),
            "no cyclic dependency: {:?}",
            report.issues.iter().map(|e| (&e.name, &e.errors)).collect::<Vec<_>>()
        );

        let out = sensei_bootstrap::database::deploy_to(&db_url, root, &root.join("backup")).await;
        assert!(out.is_ok(), "deploy the now-view tree: {out:?}");

        assert!(
            run(&db_url, "insert into dbd112.widgets (id) values ('w1')").success(),
            "the now() default fires"
        );
        let probe = Command::new("psql")
            .arg(&db_url)
            .arg("-X")
            .arg("-q")
            .arg("-t")
            .arg("-c")
            .arg("select count(*) from dbd112.now")
            .output()
            .unwrap();
        assert!(probe.status.success(), "the deployed view answers");
        assert_eq!(String::from_utf8_lossy(&probe.stdout).trim(), "1");

        assert!(
            run(&admin_url, &format!("DROP DATABASE IF EXISTS \"{name}\" WITH (FORCE)")).success(),
            "scratch database dropped"
        );
    }
}
