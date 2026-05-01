//! Bootstrap commands — prerequisite detection, installation, and hardware profiling.
//!
//! Phase commands delegate entirely to the prereq factory + runner.
//! No orchestration logic lives here — commands are thin wrappers that:
//!   1. Build a prerequisite list via the platform factory
//!   2. Run the generic runner with an event-emitting progress callback
//!   3. Return immediately — progress arrives on the "bootstrap" channel

use sensei_bootstrap::{
    self as bootstrap,
    BootstrapResult, HardwareInfo,
    prereq::{GateStatus, ProgressEvent, factory, runner},
};
use std::sync::Arc;
use tauri::Emitter;

// ---------------------------------------------------------------------------
// Event helpers
// ---------------------------------------------------------------------------

fn emit_gate(
    app: &tauri::AppHandle,
    id: &str,
    status: &str,
    version: Option<&str>,
    detail: Option<&str>,
) {
    let _ = app.emit(
        "bootstrap",
        serde_json::json!({
            "action": "update",
            "entity": "gate",
            "id": id,
            "data": { "status": status, "version": version, "detail": detail }
        }),
    );
}

fn emit_phase_complete(app: &tauri::AppHandle, phase: &str, success: bool) {
    let _ = app.emit(
        "bootstrap",
        serde_json::json!({
            "action": "set",
            "entity": "phase",
            "id": phase,
            "data": { "complete": true, "success": success }
        }),
    );
}

/// Map a ProgressEvent from the runner to Tauri events on the "bootstrap" channel.
fn dispatch(app: &tauri::AppHandle, event: ProgressEvent) {
    match event {
        ProgressEvent::Gate { id, status } => {
            let (s, version, detail) = match status {
                GateStatus::Checking            => ("checking",   None, None),
                GateStatus::Installing          => ("installing", None, None),
                GateStatus::Starting            => ("starting",   None, None),
                GateStatus::Ready { version, detail } => ("ready", version, detail),
                GateStatus::Failed { error }    => ("error",      None, Some(error)),
            };
            emit_gate(app, &id, s, version.as_deref(), detail.as_deref());
        }
        ProgressEvent::PhaseComplete { phase, success } => {
            emit_phase_complete(app, &phase, success);
        }
    }
}

// ---------------------------------------------------------------------------
// Read-only commands (unchanged)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn run_bootstrap() -> BootstrapResult {
    bootstrap::run()
}

#[tauri::command]
pub fn detect_hardware() -> HardwareInfo {
    bootstrap::hardware::detect()
}

#[tauri::command]
pub fn list_models() -> Vec<String> {
    bootstrap::models::list()
}

#[tauri::command]
pub fn missing_models() -> Vec<String> {
    let hw = bootstrap::hardware::detect();
    bootstrap::models::missing_models(&hw.recommended_tier)
}

#[tauri::command]
pub fn get_platform() -> serde_json::Value {
    let provider = bootstrap::provider();
    serde_json::json!({
        "platform": provider.platform(),
        "package_manager": provider.package_manager_name(),
        "prereq_remedy": provider.prereq_install_remedy(),
        "pkgmgr_remedy": provider.package_manager_remedy(),
    })
}

// ---------------------------------------------------------------------------
// Phase commands — factory + runner
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn install_prerequisites(app: tauri::AppHandle) -> Result<(), String> {
    std::thread::spawn(move || {
        let provider = Arc::from(bootstrap::provider());
        let prereqs = factory::install_prerequisites(provider);
        runner::run("install", prereqs, |e| dispatch(&app, e));
    });
    Ok(())
}

#[tauri::command]
pub fn start_services(app: tauri::AppHandle) -> Result<(), String> {
    std::thread::spawn(move || {
        let provider = Arc::from(bootstrap::provider());
        let prereqs = factory::start_services(provider);
        runner::run("services", prereqs, |e| dispatch(&app, e));
    });
    Ok(())
}

#[tauri::command]
pub fn setup_database(app: tauri::AppHandle) -> Result<(), String> {
    std::thread::spawn(move || {
        let prereqs = factory::setup_database();
        runner::run("database", prereqs, |e| dispatch(&app, e));
    });
    Ok(())
}
