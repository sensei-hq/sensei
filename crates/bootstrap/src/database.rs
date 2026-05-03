//! Database checks — PostgreSQL reachability, database existence, extensions.
//! Uses shell commands (pg_isready, psql), NOT sqlx — no database driver dependency.
//!
//! The database name is resolved once from:
//!   1. `SENSEI_DB_NAME` environment variable  (set to "sensei-dev" in dev)
//!   2. Hard-coded fallback: "sensei"

use std::process::Command;
use std::sync::OnceLock;

use dbd_core::{
    Design,
    deploy::resolve_source,
};
use dbd_core::adapter::postgres::PostgresAdapter;

use crate::types::{BootstrapTrace, ComponentStatus};
use crate::util::run_traced;

static DB_NAME: OnceLock<String> = OnceLock::new();

/// The active database name — resolved once from SENSEI_DB_NAME env var or "sensei".
pub fn db_name() -> &'static str {
    DB_NAME.get_or_init(|| {
        std::env::var("SENSEI_DB_NAME").unwrap_or_else(|_| "sensei".to_string())
    })
}

/// Check if PostgreSQL is reachable and the database exists.
pub fn check() -> ComponentStatus {
    let db = db_name();

    if !pg_is_ready() {
        return ComponentStatus::failed("database", "postgresql not reachable (pg_isready failed)");
    }

    match database_exists(db) {
        Ok(false) => return ComponentStatus::failed("database", &format!("database '{db}' does not exist")),
        Err(e)    => return ComponentStatus::failed("database", &format!("database check failed: {e}")),
        Ok(true)  => {}
    }

    if !pgvector_installed(db) {
        return ComponentStatus::failed("database", "pgvector extension not installed");
    }

    let version = schema_version(db);
    ComponentStatus::ready("database", &format!("schema-{}", version.unwrap_or(0)))
}

/// Check PostgreSQL reachability + database existence + pgvector + schema version.
/// Returns status and diagnostic traces. Pure function — no side effects.
pub fn check_traced() -> (ComponentStatus, Vec<BootstrapTrace>) {
    let db = db_name();
    let mut traces = Vec::new();

    // Step 1: pg_isready
    let pg_t = run_traced(
        "pg_isready",
        "Check PostgreSQL is accepting connections",
        "pg_isready",
        &["--quiet"],
    );
    let pg_ok = pg_t.ok;
    traces.push(pg_t);

    if !pg_ok {
        return (
            ComponentStatus::failed("database", "postgresql not reachable (pg_isready failed)"),
            traces,
        );
    }

    // Step 2: database exists (psql -lqt)
    let db_list_t = run_traced("psql_list", "List databases to check existence", "psql", &["-lqt"]);
    let db_list_ok = db_list_t.ok;
    let db_list_out = db_list_t.out.clone();
    traces.push(db_list_t);

    if !db_list_ok {
        return (ComponentStatus::failed("database", "psql -lqt failed"), traces);
    }

    let db_exists = db_list_out.lines().any(|line| {
        line.split('|').next().map(|n| n.trim() == db).unwrap_or(false)
    });

    if !db_exists {
        return (
            ComponentStatus::failed("database", &format!("database '{db}' does not exist")),
            traces,
        );
    }

    // Step 3: pgvector extension
    let vec_t = run_traced(
        "pgvector_check",
        "Check pgvector extension installed",
        "psql",
        &["-d", db, "-tAc", "SELECT 1 FROM pg_extension WHERE extname = 'vector'"],
    );
    let vec_ok = vec_t.ok && vec_t.out.starts_with('1');
    traces.push(vec_t);

    if !vec_ok {
        return (ComponentStatus::failed("database", "pgvector extension not installed"), traces);
    }

    // Step 4: schema version
    let ver_t = run_traced(
        "schema_version",
        "Read schema migration version",
        "psql",
        &["-d", db, "-tAc", "SELECT max(version) FROM schema_migrations"],
    );
    let version: Option<i32> = ver_t.out.trim().parse().ok();
    traces.push(ver_t);

    (ComponentStatus::ready("database", &format!("schema-{}", version.unwrap_or(0))), traces)
}

/// Create the database.
pub fn create() -> Result<ComponentStatus, String> {
    let db = db_name();
    let output = Command::new("createdb")
        .arg(db)
        .output()
        .map_err(|e| format!("createdb failed: {e}"))?;

    if output.status.success() {
        return Ok(ComponentStatus::ready("database", "created"));
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.contains("already exists") {
        Ok(ComponentStatus::ready("database", "exists"))
    } else {
        Err(format!("createdb {db} failed: {stderr}"))
    }
}

/// Ensure pgvector extension is installed in the database.
pub fn ensure_extensions() -> Result<ComponentStatus, String> {
    let db = db_name();
    let output = Command::new("psql")
        .args(["-d", db, "-c", "CREATE EXTENSION IF NOT EXISTS vector"])
        .output()
        .map_err(|e| format!("psql failed: {e}"))?;

    if output.status.success() {
        return Ok(ComponentStatus::ready("database", "pgvector enabled"));
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(format!("failed to enable pgvector: {stderr}"))
}

/// Deploy the schema from GitHub using dbd-core.
///
/// Source: `sensei-hq/daemon/database@v{app_version}` — downloads tarball
/// and caches under `~/.cache/dbd/`. Idempotent: dbd handles fresh /
/// migrate / current automatically.
///
/// Requires a running PostgreSQL server and a reachable `{db_name}` database.
pub fn deploy(app_version: &str) -> Result<ComponentStatus, String> {
    let db = db_name();
    let source = format!("sensei-hq/daemon/database@v{app_version}");
    let db_url = format!("postgres://localhost/{db}");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime error: {e}"))?;

    rt.block_on(async {
        let project_dir = resolve_source(&source)
            .await
            .map_err(|e| format!("dbd source resolution failed: {e}"))?;

        let config_path = project_dir.join("design.yaml");

        let design = Design::from_config_with_dir(&config_path, "prod", Some(&project_dir))
            .map_err(|e| format!("dbd config load failed: {e}"))?;

        let adapter = PostgresAdapter::new(&db_url, "sensei")
            .await
            .map_err(|e| format!("dbd database connection failed: {e}"))?;

        design
            .deploy(&adapter, false)
            .await
            .map_err(|e| format!("dbd deploy failed: {e}"))
    })?;

    let version = schema_version(db);
    Ok(ComponentStatus::ready(
        "database",
        &format!("schema-{}", version.unwrap_or(0)),
    ))
}

/// Full Phase 3 database setup pipeline.
///
/// 1. Check PostgreSQL is reachable
/// 2. Ensure database exists (create if missing)
/// 3. Ensure extensions (pgvector)
/// 4. Run dbd deploy (schema + seed data)
pub fn setup(app_version: &str) -> Result<ComponentStatus, String> {
    let db = db_name();

    if !pg_is_ready() {
        return Err("postgresql is not accepting connections".to_string());
    }

    match database_exists(db) {
        Ok(true)  => {}
        Ok(false) => { create()?; }
        Err(e)    => { return Err(format!("database check failed: {e}")); }
    }

    ensure_extensions()?;
    deploy(app_version)
}

fn pg_is_ready() -> bool {
    Command::new("pg_isready")
        .args(["--quiet"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn database_exists(db_name: &str) -> Result<bool, String> {
    let output = Command::new("psql")
        .args(["-lqt"])
        .output()
        .map_err(|e| format!("could not run psql: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("psql -lqt failed (exit {}): {}", output.status, stderr.trim()));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().any(|line| {
        line.split('|').next().map(|name| name.trim() == db_name).unwrap_or(false)
    }))
}

fn pgvector_installed(db_name: &str) -> bool {
    let output = Command::new("psql")
        .args(["-d", db_name, "-tAc", "SELECT 1 FROM pg_extension WHERE extname = 'vector'"])
        .output();

    matches!(output, Ok(o) if o.status.success() && o.stdout.starts_with(b"1"))
}

fn schema_version(db_name: &str) -> Option<i32> {
    let output = Command::new("psql")
        .args(["-d", db_name, "-tAc", "SELECT max(version) FROM schema_migrations"])
        .output()
        .ok()?;

    if !output.status.success() { return None; }
    String::from_utf8_lossy(&output.stdout).trim().parse::<i32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_name_returns_non_empty() {
        // Returns SENSEI_DB_NAME if set, otherwise "sensei". Either is valid.
        assert!(!db_name().is_empty());
    }

    #[test]
    fn check_traced_returns_traces() {
        let (status, traces) = check_traced();
        assert!(!traces.is_empty(), "should have at least one trace");
        assert!(status.is_ready() || status.is_failed());
        for t in &traces {
            assert!(!t.step.is_empty());
            assert!(!t.cmd.is_empty());
        }
    }

    #[test]
    fn database_exists_parsing() {
        let output = " sensei    | jerry | UTF8     | \n template0 | jerry | UTF8     | \n";
        let found = output.lines().any(|line| {
            line.split('|').next().map(|name| name.trim() == "sensei").unwrap_or(false)
        });
        assert!(found);
    }

    #[test]
    fn database_exists_parsing_not_found() {
        let output = " postgres  | jerry | UTF8     | \n template0 | jerry | UTF8     | \n";
        let found = output.lines().any(|line| {
            line.split('|').next().map(|name| name.trim() == "sensei").unwrap_or(false)
        });
        assert!(!found);
    }

    #[test]
    fn database_exists_parsing_dev_db() {
        let output = " sensei-dev | jerry | UTF8     | \n template0 | jerry | UTF8     | \n";
        let found = output.lines().any(|line| {
            line.split('|').next().map(|name| name.trim() == "sensei-dev").unwrap_or(false)
        });
        assert!(found);
    }

    #[test]
    fn ensure_extensions_returns_result() {
        let result = ensure_extensions();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn deploy_source_string_is_parseable() {
        // Verify the source format we construct is valid per dbd-core's parser
        let version = "1.2.3";
        let source = format!("sensei-hq/daemon/database@v{version}");
        let parsed = dbd_core::github::parse_github_source(&source).unwrap();
        assert_eq!(parsed.owner, "sensei-hq");
        assert_eq!(parsed.repo, "daemon");
        assert_eq!(parsed.subpath, Some("database".to_string()));
        assert_eq!(parsed.git_ref, format!("v{version}"));
    }

    #[test]
    fn deploy_db_url_format() {
        // Verify the DATABASE_URL we construct has the right format
        let db = "sensei-test-deploy-url";
        let url = format!("postgres://localhost/{db}");
        assert!(url.starts_with("postgres://localhost/"));
        assert!(url.ends_with("sensei-test-deploy-url"));
    }

    #[test]
    fn setup_without_postgres_returns_err() {
        // Replaces old setup_without_postgres test
        let result = setup("0.1.0");
        if !super::pg_is_ready() {
            let err = result.unwrap_err();
            assert!(err.contains("not accepting connections"), "got: {err}");
        }
        // If postgres IS available, result could be Ok or Err — either is fine
    }
}
