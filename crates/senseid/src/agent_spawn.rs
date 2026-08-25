//! Relay P3.3a — the agent-spawn primitive.
//!
//! A self-contained, tested async wrapper for spawning and supervising a child
//! process with a hard wall-clock timeout. This is the foundation for P3.3b
//! (driving `claude -p` headless per feature) — but this chunk deliberately
//! spawns nothing real: it is only the primitive plus unit tests against stub
//! commands (`echo`, `sh -c …`, `cat`, `sleep`). It is NOT wired into the run
//! engine (`AdvanceRun`) here.
//!
//! Safety contract (relay-engine): the spawned child is bounded by a timeout,
//! never leaks (no zombies), is killed on timeout AND on handle-drop, its
//! output is captured, and no path panics. We use async [`tokio::process`]
//! throughout — never `std::process` + `spawn_blocking`.
//!
//! ## How the no-zombie guarantee holds
//!
//! - `.kill_on_drop(true)` on the [`tokio::process::Command`] means that if the
//!   [`tokio::process::Child`] handle is dropped early (e.g. this future is
//!   cancelled), tokio sends SIGKILL to the child. Tokio's process driver also
//!   reaps the killed pid, so a dropped handle does not leave a zombie.
//! - On timeout we do NOT drop-and-hope: we explicitly `start_kill()` (send
//!   SIGKILL) and then `wait().await` the child, which reaps it and yields its
//!   exit status. Only after the child is confirmed dead do we return.
//! - On normal completion we `wait().await` the child, which reaps it.
//! - stdout/stderr are drained by dedicated tasks so a child that fills a pipe
//!   buffer can still exit (and be reaped) rather than deadlocking against a
//!   full pipe. Those drain tasks are awaited (with a bounded timeout) after the
//!   child is reaped so their captured bytes are available even on timeout.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

/// The captured result of one supervised agent process.
///
/// Serde-derived because callers (P3.3b onward) log it and surface it on run
/// events. `exit_code` is `None` when the process was killed (timeout) or died
/// on a signal without a numeric exit code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentOutput {
    /// Lossy-UTF8 decode of everything the child wrote to stdout.
    pub stdout: String,
    /// Lossy-UTF8 decode of everything the child wrote to stderr.
    pub stderr: String,
    /// The child's exit code, or `None` if it was killed / had no code.
    pub exit_code: Option<i32>,
    /// `true` iff the child hit the wall-clock timeout and was killed.
    pub timed_out: bool,
}

/// A failed agent spawn/supervision. Graceful by design — mirrors the house
/// error style (see [`crate::dojo::client::DojoClientError`]): `Debug` derive,
/// hand-written `Display`, `impl std::error::Error`. A *timeout* is NOT an
/// error — it resolves to an `Ok(AgentOutput { timed_out: true, .. })` so the
/// caller can inspect partial output; these variants cover only cases where no
/// meaningful output could be produced.
#[derive(Debug)]
pub enum AgentSpawnError {
    /// The process could not be launched (bad program, missing binary, EACCES).
    Spawn(String),
    /// An I/O error while writing stdin, draining pipes, or reaping the child.
    Io(String),
    /// The child could not be terminated/reaped after a timeout kill.
    Killed(String),
}

impl std::fmt::Display for AgentSpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentSpawnError::Spawn(m) => write!(f, "agent spawn failed: {m}"),
            AgentSpawnError::Io(m) => write!(f, "agent i/o error: {m}"),
            AgentSpawnError::Killed(m) => write!(f, "agent kill/reap failed: {m}"),
        }
    }
}

impl std::error::Error for AgentSpawnError {}

/// A fully-specified request to run one agent process. Plain fields (ergonomic
/// enough — a builder would be over-engineering for the primitive) plus a
/// couple of constructors for the common shapes.
#[derive(Debug, Clone)]
pub struct AgentCommand {
    /// The program to exec (looked up on PATH by the OS if not absolute).
    pub program: String,
    /// Arguments passed verbatim (no shell interpolation).
    pub args: Vec<String>,
    /// Working directory for the child; the parent's cwd is inherited if `None`.
    pub cwd: Option<PathBuf>,
    /// Bytes to write to the child's stdin, then close it. `None` → stdin is
    /// `/dev/null` so the child never blocks waiting for input.
    pub stdin: Option<String>,
    /// Hard wall-clock cap. On expiry the child is killed and reaped and the
    /// result is `AgentOutput { timed_out: true, .. }`.
    pub timeout: Duration,
}

impl AgentCommand {
    /// Build a command from a program and its args with a timeout, no stdin and
    /// the inherited cwd. Chain [`AgentCommand::with_cwd`] /
    /// [`AgentCommand::with_stdin`] to set the rest.
    pub fn new(program: impl Into<String>, args: Vec<String>, timeout: Duration) -> Self {
        Self { program: program.into(), args, cwd: None, stdin: None, timeout }
    }

    /// Run the child in `cwd`.
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Feed `input` to the child's stdin (then close it).
    pub fn with_stdin(mut self, input: impl Into<String>) -> Self {
        self.stdin = Some(input.into());
        self
    }
}

/// A bounded grace period for the stdout/stderr drain tasks to finish after the
/// child has already been reaped. The pipes are closed once the child is gone,
/// so these reads return promptly; the cap only guards against a pathological
/// hang and keeps `run_agent` from ever blocking unboundedly.
const DRAIN_GRACE: Duration = Duration::from_secs(2);

/// Spawn `cmd` and supervise it to completion or the timeout, whichever comes
/// first. Never panics; the child is always killed + reaped (see module docs).
///
/// - Normal exit → `AgentOutput { timed_out: false, exit_code, stdout, stderr }`.
/// - Timeout → the child is SIGKILLed and reaped, then
///   `AgentOutput { timed_out: true, exit_code: None, .. }` with whatever was
///   captured before the deadline.
/// - Spawn failure → `Err(AgentSpawnError::Spawn(_))`.
pub async fn run_agent(cmd: &AgentCommand) -> Result<AgentOutput, AgentSpawnError> {
    let mut command = Command::new(&cmd.program);
    command.args(&cmd.args);
    if let Some(dir) = &cmd.cwd {
        command.current_dir(dir);
    }
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Close stdin unless the caller supplies input, so the child never
        // blocks on a read from an inherited-but-empty stdin.
        .stdin(if cmd.stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        // Belt-and-braces: if this future is cancelled and the Child handle is
        // dropped, tokio SIGKILLs + reaps the child rather than leaking it.
        .kill_on_drop(true);

    let mut child = command.spawn().map_err(|e| AgentSpawnError::Spawn(e.to_string()))?;

    // Take the stdin handle (if any) BEFORE we spawn the drains and start
    // waiting. We write to it concurrently with — and under the same deadline
    // as — the wait, so a large stdin (> the ~64KB OS pipe buffer) can never
    // deadlock against a child that fills its own stdout/stderr while reading.
    let stdin = if cmd.stdin.is_some() {
        Some(
            child
                .stdin
                .take()
                .ok_or_else(|| AgentSpawnError::Io("child stdin unavailable".into()))?,
        )
    } else {
        None
    };

    // Drain stdout/stderr concurrently FIRST — before any stdin write — so a
    // child that fills a pipe buffer while it consumes stdin can still make
    // progress and exit instead of deadlocking on a full stdout/stderr pipe
    // with nobody draining it. Taking the handles here also frees `child` to be
    // `wait()`ed.
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let stdout_task = tokio::spawn(drain(stdout_pipe));
    let stderr_task = tokio::spawn(drain(stderr_pipe));

    // Feed stdin concurrently with the wait, all under the single deadline. The
    // stdin write must never escape the timeout budget: if it blocks (child not
    // reading, pipe full) the whole `timeout` wrapper trips and we kill+reap,
    // exactly like a slow child. `feed_stdin` closes the pipe on completion so
    // the child sees EOF; a short-reading child that closes its read end early
    // yields an EPIPE which `feed_stdin` swallows (see its docs).
    let input = cmd.stdin.clone().unwrap_or_default();
    let supervise = async {
        // Run the stdin write and the child wait concurrently. Draining the
        // write buffer as the child reads is what keeps a large stdin from
        // deadlocking; a child that never finishes reading is bounded by the
        // outer `timeout`, not by this write.
        let feeder = feed_stdin(stdin, input);
        let waiter = child.wait();
        tokio::pin!(feeder);
        tokio::pin!(waiter);
        tokio::select! {
            // stdin fully written (or its write errored/EPIPE — swallowed) and
            // the pipe is closed. Now just wait for the child to exit.
            _ = &mut feeder => (&mut waiter).await,
            // Child exited before we finished feeding stdin (e.g. a
            // short-reading child). Its status is authoritative; stop feeding.
            status = &mut waiter => status,
        }
    };

    // Wait for the child (and stdin feed) under the deadline. On timeout we
    // still hold `child`, so we can kill + reap it explicitly.
    let (exit_code, timed_out) = match tokio::time::timeout(cmd.timeout, supervise).await {
        Ok(status_res) => {
            let status = status_res.map_err(|e| AgentSpawnError::Io(format!("wait: {e}")))?;
            (status.code(), false)
        }
        Err(_) => {
            // Deadline hit — either the child ran long or the stdin write
            // blocked on a full pipe. Either way: SIGKILL then reap so no
            // zombie is left behind, and report a timeout.
            child.start_kill().map_err(|e| AgentSpawnError::Killed(format!("start_kill: {e}")))?;
            child
                .wait()
                .await
                .map_err(|e| AgentSpawnError::Killed(format!("reap after kill: {e}")))?;
            (None, true)
        }
    };

    // The child is reaped and its pipes are closed, so the drain tasks return
    // promptly; the grace timeout only guards against a pathological hang.
    let stdout = join_drain(stdout_task).await;
    let stderr = join_drain(stderr_task).await;

    Ok(AgentOutput { stdout, stderr, exit_code, timed_out })
}

/// Write `input` to the child's stdin (if any) and close the pipe so the child
/// sees EOF. Errors are intentionally swallowed: a child that stops reading and
/// exits early (e.g. `head -c 10`) closes its read end, and the parent's
/// remaining `write_all` then hits `BrokenPipe`/EPIPE. That is NOT a failure —
/// the child may still have succeeded — so we ignore the write error, close the
/// pipe, and let the caller reap the child normally. When there is no stdin the
/// future resolves immediately. Dropping the handle (via `shutdown` then scope
/// exit) closes the pipe.
async fn feed_stdin(stdin: Option<tokio::process::ChildStdin>, input: String) {
    if let Some(mut stdin) = stdin {
        // Ignore a broken-pipe/write error: a short-reading child that has
        // already exited is a success case, not a hard failure.
        let _ = stdin.write_all(input.as_bytes()).await;
        // `shutdown` flushes then half-closes; also non-fatal on a gone child.
        let _ = stdin.shutdown().await;
        // Explicit drop closes the pipe → EOF for a child still reading.
        drop(stdin);
    }
}

/// Read an optional child pipe to end, returning a lossy-UTF8 string. A `None`
/// pipe (should not happen given we pipe both) yields an empty string.
async fn drain<R: AsyncReadExt + Unpin>(pipe: Option<R>) -> String {
    let mut buf = Vec::new();
    if let Some(mut pipe) = pipe {
        // Ignore read errors: a killed child's pipe may error mid-read; we
        // still return whatever bytes we captured up to that point.
        let _ = pipe.read_to_end(&mut buf).await;
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// Await a drain task within [`DRAIN_GRACE`], returning "" on join failure or
/// grace-timeout so a hung/aborted drainer never blocks or panics the caller.
async fn join_drain(task: tokio::task::JoinHandle<String>) -> String {
    match tokio::time::timeout(DRAIN_GRACE, task).await {
        Ok(Ok(s)) => s,
        // JoinError (panic/cancel) or grace-timeout → best-effort empty.
        Ok(Err(_)) | Err(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn short() -> Duration {
        Duration::from_secs(10)
    }

    #[tokio::test]
    async fn success_captures_stdout_and_zero_exit() {
        let cmd = AgentCommand::new("echo", vec!["hello".into()], short());
        let out = run_agent(&cmd).await.expect("echo should run");
        assert!(out.stdout.contains("hello"), "stdout was {:?}", out.stdout);
        assert_eq!(out.exit_code, Some(0));
        assert!(!out.timed_out);
    }

    #[tokio::test]
    async fn nonzero_exit_is_captured() {
        let cmd = AgentCommand::new("sh", vec!["-c".into(), "exit 3".into()], short());
        let out = run_agent(&cmd).await.expect("sh should run");
        assert_eq!(out.exit_code, Some(3));
        assert!(!out.timed_out);
    }

    #[tokio::test]
    async fn timeout_kills_child_quickly() {
        // `sleep 5` under a 300ms cap must return in ~300ms (proving the kill),
        // NOT after 5s. We assert timed_out and a wall-clock well under 5s.
        let cmd = AgentCommand::new("sleep", vec!["5".into()], Duration::from_millis(300));
        let started = Instant::now();
        let out = run_agent(&cmd).await.expect("sleep should spawn");
        let elapsed = started.elapsed();

        assert!(out.timed_out, "expected timed_out");
        assert_eq!(out.exit_code, None);
        assert!(
            elapsed < Duration::from_secs(2),
            "kill did not take effect: elapsed {elapsed:?} (child likely ran the full 5s)"
        );
    }

    #[tokio::test]
    async fn timeout_leaves_no_lingering_process() {
        // Give the child a distinctive marker arg so we can grep for it, then
        // confirm no process with that marker survives the kill+reap.
        let marker = format!("agent_spawn_probe_{}", std::process::id());
        // `sh -c 'exec -a <marker> sleep 30'`-style isn't portable; instead run
        // sleep with the marker as a comment-ish arg via env so `ps` shows it.
        let cmd = AgentCommand::new(
            "sh",
            vec!["-c".into(), format!("sleep 30 # {marker}")],
            Duration::from_millis(300),
        );
        let out = run_agent(&cmd).await.expect("should spawn");
        assert!(out.timed_out);

        // Allow the OS a beat to finish reaping, then assert nothing lingers.
        tokio::time::sleep(Duration::from_millis(200)).await;
        let ps = std::process::Command::new("ps")
            .args(["-A", "-o", "command"])
            .output()
            .expect("ps should run");
        let listing = String::from_utf8_lossy(&ps.stdout);
        let lingering: Vec<&str> =
            listing.lines().filter(|l| l.contains(&marker) && !l.contains("ps -A")).collect();
        assert!(lingering.is_empty(), "child survived kill+reap: {lingering:?}");
    }

    #[tokio::test]
    async fn stdin_is_piped_to_child() {
        let cmd = AgentCommand::new("cat", vec![], short()).with_stdin("piped input");
        let out = run_agent(&cmd).await.expect("cat should run");
        assert!(out.stdout.contains("piped input"), "stdout was {:?}", out.stdout);
        assert_eq!(out.exit_code, Some(0));
        assert!(!out.timed_out);
    }

    #[tokio::test]
    async fn large_stdin_round_trips_without_deadlock() {
        // Regression for the bidirectional-pipe deadlock: a 256KB stdin (well
        // over the ~64KB OS pipe buffer) fed to `cat`, which echoes it back to
        // stdout WHILE it is still consuming stdin. Under the old ordering
        // (stdin fully written before the stdout/stderr drains were spawned)
        // the child would block writing to a full stdout pipe with nobody
        // draining it, while the parent blocked writing to a full stdin pipe —
        // a deadlock that would only unblock at the 10s timeout. With the fix
        // (drains spawned first, stdin written concurrently under the budget)
        // this completes in a few ms and the payload round-trips intact.
        let payload = "x".repeat(256 * 1024);
        let cmd = AgentCommand::new("cat", vec![], short()).with_stdin(payload.clone());
        let started = Instant::now();
        let out = run_agent(&cmd).await.expect("cat should run");
        let elapsed = started.elapsed();

        assert!(!out.timed_out, "large-stdin round-trip hit the timeout");
        assert_eq!(out.exit_code, Some(0));
        assert_eq!(
            out.stdout.len(),
            payload.len(),
            "cat did not echo the full payload back (got {} bytes, sent {})",
            out.stdout.len(),
            payload.len()
        );
        // Must complete far below the 10s timeout — proves no deadlock.
        assert!(
            elapsed < Duration::from_secs(3),
            "large stdin approached the timeout: elapsed {elapsed:?} (likely deadlocked under old ordering)"
        );
    }

    #[tokio::test]
    async fn short_reading_child_with_large_stdin_does_not_hang_or_error() {
        // A child that reads only a few bytes of a large stdin then exits
        // (`head -c 10`) closes its read end early. The parent's remaining
        // `write_all` then hits EPIPE / broken pipe. That must NOT become a
        // hard error or a hang: the child exited cleanly, so we expect a normal
        // (not-timed-out) result with the child's success exit code.
        let payload = "y".repeat(256 * 1024);
        let cmd =
            AgentCommand::new("sh", vec!["-c".into(), "head -c 10 >/dev/null".into()], short())
                .with_stdin(payload);
        let started = Instant::now();
        let out =
            run_agent(&cmd).await.expect("short-reading child should not surface a hard error");
        let elapsed = started.elapsed();

        assert!(!out.timed_out, "short-reading child hit the timeout");
        assert_eq!(
            out.exit_code,
            Some(0),
            "short-reading child should exit cleanly despite the parent's EPIPE"
        );
        assert!(elapsed < Duration::from_secs(3), "short-reading child hung: elapsed {elapsed:?}");
    }

    #[tokio::test]
    async fn spawn_failure_for_missing_program() {
        let cmd = AgentCommand::new("definitely_not_a_real_program_xyzzy_42", vec![], short());
        let err = run_agent(&cmd).await.expect_err("nonexistent program");
        match err {
            AgentSpawnError::Spawn(_) => {}
            other => panic!("expected Spawn error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn with_cwd_runs_in_directory() {
        // pwd should print the cwd we set. Use the OS temp dir (portable) and
        // compare on the trailing component to sidestep /var vs /private/var
        // symlink canonicalisation on macOS.
        let dir = std::env::temp_dir();
        let cmd = AgentCommand::new("pwd", vec![], short()).with_cwd(&dir);
        let out = run_agent(&cmd).await.expect("pwd should run");
        let last = dir.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        assert!(
            out.stdout.trim().ends_with(&last) || out.stdout.trim() == dir.to_string_lossy(),
            "pwd {:?} did not reflect cwd {:?}",
            out.stdout,
            dir
        );
    }
}
