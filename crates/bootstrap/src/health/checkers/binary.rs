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
        tracing::debug!(check = "binary", binary = self.bin, "looking up on PATH");
        // Resolve to an absolute path via which_binary's PATH lookup + the
        // EXTRA_BIN_DIRS fallback (covers /opt/homebrew/bin, /usr/local/bin,
        // /home/linuxbrew/.linuxbrew/bin). The fallback is what keeps the
        // Tauri .app working — its launched-from-Finder PATH is narrow and
        // doesn't include /opt/homebrew/bin, so without this we'd report
        // brew/pg_isready/ollama as "not installed" on every prod cold-start
        // even when they exist on the user's machine.
        let resolved = match crate::util::which_binary(self.bin) {
            None => {
                tracing::info!(check = "binary", binary = self.bin, result = "not_found", "binary not on PATH");
                return CheckOutcome::failed(format!("{} not installed", self.bin));
            }
            Some(path) => {
                tracing::debug!(check = "binary", binary = self.bin, path = %path, "located");
                path
            }
        };
        match self.version_arg {
            None => {
                tracing::debug!(check = "binary", binary = self.bin, result = "ready", "ready (no version probe)");
                CheckOutcome::ready_no_version()
            }
            Some(arg) => {
                tracing::debug!(check = "binary", binary = self.bin, arg = arg, path = %resolved, "probing version");
                // Spawn the ABSOLUTE path from which_binary, not `self.bin`.
                // Using `self.bin` would re-search the process's PATH and
                // fail with ENOENT in any environment whose PATH is narrower
                // than the dirs scanned by which_binary's fallback. That's
                // exactly how the prod desktop app surfaced "brew: No such
                // file or directory (os error 2)" while brew was installed
                // at /opt/homebrew/bin/brew the whole time.
                let mut cmd = Command::new(&resolved);
                cmd.arg(arg);
                match output_with_timeout(cmd, DEFAULT_CHECKER_TIMEOUT) {
                    TimedOutcome::Done(out) if out.status.success() => {
                        let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        let v = if raw.is_empty() { "unknown".to_string() } else { raw };
                        tracing::debug!(check = "binary", binary = self.bin, result = "ready", version = %v, "ready");
                        CheckOutcome::ready(v)
                    }
                    TimedOutcome::Done(out) => {
                        tracing::warn!(check = "binary", binary = self.bin, exit = %out.status, "version probe exited non-zero");
                        CheckOutcome::failed(format!("{} {} exited {}", self.bin, arg, out.status))
                    }
                    TimedOutcome::TimedOut => {
                        tracing::warn!(check = "binary", binary = self.bin, "version probe timed out");
                        CheckOutcome::failed(format!("{} {} timed out after {}s", self.bin, arg, DEFAULT_CHECKER_TIMEOUT.as_secs()))
                    }
                    TimedOutcome::Failed(e) => {
                        tracing::warn!(check = "binary", binary = self.bin, error = %e, path = %resolved, "spawn failed even after which_binary resolved an absolute path");
                        CheckOutcome::failed(format!("{}: {e}", self.bin))
                    }
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
        // Message says "not installed" — readable by end users in the
        // health ledger, no raw "os error 2" leakage.
        let detail = o.detail.as_deref().unwrap();
        assert!(detail.contains("not installed"),
            "detail should say 'not installed', got: {}", detail);
        assert!(!detail.contains("os error"),
            "detail must not leak raw OS error codes, got: {}", detail);
    }

    /// Regression for the prod-desktop bug: BinaryChecker must spawn the
    /// absolute path resolved by which_binary, not re-search PATH by name.
    /// In the Tauri .app, the launched-from-Finder PATH doesn't include
    /// /opt/homebrew/bin, so spawning by bare name fails with ENOENT even
    /// when the binary exists at the resolved path.
    #[test]
    fn version_probe_succeeds_for_binary_resolved_via_fallback() {
        // Use `ls` — present on every unix, has --version on GNU coreutils;
        // on macOS the BSD ls doesn't, so we use bash --version which works
        // in both. The point of this test is that the *spawn* succeeds —
        // we don't care which exit code --version returns, only that we
        // don't get ENOENT.
        let c = BinaryChecker::with_version("bash", "--version");
        let o = c.check();
        if cfg!(unix) {
            assert!(
                !matches!(o.status, ComponentStatus::Failed)
                || !o.detail.as_deref().unwrap_or("").contains("os error"),
                "version probe must not surface raw OS errors when the binary is resolvable; got status={:?} detail={:?}",
                o.status, o.detail,
            );
        }
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
