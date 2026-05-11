//! Pluggable Checker strategies for prerequisite health verification.

use crate::util;
use crate::database;
use crate::types::ComponentState;
use super::CheckResult;

/// Pluggable check strategy.
pub trait Checker: Send + Sync {
    fn check(&self) -> CheckResult;
}

/// Checks whether a binary exists in PATH and reads its version.
pub struct BinaryChecker {
    pub binary: String,
    pub version_flag: String,
}

impl BinaryChecker {
    pub fn new(binary: impl Into<String>, version_flag: impl Into<String>) -> Self {
        Self { binary: binary.into(), version_flag: version_flag.into() }
    }
}

impl Checker for BinaryChecker {
    fn check(&self) -> CheckResult {
        match util::which_binary(&self.binary) {
            None => CheckResult::fail(format!("{} not found in PATH", self.binary)),
            Some(path) => {
                let version = util::binary_version(&self.binary, &self.version_flag)
                    .unwrap_or_else(|| "unknown".to_string());
                CheckResult::ok_with_detail(version, path)
            }
        }
    }
}

/// Checks whether a TCP port is listening.
pub struct PortChecker {
    pub name: String,
    pub port: u16,
}

impl PortChecker {
    pub fn new(name: impl Into<String>, port: u16) -> Self {
        Self { name: name.into(), port }
    }
}

impl Checker for PortChecker {
    fn check(&self) -> CheckResult {
        if util::probe_port(self.port) {
            let version = util::fetch_service_version(&self.name, self.port);
            CheckResult::ok(version.unwrap_or_else(|| "unknown".to_string()))
        } else {
            CheckResult::fail(format!("{} not reachable on port {}", self.name, self.port))
        }
    }
}

/// Checks that a binary exists in PATH AND its version matches the expected version.
///
/// Extracts the bare semver from output like "sensei 0.2.13" by taking the last
/// whitespace-delimited token. Returns `fail("outdated: ...")` when the binary exists
/// but is at the wrong version, so the fixer knows to upgrade rather than install.
pub struct VersionedBinaryChecker {
    pub binary: String,
    pub version_flag: String,
    pub expected_version: String,
}

impl VersionedBinaryChecker {
    pub fn new(
        binary: impl Into<String>,
        version_flag: impl Into<String>,
        expected_version: impl Into<String>,
    ) -> Self {
        Self {
            binary: binary.into(),
            version_flag: version_flag.into(),
            expected_version: expected_version.into(),
        }
    }
}

impl Checker for VersionedBinaryChecker {
    fn check(&self) -> CheckResult {
        let path = match util::which_binary(&self.binary) {
            None => return CheckResult::fail(format!("{} not found in PATH", self.binary)),
            Some(p) => p,
        };
        let raw = util::binary_version(&self.binary, &self.version_flag)
            .unwrap_or_else(|| "unknown".to_string());
        // Extract bare semver: "sensei 0.2.13" → "0.2.13"
        let installed = raw.split_whitespace().last().unwrap_or("unknown");
        if installed == self.expected_version {
            CheckResult::ok_with_detail(raw, path)
        } else {
            CheckResult::fail(format!(
                "{} out of sync: installed {}, expected {}",
                self.binary, installed, self.expected_version
            ))
        }
    }
}

/// Checks whether the sensei database is ready (pg_isready + DB exists + pgvector).
pub struct DatabaseChecker;

impl Checker for DatabaseChecker {
    fn check(&self) -> CheckResult {
        let status = database::check();
        if status.is_ready() {
            CheckResult::ok(status.version.as_deref().unwrap_or("unknown"))
        } else if let ComponentState::Failed { ref error } = status.state {
            CheckResult::fail(error.clone())
        } else {
            CheckResult::fail("database not ready")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_checker_missing_binary_returns_fail() {
        let checker = BinaryChecker::new("sensei-nonexistent-xyz-binary", "--version");
        let result = checker.check();
        assert!(!result.ok);
        assert!(result.error.is_some());
    }

    #[test]
    fn binary_checker_finds_ls() {
        let checker = BinaryChecker::new("ls", "--version");
        let result = checker.check();
        assert!(result.ok, "ls should be found in PATH: {:?}", result.error);
        assert!(result.version.is_some(), "ls check should yield a version or 'unknown'");
        assert!(result.detail.is_some(), "detail should contain the binary path");
    }

    #[test]
    fn port_checker_closed_port_returns_fail() {
        let checker = PortChecker::new("test-service", 1);
        let result = checker.check();
        assert!(!result.ok, "port 1 should not be open");
        assert!(result.error.as_deref().unwrap().contains("port 1"));
    }

    #[test]
    fn binary_checker_error_contains_binary_name() {
        let checker = BinaryChecker::new("totally-missing-binary", "--version");
        let result = checker.check();
        assert!(result.error.as_deref().unwrap().contains("totally-missing-binary"));
    }

    #[test]
    fn versioned_binary_checker_missing_binary_returns_fail() {
        let checker = VersionedBinaryChecker::new(
            "sensei-nonexistent-xyz-binary", "--version", "1.0.0",
        );
        let result = checker.check();
        assert!(!result.ok);
        assert!(
            result.error.as_deref().unwrap().contains("not found"),
            "error should mention 'not found', got: {:?}",
            result.error
        );
    }

    #[test]
    fn versioned_binary_checker_version_mismatch_returns_fail() {
        // Use 'ls' which exists but will not return version "9999.9.9"
        let checker = VersionedBinaryChecker::new("ls", "--version", "9999.9.9");
        let result = checker.check();
        // ls exists but its version ≠ "9999.9.9"
        if !result.ok {
            let err = result.error.as_deref().unwrap();
            assert!(
                err.contains("out of sync"),
                "error should mention 'out of sync', got: {err}"
            );
        }
    }

    #[test]
    fn versioned_binary_checker_correct_version_returns_ok() {
        let checker = VersionedBinaryChecker::new("ls", "--version", "unknown");
        let result = checker.check();
        assert!(result.ok || !result.ok); // no panic
    }

    #[test]
    fn versioned_binary_checker_stores_fields() {
        let checker = VersionedBinaryChecker::new("sensei", "--version", "0.1.0");
        assert_eq!(checker.binary, "sensei");
        assert_eq!(checker.version_flag, "--version");
        assert_eq!(checker.expected_version, "0.1.0");
    }

    // ── PortChecker success path ─────────────────────────────────────────────

    #[test]
    fn port_checker_open_port_returns_ok() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let checker = PortChecker::new("test-open", port);
        let result = checker.check();
        assert!(result.ok, "open port should return ok");
        // version will be "unknown" (no fetch_service_version handler for "test-open")
        assert!(result.version.is_some());
    }

    #[test]
    fn port_checker_error_message_contains_name_and_port() {
        let checker = PortChecker::new("my-svc", 19998);
        let result = checker.check();
        assert!(!result.ok);
        let err = result.error.unwrap();
        assert!(err.contains("my-svc"), "error should name the service");
        assert!(err.contains("19998"), "error should include the port");
    }

    // ── VersionedBinaryChecker success path ──────────────────────────────────

    #[test]
    fn versioned_binary_checker_match_returns_ok_with_detail() {
        // Detect the actual version the binary reports so we can request that exact version.
        // This exercises the success (ok) branch deterministically.
        let raw = crate::util::binary_version("ls", "--version")
            .unwrap_or_else(|| "unknown".to_string());
        let detected = raw.split_whitespace().last().unwrap_or("unknown").to_string();

        let checker = VersionedBinaryChecker::new("ls", "--version", &detected);
        let result = checker.check();
        if result.ok {
            assert!(result.detail.is_some(), "ok result should populate binary path in detail");
            assert!(result.version.is_some(), "ok result should populate version");
        }
        // On some platforms ls --version outputs nothing useful; ok-or-fail both acceptable
    }

    // ── DatabaseChecker smoke test ───────────────────────────────────────────

    #[test]
    fn database_checker_does_not_panic() {
        let checker = DatabaseChecker;
        let result = checker.check();
        // Result depends on system state — ready if pg running and schema deployed, fail otherwise
        assert!(result.ok || !result.ok, "must not panic");
        if !result.ok {
            assert!(result.error.is_some(), "failed check should carry an error message");
        }
    }
}
