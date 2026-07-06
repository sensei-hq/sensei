//! Embedded-Postgres lifecycle + schema deploy for the hive.
//!
//! `HiveDb::bootstrap` spins up an embedded PostgreSQL instance, creates the
//! `hive` database, deploys the `hive` scope from the single `database/design.yaml`
//! (its own tables + the shared governance closure — schemas, scope ladder,
//! enforcement enum — and NO extensions), then seeds the scope ladder from the
//! canonical `scopes.jsonl`. There is NO `design.hive.yaml`; the scope selection
//! lives in `design.yaml`.

use std::path::{Path, PathBuf};

use postgresql_embedded::{PostgreSQL, Settings};
use sqlx_postgres::{PgPool, PgPoolOptions};

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("embedded postgres: {0}")]
    Embedded(String),
    #[error("dbd deploy: {0}")]
    Deploy(String),
    #[error("seed: {0}")]
    Seed(String),
    #[error("pool: {0}")]
    Pool(String),
}

/// A running embedded Postgres with the hive schema applied + scopes seeded.
pub struct HiveDb {
    _pg: PostgreSQL, // owns the process; dropped on shutdown
    pool: PgPool,
}

impl HiveDb {
    /// The connection pool to the hive database.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Boot an embedded Postgres at `data_dir`, deploy the `hive` scope from the
    /// `database/` tree at `database_dir`, and seed the scope ladder.
    ///
    /// `temporary`: caller decides. Tests should set this to `true` (via
    /// `bootstrap_temp`) so the crate's Drop wipes the data dir. Production
    /// callers pass `false` to persist across daemon restarts.
    pub async fn bootstrap(data_dir: PathBuf, database_dir: PathBuf) -> Result<Self, DbError> {
        Self::bootstrap_with_settings(data_dir, database_dir, false).await
    }

    async fn bootstrap_with_settings(
        data_dir: PathBuf,
        database_dir: PathBuf,
        temporary: bool,
    ) -> Result<Self, DbError> {
        let settings = Settings {
            data_dir,
            temporary,
            ..Default::default()
        };
        let mut pg = PostgreSQL::new(settings);
        pg.setup()
            .await
            .map_err(|e| DbError::Embedded(e.to_string()))?;
        pg.start()
            .await
            .map_err(|e| DbError::Embedded(e.to_string()))?;
        if !pg
            .database_exists("hive")
            .await
            .map_err(|e| DbError::Embedded(e.to_string()))?
        {
            pg.create_database("hive")
                .await
                .map_err(|e| DbError::Embedded(e.to_string()))?;
        }
        let url = pg.settings().url("hive");
        deploy_hive_schema(&url, &database_dir).await?;
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&url)
            .await
            .map_err(|e| DbError::Pool(e.to_string()))?;
        seed_scopes(&pool, &database_dir).await?;
        Ok(Self { _pg: pg, pool })
    }

    /// Convenience for tests: boot into a unique temp data dir using the
    /// workspace `database/` tree. The dir is unique per call (process id +
    /// monotonic counter) so multiple `#[tokio::test]`s in one test binary do
    /// not collide on the same embedded-Postgres data directory.
    ///
    /// Marks the embedded-pg as `temporary: true` so the crate's Drop wipes
    /// the data dir when the test finishes normally. As a defensive layer
    /// against SIGKILL / panic-across-runtime paths that skip Drop, this
    /// also runs `sweep_orphaned_test_dirs` at the top of every call —
    /// prior-run leaks stop accumulating even when a Drop is missed.
    pub async fn bootstrap_temp() -> Result<Self, DbError> {
        sweep_orphaned_test_dirs();

        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let tmp = std::env::temp_dir()
            .join(format!("sensei-hive-test-{}-{n}", std::process::id()));
        Self::bootstrap_with_settings(tmp, workspace_database_dir(), true).await
    }
}

/// Sweep `$TMPDIR/sensei-hive-test-*` directories left behind by prior test
/// runs whose parent PID is no longer alive, killing any embedded-Postgres
/// child that's still holding the dir and then rm'ing the tree.
///
/// This runs at the top of every `bootstrap_temp` — it's cheap (a single
/// `ps` + a directory walk) and keeps the temp filesystem from filling
/// up when a test binary gets SIGKILL'd (Ctrl+C, cargo test timeout,
/// OOM) and skips Drop.
///
/// Only touches directories whose PID prefix belongs to a dead process, so
/// concurrent `cargo test` invocations against the same crate leave each
/// other's live dirs alone.
fn sweep_orphaned_test_dirs() {
    let tmp = std::env::temp_dir();
    let Ok(entries) = std::fs::read_dir(&tmp) else { return };

    for entry in entries.flatten() {
        let Ok(file_name) = entry.file_name().into_string() else { continue };
        let Some(rest) = file_name.strip_prefix("sensei-hive-test-") else { continue };
        // Name shape: sensei-hive-test-{pid}-{n} — parse the pid segment.
        let Some((pid_str, _)) = rest.split_once('-') else { continue };
        let Ok(pid) = pid_str.parse::<u32>() else { continue };

        if is_pid_alive(pid) { continue }

        // The parent is gone. Kill any embedded-pg processes still
        // holding this dir (using -D <path> in their command line), then
        // remove the tree. Failures here are best-effort — logging noise
        // is worse than an orphan directory the next run will retry.
        let dir_path = entry.path();
        let _ = std::process::Command::new("pkill")
            .arg("-f")
            .arg(dir_path.to_string_lossy().as_ref())
            .output();
        let _ = std::fs::remove_dir_all(&dir_path);
    }
}

/// Cheap `kill -0 <pid>` — returns true when the process exists.
/// stderr is captured (not inherited) so a "No such process" message for
/// every orphan doesn't spam the test-runner output on cleanup.
fn is_pid_alive(pid: u32) -> bool {
    // `kill -0` returns 0 when the process exists; nonzero when it's
    // gone or we lack permission. `Command::output` captures stderr so
    // the "No such process" line stays out of the test runner's
    // console.
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// `crates/hive-mind/ -> ../../database`
fn workspace_database_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database")
}

async fn deploy_hive_schema(db_url: &str, database_dir: &Path) -> Result<(), DbError> {
    use dbd_core::adapter::postgres::PostgresAdapter;
    use dbd_core::Design;

    let cfg = database_dir.join("design.yaml");
    let design = Design::from_config_with_dir(&cfg, "prod", Some(database_dir))
        .map_err(|e| DbError::Deploy(format!("config load: {e}")))?;
    let scope = design
        .resolve_scope(Some("hive"), None)
        .map_err(|e| DbError::Deploy(format!("resolve scope: {e}")))?;
    let adapter = PostgresAdapter::new(db_url, "hive")
        .await
        .map_err(|e| DbError::Deploy(format!("connect: {e}")))?;
    design
        .apply(
            &adapter,
            None,
            false,
            Some(&scope),
            |_| {},
            |desc: &str, err: Option<&str>| {
                if let Some(e) = err {
                    tracing::warn!(dbd_step = "apply", desc, error = e, "failed");
                }
            },
            |_| tracing::info!("hive schema applied"),
        )
        .await
        .map_err(|e| DbError::Deploy(format!("apply: {e}")))?;
    Ok(())
}

/// Seed the scope ladder from the canonical scopes.jsonl (DRY; no staging machinery).
async fn seed_scopes(pool: &PgPool, database_dir: &Path) -> Result<(), DbError> {
    #[derive(serde::Deserialize)]
    struct ScopeRow {
        key: String,
        name: String,
        level: i32,
        shareable: bool,
        description: Option<String>,
    }
    let path = database_dir.join("import/staging/scopes.jsonl");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| DbError::Seed(format!("read {}: {e}", path.display())))?;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let r: ScopeRow =
            serde_json::from_str(line).map_err(|e| DbError::Seed(format!("parse: {e}")))?;
        sqlx_core::query::query(
            "INSERT INTO sensei.scopes(key,name,level,shareable,description) VALUES($1,$2,$3,$4,$5)
             ON CONFLICT(key) DO UPDATE SET name=EXCLUDED.name, level=EXCLUDED.level, shareable=EXCLUDED.shareable, description=EXCLUDED.description",
        )
        .bind(&r.key)
        .bind(&r.name)
        .bind(r.level)
        .bind(r.shareable)
        .bind(&r.description)
        .execute(pool)
        .await
        .map_err(|e| DbError::Seed(e.to_string()))?;
    }
    Ok(())
}
