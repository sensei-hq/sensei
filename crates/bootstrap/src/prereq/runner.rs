//! Generic check→fix→recheck orchestration loop.
//!
//! The runner knows nothing about specific gates, platforms, or Tauri events.
//! It emits ProgressEvents via a callback that the Tauri command layer maps to
//! emit_gate() calls.

use super::{GateKind, GateStatus, Prerequisite, ProgressEvent};

/// Run a list of prerequisites in order.
///
/// For each prerequisite:
///   1. Emit `Checking`
///   2. `check()` — if ok, emit `Ready` and move on
///   3. Emit `Installing` or `Starting` based on `gate_kind()`
///   4. `fix()` — if Err, emit `Failed` and move on
///   5. Re-check — emit `Ready` or `Failed`
///
/// After all prerequisites, emit `PhaseComplete`.
/// Returns `true` if all prerequisites ended in `Ready`.
pub fn run<F>(phase: &str, prereqs: Vec<Box<dyn Prerequisite>>, on_progress: F) -> bool
where
    F: Fn(ProgressEvent),
{
    let mut all_ok = true;

    for prereq in &prereqs {
        let id = prereq.id().to_string();

        on_progress(ProgressEvent::Gate { id: id.clone(), status: GateStatus::Checking });

        let check = prereq.check();
        if check.ok {
            on_progress(ProgressEvent::Gate {
                id: id.clone(),
                status: GateStatus::Ready { version: check.version, detail: check.detail },
            });
            continue;
        }

        // Not ready — attempt fix
        let fixing_status = match prereq.gate_kind() {
            GateKind::Service => GateStatus::Starting,
            GateKind::Install => GateStatus::Installing,
        };
        on_progress(ProgressEvent::Gate { id: id.clone(), status: fixing_status });

        match prereq.fix() {
            Err(e) => {
                all_ok = false;
                on_progress(ProgressEvent::Gate {
                    id: id.clone(),
                    status: GateStatus::Failed { error: e },
                });
            }
            Ok(_) => {
                let recheck = prereq.check();
                if recheck.ok {
                    on_progress(ProgressEvent::Gate {
                        id: id.clone(),
                        status: GateStatus::Ready { version: recheck.version, detail: recheck.detail },
                    });
                } else {
                    all_ok = false;
                    on_progress(ProgressEvent::Gate {
                        id: id.clone(),
                        status: GateStatus::Failed {
                            error: recheck.error.unwrap_or_else(|| "still failing after fix".to_string()),
                        },
                    });
                }
            }
        }
    }

    on_progress(ProgressEvent::PhaseComplete { phase: phase.to_string(), success: all_ok });
    all_ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prereq::{CheckResult, FixResult, GateKind, Prerequisite};
    use crate::prereq::checker::Checker;
    use crate::prereq::fixer::Fixer;
    use crate::prereq::generic::GenericPrerequisite;
    use std::sync::{Arc, Mutex};

    // --- Test doubles ---

    struct ReadyChecker;
    impl Checker for ReadyChecker {
        fn check(&self) -> CheckResult { CheckResult::ok("1.0") }
    }

    struct FailChecker;
    impl Checker for FailChecker {
        fn check(&self) -> CheckResult { CheckResult::fail("missing") }
    }

    /// Fails on first call, succeeds on subsequent calls (simulates post-fix state).
    struct FailThenPassChecker { calls: Arc<Mutex<u32>> }
    impl FailThenPassChecker {
        fn new() -> Self { Self { calls: Arc::new(Mutex::new(0)) } }
    }
    impl Checker for FailThenPassChecker {
        fn check(&self) -> CheckResult {
            let mut c = self.calls.lock().unwrap();
            *c += 1;
            if *c == 1 { CheckResult::fail("not yet") } else { CheckResult::ok("1.0") }
        }
    }

    struct SuccessFixer;
    impl Fixer for SuccessFixer {
        fn fix(&self) -> Result<FixResult, String> { Ok(FixResult::new("fixed")) }
    }

    struct FailFixer;
    impl Fixer for FailFixer {
        fn fix(&self) -> Result<FixResult, String> { Err("could not fix".into()) }
    }

    // --- Helpers ---

    fn make_prereq(id: &str, checker: Box<dyn Checker>, fixer: Box<dyn Fixer>, kind: GateKind) -> Box<dyn Prerequisite> {
        Box::new(GenericPrerequisite::new(id, id, checker, fixer, kind, None))
    }

    fn collect(prereqs: Vec<Box<dyn Prerequisite>>) -> Vec<ProgressEvent> {
        let acc = Arc::new(Mutex::new(Vec::new()));
        let acc2 = acc.clone();
        run("phase", prereqs, move |e| acc2.lock().unwrap().push(e));
        Arc::try_unwrap(acc).unwrap().into_inner().unwrap()
    }

    fn statuses_for<'a>(events: &'a [ProgressEvent], id: &str) -> Vec<&'a str> {
        events.iter().filter_map(|e| {
            if let ProgressEvent::Gate { id: eid, status } = e {
                if eid == id {
                    return Some(match status {
                        GateStatus::Checking    => "checking",
                        GateStatus::Installing  => "installing",
                        GateStatus::Starting    => "starting",
                        GateStatus::Ready { .. } => "ready",
                        GateStatus::Failed { .. } => "failed",
                    });
                }
            }
            None
        }).collect()
    }

    // --- Tests ---

    #[test]
    fn already_ready_prereq_emits_checking_then_ready() {
        let events = collect(vec![
            make_prereq("pg", Box::new(ReadyChecker), Box::new(SuccessFixer), GateKind::Install),
        ]);
        assert_eq!(statuses_for(&events, "pg"), vec!["checking", "ready"]);
    }

    #[test]
    fn failing_prereq_with_successful_fix_emits_installing_then_ready() {
        let events = collect(vec![
            make_prereq("ollama", Box::new(FailThenPassChecker::new()), Box::new(SuccessFixer), GateKind::Install),
        ]);
        assert_eq!(statuses_for(&events, "ollama"), vec!["checking", "installing", "ready"]);
    }

    #[test]
    fn service_prereq_emits_starting_not_installing() {
        let events = collect(vec![
            make_prereq("daemon", Box::new(FailThenPassChecker::new()), Box::new(SuccessFixer), GateKind::Service),
        ]);
        assert_eq!(statuses_for(&events, "daemon"), vec!["checking", "starting", "ready"]);
    }

    #[test]
    fn failing_prereq_with_failed_fix_emits_failed() {
        let events = collect(vec![
            make_prereq("sensei", Box::new(FailChecker), Box::new(FailFixer), GateKind::Install),
        ]);
        assert_eq!(statuses_for(&events, "sensei"), vec!["checking", "installing", "failed"]);
    }

    #[test]
    fn fix_succeeds_but_recheck_fails_emits_failed() {
        // FailChecker always fails — even after fix, recheck fails
        let events = collect(vec![
            make_prereq("bad", Box::new(FailChecker), Box::new(SuccessFixer), GateKind::Install),
        ]);
        assert_eq!(statuses_for(&events, "bad"), vec!["checking", "installing", "failed"]);
    }

    #[test]
    fn phase_complete_emitted_last_with_success() {
        let events = collect(vec![
            make_prereq("a", Box::new(ReadyChecker), Box::new(SuccessFixer), GateKind::Install),
        ]);
        assert!(matches!(
            events.last().unwrap(),
            ProgressEvent::PhaseComplete { phase, success: true } if phase == "phase"
        ));
    }

    #[test]
    fn phase_complete_false_when_any_fails() {
        let events = collect(vec![
            make_prereq("a", Box::new(ReadyChecker), Box::new(SuccessFixer), GateKind::Install),
            make_prereq("b", Box::new(FailChecker), Box::new(FailFixer), GateKind::Install),
        ]);
        assert!(matches!(
            events.last().unwrap(),
            ProgressEvent::PhaseComplete { success: false, .. }
        ));
    }

    #[test]
    fn run_returns_true_when_all_pass() {
        let ok = run("p", vec![
            make_prereq("a", Box::new(ReadyChecker), Box::new(SuccessFixer), GateKind::Install),
        ], |_| {});
        assert!(ok);
    }

    #[test]
    fn run_returns_false_when_any_fails() {
        let ok = run("p", vec![
            make_prereq("a", Box::new(FailChecker), Box::new(FailFixer), GateKind::Install),
        ], |_| {});
        assert!(!ok);
    }
}
