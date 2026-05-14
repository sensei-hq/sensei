//! Check that the sensei database exists with pgvector + the expected
//! `sessions` table.

use std::process::Command;
use crate::health::checker::{Checker, CheckOutcome};

pub struct PostgresDatabaseChecker {
    pub db_name: String,
}

impl Checker for PostgresDatabaseChecker {
    fn check(&self) -> CheckOutcome {
        // 1) database exists?
        let exists = Command::new("psql")
            .args(["-tAc", &format!("SELECT 1 FROM pg_database WHERE datname='{}'", self.db_name)])
            .output();
        match exists {
            Ok(o) if o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "1" => {}
            Ok(o) => return CheckOutcome::failed(format!(
                "database {} not found (psql exit {}): {}",
                self.db_name, o.status, String::from_utf8_lossy(&o.stderr).trim()
            )),
            Err(e) => return CheckOutcome::failed(format!("psql failed: {e}")),
        }
        // 2) pgvector + sessions table present?
        let check_sql =
            "SELECT 1 FROM pg_extension WHERE extname='vector' UNION ALL \
             SELECT 1 FROM information_schema.tables WHERE table_name='sessions' AND table_schema='public'";
        let probe = Command::new("psql")
            .args(["-d", &self.db_name, "-tAc", check_sql])
            .output();
        match probe {
            Ok(o) if o.status.success() => {
                let count = String::from_utf8_lossy(&o.stdout).lines().count();
                if count >= 2 {
                    CheckOutcome::ready_no_version()
                } else {
                    CheckOutcome::failed(
                        "database exists but pgvector or sessions table missing".to_string()
                    )
                }
            }
            Ok(o) => CheckOutcome::failed(format!(
                "schema probe failed: {}", String::from_utf8_lossy(&o.stderr).trim()
            )),
            Err(e) => CheckOutcome::failed(format!("psql: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::types::ComponentStatus;

    /// Smoke test — the checker constructs without panic and produces an
    /// outcome (likely Failed unless psql + db are available on the CI host;
    /// either is OK here, we just want to exercise the code path).
    #[test]
    fn construct_and_check_does_not_panic() {
        let c = PostgresDatabaseChecker { db_name: "definitely_not_a_real_db_xyz".to_string() };
        let o = c.check();
        // We don't assert status — psql might be missing OR the DB doesn't exist;
        // both produce Failed; what matters is no panic and the outcome is valid.
        assert!(matches!(o.status, ComponentStatus::Failed | ComponentStatus::Ready));
    }
}
