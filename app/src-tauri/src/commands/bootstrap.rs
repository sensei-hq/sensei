//! Bootstrap commands — prerequisite detection, installation, and hardware profiling.
//!
//! The single `check_and_fix_bootstrap` command replaces the old three-phase
//! commands (run_bootstrap / install_prerequisites / start_services / setup_database).
//! It delegates entirely to the BootstrapEngine which:
//!   1. Checks all components in parallel (Phase A)
//!   2. Builds a dependency-aware fix plan (Phase B)
//!   3. Executes fixes sequentially (Phase C)
//!   4. Returns a BootstrapReport (Phase D)
//!
//! Progress arrives on the "bootstrap" Tauri event channel.

use crate::flog;
use crate::log_collector::{LogCollector, LogSession, SystemInfo};
use sensei_bootstrap::{
    self as bootstrap,
    BootstrapReport, HardwareInfo,
    prereq::{GateStatus, ProgressEvent},
};
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

/// Map a ProgressEvent from the engine to Tauri events on the "bootstrap" channel.
fn dispatch(app: &tauri::AppHandle, event: ProgressEvent) {
    match event {
        ProgressEvent::Gate { id, status } => {
            let (s, version, detail) = match status {
                GateStatus::Checking            => ("checking",   None, None),
                GateStatus::Installing          => ("installing", None, None),
                GateStatus::Starting            => ("starting",   None, None),
                GateStatus::Ready { version, detail } => ("ready", version, detail),
                GateStatus::Failed { error }    => ("blocked",    None, Some(error)),
            };
            flog::log(&format!(
                "gate  [{id}] {s}{}{}",
                version.as_deref().map(|v| format!(" v={v}")).unwrap_or_default(),
                detail.as_deref().map(|d| format!(" detail={d}")).unwrap_or_default(),
            ));
            emit_gate(app, &id, s, version.as_deref(), detail.as_deref());
        }
        ProgressEvent::PhaseComplete { phase, success } => {
            flog::log(&format!("phase [{phase}] complete success={success}"));
            emit_phase_complete(app, &phase, success);
        }
    }
}

// ---------------------------------------------------------------------------
// Bootstrap command
// ---------------------------------------------------------------------------

/// Run the full bootstrap check-and-fix pipeline.
///
/// Checks all prerequisites in parallel, then fixes anything broken via the
/// dependency-aware engine. Progress is streamed as Tauri "bootstrap" events.
/// Returns immediately — the pipeline runs on a background thread.
#[tauri::command]
pub fn check_and_fix_bootstrap(app: tauri::AppHandle) -> Result<(), String> {
    let version = app.package_info().version.to_string();
    flog::log(&format!("=== check_and_fix_bootstrap called v={version} ==="));
    std::thread::spawn(move || {
        let app_for_events = app.clone();
        let report = bootstrap::check_and_fix(&version, move |e| dispatch(&app_for_events, e));
        flog::log(&format!(
            "check_and_fix_bootstrap complete: all_ok={} gates={}",
            report.all_ok,
            report.gates.len()
        ));
        for gate in &report.gates {
            flog::log(&format!(
                "  gate [{}] {:?} fix_attempted={}",
                gate.id, gate.status, gate.fix_attempted
            ));
        }
        write_bootstrap_session(&app, &report);
        // Emit final report so the frontend knows the engine is done
        let _ = app.emit("bootstrap-report", &report);
    });
    Ok(())
}

fn write_bootstrap_session(app: &tauri::AppHandle, report: &BootstrapReport) {
    let hw = bootstrap::hardware::detect();
    let system_info = collect_system_info(&hw);

    let outcome = if report.all_ok {
        "success"
    } else if report.blocked_on.is_some() {
        "blocked"
    } else {
        "failed"
    };

    let traces: Vec<serde_json::Value> = report
        .gates
        .iter()
        .filter_map(|g| serde_json::to_value(g).ok())
        .collect();

    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let session_id = format!(
        "sess-bs-{}-{:04x}",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ"),
        COUNTER.fetch_add(1, Ordering::Relaxed) & 0xFFFF,
    );

    let session = LogSession {
        id:          session_id,
        module:      "bootstrap".to_string(),
        started_at:  chrono::Utc::now().to_rfc3339(),
        app_version: app.package_info().version.to_string(),
        system_info,
        outcome:     outcome.to_string(),
        duration_ms: 0,
        traces,
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

// ---------------------------------------------------------------------------
// Read-only commands (hardware, models, platform, port)
// ---------------------------------------------------------------------------

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

/// Return the daemon port for the current runtime mode.
///
/// Delegates to [`SenseiConfig::from_env`] so the frontend never hard-codes a port.
/// The frontend calls this once at startup to initialise `appState.port`.
#[tauri::command]
pub fn get_daemon_port() -> u16 {
    let cfg = sensei_bootstrap::SenseiConfig::from_env();
    flog::log(&format!(
        "get_daemon_port: mode={:?} port={} db={}",
        cfg.mode, cfg.daemon_port, cfg.db_name
    ));
    cfg.daemon_port
}
