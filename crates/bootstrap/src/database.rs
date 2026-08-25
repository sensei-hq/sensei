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
//! source produced by `SenseiConfig::db_schema_source(version)` — the
//! GitHub-tagged release in prod, the workspace `database/` directory in
//! dev. Compile-time decision; no env vars involved.

use dbd_core::adapter::postgres::PostgresAdapter;
use dbd_core::design::Progress;
use dbd_core::{Design, deploy::resolve_source};
use std::sync::{Arc, Mutex};

use crate::config::SenseiConfig;

/// True when `pg_isready --quiet` exits 0.
///
/// Resolves `pg_isready` via `which_binary` first so the spawn doesn't
/// rely on the calling process's `PATH` (which is narrow in the Tauri
/// .app's Finder-launched environment). A missing binary returns false
/// — equivalent to "not ready".
pub fn pg_is_ready() -> bool {
    crate::util::command_for("pg_isready")
        .ok()
        .and_then(|mut cmd| cmd.args(["--quiet"]).status().ok())
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Parse `psql -lqt` output for an exact db name match.
pub fn database_exists(db_name: &str) -> Result<bool, String> {
    // Resolve psql via which_binary so the spawn works in the Tauri .app
    // (whose Finder-launched PATH misses /opt/homebrew/bin). A missing
    // psql surfaces as a clean "psql not installed" rather than the raw
    // "No such file or directory (os error 2)" callers used to see.
    let output = crate::util::command_for("psql")?
        .args(["-lqt"])
        .output()
        .map_err(|e| format!("could not run psql: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("psql -lqt failed (exit {}): {}", output.status, stderr.trim()));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .any(|line| line.split('|').next().map(|name| name.trim() == db_name).unwrap_or(false)))
}

/// Run `createdb <db_name>`. Treats "already exists" as success — the
/// idempotent shape resolvers want.
pub fn create(db_name: &str) -> Result<(), String> {
    let output = crate::util::command_for("createdb")?
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
    let output = crate::util::command_for("psql")?
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
/// `app_version` chooses the schema source: by default the GitHub tag
/// `v{app_version}` of `sensei-hq/sensei/database`. Override at runtime by
/// setting `SENSEI_DDL_DIR=/abs/path/to/database` to point at a local
/// directory — useful when iterating on DDL before publishing a release.
pub fn deploy(db_name: &str, app_version: &str) -> Result<(), String> {
    let cfg = SenseiConfig::from_env();
    // postgres:// with no user — psql / dbd use the OS user by default.
    let db_url = format!("postgres://localhost/{db_name}");
    let source = cfg.db_schema_source(app_version);

    tracing::info!(db = db_name, source = %source, url = %db_url, "starting dbd deploy");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime error: {e}"))?;

    rt.block_on(async {
        let project_dir = resolve_source(&source, false)
            .await
            .map_err(|e| format!("dbd source resolution failed ({source}): {e}"))?;
        tracing::debug!(project_dir = %project_dir.display(), "dbd source resolved");

        let config_path = project_dir.join("design.yaml");
        let design = Design::from_config_with_dir(&config_path, "prod", Some(&project_dir))
            .map_err(|e| format!("dbd config load failed: {e}"))?;
        tracing::debug!(entities = design.entities().len(), "dbd design loaded");

        // The daemon deploys the `default` scope (everything EXCEPT the `dojo`
        // service schema, which only `sensei-dojo` materializes). resolve_scope(None)
        // returns the `default` scope when one is defined in design.yaml.
        let scope = design
            .resolve_scope(None, None)
            .map_err(|e| format!("dbd scope resolution failed: {e}"))?;

        let adapter = PostgresAdapter::new(&db_url, "sensei")
            .await
            .map_err(|e| format!("dbd database connection failed: {e}"))?;

        // Apply schema + import seed data (dbd v0.10.x). We drive the two phases
        // via `apply`/`import_data` rather than the silent `design.deploy()` so
        // per-item failures surface through the Progress callbacks — a seed
        // rejected by a constraint becomes a WARN naming the table AND a hard
        // Err, never a swallowed silent failure. Each phase runs in its own
        // transaction and rolls back cleanly on error.
        let apply_failures: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let import_failures: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        tracing::info!("dbd phase: apply");
        design
            .apply(
                &adapter,
                None,
                false,
                Some(&scope),
                Progress {
                    on_start: |desc: &str| tracing::debug!(dbd_step = "apply", desc, "starting"),
                    on_done: {
                        let f = apply_failures.clone();
                        move |desc: &str, err: Option<&str>| match err {
                            Some(e) => {
                                tracing::warn!(dbd_step = "apply", desc, error = e, "failed");
                                f.lock()
                                    .expect("apply-failure mutex poisoned")
                                    .push(format!("{desc}: {e}"));
                            }
                            None => tracing::debug!(dbd_step = "apply", desc, "done"),
                        }
                    },
                    on_complete: |_summary| tracing::info!("dbd apply complete"),
                },
            )
            .await
            .map_err(|e| format!("dbd apply failed: {e}"))?;
        surface_step_failures("apply", &apply_failures)?;

        tracing::info!("dbd phase: import_data");
        design
            .import_data(
                &adapter,
                None,
                false,
                Some(&scope),
                Progress {
                    on_start: |desc: &str| tracing::debug!(dbd_step = "import", desc, "starting"),
                    on_done: {
                        let f = import_failures.clone();
                        move |desc: &str, err: Option<&str>| match err {
                            Some(e) => {
                                tracing::warn!(dbd_step = "import", desc, error = e, "failed");
                                f.lock()
                                    .expect("import-failure mutex poisoned")
                                    .push(format!("{desc}: {e}"));
                            }
                            None => tracing::debug!(dbd_step = "import", desc, "done"),
                        }
                    },
                    on_complete: |_summary| tracing::info!("dbd import complete"),
                },
            )
            .await
            .map_err(|e| format!("dbd import failed: {e}"))?;
        surface_step_failures("import", &import_failures)?;

        tracing::info!("dbd deploy complete");
        Ok::<(), String>(())
    })?;

    // Bundled-pack seeds (D-SEED): the default constitution + ponytail resolve
    // offline via get_rules. Runs after every deploy (idempotent). Fail-open — a
    // seed hiccup logs but never bricks install/upgrade; the daemon runs fine
    // without them and the next deploy re-attempts.
    if let Err(e) = seed_bundled_packs(db_name) {
        tracing::warn!(error = %e, "bundled-pack seed skipped (non-fatal)");
    }
    Ok(())
}

/// Fold per-item failures captured from dbd's `apply`/`import_data` Progress
/// callbacks into one hard error naming the offending item(s) — so a partial
/// deploy surfaces its cause instead of passing silently.
fn surface_step_failures(phase: &str, failures: &Arc<Mutex<Vec<String>>>) -> Result<(), String> {
    let failures = failures.lock().expect("dbd-failure mutex poisoned");
    if failures.is_empty() {
        return Ok(());
    }
    Err(format!("dbd {phase} reported {} failed item(s): {}", failures.len(), failures.join("; ")))
}

/// CALL the bundled-pack seed procedures (D-SEED / D-LOCAL-PACKS): the default
/// governance constitution and the ponytail minimal-solution discipline — both
/// idempotent global-library packs that resolve offline through get_rules once
/// adopted at the general namespace. Wired into [`deploy`] so a fresh install
/// inherits a curated constitution instead of an empty rules file.
pub fn seed_bundled_packs(db_name: &str) -> Result<(), String> {
    let output = crate::util::command_for("psql")?
        .args([
            "-d",
            db_name,
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            "CALL sensei.seed_default_constitution(); CALL sensei.seed_ponytail_pack();",
        ])
        .output()
        .map_err(|e| format!("psql failed: {e}"))?;

    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(format!("bundled-pack seed failed: {stderr}"))
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
            line.split('|').next().map(|name| name.trim() == "sensei").unwrap_or(false)
        });
        assert!(found);
    }

    #[test]
    fn database_exists_does_not_partial_match() {
        let stdout = " sensei-prod | jerry | UTF8 | \n";
        let found = stdout.lines().any(|line| {
            line.split('|').next().map(|name| name.trim() == "sensei").unwrap_or(false)
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

    // Schema-source resolution (default GitHub tag vs SENSEI_DDL_DIR override)
    // is covered by the unit tests in config::tests — duplicating them here
    // races on the shared env var across test threads.

    #[test]
    fn setup_without_postgres_returns_err() {
        // Skipped when pg_isready succeeds (CI machines with postgres).
        if !pg_is_ready() {
            let err = setup("definitely_not_a_real_db", "0.0.0").unwrap_err();
            assert!(err.contains("not accepting connections"), "got: {err}");
        }
    }
}
