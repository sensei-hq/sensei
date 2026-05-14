//! Sensei Desktop — Tauri application entry point.

mod commands;
mod flog;
mod log_collector;

use log_collector::LogCollector;
use tauri::{Emitter, Manager};

pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            // Platform info (hardware, models, daemon port) — read-only helpers
            commands::platform_info::detect_hardware,
            commands::platform_info::list_models,
            commands::platform_info::missing_models,
            commands::platform_info::get_daemon_port,
            // Bootstrap health commands — added in Task G2
            // commands::bootstrap::health_check,
            // commands::bootstrap::health_check_and_resolve,
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
            // Update
            commands::update::run_upgrade_steps,
            commands::update::check_for_update,
        ])
        .setup(|app| {
            // ── Startup banner ────────────────────────────────────────────
            let cfg = sensei_bootstrap::SenseiConfig::from_env();
            flog::log(&format!(
                "=== Sensei.app starting v={} mode={:?} db={} port={} ===",
                app.package_info().version,
                cfg.mode, cfg.db_name, cfg.daemon_port,
            ));

            // ── Vibrancy ──────────────────────────────────────────────────
            let window = app.get_webview_window("main")
                .ok_or("window 'main' not found")?;
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
                .map_err(|e| format!("failed to get app data dir: {e}"))?
                .join("sensei")
                .join("logs");
            app.manage(LogCollector::new(log_dir));

            // ── Menu ──────────────────────────────────────────────────────────
            // On macOS the FIRST submenu in MenuBuilder becomes the application
            // menu (the one shown with the app name). We must include it explicitly
            // so that "File", "Edit", etc. appear as separate menu bar items.
            use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};

            let app_submenu = SubmenuBuilder::new(app, "Sensei")
                .about(None)
                .separator()
                .text("check-for-updates", "Check for Updates…")
                .separator()
                .text("preferences", "Preferences…")
                .separator()
                .services()
                .separator()
                .hide()
                .hide_others()
                .show_all()
                .separator()
                .quit()
                .build()?;

            let file_menu = SubmenuBuilder::new(app, "File")
                .text("new-project", "New Project")
                .separator()
                .close_window()
                .build()?;

            let edit_menu = SubmenuBuilder::new(app, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?;

            let logs_item = MenuItemBuilder::with_id("open-logs", "Diagnostic Logs")
                .accelerator("CmdOrCtrl+Shift+L")
                .build(app)?;
            let view_menu = SubmenuBuilder::new(app, "View")
                .text("toggle-sidebar", "Toggle Sidebar")
                .separator()
                .item(&logs_item)
                .separator()
                .text("go-health",      "Health")
                .text("go-upgrade",     "Upgrade")
                .text("go-observatory", "Observatory")
                .text("go-setup",       "Setup")
                .build()?;

            let window_menu = SubmenuBuilder::new(app, "Window")
                .minimize()
                .maximize()
                .build()?;

            let help_menu = SubmenuBuilder::new(app, "Help")
                .text("shortcuts", "Keyboard Shortcuts")
                .text("whats-new", "What's New")
                .separator()
                .text("report-issue", "Report an Issue")
                .build()?;

            let menu = MenuBuilder::new(app)
                .item(&app_submenu)   // ← app menu first (shown as "Sensei")
                .item(&file_menu)
                .item(&edit_menu)
                .item(&view_menu)
                .item(&window_menu)
                .item(&help_menu)
                .build()?;
            app.set_menu(menu)?;
            app.on_menu_event(|app, event| {
                match event.id().as_ref() {
                    "open-logs" => {
                        let _ = app.emit("open-logs", ());
                    }
                    "check-for-updates" => {
                        let _ = app.emit("update-check-requested", ());
                    }
                    "go-health"      => { let _ = app.emit("dev-navigate", "/health"); }
                    "go-upgrade"     => { let _ = app.emit("dev-navigate", "/upgrade"); }
                    "go-observatory" => { let _ = app.emit("dev-navigate", "/observatory"); }
                    "go-setup"       => { let _ = app.emit("dev-navigate", "/setup/welcome"); }
                    "report-issue" => {
                        use tauri_plugin_opener::OpenerExt;
                        let url = format!(
                            "https://github.com/{}/{}/issues",
                            sensei_bootstrap::GITHUB_ORG,
                            sensei_bootstrap::GITHUB_REPO,
                        );
                        let _ = app.opener().open_url(&url, None::<&str>);
                    }
                    "shortcuts" => {
                        // TODO: open help window (Task 19)
                    }
                    _ => {}
                }
            });

            Ok(())
        });

    // Shadow with mut only when the e2e-testing feature needs to add the plugin
    #[cfg(feature = "e2e-testing")]
    let builder = builder.plugin(tauri_plugin_playwright::init());

    builder
        .run(tauri::generate_context!())
        .expect("error while running sensei desktop")
}
