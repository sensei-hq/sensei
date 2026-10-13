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
use dbd_core::{Design, EntityType, deploy::resolve_source};
use std::path::{Path, PathBuf};
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
        deploy_to(&db_url, &project_dir, &cfg.sensei_dir().join("backup").join("pre-reconcile"))
            .await
    })?;

    // The bundled packs (D-SEED: the default constitution + ponytail) are seeded
    // by dbd's import phase above, from import/staging/rule_pack*.jsonl — the
    // same path every other seeded row takes. There is deliberately no second
    // mechanism here: the previous one shelled out to psql, fail-open, and had
    // been silently seeding NOTHING since rule_packs.source was renamed to
    // attribution. A datafile cannot drift from its table that way, and an
    // import failure surfaces through surface_step_failures instead of a warning.
    Ok(())
}

/// Deploy the design at `project_dir` to the database at `db_url`: the schema,
/// then the seed data. A schema change that drops data first copies every
/// table it alters into `backup_dir`. The source-resolving half lives in
/// [`deploy`]; this half takes its inputs as arguments so a test can point it
/// at a scratch database.
pub async fn deploy_to(db_url: &str, project_dir: &Path, backup_dir: &Path) -> Result<(), String> {
    let config_path = project_dir.join("design.yaml");
    let design = Design::from_config_with_dir(&config_path, "prod", Some(project_dir))
        .map_err(|e| format!("dbd config load failed: {e}"))?;
    tracing::debug!(entities = design.entities().len(), "dbd design loaded");

    // The daemon deploys the `default` scope (everything EXCEPT the `dojo`
    // service schema, which only `sensei-dojo` materializes). resolve_scope(None)
    // returns the `default` scope when one is defined in design.yaml.
    let scope = design
        .resolve_scope(None, None)
        .map_err(|e| format!("dbd scope resolution failed: {e}"))?;

    let adapter = PostgresAdapter::new(db_url, "sensei")
        .await
        .map_err(|e| format!("dbd database connection failed: {e}"))?;

    // Converge the schema, then import the seed data. `apply` alone creates
    // what is missing but never ALTERs what an older release left — an enum
    // that gained a value stayed at the old shape on every upgraded install, and
    // the seed that used the value failed the import (0.11 → 0.12). So:
    //   1. plan the reconcile; when it drops data, copy the tables it alters out
    //      and drop the views that read them (views hold no rows);
    //   2. apply — creates what is missing and every view and function at this
    //      release's definition, those views included;
    //   3. reconcile with allow_destructive — ALTERs tables and enums to the
    //      design. Whole tables are never pruned here;
    //   4. import the seed.
    // Reconcile alters tables before it re-applies views, and `create or
    // replace view` cannot drop a view's column, so without step 1 a column an
    // old view read could not be dropped (0.11's views read folders.project_id).
    // Each phase surfaces per-item failures as a hard Err.
    let schema_failures: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let import_failures: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    let preview = design
        .reconcile(
            &adapter,
            true,
            true,
            false,
            Some(&scope),
            step_progress("reconcile plan", &schema_failures),
        )
        .await
        .map_err(|e| format!("dbd reconcile plan failed: {e}"))?;
    surface_step_failures("reconcile plan", &schema_failures)?;
    if preview.destructive {
        let tables = tables_among(&design, preview.altered.iter().map(|s| s.entity_name.as_str()));
        let saved = back_up_tables(db_url, &tables, backup_dir)?;
        tracing::warn!(backup = %saved.display(), ?tables, "the schema drops data the design no longer declares; backed up first");
        drop_views_reading(db_url, &tables)?;
    }

    tracing::info!("dbd phase: apply");
    design
        .apply(&adapter, None, false, Some(&scope), step_progress("apply", &schema_failures))
        .await
        .map_err(|e| format!("dbd apply failed: {e}"))?;
    surface_step_failures("apply", &schema_failures)?;

    tracing::info!("dbd phase: reconcile");
    let plan = design
        .reconcile(
            &adapter,
            false,
            true,
            false,
            Some(&scope),
            step_progress("reconcile", &schema_failures),
        )
        .await
        .map_err(|e| format!("dbd reconcile failed: {e}"))?;
    surface_step_failures("reconcile", &schema_failures)?;
    for warning in &plan.warnings {
        tracing::warn!(dbd_step = "reconcile", warning, "reconcile advisory");
    }

    tracing::info!("dbd phase: import_data");
    design
        .import_data(&adapter, None, false, Some(&scope), step_progress("import", &import_failures))
        .await
        .map_err(|e| format!("dbd import failed: {e}"))?;
    surface_step_failures("import", &import_failures)?;

    tracing::info!("dbd deploy complete");
    Ok(())
}

/// The names among `altered` the design declares as tables: a plan also alters
/// enums, and only a table holds rows to copy.
fn tables_among<'a>(design: &Design, altered: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    altered
        .filter(|name| {
            design
                .entities()
                .iter()
                .any(|e| e.entity_type == EntityType::Table && e.qualified_key() == *name)
        })
        .collect()
}

/// Copy each of `tables` (`schema.table`) to `<backup_dir>/<UTC time>/<table>.csv`
/// with psql's `\copy` — client-side, so it works whatever the server's major
/// version. Returns the directory written. Any failure is an Err, and the
/// caller does not run the destructive plan without its backup.
fn back_up_tables(db_url: &str, tables: &[&str], backup_dir: &Path) -> Result<PathBuf, String> {
    let dir = backup_dir.join(chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string());
    std::fs::create_dir_all(&dir).map_err(|e| format!("backup dir {}: {e}", dir.display()))?;
    for table in tables {
        let file = dir.join(format!("{table}.csv"));
        let copy = format!(
            "\\copy {} to '{}' with (format csv, header)",
            quoted_relation(table),
            file.display().to_string().replace('\'', "''")
        );
        run_psql(db_url, &copy).map_err(|e| {
            format!("backing up {table} before a destructive schema change failed: {e}")
        })?;
    }
    Ok(dir)
}

/// Drop every view that reads one of `tables`, and what depends on those views
/// (cascade), so the reconcile after can drop a column they read. The apply
/// that follows recreates each one the design declares.
fn drop_views_reading(db_url: &str, tables: &[&str]) -> Result<(), String> {
    if tables.is_empty() {
        return Ok(());
    }
    // Each table as a regclass literal: its quoted name inside a SQL string.
    let relations = tables
        .iter()
        .map(|t| format!("'{}'", quoted_relation(t).replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "do $$ declare v text; begin \
           for v in select distinct format('%I.%I', vn.nspname, vc.relname) \
               from pg_depend d \
               join pg_rewrite r on r.oid = d.objid \
               join pg_class vc on vc.oid = r.ev_class and vc.relkind = 'v' \
               join pg_namespace vn on vn.oid = vc.relnamespace \
              where d.refobjid = any(array[{relations}]::regclass[]) and vc.oid <> d.refobjid \
           loop execute 'drop view if exists ' || v || ' cascade'; end loop; end $$"
    );
    run_psql(db_url, &sql).map_err(|e| {
        format!(
            "dropping the views that read {tables:?} before a destructive schema change failed: {e}"
        )
    })
}

/// Run one command through psql, stopping on its first error; the Err is
/// psql's own message.
fn run_psql(db_url: &str, command: &str) -> Result<(), String> {
    let out = crate::util::command_for("psql")?
        .args([db_url, "-v", "ON_ERROR_STOP=1", "-c", command])
        .output()
        .map_err(|e| format!("could not run psql: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// `schema.table` as a quoted SQL identifier pair.
fn quoted_relation(name: &str) -> String {
    name.split('.')
        .map(|part| format!("\"{}\"", part.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(".")
}

/// The Progress a deploy phase reports through: each step logged, each failed
/// step collected into `failures` for [`surface_step_failures`].
#[allow(clippy::type_complexity)] // three closure types, named by Progress's own parameters
fn step_progress<T>(
    step: &'static str,
    failures: &Arc<Mutex<Vec<String>>>,
) -> Progress<impl FnMut(&str), impl FnMut(&str, Option<&str>), impl FnMut(T)> {
    let failures = failures.clone();
    Progress {
        on_start: move |desc: &str| tracing::debug!(dbd_step = step, desc, "starting"),
        on_done: move |desc: &str, err: Option<&str>| match err {
            Some(e) => {
                tracing::warn!(dbd_step = step, desc, error = e, "failed");
                failures.lock().expect("dbd-failure mutex poisoned").push(format!("{desc}: {e}"));
            }
            None => tracing::debug!(dbd_step = step, desc, "done"),
        },
        on_complete: move |_summary: T| tracing::info!(dbd_step = step, "complete"),
    }
}

/// Fold per-item failures captured from dbd's `reconcile`/`import_data` Progress
/// callbacks into one hard error naming the offending item(s) — so a partial
/// deploy surfaces its cause instead of passing silently.
fn surface_step_failures(phase: &str, failures: &Arc<Mutex<Vec<String>>>) -> Result<(), String> {
    let failures = failures.lock().expect("dbd-failure mutex poisoned");
    if failures.is_empty() {
        return Ok(());
    }
    Err(format!("dbd {phase} reported {} failed item(s): {}", failures.len(), failures.join("; ")))
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
    fn only_the_tables_a_plan_alters_are_backed_up_not_its_enums() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../database");
        let design =
            Design::from_config_with_dir(&dir.join("design.yaml"), "prod", Some(&dir)).unwrap();
        let altered = ["sensei.scan_skip_reason", "sensei.folders", "sensei.task_execution_kind"];
        assert_eq!(tables_among(&design, altered.into_iter()), ["sensei.folders"]);
    }

    #[test]
    fn database_exists_parses_psql_output() {
        let stdout = " sensei    | dev | UTF8     | \n template0 | dev | UTF8     | \n";
        let found = stdout.lines().any(|line| {
            line.split('|').next().map(|name| name.trim() == "sensei").unwrap_or(false)
        });
        assert!(found);
    }

    #[test]
    fn database_exists_does_not_partial_match() {
        let stdout = " sensei-prod | dev | UTF8 | \n";
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
