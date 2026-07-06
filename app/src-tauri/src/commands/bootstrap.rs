//! Health commands — thin wrappers over sensei_bootstrap.
//!
//! Two commands and two only:
//! * `health_check`             — sync. Returns HealthPayload.
//! * `health_check_and_resolve` — fire-and-forget. Runs check() then
//!   resolve() on a background thread; events stream on the "health" channel.
//!
//! `health_check_and_resolve` is guarded by an in-flight `AtomicBool` so a
//! second invocation while a run is active is a no-op rather than spawning a
//! parallel `brew install` thread. The guard is reset via RAII so a panic in
//! the worker still releases the slot. This matters because:
//!   * Brew acquires a global update lock — two parallel `brew install`s
//!     race, the loser fails with stderr that maps to BrewError::Other.
//!   * Tauri webview reloads (Vite HMR, route remount, etc.) cause the
//!     frontend to invoke this command repeatedly while a long-running
//!     resolver is still emitting events to the now-dead listener — the
//!     "Couldn't find callback id" warning.

use std::sync::atomic::{AtomicBool, Ordering};
use sensei_bootstrap::{self as bootstrap, HealthEvent, HealthPayload};
use tauri::{Emitter, State};

use crate::flog;
use crate::log_collector::LogCollector;

static IN_FLIGHT: AtomicBool = AtomicBool::new(false);

/// RAII guard that flips IN_FLIGHT back to false on drop. Using a guard
/// (instead of a manual `store(false)` at the end of the worker) means a
/// panic inside the bootstrap layer still releases the slot — otherwise
/// every subsequent health request would be silently rejected.
struct InFlightGuard;
impl Drop for InFlightGuard {
    fn drop(&mut self) { IN_FLIGHT.store(false, Ordering::Release); }
}

/// Attempt to claim the slot. `Some(_)` means we own it and must keep the
/// guard alive for the whole worker; `None` means another worker has it.
fn try_acquire() -> Option<InFlightGuard> {
    IN_FLIGHT
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .ok()
        .map(|_| InFlightGuard)
}

#[tauri::command]
pub fn health_check(app: tauri::AppHandle) -> HealthPayload {
    let version = app.package_info().version.to_string();
    bootstrap::check(&version)
}

/// Traced variant of `health_check` — runs the same probe pass but captures a
/// `BootstrapTrace` for every instrumented `Command::new` inside the pipeline
/// (#39). Traces are persisted into a fresh log session named "bootstrap-check"
/// so the diagnostic logs page can render the timeline immediately after; the
/// payload itself is returned to the caller so existing UI wiring keeps
/// working. Non-instrumented sites still run correctly — tracing is additive.
#[tauri::command]
pub fn health_check_traced(
    app: tauri::AppHandle,
    collector: State<LogCollector>,
) -> HealthPayload {
    let version = app.package_info().version.to_string();
    let (payload, traces) = bootstrap::health::check_traced(&version);

    // Only bother opening a log session when we actually captured something.
    // A CI environment where no probed binary resolves will produce zero
    // traces; skipping the session in that case keeps the on-disk log dir
    // clean of empty envelopes.
    if !traces.is_empty() {
        let system_info = crate::log_collector::SystemInfo {
            os:        std::env::consts::OS.to_string(),
            arch:      std::env::consts::ARCH.to_string(),
            ram_gb:    0,   // not needed for a diagnostic entry; keeps the payload small
            cpu_cores: 0,
        };
        let session_id = collector.start_session("bootstrap-check", &version, system_info);
        for t in &traces {
            collector.append_trace(&session_id, t);
        }
        collector.end_session(&session_id);
    }
    payload
}

#[tauri::command]
pub fn health_check_and_resolve(app: tauri::AppHandle) -> Result<(), String> {
    let version = app.package_info().version.to_string();

    let guard = match try_acquire() {
        Some(g) => g,
        None => {
            flog::log("health_check_and_resolve: already in flight, skipping");
            return Ok(());
        }
    };
    flog::log(&format!("=== health_check_and_resolve called v={version} ==="));

    std::thread::spawn(move || {
        // Move the guard into the worker so Drop runs after the resolve
        // pipeline, regardless of success / panic.
        let _guard = guard;

        let emit = {
            let app = app.clone();
            move |ev: HealthEvent| { let _ = app.emit("health", &ev); }
        };

        // The whole check+report+resolve pipeline lives in the library.
        // This command's only job is the IPC glue: emit closure, threading,
        // and the in-flight guard.
        let _final = bootstrap::health::check_and_resolve(&version, &emit);
        flog::log("health_check_and_resolve complete");
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Tests exercise the shared `IN_FLIGHT` static, so they must run
    /// serially. Cargo's default test runner parallelises within a binary —
    /// this mutex serialises just this module without imposing a
    /// `--test-threads=1` constraint on the whole crate.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn reset() { IN_FLIGHT.store(false, Ordering::Release); }

    #[test]
    fn try_acquire_succeeds_when_idle() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        let g = try_acquire().expect("idle slot should acquire");
        drop(g);
        assert!(!IN_FLIGHT.load(Ordering::Acquire));
    }

    #[test]
    fn try_acquire_fails_when_already_held() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        let first = try_acquire().expect("first acquire ok");
        assert!(try_acquire().is_none(), "second acquire while first is alive must fail");
        drop(first);
        assert!(try_acquire().is_some(), "after drop, slot is reusable");
    }

    #[test]
    fn guard_releases_slot_on_panic() {
        let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        let result = std::panic::catch_unwind(|| {
            let _guard = try_acquire().expect("acquire");
            panic!("simulated worker panic");
        });
        assert!(result.is_err());
        assert!(!IN_FLIGHT.load(Ordering::Acquire),
            "RAII guard must release slot even when the worker panics");
        // And the slot is reusable.
        assert!(try_acquire().is_some());
    }
}
