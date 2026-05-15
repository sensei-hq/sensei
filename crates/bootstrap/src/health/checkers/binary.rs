//! Check whether a binary exists on PATH and optionally probe its version.

use std::process::Command;
use crate::health::checker::{Checker, CheckOutcome};
use crate::health::process_util::{output_with_timeout, TimedOutcome, DEFAULT_CHECKER_TIMEOUT};

pub struct BinaryChecker {
    pub bin:         &'static str,
    pub version_arg: Option<&'static str>,
}

impl BinaryChecker {
    pub const fn new(bin: &'static str) -> Self {
        Self { bin, version_arg: None }
    }
    pub const fn with_version(bin: &'static str, arg: &'static str) -> Self {
        Self { bin, version_arg: Some(arg) }
    }
}

impl Checker for BinaryChecker {
    fn check(&self) -> CheckOutcome {
        if crate::util::which_binary(self.bin).is_none() {
            return CheckOutcome::failed(format!("{} not found on PATH", self.bin));
        }
        match self.version_arg {
            None => CheckOutcome::ready_no_version(),
            Some(arg) => {
                let mut cmd = Command::new(self.bin);
                cmd.arg(arg);
                match output_with_timeout(cmd, DEFAULT_CHECKER_TIMEOUT) {
                    TimedOutcome::Done(out) if out.status.success() => {
                        let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        let v = if raw.is_empty() { "unknown".to_string() } else { raw };
                        CheckOutcome::ready(v)
                    }
                    TimedOutcome::Done(out) =>
                        CheckOutcome::failed(format!("{} {} exited {}", self.bin, arg, out.status)),
                    TimedOutcome::TimedOut =>
                        CheckOutcome::failed(format!("{} {} timed out after {}s", self.bin, arg, DEFAULT_CHECKER_TIMEOUT.as_secs())),
                    TimedOutcome::Failed(e) =>
                        CheckOutcome::failed(format!("{}: {e}", self.bin)),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::types::ComponentStatus;

    #[test]
    fn missing_binary_returns_failed() {
        let c = BinaryChecker::new("definitely_not_a_real_binary_xyz_77");
        let o = c.check();
        assert!(matches!(o.status, ComponentStatus::Failed));
        assert!(o.detail.as_deref().unwrap().contains("not found on PATH"));
    }

    #[test]
    fn present_binary_without_version_arg_is_ready_no_version() {
        // `ls` exists on every unix system and we use it without --version.
        let c = BinaryChecker::new("ls");
        let o = c.check();
        if cfg!(unix) {
            assert!(matches!(o.status, ComponentStatus::Ready));
            assert!(o.version.is_none());
        }
    }

    #[test]
    fn present_binary_with_version_returns_ready_with_version_string() {
        // `ls --version` works on GNU coreutils; on macOS `ls` doesn't
        // support --version (BSD). To keep this test portable, use `bash --version`
        // which works on both.
        let c = BinaryChecker::with_version("bash", "--version");
        let o = c.check();
        if cfg!(unix) {
            assert!(matches!(o.status, ComponentStatus::Ready));
            assert!(o.version.is_some(), "version must be populated");
        }
    }
}
