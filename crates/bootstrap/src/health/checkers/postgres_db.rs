//! Check that the sensei database exists with pgvector + the expected
//! `sessions` table.

use std::process::Command;
use crate::health::checker::{Checker, CheckOutcome};
use crate::health::process_util::{output_with_timeout, TimedOutcome, DEFAULT_CHECKER_TIMEOUT};

pub struct PostgresDatabaseChecker {
    pub db_name: String,
}

impl Checker for PostgresDatabaseChecker {
    fn check(&self) -> CheckOutcome {
        // 1) database exists?
        let mut exists_cmd = Command::new("psql");
        exists_cmd.args(["-tAc", &format!("SELECT 1 FROM pg_database WHERE datname='{}'", self.db_name)]);
        match output_with_timeout(exists_cmd, DEFAULT_CHECKER_TIMEOUT) {
            TimedOutcome::Done(o) if o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "1" => {}
            TimedOutcome::Done(o) => return CheckOutcome::failed(format!(
                "database {} not found (psql exit {}): {}",
                self.db_name, o.status, String::from_utf8_lossy(&o.stderr).trim()
            )),
            TimedOutcome::TimedOut => return CheckOutcome::failed(format!(
                "psql timed out probing database existence after {}s", DEFAULT_CHECKER_TIMEOUT.as_secs()
            )),
            TimedOutcome::Failed(e) => return CheckOutcome::failed(format!("psql failed: {e}")),
        }
        // 2) pgvector extension + sensei schema deployed.
        // We probe for the `sensei` schema rather than a specific table —
        // the schema is the stable "is deploy done" signal, and survives
        // DDL reorganization (tables move between `sensei`/`activity`/
        // `extensions` schemas as the model evolves).
        let check_sql =
            "SELECT 1 FROM pg_extension WHERE extname='vector' UNION ALL \
             SELECT 1 FROM information_schema.schemata WHERE schema_name='sensei'";
        let mut probe_cmd = Command::new("psql");
        probe_cmd.args(["-d", &self.db_name, "-tAc", check_sql]);
        match output_with_timeout(probe_cmd, DEFAULT_CHECKER_TIMEOUT) {
            TimedOutcome::Done(o) if o.status.success() => {
                let count = String::from_utf8_lossy(&o.stdout).lines().count();
                if count >= 2 {
                    CheckOutcome::ready_no_version()
                } else {
                    CheckOutcome::failed(
                        "database exists but pgvector or sensei schema missing".to_string()
                    )
                }
            }
            TimedOutcome::Done(o) => CheckOutcome::failed(format!(
                "schema probe failed: {}", String::from_utf8_lossy(&o.stderr).trim()
            )),
            TimedOutcome::TimedOut => CheckOutcome::failed(format!(
                "psql timed out on schema probe after {}s", DEFAULT_CHECKER_TIMEOUT.as_secs()
            )),
            TimedOutcome::Failed(e) => CheckOutcome::failed(format!("psql: {e}")),
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
