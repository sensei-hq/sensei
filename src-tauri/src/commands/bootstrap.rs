//! Bootstrap commands — prerequisite detection, installation, and hardware profiling.
//!
//! Delegates to the sensei-bootstrap crate. All functions are synchronous
//! (bootstrap uses shell commands and port probes, not async I/O).

use sensei_bootstrap::{
    self as bootstrap,
    BootstrapResult, ComponentStatus, HardwareInfo,
};

/// Run the full bootstrap check — all components + hardware detection.
#[tauri::command]
pub fn run_bootstrap() -> BootstrapResult {
    bootstrap::run()
}

/// Check all components individually (without hardware detection).
#[tauri::command]
pub fn check_all_components() -> Vec<ComponentStatus> {
    let result = bootstrap::run();
    result.components
}

/// Install a component by name via Homebrew.
#[tauri::command]
pub fn install_component(name: String) -> Result<ComponentStatus, String> {
    match name.as_str() {
        "homebrew" => Err("Homebrew requires terminal installation — run the command shown in the UI".into()),
        "daemon" => bootstrap::service::start_daemon(bootstrap::DAEMON_PORT),
        _ => bootstrap::homebrew::install_formula(&name),
    }
}

/// Start a service by name (brew services or daemon).
#[tauri::command]
pub fn start_component(name: String) -> Result<ComponentStatus, String> {
    match name.as_str() {
        "daemon" => bootstrap::service::start_daemon(bootstrap::DAEMON_PORT),
        "postgresql" | "postgresql@17" => {
            bootstrap::service::start_brew_service("postgresql@17")?;
            // Wait briefly and re-check
            std::thread::sleep(std::time::Duration::from_secs(2));
            Ok(bootstrap::service::check("postgresql", bootstrap::POSTGRES_PORT))
        }
        "ollama" => {
            bootstrap::service::start_brew_service("ollama")?;
            std::thread::sleep(std::time::Duration::from_secs(2));
            Ok(bootstrap::service::check("ollama", bootstrap::OLLAMA_PORT))
        }
        _ => Err(format!("unknown service: {name}")),
    }
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

/// Create the sensei database if it doesn't exist.
#[tauri::command]
pub fn create_database() -> Result<ComponentStatus, String> {
    bootstrap::database::create(None)
}
