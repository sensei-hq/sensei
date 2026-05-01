//! Sensei Desktop — Tauri application entry point.

mod commands;
mod log_collector;

use tauri::Manager;

pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .manage(log_collector::LogCollector::new(
            std::path::PathBuf::from(".sensei/logs"),
        ))
        .invoke_handler(tauri::generate_handler![
            // Bootstrap (prereqs, hardware, models)
            commands::bootstrap::run_bootstrap,
            commands::bootstrap::detect_hardware,
            commands::bootstrap::list_models,
            commands::bootstrap::missing_models,
            commands::bootstrap::get_platform,
            commands::bootstrap::install_prerequisites,
            commands::bootstrap::start_services,
            commands::bootstrap::setup_database,
            // Assistants (detection, MCP config)
            commands::assistants::detect_assistants,
            commands::assistants::configure_mcp,
            commands::assistants::check_assistant_configs,
            // Repos (scanning, analysis, dependencies)
            commands::repos::get_repo_id,
            commands::repos::analyze_folder,
            commands::repos::detect_dependencies,
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            #[cfg(target_os = "macos")]
            {
                use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
                let _ = apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None);
            }

            #[cfg(debug_assertions)]
            window.open_devtools();

            Ok(())
        });

    #[cfg(feature = "e2e-testing")]
    {
        builder = builder.plugin(tauri_plugin_playwright::init());
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running sensei desktop")
}
