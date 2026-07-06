//! Public health surface.

pub mod types;
pub mod ids;
pub mod graph;
pub mod checker;
pub mod resolver;
pub mod provider;
pub mod platforms;
pub mod checkers;
pub mod resolvers;
pub mod process_util;
pub mod trace;

pub use types::*;
pub use trace::{ActionType, BootstrapTrace, TraceRecorder, TraceSpec, current_recorder, run_traced, run_traced_current, scoped};
pub use graph::{DependencySpec, dependency_specs, spec_for, installing_verb_for};
pub use checker::{Checker, CheckOutcome};
pub use resolver::{Resolver, ResolveOutcome};
pub use provider::{PlatformProvider, detect_provider};

/// Sync fast path — runs every checker once, returns a validated HealthPayload.
/// Daemon `GET /health` uses this. No events emitted.
pub fn check(app_version: &str) -> HealthPayload {
    detect_provider().check(app_version)
}

/// Same as [`check`] but returns the `BootstrapTrace`s captured during the
/// check pass alongside the payload. Every instrumented `Command::new` site
/// hit by [`check`] emits one trace record; sites not yet migrated to
/// [`run_traced_current`] contribute nothing (they still run correctly —
/// tracing is additive, not gating).
///
/// The Tauri log-collector wraps its bootstrap events with this so the
/// diagnostic logs page (`(health)/logs/+page.svelte`) can render a
/// step-by-step timeline with expandable stdout/stderr (#39).
pub fn check_traced(app_version: &str) -> (HealthPayload, Vec<BootstrapTrace>) {
    let recorder = TraceRecorder::new();
    let payload = trace::scoped(&recorder, || check(app_version));
    (payload, recorder.drain())
}

/// Streaming resolve — runs resolvers covering any failed components in
/// `current`, re-checks affected deps, returns terminal payload. Emits
/// HealthEvent values throughout.
///
/// Internal helper for `check_and_resolve`. Callers outside this module
/// should use `check_and_resolve` so the initial `Phase(Checking)` and
/// `Report` events always precede the resolve walk.
///
/// `emit` is `&dyn Fn` (not generic) to keep PlatformProvider dyn-compatible.
pub(crate) fn resolve(current: &HealthPayload, app_version: &str, emit: &dyn Fn(HealthEvent)) -> HealthPayload {
    detect_provider().resolve(current, app_version, emit)
}

/// Full pipeline: emit Phase(Checking) → run `check_streaming()` which
/// emits a Component event after each probe finishes → emit
/// Report(initial) → if not Ok, run `resolve()` which emits its own
/// Phase(Resolving), per-component patches, optional Remedy, and a final
/// Report(terminal).
///
/// This is the single entry point every transport (Tauri sidecar, CLI
/// `doctor`, daemon HTTP) should use when it wants the full
/// check-and-fix flow. The transport's only responsibility is the `emit`
/// closure.
pub fn check_and_resolve(app_version: &str, emit: &dyn Fn(HealthEvent)) -> HealthPayload {
    emit(HealthEvent::Phase { phase: HealthStatus::Checking });
    let state = detect_provider().check_streaming(app_version, emit);
    emit(HealthEvent::Report { payload: state.clone() });
    if state.status == HealthStatus::Ok {
        return state;
    }
    resolve(&state, app_version, emit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn check_returns_validated_payload() {
        let payload = check("0.0.0-test");
        payload.validate().expect("validate must pass");
        assert_eq!(payload.components.len(), 5);
    }

    #[test]
    fn check_traced_records_binary_probes() {
        let (payload, traces) = check_traced("0.0.0-test");
        payload.validate().expect("payload validates as usual");
        assert_eq!(payload.components.len(), 5, "traced check produces same shape");

        // The instrumented BinaryChecker probes every configured binary that
        // resolves — brew, postgres, ollama, senseid, sensei. On CI without
        // those installed we may still have zero (which_binary returns None
        // and no probe fires), so the assertion is only that IF any probe
        // fires it lands in the recorder — not a strict count.
        for t in &traces {
            assert_eq!(t.action_type, ActionType::Check, "binary probes are Check-type");
            assert!(!t.step.is_empty(), "step name populated: {t:?}");
            assert!(!t.cmd.is_empty(), "cmd string populated: {t:?}");
            // exit is Some(_) on either success or non-zero, None only on spawn/timeout failure.
            // ok correlates with status.success() on Done arms.
        }
    }

    #[test]
    fn check_and_resolve_emits_initial_report_before_resolving() {
        let events = Mutex::new(Vec::<HealthEvent>::new());
        let _final = check_and_resolve("0.0.0-test", &|e| events.lock().unwrap().push(e));
        let evs = events.lock().unwrap();

        // Phase(Checking) is always first.
        assert!(matches!(evs.first(), Some(HealthEvent::Phase { phase: HealthStatus::Checking })));

        // A Report event arrives before any Phase(Resolving) — that's the
        // initial broadcast so the UI never goes blank.
        let first_report = evs.iter().position(|e| matches!(e, HealthEvent::Report { .. }));
        let phase_resolving = evs.iter().position(|e| matches!(
            e, HealthEvent::Phase { phase: HealthStatus::Resolving }
        ));
        assert!(first_report.is_some(), "must emit at least one Report");
        if let Some(pr) = phase_resolving {
            assert!(first_report.unwrap() < pr,
                "initial Report must precede Phase(Resolving)");
        }
    }

    #[test]
    fn resolve_emits_phase_and_terminal_report_when_not_ok() {
        let initial = check("0.0.0-test");
        let events = Mutex::new(Vec::<HealthEvent>::new());
        let terminal = resolve(&initial, "0.0.0-test", &|e| events.lock().unwrap().push(e));
        terminal.validate().expect("terminal must validate");

        let evs = events.lock().unwrap();
        if initial.status == HealthStatus::Ok {
            // System is already healthy: resolve returns without doing real work.
            // It still emits Phase(Resolving) and a final Report — that's the contract.
            assert!(matches!(evs.first(), Some(HealthEvent::Phase { phase: HealthStatus::Resolving })));
        } else {
            // Not ok: must emit at least Phase + Report.
            assert!(matches!(evs.first(), Some(HealthEvent::Phase { phase: HealthStatus::Resolving })));
            assert!(matches!(evs.last(),  Some(HealthEvent::Report { .. })));
        }
    }
}
