//! Bootstrap commands — prerequisite detection, installation, and hardware profiling.
//!
//! Phase commands delegate entirely to the prereq factory + runner.
//! No orchestration logic lives here — commands are thin wrappers that:
//!   1. Build a prerequisite list via the platform factory
//!   2. Run the generic runner with an event-emitting progress callback
//!   3. Return immediately — progress arrives on the "bootstrap" channel

use crate::log_collector::{LogCollector, LogSession, SystemInfo};
use sensei_bootstrap::{
    self as bootstrap,
    BootstrapResult, BootstrapTrace, HardwareInfo,
    prereq::{GateStatus, ProgressEvent, factory, runner},
};
use std::sync::Arc;
use tauri::{Emitter, Manager};

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

/// Run the full bootstrap check — all components + hardware detection.
/// Traces all checks and writes a session log to disk.
#[tauri::command]
pub fn run_bootstrap(app: tauri::AppHandle) -> BootstrapResult {
    let (result, traces) = bootstrap::run_with_traces();

    // Write session log (best-effort — never fail the bootstrap over logging)
    write_bootstrap_session(&app, &result, &traces);

    result
}

fn write_bootstrap_session(
    app: &tauri::AppHandle,
    result: &BootstrapResult,
    traces: &[BootstrapTrace],
) {
    let system_info = collect_system_info(&result.hardware);

    let outcome = if result.ready {
        "success"
    } else if result.components.iter().any(|c| c.is_failed()) {
        "failed"
    } else {
        "partial"
    };

    let duration_ms: u64 = traces.iter().map(|t| t.ms).sum();

    let trace_values: Vec<serde_json::Value> = traces
        .iter()
        .map(|t| serde_json::to_value(t).unwrap_or_default())
        .collect();

    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let session_id = format!("sess-bs-{:08x}", COUNTER.fetch_add(1, Ordering::Relaxed));

    let session = LogSession {
        id:          session_id,
        module:      "bootstrap".to_string(),
        started_at:  chrono::Utc::now().to_rfc3339(),
        app_version: app.package_info().version.to_string(),
        system_info,
        outcome:     outcome.to_string(),
        duration_ms,
        traces:      trace_values,
    };

    let collector = app.state::<LogCollector>();
    collector.write_session(&session);
}

fn collect_system_info(hw: &HardwareInfo) -> SystemInfo {
    SystemInfo {
        os:        sysinfo::System::long_os_version()
                       .unwrap_or_else(|| "unknown".to_string()),
        arch:      std::env::consts::ARCH.to_string(),
        ram_gb:    hw.ram_gb as u64,
        cpu_cores: hw.cpu_cores as usize,
    }
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
