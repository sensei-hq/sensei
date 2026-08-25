//! Diagnostic tracing for the health check/resolve pipeline (#39).
//!
//! Every `Command::new` site inside a `check()` or `resolve()` pass is wrapped
//! by a helper that records timing, exit status, and stdio into a shared
//! `TraceRecorder`. The daemon returns the accumulated traces alongside the
//! `HealthPayload` so the Tauri log-collector can persist them and the app's
//! log-viewer screen can render a color-coded timeline with expandable
//! stdout/stderr — the shape the frontend's `BootstrapTrace` interface
//! already types.
//!
//! Field names deliberately match the frontend TS interface:
//! `app/src/lib/types.ts::BootstrapTrace`. Any rename here would silently
//! break the type discriminator (`isBootstrapTrace` in
//! `app/src/routes/(health)/logs/helpers.ts`), so both sides move together.

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use uuid::Uuid;

use super::process_util::{TimedOutcome, output_with_timeout};

thread_local! {
    /// Scoped recorder for the current thread. Set by [`scoped`], read by
    /// [`current_recorder`]. Keeps every checker / resolver free of an
    /// explicit recorder parameter — a passing thread through `check()` or
    /// `resolve()` just calls [`run_traced_current`] on its `Command`s.
    static CURRENT_RECORDER: RefCell<Option<TraceRecorder>> = const { RefCell::new(None) };
}

/// Run `f` with `recorder` installed as the thread-local trace sink. Any
/// [`run_traced_current`] call inside `f` records into it. Nested calls stack
/// LIFO so the outer scope's recorder is restored on unwind.
pub fn scoped<R>(recorder: &TraceRecorder, f: impl FnOnce() -> R) -> R {
    let prev = CURRENT_RECORDER.with(|c| c.borrow_mut().replace(recorder.clone()));
    // Use a guard so a panic in `f` still restores the previous recorder.
    struct Guard(Option<TraceRecorder>);
    impl Drop for Guard {
        fn drop(&mut self) {
            let prev = self.0.take();
            CURRENT_RECORDER.with(|c| *c.borrow_mut() = prev);
        }
    }
    let _g = Guard(prev);
    f()
}

/// The recorder currently in scope for this thread, if any.
pub fn current_recorder() -> Option<TraceRecorder> {
    CURRENT_RECORDER.with(|c| c.borrow().clone())
}

/// Convenience: wrap [`run_traced`] with the current thread-local recorder.
/// A checker or resolver just replaces `output_with_timeout(cmd, t)` with
/// `run_traced_current(cmd, spec)` and its command lands in whatever recorder
/// is currently scoped; if nothing is scoped, it's a plain
/// `output_with_timeout` call.
pub fn run_traced_current(cmd: Command, spec: TraceSpec<'_>) -> TimedOutcome {
    let rec = current_recorder();
    run_traced(cmd, spec, rec.as_ref())
}

/// One recorded command invocation. Serialised into a log session entry
/// alongside `LogEntry` records; the log viewer discriminates by presence of
/// `action_type`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapTrace {
    /// Stable per-trace id (UUID v4).
    pub id: String,
    /// RFC3339 timestamp of when the command was spawned.
    pub ts: String,
    /// Which phase of the pipeline emitted this trace.
    pub action_type: ActionType,
    /// Machine-friendly step name (`brew_install_sensei`, `probe_version_psql`).
    pub step: String,
    /// Human-friendly description shown in the log viewer.
    pub desc: String,
    /// The command line as invoked (program + args, shell-safe).
    pub cmd: String,
    /// Exit code; `None` on spawn failure or timeout.
    pub exit: Option<i32>,
    /// Captured stdout (may be truncated at higher layers).
    pub out: String,
    /// Captured stderr (may be truncated at higher layers).
    pub err: String,
    /// Wall-clock duration in milliseconds.
    pub ms: u64,
    /// Whether the invocation succeeded from the caller's perspective.
    pub ok: bool,
    /// True when this trace represents a fix (resolver) rather than a probe.
    pub fix_attempted: bool,
    /// Free-text fix approach (`brew install`, `link-fix`, …); `None` for probes.
    pub fix_approach: Option<String>,
    /// Outcome of the fix (Some(true|false) for resolvers, None for probes).
    pub fix_ok: Option<bool>,
}

/// Phase of the health pipeline that emitted a given trace. Matches the TS
/// `action_type` discriminator: `'check' | 'resolve' | 'instruct'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    /// Read-only probe — a version check, port probe, `pg_isready`.
    Check,
    /// A resolver actively fixing state (install, link, restart).
    Resolve,
    /// A step whose only effect is emitting user-facing guidance (rare).
    Instruct,
}

/// Cheap-to-clone accumulator threaded through the health pipeline. Multiple
/// tasks record concurrently; the `Mutex` cost is negligible next to the child
/// process spawn we're already paying for.
#[derive(Debug, Clone, Default)]
pub struct TraceRecorder {
    inner: Arc<Mutex<Vec<BootstrapTrace>>>,
}

impl TraceRecorder {
    /// Create a fresh, empty recorder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one trace. Never blocks; on a poisoned mutex the record is
    /// dropped (test-only edge case).
    pub fn push(&self, t: BootstrapTrace) {
        if let Ok(mut g) = self.inner.lock() {
            g.push(t);
        }
    }

    /// Consume the recorder and return every recorded trace in insertion order.
    /// Callers typically call this once at the end of `check()` / `resolve()`.
    pub fn drain(&self) -> Vec<BootstrapTrace> {
        self.inner.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default()
    }

    /// Non-consuming snapshot count — handy for tests.
    pub fn len(&self) -> usize {
        self.inner.lock().map(|g| g.len()).unwrap_or(0)
    }

    /// True when nothing has been recorded yet.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The maximum bytes of stdout/stderr recorded per trace. Prevents a runaway
/// `brew install` from bloating the log session. Truncated payloads gain a
/// `\n… (truncated)` suffix so the viewer can flag them.
pub const STDIO_CAP: usize = 8 * 1024;

/// Truncate captured stdio at [`STDIO_CAP`], appending a marker.
fn cap_stdio(mut s: String) -> String {
    if s.len() > STDIO_CAP {
        s.truncate(STDIO_CAP);
        s.push_str("\n… (truncated)");
    }
    s
}

/// Render a `Command` back to a shell-safe-ish string. Not a real shell
/// escaper — the trace is diagnostic, not an executable script — but good
/// enough for a human to read.
fn render_cmd(cmd: &Command) -> String {
    let program = cmd.get_program().to_string_lossy();
    let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
    if args.is_empty() { program.into_owned() } else { format!("{} {}", program, args.join(" ")) }
}

/// Configuration for a traced command invocation.
pub struct TraceSpec<'a> {
    pub step: &'a str,
    pub desc: &'a str,
    pub action_type: ActionType,
    pub fix_approach: Option<&'a str>,
    pub timeout: std::time::Duration,
}

/// Spawn `cmd` under the same timeout discipline as
/// [`output_with_timeout`], recording a [`BootstrapTrace`] into `recorder`
/// and returning the underlying `TimedOutcome` unchanged. Existing call
/// sites can migrate one at a time: pass a `Some(&recorder)` and get
/// instrumentation for free, or pass `None` to keep the raw call.
pub fn run_traced(
    cmd: Command,
    spec: TraceSpec<'_>,
    recorder: Option<&TraceRecorder>,
) -> TimedOutcome {
    let cmd_str = render_cmd(&cmd);
    let start = Instant::now();
    let outcome = output_with_timeout(cmd, spec.timeout);
    let ms = start.elapsed().as_millis() as u64;

    let Some(rec) = recorder else { return outcome };

    let (exit, out, err, ok) = match &outcome {
        TimedOutcome::Done(o) => (
            o.status.code(),
            cap_stdio(String::from_utf8_lossy(&o.stdout).into_owned()),
            cap_stdio(String::from_utf8_lossy(&o.stderr).into_owned()),
            o.status.success(),
        ),
        TimedOutcome::TimedOut => (None, String::new(), format!("timed out after {ms}ms"), false),
        TimedOutcome::Failed(e) => (None, String::new(), format!("spawn failed: {e}"), false),
    };

    let is_fix = matches!(spec.action_type, ActionType::Resolve);
    rec.push(BootstrapTrace {
        id: Uuid::new_v4().to_string(),
        ts: chrono::Utc::now().to_rfc3339(),
        action_type: spec.action_type,
        step: spec.step.to_string(),
        desc: spec.desc.to_string(),
        cmd: cmd_str,
        exit,
        out,
        err,
        ms,
        ok,
        fix_attempted: is_fix,
        fix_approach: spec.fix_approach.map(String::from),
        fix_ok: if is_fix { Some(ok) } else { None },
    });

    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::time::Duration;

    fn spec_check<'a>(step: &'a str) -> TraceSpec<'a> {
        TraceSpec {
            step,
            desc: "test",
            action_type: ActionType::Check,
            fix_approach: None,
            timeout: Duration::from_secs(2),
        }
    }

    fn spec_resolve<'a>(step: &'a str, approach: &'a str) -> TraceSpec<'a> {
        TraceSpec {
            step,
            desc: "test",
            action_type: ActionType::Resolve,
            fix_approach: Some(approach),
            timeout: Duration::from_secs(2),
        }
    }

    #[test]
    fn recorder_captures_success_probe() {
        let rec = TraceRecorder::new();
        let outcome = run_traced(Command::new("true"), spec_check("probe_true"), Some(&rec));
        assert!(matches!(outcome, TimedOutcome::Done(_)));
        let traces = rec.drain();
        assert_eq!(traces.len(), 1);
        let t = &traces[0];
        assert_eq!(t.step, "probe_true");
        assert_eq!(t.action_type, ActionType::Check);
        assert_eq!(t.exit, Some(0));
        assert!(t.ok, "true exits 0 → ok=true");
        assert!(!t.fix_attempted);
        assert_eq!(t.fix_ok, None, "probes don't carry a fix_ok");
    }

    #[test]
    fn recorder_captures_resolver_failure() {
        let rec = TraceRecorder::new();
        // `false` exits with code 1 — a resolver whose command failed.
        run_traced(Command::new("false"), spec_resolve("install_fail", "brew install"), Some(&rec));
        let traces = rec.drain();
        assert_eq!(traces.len(), 1);
        let t = &traces[0];
        assert_eq!(t.action_type, ActionType::Resolve);
        assert_eq!(t.exit, Some(1));
        assert!(!t.ok);
        assert!(t.fix_attempted);
        assert_eq!(t.fix_ok, Some(false), "resolver exit=1 → fix_ok=false");
        assert_eq!(t.fix_approach.as_deref(), Some("brew install"));
    }

    #[test]
    fn recorder_records_spawn_failure() {
        let rec = TraceRecorder::new();
        run_traced(
            Command::new("definitely_not_a_real_binary_xyz_77"),
            spec_check("probe_missing"),
            Some(&rec),
        );
        let t = &rec.drain()[0];
        assert_eq!(t.exit, None, "spawn failure has no exit code");
        assert!(!t.ok);
        assert!(t.err.starts_with("spawn failed:"));
    }

    #[test]
    fn recorder_records_timeout() {
        let rec = TraceRecorder::new();
        let mut cmd = Command::new("sleep");
        cmd.arg("10");
        let spec = TraceSpec {
            step: "probe_slow",
            desc: "test",
            action_type: ActionType::Check,
            fix_approach: None,
            timeout: Duration::from_millis(150),
        };
        run_traced(cmd, spec, Some(&rec));
        let t = &rec.drain()[0];
        assert_eq!(t.exit, None);
        assert!(!t.ok);
        assert!(t.err.starts_with("timed out"), "err surfaces the timeout: {}", t.err);
    }

    #[test]
    fn none_recorder_is_no_op_side_of_check() {
        // Passing recorder=None still runs the command and returns its outcome;
        // no trace is buffered anywhere.
        let outcome = run_traced(Command::new("true"), spec_check("probe_none"), None);
        assert!(matches!(outcome, TimedOutcome::Done(_)));
        // Nothing to assert on the recorder side — that's the point.
    }

    #[test]
    fn render_cmd_joins_program_and_args() {
        let mut c = Command::new("brew");
        c.arg("install").arg("--formula").arg("sensei");
        let mut c2 = Command::new("brew");
        c2.arg("install").arg("--formula").arg("sensei");
        assert_eq!(render_cmd(&c), "brew install --formula sensei");
        // Zero-arg case.
        assert_eq!(render_cmd(&Command::new("true")), "true");
    }

    #[test]
    fn cap_stdio_truncates_with_marker_only_over_the_limit() {
        let short = "hello".to_string();
        assert_eq!(cap_stdio(short.clone()), short);
        let long = "x".repeat(STDIO_CAP + 100);
        let capped = cap_stdio(long);
        assert!(capped.ends_with("… (truncated)"), "marker present");
        assert!(capped.len() <= STDIO_CAP + 32, "not much longer than the cap");
    }

    #[test]
    fn thread_local_scoped_installs_and_restores() {
        // Nothing is set by default.
        assert!(current_recorder().is_none());

        let rec = TraceRecorder::new();
        let inner_count = scoped(&rec, || {
            run_traced_current(Command::new("true"), spec_check("scoped_probe"));
            rec.len()
        });
        assert_eq!(inner_count, 1, "run_traced_current found the scoped recorder");

        // After `scoped` returns, the TL slot is cleared.
        assert!(current_recorder().is_none(), "TL slot cleared after scoped()");
    }

    #[test]
    fn nested_scoped_restores_outer_on_unwind() {
        let outer = TraceRecorder::new();
        let inner = TraceRecorder::new();

        scoped(&outer, || {
            run_traced_current(Command::new("true"), spec_check("outer_1"));
            scoped(&inner, || {
                run_traced_current(Command::new("true"), spec_check("inner_1"));
            });
            // After inner drops, outer must be reinstalled.
            run_traced_current(Command::new("true"), spec_check("outer_2"));
        });

        let outer_traces = outer.drain();
        assert_eq!(outer_traces.len(), 2, "outer got both outer_1 and outer_2");
        assert_eq!(outer_traces[0].step, "outer_1");
        assert_eq!(outer_traces[1].step, "outer_2");

        let inner_traces = inner.drain();
        assert_eq!(inner_traces.len(), 1);
        assert_eq!(inner_traces[0].step, "inner_1");
    }

    #[test]
    fn run_traced_current_is_a_noop_outside_a_scope() {
        // Calling outside `scoped` still runs the command but records nothing.
        let outcome = run_traced_current(Command::new("true"), spec_check("no_scope"));
        assert!(matches!(outcome, TimedOutcome::Done(_)));
        // Nothing else to assert — no observable side effect.
    }

    #[test]
    fn recorder_is_clone_share_safe() {
        // The Arc<Mutex<_>> handle is designed to be cloned into worker
        // threads; verify pushes from clones show up in the original.
        let rec = TraceRecorder::new();
        let rec2 = rec.clone();
        run_traced(Command::new("true"), spec_check("thread_1"), Some(&rec));
        run_traced(Command::new("true"), spec_check("thread_2"), Some(&rec2));
        let traces = rec.drain();
        assert_eq!(traces.len(), 2);
        assert_eq!(traces[0].step, "thread_1");
        assert_eq!(traces[1].step, "thread_2");
    }
}
