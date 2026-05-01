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

/// Checks that a binary exists AND its service port is open.
pub struct BinaryAndPortChecker {
    pub binary: String,
    pub version_flag: String,
    pub name: String,
    pub port: u16,
}

impl BinaryAndPortChecker {
    pub fn new(
        binary: impl Into<String>,
        version_flag: impl Into<String>,
        name: impl Into<String>,
        port: u16,
    ) -> Self {
        Self { binary: binary.into(), version_flag: version_flag.into(), name: name.into(), port }
    }
}

impl Checker for BinaryAndPortChecker {
    fn check(&self) -> CheckResult {
        let path = match util::which_binary(&self.binary) {
            None => return CheckResult::fail(format!("{} binary not found in PATH", self.binary)),
            Some(p) => p,
        };
        if !util::probe_port(self.port) {
            return CheckResult::fail(format!(
                "{} installed at {} but service not running on port {}",
                self.binary, path, self.port
            ));
        }
        let version = util::fetch_service_version(&self.name, self.port)
            .or_else(|| util::binary_version(&self.binary, &self.version_flag))
            .unwrap_or_else(|| "unknown".to_string());
        CheckResult::ok_with_detail(version, path)
    }
}

/// Checks whether the sensei database is ready (pg_isready + DB exists + pgvector).
pub struct DatabaseChecker;

impl Checker for DatabaseChecker {
    fn check(&self) -> CheckResult {
        let status = database::check(None);
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
    fn binary_and_port_checker_missing_binary_returns_fail() {
        let checker = BinaryAndPortChecker::new(
            "sensei-nonexistent-xyz-binary", "--version", "test", 9999,
        );
        let result = checker.check();
        assert!(!result.ok);
        assert!(result.error.as_deref().unwrap().contains("not found"));
    }

    #[test]
    fn binary_checker_error_contains_binary_name() {
        let checker = BinaryChecker::new("totally-missing-binary", "--version");
        let result = checker.check();
        assert!(result.error.as_deref().unwrap().contains("totally-missing-binary"));
    }
}
