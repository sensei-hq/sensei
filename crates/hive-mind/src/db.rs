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
    pub async fn bootstrap(data_dir: PathBuf, database_dir: PathBuf) -> Result<Self, DbError> {
        let settings = Settings {
            data_dir,
            temporary: false,
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
    pub async fn bootstrap_temp() -> Result<Self, DbError> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let tmp = std::env::temp_dir()
            .join(format!("sensei-hive-test-{}-{n}", std::process::id()));
        Self::bootstrap(tmp, workspace_database_dir()).await
    }
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
