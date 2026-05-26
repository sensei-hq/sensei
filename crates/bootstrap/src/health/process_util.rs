//! Run an external process with a wall-clock timeout.
//!
//! Health checkers shell out to `psql`, `brew`, version probes, etc. A hung
//! `pg_isready` or an unreachable-but-syn-pending checker would otherwise
//! leave the entire health phase spinning indefinitely (L6 in the
//! 2026-05-14 app-load review). This helper wraps `Command::output()` with a
//! deadline: if the child has not exited by then it is killed and the caller
//! sees a `TimedOut` outcome distinct from spawn failure.
//!
//! Implementation note: stdout/stderr are captured via piped descriptors
//! and drained after the child exits. The pipe buffer (~16KB on macOS) is
//! larger than any checker we currently invoke, so we do not need
//! concurrent draining. Callers that expect large output should not use
//! this helper.
use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Result of `output_with_timeout` — three distinct outcomes the caller
/// may want to surface differently to the user.
pub enum TimedOutcome {
    /// Child exited within the deadline. `Output` carries status + stdio.
    Done(Output),
    /// Deadline expired before exit; child was killed.
    TimedOut,
    /// Spawning or waiting on the child failed at the OS level.
    Failed(std::io::Error),
}

/// Spawn `cmd`, wait up to `timeout`, then kill if still running.
pub fn output_with_timeout(mut cmd: Command, timeout: Duration) -> TimedOutcome {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return TimedOutcome::Failed(e),
    };

    let deadline = Instant::now() + timeout;
    let poll = Duration::from_millis(50);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut s) = child.stdout.take() {
                    let _ = s.read_to_end(&mut stdout);
                }
                if let Some(mut s) = child.stderr.take() {
                    let _ = s.read_to_end(&mut stderr);
                }
                return TimedOutcome::Done(Output { status, stdout, stderr });
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return TimedOutcome::TimedOut;
                }
                thread::sleep(poll);
            }
            Err(e) => return TimedOutcome::Failed(e),
        }
    }
}

/// Default wall-clock budget for a single health checker invocation.
/// Five seconds covers a slow Homebrew cold-cache version probe but cuts
/// off a hung daemon long before the user wonders if the app froze.
pub const DEFAULT_CHECKER_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_command_returns_done() {
        let cmd = Command::new("true");
        match output_with_timeout(cmd, Duration::from_secs(2)) {
            TimedOutcome::Done(out) => assert!(out.status.success()),
            other => panic!("expected Done, got {:?}", match other {
                TimedOutcome::TimedOut => "TimedOut",
                TimedOutcome::Failed(_) => "Failed",
                _ => "?",
            }),
        }
    }

    #[test]
    fn slow_command_returns_timed_out_and_kills_child() {
        // `sleep 10` would normally block 10s; we expect TimedOut at ~200ms.
        let mut cmd = Command::new("sleep");
        cmd.arg("10");
        let start = Instant::now();
        let outcome = output_with_timeout(cmd, Duration::from_millis(200));
        let elapsed = start.elapsed();
        assert!(matches!(outcome, TimedOutcome::TimedOut));
        assert!(
            elapsed < Duration::from_secs(1),
            "expected to return near the 200ms timeout, took {:?}",
            elapsed,
        );
    }

    #[test]
    fn nonexistent_command_returns_failed() {
        let cmd = Command::new("definitely_not_a_real_binary_xyz_77");
        assert!(matches!(
            output_with_timeout(cmd, Duration::from_secs(1)),
            TimedOutcome::Failed(_),
        ));
    }
}
