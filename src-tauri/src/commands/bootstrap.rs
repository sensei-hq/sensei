//! Bootstrap commands — prerequisite detection, installation, and hardware profiling.
//!
//! Delegates to the sensei-bootstrap crate. All functions are synchronous
//! (bootstrap uses shell commands and port probes, not async I/O).

use sensei_bootstrap::{
    self as bootstrap,
    BootstrapResult, HardwareInfo,
};

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

