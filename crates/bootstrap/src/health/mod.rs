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

pub use types::*;
pub use graph::{DependencySpec, dependency_specs, spec_for};
pub use checker::{Checker, CheckOutcome};
pub use resolver::{Resolver, ResolveOutcome};
pub use provider::{PlatformProvider, detect_provider};

/// Sync fast path — runs every checker once, returns a validated HealthPayload.
/// Daemon `GET /health` uses this. No events emitted.
pub fn check(app_version: &str) -> HealthPayload {
    detect_provider().check(app_version)
}

/// Streaming resolve — runs resolvers covering any failed components in
/// `current`, re-checks affected deps, returns terminal payload. Emits
/// HealthEvent values throughout. Tauri sidecar uses this AFTER calling
/// `check()` and broadcasting its initial state.
///
/// `emit` is `&dyn Fn` (not generic) to keep PlatformProvider dyn-compatible.
pub fn resolve(current: &HealthPayload, app_version: &str, emit: &dyn Fn(HealthEvent)) -> HealthPayload {
    detect_provider().resolve(current, app_version, emit)
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
