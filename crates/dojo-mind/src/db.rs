//! Data-layer lifecycle + schema deploy for the Dōjō service.
//!
//! `DojoDb::bootstrap` connects to the dojo Postgres — Supabase when
//! `SUPABASE_DB_URL` is set (a local Supabase for a self-hosted dojo, or a hosted
//! Supabase for SaaS), otherwise an embedded PostgreSQL for zero-config local dev
//! (and tests, via `bootstrap_temp`). Either way it deploys the unified `dojo`
//! scope from the single `database/design.yaml` — BOTH the governance-federation
//! tables
//! (shared_rules/members/api_keys/audit_log) AND the SaaS-layer artifact
//! tables/enums/procedure, plus the shared governance closure (schemas, scope
//! ladder, enforcement enum) and NO extensions — then seeds the scope ladder
//! from the canonical `scopes.jsonl`. The scope selection lives in `design.yaml`.
//!
//! It idempotently seeds the special-case global-dojo tenant on every boot.

use std::path::{Path, PathBuf};

use postgresql_embedded::{PostgreSQL, Settings};
use sqlx_postgres::{PgPool, PgPoolOptions};

/// Fallback embedded-Postgres superuser password. `postgresql_embedded`'s
/// `Settings::default()` mints a fresh RANDOM password every process, and the
/// password is baked into the cluster at `initdb`. That silently breaks
/// persistence: a `serve` after a prior process (or a restart) can't reopen the
/// existing data dir because the new random password no longer matches — auth
/// fails. Pinning it (env-overridable) makes a persisted cluster reopenable
/// across restarts. The embedded PG binds loopback only, so a stable local
/// default is acceptable; override via `SENSEI_DOJO_DB_PASSWORD` for anything
/// sensitive.
const DEFAULT_EMBEDDED_PASSWORD: &str = "sensei-dojo-local";

fn embedded_password() -> String {
    std::env::var("SENSEI_DOJO_DB_PASSWORD")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_EMBEDDED_PASSWORD.to_string())
}

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

/// A connected dojo data layer with the schema applied + scopes seeded. `_pg` is
/// `Some` only for the embedded fallback (it owns the process, dropped on
/// shutdown); the Supabase path holds just the pool.
pub struct DojoDb {
    _pg: Option<PostgreSQL>,
    pool: PgPool,
}

impl DojoDb {
    /// The connection pool to the dojo database.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Connect the dojo data layer, deploy the `dojo` scope from the `database/`
    /// tree at `database_dir`, and seed the scope ladder.
    ///
    /// When `supabase_db_url` is `Some`, connect to that Postgres (Supabase —
    /// local self-hosted or hosted SaaS). Otherwise boot an embedded Postgres at
    /// `data_dir` (zero-config local dev), persisting across daemon restarts.
    pub async fn bootstrap(
        supabase_db_url: Option<String>,
        data_dir: PathBuf,
        database_dir: PathBuf,
    ) -> Result<Self, DbError> {
        match supabase_db_url {
            Some(url) => Self::bootstrap_url(url, database_dir).await,
            None => Self::bootstrap_with_settings(data_dir, database_dir, false).await,
        }
    }

    /// Connect to an existing Postgres (Supabase) and deploy + seed the `dojo`
    /// scope into it. No embedded process — Supabase owns the cluster; the `dojo`
    /// and `sensei` schemas are created inside the URL's database.
    async fn bootstrap_url(db_url: String, database_dir: PathBuf) -> Result<Self, DbError> {
        deploy_dojo_schema(&db_url, &database_dir).await?;
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&db_url)
            .await
            .map_err(|e| DbError::Pool(e.to_string()))?;
        seed_scopes(&pool, &database_dir).await?;
        seed_global_dojo(&pool).await?;
        seed_default_governance(&pool).await?;
        Ok(Self { _pg: None, pool })
    }

    async fn bootstrap_with_settings(
        data_dir: PathBuf,
        database_dir: PathBuf,
        temporary: bool,
    ) -> Result<Self, DbError> {
        let settings = Settings {
            data_dir,
            temporary,
            // Pin the superuser password so a persisted cluster reopens across
            // restarts (default() would randomise it per process — see
            // DEFAULT_EMBEDDED_PASSWORD).
            password: embedded_password(),
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
            .database_exists("dojo")
            .await
            .map_err(|e| DbError::Embedded(e.to_string()))?
        {
            pg.create_database("dojo")
                .await
                .map_err(|e| DbError::Embedded(e.to_string()))?;
        }
        let url = pg.settings().url("dojo");
        // Deploy the unified `dojo` scope — the governance-federation tables
        // (shared_rules/members/api_keys/audit_log) AND the artifact-federation
        // tables/enums/procedure — plus the shared sensei governance closure.
        deploy_dojo_schema(&url, &database_dir).await?;
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&url)
            .await
            .map_err(|e| DbError::Pool(e.to_string()))?;
        seed_scopes(&pool, &database_dir).await?;
        // Idempotently seed the special-case global-dojo tenant (the procedure's
        // ON CONFLICT DO NOTHING makes re-running on every boot safe).
        seed_global_dojo(&pool).await?;
        // Idempotently seed the default governance constitution (principles,
        // guardrails, guidelines, stack templates) into the global-dōjō
        // namespace. Same ON CONFLICT DO NOTHING safety.
        seed_default_governance(&pool).await?;
        Ok(Self { _pg: Some(pg), pool })
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
            .join(format!("sensei-dojo-test-{}-{n}", std::process::id()));
        Self::bootstrap_with_settings(tmp, workspace_database_dir(), true).await
    }
}

/// Sweep `$TMPDIR/sensei-dojo-test-*` directories left behind by prior test
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
        let Some(rest) = file_name.strip_prefix("sensei-dojo-test-") else { continue };
        // Name shape: sensei-dojo-test-{pid}-{n} — parse the pid segment.
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

/// `crates/dojo-mind/ -> ../../database`
fn workspace_database_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database")
}

/// Deploy the unified `dojo` scope (governance + artifact federation). Brings up
/// the `dojo.*` enums, tables (the governance-federation tables
/// shared_rules/members/api_keys/audit_log AND the artifact tables incl.
/// `dojo.artifacts` with its `seq` cursor), the `dojo.seed_global_dojo()`
/// procedure, and the shared sensei governance closure.
async fn deploy_dojo_schema(db_url: &str, database_dir: &Path) -> Result<(), DbError> {
    apply_scope(db_url, database_dir, "dojo").await
}

/// Resolve `scope_name` from the single `database/design.yaml` and apply it to
/// `db_url`. The scope name doubles as the adapter's default schema (the scope's
/// tables live in the schema of the same name).
async fn apply_scope(db_url: &str, database_dir: &Path, scope_name: &str) -> Result<(), DbError> {
    use dbd_core::adapter::postgres::PostgresAdapter;
    use dbd_core::Design;

    let cfg = database_dir.join("design.yaml");
    let design = Design::from_config_with_dir(&cfg, "prod", Some(database_dir))
        .map_err(|e| DbError::Deploy(format!("config load: {e}")))?;
    let scope = design
        .resolve_scope(Some(scope_name), None)
        .map_err(|e| DbError::Deploy(format!("resolve scope {scope_name}: {e}")))?;
    let adapter = PostgresAdapter::new(db_url, scope_name)
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
                    tracing::warn!(dbd_step = "apply", scope = scope_name, desc, error = e, "failed");
                }
            },
            |_| tracing::info!(scope = scope_name, "schema applied"),
        )
        .await
        .map_err(|e| DbError::Deploy(format!("apply {scope_name}: {e}")))?;
    Ok(())
}

/// Idempotently seed the special-case global-dojo tenant via the deployed
/// `dojo.seed_global_dojo()` procedure (its internal ON CONFLICT DO NOTHING
/// makes re-running on every boot a no-op once the row exists).
async fn seed_global_dojo(pool: &PgPool) -> Result<(), DbError> {
    sqlx_core::query::query("CALL dojo.seed_global_dojo()")
        .execute(pool)
        .await
        .map_err(|e| DbError::Seed(format!("seed_global_dojo: {e}")))?;
    Ok(())
}

/// Idempotently seed the default governance constitution (principles,
/// guardrails, guidelines, stack templates) into the global-dōjō namespace via
/// the deployed `dojo.seed_default_governance()` procedure. Its internal
/// ON CONFLICT (namespace_id, content_hash) DO NOTHING makes re-running on every
/// boot a no-op once the rows exist (a reworded rule is a new content_hash =>
/// new row, never a dup).
async fn seed_default_governance(pool: &PgPool) -> Result<(), DbError> {
    sqlx_core::query::query("CALL dojo.seed_default_governance()")
        .execute(pool)
        .await
        .map_err(|e| DbError::Seed(format!("seed_default_governance: {e}")))?;
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
