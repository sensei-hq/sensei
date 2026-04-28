//! Database checks — PostgreSQL reachability, database existence, extensions.
//! Uses shell commands (pg_isready, psql), NOT sqlx — no database driver dependency.

use std::process::Command;

use crate::types::ComponentStatus;

const DEFAULT_DB_NAME: &str = "sensei";

/// Check if PostgreSQL is reachable and the sensei database exists.
pub fn check(db_name: Option<&str>) -> ComponentStatus {
    let db = db_name.unwrap_or(DEFAULT_DB_NAME);

    if !pg_is_ready() {
        return ComponentStatus::failed("database", "postgresql not reachable (pg_isready failed)");
    }

    if !database_exists(db) {
        return ComponentStatus::failed("database", &format!("database '{db}' does not exist"));
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

/// Check PostgreSQL is accepting connections.
fn pg_is_ready() -> bool {
    Command::new("pg_isready")
        .args(["--quiet"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Check if a database exists.
fn database_exists(db_name: &str) -> bool {
    let output = Command::new("psql")
        .args(["-lqt"])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            text.lines().any(|line| {
                line.split('|')
                    .next()
                    .map(|name| name.trim() == db_name)
                    .unwrap_or(false)
            })
        }
        _ => false,
    }
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
}
