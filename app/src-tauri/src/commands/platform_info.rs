//! Auxiliary read-only commands — hardware + models + daemon-port.
//! Each delegates to `sensei_bootstrap` and adds zero logic.

use sensei_bootstrap::{self as bootstrap, HardwareInfo};

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
pub fn get_daemon_port() -> u16 {
    bootstrap::daemon_port()
}
