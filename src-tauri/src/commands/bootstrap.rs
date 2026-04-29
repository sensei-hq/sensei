//! Bootstrap commands — prerequisite detection, installation, and hardware profiling.
//!
//! Delegates to the sensei-bootstrap crate. All functions are synchronous
//! (bootstrap uses shell commands and port probes, not async I/O).
//!
//! Phase commands (`install_prerequisites`, `start_services`, `setup_database`)
//! run blocking work on a background thread and emit progress events on the
//! `"bootstrap"` channel so the frontend can drive a gate-based UI.

use sensei_bootstrap::{
    self as bootstrap,
    BootstrapResult, HardwareInfo,
};
use tauri::Emitter;

// ---------------------------------------------------------------------------
// Event helpers
// ---------------------------------------------------------------------------

/// Emit a gate status update on the `"bootstrap"` channel.
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
            "data": {
                "status": status,
                "version": version,
                "detail": detail,
            }
        }),
    );
}

/// Emit a phase-completion event on the `"bootstrap"` channel.
fn emit_phase_complete(app: &tauri::AppHandle, phase: &str, success: bool) {
    let _ = app.emit(
        "bootstrap",
        serde_json::json!({
            "action": "set",
            "entity": "phase",
            "id": phase,
            "data": {
                "complete": true,
                "success": success,
            }
        }),
    );
}

// ---------------------------------------------------------------------------
// Existing commands
// ---------------------------------------------------------------------------

/// Run the full bootstrap check — all components + hardware detection.
#[tauri::command]
pub fn run_bootstrap() -> BootstrapResult {
    bootstrap::run()
}

/// Detect hardware capabilities.
#[tauri::command]
pub fn detect_hardware() -> HardwareInfo {
    bootstrap::hardware::detect()
}

/// List installed Ollama models.
#[tauri::command]
pub fn list_models() -> Vec<String> {
    bootstrap::models::list()
}

/// Check which models are missing for the recommended tier.
#[tauri::command]
pub fn missing_models() -> Vec<String> {
    let hw = bootstrap::hardware::detect();
    bootstrap::models::missing_models(&hw.recommended_tier)
}

// ---------------------------------------------------------------------------
// New commands
// ---------------------------------------------------------------------------

/// Return platform info for the frontend (package manager, remedies).
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

/// Phase 1 — install prerequisites (postgresql, ollama, sensei binaries).
///
/// Spawns a background thread that emits gate events as each binary is
/// checked, then emits a phase-completion event.
#[tauri::command]
pub fn install_prerequisites(app: tauri::AppHandle) -> Result<(), String> {
    std::thread::spawn(move || {
        let gates = ["postgresql", "ollama", "sensei"];

        // 1. Mark all gates as "checking"
        for gate in &gates {
            emit_gate(&app, gate, "checking", None, None);
        }

        // 2. Run the blocking install
        let provider = bootstrap::provider();
        if let Err(err) = provider.install_prerequisites() {
            for gate in &gates {
                emit_gate(&app, gate, "error", None, Some(&err));
            }
            emit_phase_complete(&app, "install", false);
            return;
        }

        // 3. Verify each binary and emit final status
        for gate in &gates {
            let binary = match *gate {
                "postgresql" => "postgres",
                "ollama" => "ollama",
                "sensei" => "sensei",
                other => other,
            };
            let check = bootstrap::util::check_binary(gate, binary, "--version");
            if check.is_ready() {
                emit_gate(
                    &app,
                    gate,
                    "ready",
                    check.version.as_deref(),
                    check.detail.as_deref(),
                );
            } else {
                emit_gate(&app, gate, "missing", None, None);
            }
        }

        emit_phase_complete(&app, "install", true);
    });

    Ok(())
}

/// Phase 2 — start services (postgresql, ollama, daemon).
///
/// Spawns a background thread that starts each service sequentially,
/// polling its port until responsive (max 30 attempts, 1 s apart).
#[tauri::command]
pub fn start_services(app: tauri::AppHandle) -> Result<(), String> {
    std::thread::spawn(move || {
        let services: [(&str, u16); 3] = [
            ("postgresql", bootstrap::POSTGRES_PORT),
            ("ollama", bootstrap::OLLAMA_PORT),
            ("daemon", bootstrap::DAEMON_PORT),
        ];

        let provider = bootstrap::provider();
        let mut all_ok = true;

        for (name, port) in &services {
            // Already listening? Skip straight to ready.
            if bootstrap::util::probe_port(*port) {
                let version = bootstrap::util::fetch_service_version(name, *port);
                emit_gate(&app, name, "ready", version.as_deref(), None);
                continue;
            }

            // Emit "starting" and attempt to start
            emit_gate(&app, name, "starting", None, None);
            if let Err(err) = provider.start_service(name) {
                emit_gate(&app, name, "error", None, Some(&err));
                all_ok = false;
                continue;
            }

            // Poll up to 30 s
            let mut ready = false;
            for _ in 0..30 {
                std::thread::sleep(std::time::Duration::from_secs(1));
                if bootstrap::util::probe_port(*port) {
                    ready = true;
                    break;
                }
            }

            if ready {
                let version = bootstrap::util::fetch_service_version(name, *port);
                emit_gate(&app, name, "ready", version.as_deref(), None);
            } else {
                emit_gate(
                    &app,
                    name,
                    "error",
                    None,
                    Some(&format!("timed out waiting for port {port}")),
                );
                all_ok = false;
            }
        }

        emit_phase_complete(&app, "services", all_ok);
    });

    Ok(())
}

/// Phase 3 — database setup (create db, extensions, migrations).
///
/// Spawns a background thread that runs the full database pipeline and
/// emits gate + phase events.
#[tauri::command]
pub fn setup_database(app: tauri::AppHandle) -> Result<(), String> {
    std::thread::spawn(move || {
        emit_gate(&app, "database", "checking", None, None);

        match bootstrap::database::setup(None) {
            Ok(status) => {
                emit_gate(
                    &app,
                    "database",
                    "ready",
                    status.version.as_deref(),
                    status.detail.as_deref(),
                );
                emit_phase_complete(&app, "database", true);
            }
            Err(err) => {
                emit_gate(&app, "database", "error", None, Some(&err));
                emit_phase_complete(&app, "database", false);
            }
        }
    });

    Ok(())
}

