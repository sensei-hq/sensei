//! Health commands — thin wrappers over sensei_bootstrap.
//!
//! Two commands and two only:
//!   * `health_check`             — sync. Returns HealthPayload.
//!   * `health_check_and_resolve` — fire-and-forget. Runs check() then
//!                                  resolve() on a background thread; events
//!                                  stream on the "health" channel.

use sensei_bootstrap::{self as bootstrap, HealthEvent, HealthPayload, HealthStatus};
use tauri::Emitter;

use crate::flog;

#[tauri::command]
pub fn health_check(app: tauri::AppHandle) -> HealthPayload {
    let version = app.package_info().version.to_string();
    bootstrap::check(&version)
}

#[tauri::command]
pub fn health_check_and_resolve(app: tauri::AppHandle) -> Result<(), String> {
    let version = app.package_info().version.to_string();
    flog::log(&format!("=== health_check_and_resolve called v={version} ==="));

    std::thread::spawn(move || {
        let emit = {
            let app = app.clone();
            move |ev: HealthEvent| { let _ = app.emit("health", &ev); }
        };

        // 1. Phase: checking → compute initial state → broadcast it.
        emit(HealthEvent::Phase { phase: HealthStatus::Checking });
        let state = bootstrap::check(&version);
        emit(HealthEvent::Report { payload: state.clone() });

        // 2. If anything failed, run the resolvers. resolve() emits
        //    Phase(Resolving), per-component patches, and a final Report.
        if state.status != HealthStatus::Ok {
            bootstrap::resolve(&state, &version, &emit);
        }

        flog::log("health_check_and_resolve complete");
    });
    Ok(())
}
