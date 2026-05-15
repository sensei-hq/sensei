//! Database operations: pg readiness check, db existence/creation, extensions,
//! and schema deploy via dbd-core.
//!
//! Restored from the deleted `_legacy/database.rs` (lost in commit `7a73eb70`
//! when the bootstrap engine was rewritten and the `_legacy/` quarantine was
//! cleared). The new checker/resolver layer pointed `DatabaseResolver` at a
//! `sensei db:create` CLI subcommand that does not exist, so initial DB setup
//! and post-upgrade schema deploy stopped working silently.
//!
//! All public functions return `Result<(), String>` so resolvers can convert
//! errors to a `Remedy`. The `deploy` path uses dbd-core with the schema
//! source produced by `SenseiConfig::db_schema_source(version)` — GitHub
//! download at the matching release tag in prod, local directory via
//! `SENSEI_DB_SCHEMA_PATH` in dev.

use dbd_core::adapter::postgres::PostgresAdapter;
use dbd_core::{deploy::resolve_source, Design};
use std::process::Command;

use crate::config::SenseiConfig;

/// True when `pg_isready --quiet` exits 0.
pub fn pg_is_ready() -> bool {
    Command::new("pg_isready")
        .args(["--quiet"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Parse `psql -lqt` output for an exact db name match.
pub fn database_exists(db_name: &str) -> Result<bool, String> {
    let output = Command::new("psql")
        .args(["-lqt"])
        .output()
        .map_err(|e| format!("could not run psql: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "psql -lqt failed (exit {}): {}",
            output.status,
            stderr.trim()
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().any(|line| {
        line.split('|')
            .next()
            .map(|name| name.trim() == db_name)
            .unwrap_or(false)
    }))
}

/// Run `createdb <db_name>`. Treats "already exists" as success — the
/// idempotent shape resolvers want.
pub fn create(db_name: &str) -> Result<(), String> {
    let output = Command::new("createdb")
        .arg(db_name)
        .output()
        .map_err(|e| format!("createdb failed: {e}"))?;

    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.contains("already exists") {
        return Ok(());
    }
    Err(format!("createdb {db_name} failed: {stderr}"))
}

/// `CREATE EXTENSION IF NOT EXISTS vector` against `db_name`.
pub fn ensure_extensions(db_name: &str) -> Result<(), String> {
    let output = Command::new("psql")
        .args(["-d", db_name, "-c", "CREATE EXTENSION IF NOT EXISTS vector"])
        .output()
        .map_err(|e| format!("psql failed: {e}"))?;

    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(format!("failed to enable pgvector: {stderr}"))
}

/// Deploy the sensei schema via dbd-core. Idempotent — dbd tracks state in
/// `_dbd_meta` and only applies pending changes. Requires a reachable DB.
///
/// `app_version` chooses the schema source: in prod builds the GitHub tag
/// `v{app_version}` of `sensei-hq/sensei/database`; in dev builds the local
/// path from `SENSEI_DB_SCHEMA_PATH` if set, otherwise GitHub.
pub fn deploy(db_name: &str, app_version: &str) -> Result<(), String> {
    let cfg = SenseiConfig::from_env();
    let env = if cfg.is_dev() { "dev" } else { "prod" };
    // postgres:// with no user — psql / dbd use the OS user by default.
    let db_url = format!("postgres://localhost/{db_name}");
    let source = SenseiConfig::db_schema_source(app_version);

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime error: {e}"))?;

    rt.block_on(async {
        let project_dir = resolve_source(&source)
            .await
            .map_err(|e| format!("dbd source resolution failed ({source}): {e}"))?;
        let config_path = project_dir.join("design.yaml");
        let design = Design::from_config_with_dir(&config_path, env, Some(&project_dir))
            .map_err(|e| format!("dbd config load failed: {e}"))?;
        let adapter = PostgresAdapter::new(&db_url, "sensei")
            .await
            .map_err(|e| format!("dbd database connection failed: {e}"))?;
        design
            .deploy(&adapter, false)
            .await
            .map_err(|e| format!("dbd deploy failed: {e}"))
    })
}

/// Full initial setup: confirm pg is up → create the DB if missing →
/// ensure pgvector → deploy schema. Used by `DatabaseResolver`.
pub fn setup(db_name: &str, app_version: &str) -> Result<(), String> {
    if !pg_is_ready() {
        return Err("postgresql is not accepting connections".to_string());
    }
    match database_exists(db_name) {
        Ok(true) => {}
        Ok(false) => create(db_name)?,
        Err(e) => return Err(format!("database check failed: {e}")),
    }
    ensure_extensions(db_name)?;
    deploy(db_name, app_version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_exists_parses_psql_output() {
        let stdout = " sensei    | jerry | UTF8     | \n template0 | jerry | UTF8     | \n";
        let found = stdout.lines().any(|line| {
            line.split('|')
                .next()
                .map(|name| name.trim() == "sensei")
                .unwrap_or(false)
        });
        assert!(found);
    }

    #[test]
    fn database_exists_does_not_partial_match() {
        let stdout = " sensei-prod | jerry | UTF8 | \n";
        let found = stdout.lines().any(|line| {
            line.split('|')
                .next()
                .map(|name| name.trim() == "sensei")
                .unwrap_or(false)
        });
        assert!(!found, "must match the full db name only");
    }

    #[test]
    fn db_url_format_round_trips() {
        let db = "sensei_test";
        let url = format!("postgres://localhost/{db}");
        assert!(url.starts_with("postgres://localhost/"));
        assert!(url.ends_with("sensei_test"));
    }

    #[test]
    fn deploy_source_parses_as_github_tagged_release() {
        let version = "0.2.14";
        let source = SenseiConfig::db_schema_source(version);
        // In dev with SENSEI_DB_SCHEMA_PATH the source is a local path; skip then.
        if std::env::var("SENSEI_DB_SCHEMA_PATH").is_err() {
            let parsed = dbd_core::github::parse_github_source(&source).unwrap();
            assert_eq!(parsed.owner, "sensei-hq");
            assert_eq!(parsed.repo, "sensei");
            assert_eq!(parsed.subpath, Some("database".to_string()));
            assert_eq!(parsed.git_ref, format!("v{version}"));
        }
    }

    #[test]
    fn setup_without_postgres_returns_err() {
        // Skipped when pg_isready succeeds (CI machines with postgres).
        if !pg_is_ready() {
            let err = setup("definitely_not_a_real_db", "0.0.0").unwrap_err();
            assert!(err.contains("not accepting connections"), "got: {err}");
        }
    }
}
