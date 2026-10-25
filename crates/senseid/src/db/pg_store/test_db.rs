//! One database per test (#183): isolation by construction.
//!
//! Every DB test used to share `sensei_test`, so a database-wide operation in
//! one test — a heal, a prune, a rescan sweep — destroyed a sibling's fixture
//! mid-flight, and the failure surfaced in an unrelated module. Five gates
//! (`CORRECTIONS_TABLE_LOCK`, `ACTIVITY_PRUNE_GATE`, `MEMBERSHIP_COUNT_LOCK`,
//! `VERSION_KEY_LOCK`, `SCAN_STATE_SWEEP_GATE`) were each added after someone
//! first debugged a confusing failure.
//!
//! Now the first [`PgStore::connect_test`](super::PgStore::connect_test) on a
//! test's thread clones `sensei_test` (`STRATEGY FILE_COPY`, ~0.2s) into a
//! database of its own, and every later connect on that thread reuses it.
//! Libtest runs each test on its own thread, so a thread is a test. The clone
//! is dropped when the thread exits; a crashed run's clones are swept, by age,
//! at the start of the next.
//!
//! `sensei_test` itself becomes the TEMPLATE: converged by `dbd`, written by no
//! test. Set `SENSEI_TEST_SHARED_DB=1` to run against it directly instead.

use std::cell::RefCell;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};

use sqlx_core::connection::Connection;
use sqlx_core::executor::Executor;
use sqlx_postgres::PgConnection;

/// Every per-test database starts with this, so the sweep can find them.
const PREFIX: &str = "sensei_test_iso_";
/// A clone older than this is a crashed run's, never a live test's.
const STALE_AFTER_SECS: u64 = 2 * 60 * 60;

static SEQ: AtomicU32 = AtomicU32::new(0);
static SWEPT: OnceLock<()> = OnceLock::new();

thread_local! {
    static OWN: RefCell<Option<Owned>> = const { RefCell::new(None) };
}

/// This thread's clone; dropping it drops the database.
struct Owned {
    name: String,
    admin_url: String,
}

impl Drop for Owned {
    fn drop(&mut self) {
        let sql = format!("DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)", self.name);
        if let Err(e) = on_admin_blocking(&self.admin_url, vec![sql]) {
            eprintln!("test_db: could not drop {}: {e}", self.name);
        }
    }
}

/// The URL a test connects to: its own clone, or the shared database when
/// `SENSEI_TEST_SHARED_DB` is set.
pub(super) async fn url() -> Result<String, String> {
    let base = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        format!("postgresql://localhost:{}/sensei_test", sensei_bootstrap::POSTGRES_PORT)
    });
    if std::env::var_os("SENSEI_TEST_SHARED_DB").is_some() {
        return Ok(base);
    }
    if let Some(name) = OWN.with(|o| o.borrow().as_ref().map(|o| o.name.clone())) {
        return Ok(with_database(&base, &name));
    }

    let template = database_of(&base).ok_or_else(|| format!("no database in {base}"))?;
    let admin_url = with_database(&base, "postgres");
    SWEPT.get_or_init(|| {
        if let Err(e) = sweep_stale(&admin_url) {
            eprintln!("test_db: stale-clone sweep failed: {e}");
        }
    });

    let name = format!(
        "{PREFIX}{}_{}_{}",
        now_secs(),
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let mut admin =
        PgConnection::connect(&admin_url).await.map_err(|e| format!("test_db admin: {e}"))?;
    let create = format!("CREATE DATABASE \"{name}\" TEMPLATE \"{template}\" STRATEGY FILE_COPY");
    // A template must have no other session; one that is momentarily connected
    // (a migration, a psql) clears in moments, so wait it out — boundedly.
    let mut attempt = 0;
    loop {
        match admin.execute(create.as_str()).await {
            Ok(_) => break,
            Err(e) if e.to_string().contains("being accessed by other users") && attempt < 50 => {
                attempt += 1;
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            Err(e) => return Err(format!("test_db: {create}: {e}")),
        }
    }
    let _ = admin.close().await;

    OWN.with(|o| *o.borrow_mut() = Some(Owned { name: name.clone(), admin_url }));
    Ok(with_database(&base, &name))
}

/// Drop every clone a crashed run left behind — by the age in its name, so a
/// concurrent run's live clones are never touched.
fn sweep_stale(admin_url: &str) -> Result<(), String> {
    let admin_url = admin_url.to_string();
    std::thread::spawn(move || -> Result<(), String> {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
        rt.block_on(async {
            let mut c = PgConnection::connect(&admin_url).await.map_err(|e| e.to_string())?;
            let names: Vec<(String,)> = sqlx_core::query_as::query_as(
                "SELECT datname::text FROM pg_database WHERE datname LIKE 'sensei\\_test\\_iso\\_%'",
            )
            .fetch_all(&mut c)
            .await
            .map_err(|e| e.to_string())?;
            let cutoff = now_secs().saturating_sub(STALE_AFTER_SECS);
            for (name,) in names {
                if created_at(&name).is_some_and(|t| t < cutoff) {
                    let _ = c.execute(format!("DROP DATABASE IF EXISTS \"{name}\" WITH (FORCE)").as_str()).await;
                }
            }
            Ok(())
        })
    })
    .join()
    .map_err(|_| "sweep thread panicked".to_string())?
}

/// Run statements against the maintenance database from synchronous code — a
/// thread-local destructor — on a thread and runtime of their own.
fn on_admin_blocking(admin_url: &str, sqls: Vec<String>) -> Result<(), String> {
    let admin_url = admin_url.to_string();
    std::thread::spawn(move || -> Result<(), String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        rt.block_on(async {
            let mut c = PgConnection::connect(&admin_url).await.map_err(|e| e.to_string())?;
            for sql in sqls {
                c.execute(sql.as_str()).await.map_err(|e| e.to_string())?;
            }
            Ok(())
        })
    })
    .join()
    .map_err(|_| "admin thread panicked".to_string())?
}

/// The creation time a clone's name carries.
fn created_at(name: &str) -> Option<u64> {
    name.strip_prefix(PREFIX)?.split('_').next()?.parse().ok()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The database a connection URL names: its last path segment, before `?`.
fn database_of(url: &str) -> Option<String> {
    let path = url.split('?').next()?;
    let db = path.rsplit('/').next()?;
    (!db.is_empty() && path.contains("://") && path.matches('/').count() >= 3)
        .then(|| db.to_string())
}

/// `url` pointed at `database`, keeping host, credentials and query.
fn with_database(url: &str, database: &str) -> String {
    let (path, query) = match url.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (url, None),
    };
    let stem = path.rsplit_once('/').map(|(s, _)| s).unwrap_or(path);
    match query {
        Some(q) => format!("{stem}/{database}?{q}"),
        None => format!("{stem}/{database}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_url_is_repointed_at_another_database_keeping_everything_else() {
        assert_eq!(
            with_database("postgresql://u:p@localhost:5432/sensei_test", "x"),
            "postgresql://u:p@localhost:5432/x"
        );
        assert_eq!(
            with_database("postgresql://localhost:5432/sensei_test?sslmode=disable", "x"),
            "postgresql://localhost:5432/x?sslmode=disable"
        );
        assert_eq!(
            database_of("postgresql://localhost:5432/sensei_test?a=b").as_deref(),
            Some("sensei_test")
        );
        assert_eq!(database_of("postgresql://localhost:5432"), None);
    }

    #[test]
    fn only_a_clone_older_than_the_cutoff_is_stale() {
        assert_eq!(created_at(&format!("{PREFIX}1700000000_42_7")), Some(1_700_000_000));
        assert_eq!(created_at("sensei_test"), None, "the template is never a clone");
        assert_eq!(created_at(&format!("{PREFIX}notanumber_1_1")), None);
    }
}
