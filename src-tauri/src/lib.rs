//! Sensei Desktop — Tauri application entry point.

mod commands;
mod log_collector;

use log_collector::LogCollector;
use tauri::{Emitter, Manager};

pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
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
            // Logs
            commands::logs::log_session_start,
            commands::logs::log_entry,
            commands::logs::log_session_end,
            commands::logs::get_log_sessions,
        ])
        .setup(|app| {
            // ── Vibrancy ──────────────────────────────────────────────────
            let window = app.get_webview_window("main").unwrap();
            #[cfg(target_os = "macos")]
            {
                use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
                let _ = apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None);
            }
            #[cfg(debug_assertions)]
            window.open_devtools();

            // ── LogCollector managed state ────────────────────────────────
            let log_dir = app
                .path()
                .app_data_dir()
                .expect("failed to get app data dir")
                .join("sensei")
                .join("logs");
            app.manage(LogCollector::new(log_dir));

            // ── Menu: View > Diagnostic Logs ──────────────────────────────
            use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
            let logs_item = MenuItemBuilder::with_id("open-logs", "Diagnostic Logs")
                .accelerator("CmdOrCtrl+Shift+L")
                .build(app)?;
            let view_menu = SubmenuBuilder::new(app, "View")
                .item(&logs_item)
                .build()?;
            let menu = MenuBuilder::new(app)
                .item(&view_menu)
                .build()?;
            app.set_menu(menu)?;
            app.on_menu_event(|app, event| {
                if event.id() == "open-logs" {
                    let _ = app.emit("open-logs", ());
                }
            });

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
