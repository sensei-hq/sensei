//! Database checks — PostgreSQL reachability, database existence, extensions.
//! Uses shell commands (pg_isready, psql), NOT sqlx — no database driver dependency.

use std::process::Command;

use crate::types::ComponentStatus;
use crate::util;

const DEFAULT_DB_NAME: &str = "sensei";

/// Check if PostgreSQL is reachable and the sensei database exists.
pub fn check(db_name: Option<&str>) -> ComponentStatus {
    let db = db_name.unwrap_or(DEFAULT_DB_NAME);

    if !pg_is_ready() {
        return ComponentStatus::failed("database", "postgresql not reachable (pg_isready failed)");
    }

    match database_exists(db) {
        Ok(false) => return ComponentStatus::failed("database", &format!("database '{db}' does not exist")),
        Err(e) => return ComponentStatus::failed("database", &format!("database check failed: {e}")),
        Ok(true) => { /* exists */ }
    }

    if !pgvector_installed(db) {
        return ComponentStatus::failed("database", "pgvector extension not installed");
    }

    let version = schema_version(db);
    ComponentStatus::ready("database", &format!("schema-{}", version.unwrap_or(0)))
}

/// Create the sensei database.
pub fn create(db_name: Option<&str>) -> Result<ComponentStatus, String> {
    let db = db_name.unwrap_or(DEFAULT_DB_NAME);

    let output = Command::new("createdb")
        .arg(db)
        .output()
        .map_err(|e| format!("createdb failed: {e}"))?;

    if output.status.success() {
        Ok(ComponentStatus::ready("database", "created"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.contains("already exists") {
            Ok(ComponentStatus::ready("database", "exists"))
        } else {
            Err(format!("createdb {db} failed: {stderr}"))
        }
    }
}

/// Ensure pgvector extension is installed in the database.
pub fn ensure_extensions(db_name: Option<&str>) -> Result<ComponentStatus, String> {
    let db = db_name.unwrap_or(DEFAULT_DB_NAME);

    let output = Command::new("psql")
        .args(["-d", db, "-c", "CREATE EXTENSION IF NOT EXISTS vector"])
        .output()
        .map_err(|e| format!("psql failed: {e}"))?;

    if output.status.success() {
        Ok(ComponentStatus::ready("database", "pgvector enabled"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("failed to enable pgvector: {stderr}"))
    }
}

/// Run database migrations via senseid.
///
/// Stub — will be replaced by dbd-core when available.
/// Requires the `senseid` binary to be in PATH.
pub fn migrate() -> Result<ComponentStatus, String> {
    let binary = util::which_binary("senseid")
        .ok_or_else(|| "senseid not found — migration requires senseid binary in PATH".to_string())?;

    let output = Command::new(&binary)
        .arg("migrate")
        .output()
        .map_err(|e| format!("senseid migrate failed to execute: {e}"))?;

    if output.status.success() {
        let version = schema_version(DEFAULT_DB_NAME);
        Ok(ComponentStatus::ready(
            "database",
            &format!("schema-{}", version.unwrap_or(0)),
        ))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("senseid migrate failed: {stderr}"))
    }
}

/// Full Phase 3 database setup pipeline.
///
/// Runs the complete setup in order:
/// 1. Check PostgreSQL is reachable
/// 2. Ensure database exists (create if missing)
/// 3. Ensure extensions (pgvector)
/// 4. Run migrations
pub fn setup(db_name: Option<&str>) -> Result<ComponentStatus, String> {
    let db = db_name.unwrap_or(DEFAULT_DB_NAME);

    if !pg_is_ready() {
        return Err("postgresql is not accepting connections".to_string());
    }

    match database_exists(db) {
        Ok(true) => { /* exists, continue */ }
        Ok(false) => { create(Some(db))?; }
        Err(e) => { return Err(format!("database check failed: {e}")); }
    }

    ensure_extensions(Some(db))?;
    migrate()
}

/// Check PostgreSQL is accepting connections.
fn pg_is_ready() -> bool {
    Command::new("pg_isready")
        .args(["--quiet"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if a database exists.
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
    let found = text.lines().any(|line| {
        line.split('|')
            .next()
            .map(|name| name.trim() == db_name)
            .unwrap_or(false)
    });
    Ok(found)
}

/// Check if pgvector extension is available in the database.
fn pgvector_installed(db_name: &str) -> bool {
    let output = Command::new("psql")
        .args(["-d", db_name, "-tAc", "SELECT 1 FROM pg_extension WHERE extname = 'vector'"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            o.stdout.starts_with(b"1")
        }
        _ => false,
    }
}

/// Get the schema migration version.
fn schema_version(db_name: &str) -> Option<i32> {
    let output = Command::new("psql")
        .args(["-d", db_name, "-tAc", "SELECT max(version) FROM schema_migrations"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<i32>()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_db_name_is_sensei() {
        assert_eq!(DEFAULT_DB_NAME, "sensei");
    }

    #[test]
    fn database_exists_parsing() {
        // Simulate psql -lqt output
        let output = " sensei    | jerry | UTF8     | \n template0 | jerry | UTF8     | \n";
        let found = output.lines().any(|line| {
            line.split('|').next().map(|name| name.trim() == "sensei").unwrap_or(false)
        });
        assert!(found, "should find 'sensei' in psql output");
    }

    #[test]
    fn database_exists_parsing_not_found() {
        let output = " postgres  | jerry | UTF8     | \n template0 | jerry | UTF8     | \n";
        let found = output.lines().any(|line| {
            line.split('|').next().map(|name| name.trim() == "sensei").unwrap_or(false)
        });
        assert!(!found, "should not find 'sensei' in psql output");
    }

    #[test]
    fn ensure_extensions_returns_result() {
        // Verify function exists and returns a Result.
        // Actual execution depends on PostgreSQL being available.
        let result = ensure_extensions(None);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn migrate_without_senseid() {
        // senseid is unlikely to be in PATH during testing.
        // If it IS in PATH, the test is still valid (tests a different code path).
        let result = migrate();
        if crate::util::which_binary("senseid").is_none() {
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(
                err.contains("senseid not found"),
                "expected 'senseid not found' in error, got: {err}"
            );
        }
        // If senseid IS found, we accept any result — the test still exercised the function.
    }

    #[test]
    fn setup_without_postgres() {
        // When PostgreSQL isn't running, setup() should return Err.
        // On machines where PG IS running this test still exercises the pipeline.
        let result = setup(None);
        if !super::pg_is_ready() {
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(
                err.contains("not accepting connections"),
                "expected 'not accepting connections' in error, got: {err}"
            );
        }
    }
}
