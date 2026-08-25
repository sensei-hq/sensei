//! Public health surface.

pub mod checker;
pub mod checkers;
pub mod graph;
pub mod ids;
pub mod platforms;
pub mod process_util;
pub mod provider;
pub mod resolver;
pub mod resolvers;
pub mod trace;
pub mod types;

pub use checker::{CheckOutcome, Checker};
pub use graph::{DependencySpec, dependency_specs, installing_verb_for, spec_for};
pub use provider::{PlatformProvider, detect_provider};
pub use resolver::{ResolveOutcome, Resolver};
pub use trace::{
    ActionType, BootstrapTrace, TraceRecorder, TraceSpec, current_recorder, run_traced,
    run_traced_current, scoped,
};
pub use types::*;

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

/// Full pipeline: emit Phase(Checking) → run `check_streaming()` which
/// emits a Component event after each probe finishes → emit
/// Report(initial) → if not Ok, run the provider's `resolve()` which emits
/// its own Phase(Resolving), per-component patches, optional Remedy, and a
/// final Report(terminal).
///
/// This is the single entry point every transport (Tauri sidecar, CLI
/// `doctor`, daemon HTTP) should use when it wants the full
/// check-and-fix flow. The transport's only responsibility is the `emit`
/// closure.
pub fn check_and_resolve(app_version: &str, emit: &dyn Fn(HealthEvent)) -> HealthPayload {
    let provider = detect_provider();
    check_and_resolve_with(&*provider, app_version, emit)
}

/// [`check_and_resolve`] against an explicit provider — the injection seam
/// that lets unit tests drive the check→report→resolve orchestration against
/// a mock instead of probing the real machine (which is non-deterministic:
/// the live provider can report NeedsAction with no derivable remedy, which
/// would panic the terminal `validate()`). Production passes `detect_provider()`.
///
/// `emit` is `&dyn Fn` (not generic) to keep `PlatformProvider` dyn-compatible.
pub(crate) fn check_and_resolve_with(
    provider: &dyn PlatformProvider,
    app_version: &str,
    emit: &dyn Fn(HealthEvent),
) -> HealthPayload {
    emit(HealthEvent::Phase { phase: HealthStatus::Checking });
    let state = provider.check_streaming(app_version, emit);
    emit(HealthEvent::Report { payload: state.clone() });
    if state.status == HealthStatus::Ok {
        return state;
    }
    provider.resolve(&state, app_version, emit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // ── Hermetic provider (de-flakes the orchestration tests) ────────────
    // The real provider probes the live machine, so the orchestration tests
    // could hit a transient NeedsAction-without-remedy state and panic in
    // `validate()`. This mock fails one required component (Daemon) on the
    // initial probe and recovers it on the post-resolve recheck, so
    // check→report→resolve runs deterministically with a valid terminal.
    struct StubChecker(CheckOutcome);
    impl Checker for StubChecker {
        fn check(&self) -> CheckOutcome {
            self.0.clone()
        }
    }

    struct RecoveringDaemonResolver;
    impl Resolver for RecoveringDaemonResolver {
        fn id(&self) -> &'static str {
            "mock-daemon"
        }
        fn resolves(&self) -> &'static [ComponentId] {
            &[ComponentId::Daemon]
        }
        fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
            ResolveOutcome::Resolved
        }
        fn fallback_remedy(&self) -> Remedy {
            Remedy {
                message: "start the daemon".into(),
                script: "brew services start sensei".into(),
                url: None,
            }
        }
    }

    /// Daemon fails on the initial probe (`retry=false`) and is ready on the
    /// post-resolve recheck (`retry=true`); every other component is ready.
    struct HermeticProvider;
    impl PlatformProvider for HermeticProvider {
        fn platform(&self) -> Platform {
            Platform::Macos
        }
        fn package_manager_id(&self) -> PackageManagerId {
            PackageManagerId::Homebrew
        }
        fn package_manager_checker(&self) -> Box<dyn Checker> {
            Box::new(StubChecker(CheckOutcome::ready("brew 4.0")))
        }
        fn checker_for(&self, id: ComponentId, retry: bool) -> Box<dyn Checker> {
            let outcome = if id == ComponentId::Daemon && !retry {
                CheckOutcome::failed("daemon not running")
            } else {
                CheckOutcome::ready("ok")
            };
            Box::new(StubChecker(outcome))
        }
        fn resolvers(&self) -> Vec<Box<dyn Resolver>> {
            vec![Box::new(RecoveringDaemonResolver)]
        }
        fn default_remedy(&self) -> Remedy {
            Remedy { message: "m".into(), script: "s".into(), url: None }
        }
    }

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
        // Hermetic mock (not the real machine) so the orchestration is
        // deterministic and the resolve path always runs.
        let events = Mutex::new(Vec::<HealthEvent>::new());
        let _final = check_and_resolve_with(&HermeticProvider, "0.0.0-test", &|e| {
            events.lock().unwrap().push(e)
        });
        let evs = events.lock().unwrap();

        // Phase(Checking) is always first.
        assert!(matches!(evs.first(), Some(HealthEvent::Phase { phase: HealthStatus::Checking })));

        // The initial Report broadcasts before any Phase(Resolving) — so the
        // UI never goes blank. With this mock the resolve path always fires.
        let first_report = evs.iter().position(|e| matches!(e, HealthEvent::Report { .. }));
        let phase_resolving = evs
            .iter()
            .position(|e| matches!(e, HealthEvent::Phase { phase: HealthStatus::Resolving }));
        assert!(first_report.is_some(), "must emit at least one Report");
        assert!(phase_resolving.is_some(), "the mock's failed Daemon forces the resolve path");
        assert!(
            first_report.unwrap() < phase_resolving.unwrap(),
            "initial Report must precede Phase(Resolving)"
        );
    }

    #[test]
    fn resolve_emits_phase_and_terminal_report_when_not_ok() {
        // Hermetic mock: initial check is NeedsAction (daemon down), resolve
        // recovers it → a valid terminal, no real-machine probing.
        let provider = HermeticProvider;
        let initial = provider.check("0.0.0-test");
        assert_eq!(
            initial.status,
            HealthStatus::NeedsAction,
            "the mock's initial check must be not-ok to exercise resolve"
        );

        let events = Mutex::new(Vec::<HealthEvent>::new());
        let terminal =
            provider.resolve(&initial, "0.0.0-test", &|e| events.lock().unwrap().push(e));
        terminal.validate().expect("terminal must validate");

        let evs = events.lock().unwrap();
        // Not ok: must emit Phase(Resolving) first and a final Report.
        assert!(matches!(evs.first(), Some(HealthEvent::Phase { phase: HealthStatus::Resolving })));
        assert!(matches!(evs.last(), Some(HealthEvent::Report { .. })));
    }
}
