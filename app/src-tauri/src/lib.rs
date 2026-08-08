//! Sensei Desktop — Tauri application entry point.

mod commands;
mod flog;
mod log_collector;

use log_collector::LogCollector;
use tauri::{Emitter, Manager};

/// Send `tracing::info!` / `debug!` events from `sensei-bootstrap` to the
/// same `/tmp/sensei-bootstrap.log` file the `flog::log` helper writes to.
/// The shared helper handles open-failure (silent no-op) and `try_init`
/// idempotency so this is a one-liner.
fn install_tracing() {
    sensei_bootstrap::tracing_init::install_file(
        "/tmp/sensei-bootstrap.log",
        "sensei_bootstrap=info",
    );
}

/// Build the full application menu. The Window submenu lists the currently-open
/// project windows (label → focus item) after the standard minimize/maximize, so
/// the user can raise any of them from the menu bar. macOS does not auto-populate
/// a Tauri Window menu, so the caller rebuilds this whenever the set changes.
fn build_app_menu(
    app: &tauri::AppHandle,
    windows: &[(String, String)],
) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    // On macOS the FIRST submenu becomes the application menu (shown with the app
    // name); include it explicitly so File/Edit/etc. appear as separate items.
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

    // Window: standard actions, then one focus item per open project window.
    let mut window_builder = SubmenuBuilder::new(app, "Window").minimize().maximize();
    let mut win_items = Vec::with_capacity(windows.len());
    for (label, title) in windows {
        win_items.push(
            MenuItemBuilder::with_id(format!("focus-window:{label}"), title).build(app)?,
        );
    }
    if !win_items.is_empty() {
        window_builder = window_builder.separator();
        for it in &win_items {
            window_builder = window_builder.item(it);
        }
    }
    let window_menu = window_builder.build()?;

    let help_menu = SubmenuBuilder::new(app, "Help")
        .text("shortcuts", "Keyboard Shortcuts")
        .text("whats-new", "What's New")
        .separator()
        .text("report-issue", "Report an Issue")
        .build()?;

    MenuBuilder::new(app)
        .item(&app_submenu)   // ← app menu first (shown as "Sensei")
        .item(&file_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .item(&window_menu)
        .item(&help_menu)
        .build()
}

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
            commands::bootstrap::health_check,
            commands::bootstrap::health_check_traced,
            commands::bootstrap::health_check_and_resolve,
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
            // ── Tracing subscriber → flog ────────────────────────────────
            // The sidecar links `sensei-bootstrap`, which emits structured
            // tracing events at every probe / brew shell-out / dbd step.
            // Without a subscriber installed those events go nowhere; with
            // this one they get appended to /tmp/sensei-bootstrap.log next
            // to the explicit flog::log lines. Default level is `info`;
            // bump via RUST_LOG=sensei_bootstrap=debug when launching.
            install_tracing();

            // ── Startup banner ────────────────────────────────────────────
            let cfg = sensei_bootstrap::SenseiConfig::from_env();
            flog::log(&format!(
                "=== Sensei.app starting v={} db={} port={} ===",
                app.package_info().version,
                cfg.db_name, cfg.daemon_port,
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
            // Built by `build_app_menu`; the Window submenu lists open project
            // windows. Start empty — the store emits `sync-window-menu` as windows
            // open/close and we rebuild.
            let menu = build_app_menu(app.handle(), &[])?;
            app.set_menu(menu)?;

            // Rebuild the Window menu whenever the set of open project windows
            // changes. macOS does NOT auto-populate a Tauri Window menu, so the
            // app's windows store (windows.svelte.ts) emits the current list and
            // we swap the whole menu (cheap; happens only on open/close).
            {
                use tauri::Listener;
                let handle = app.handle().clone();
                app.listen_any("sync-window-menu", move |event| {
                    #[derive(serde::Deserialize)]
                    struct WinEntry { label: String, title: String }
                    match serde_json::from_str::<Vec<WinEntry>>(event.payload()) {
                        Ok(list) => {
                            let windows: Vec<(String, String)> =
                                list.into_iter().map(|w| (w.label, w.title)).collect();
                            match build_app_menu(&handle, &windows) {
                                Ok(menu) => { let _ = handle.set_menu(menu); }
                                Err(e) => flog::log(&format!("sync-window-menu: build failed: {e}")),
                            }
                        }
                        Err(e) => flog::log(&format!("sync-window-menu: bad payload: {e}")),
                    }
                });
            }
            app.on_menu_event(|app, event| {
                match event.id().as_ref() {
                    "open-logs" => {
                        let _ = app.emit("open-logs", ());
                    }
                    "check-for-updates" => {
                        let _ = app.emit("update-check-requested", ());
                    }
                    // View → Health is an explicit inspection request: pass
                    // `?auto=false` so /health stays put when status is ok
                    // instead of redirecting straight to the observatory.
                    "go-health"      => { let _ = app.emit("dev-navigate", "/health?auto=false"); }
                    "go-upgrade"     => { let _ = app.emit("dev-navigate", "/upgrade"); }
                    // "(observatory)" is a SvelteKit group, not a URL segment —
                    // the observatory page lives at "/".
                    "go-observatory" => { let _ = app.emit("dev-navigate", "/"); }
                    // `?force=1` lets the user re-enter setup after it's already
                    // been completed. The (config) layout's setupOk-redirect
                    // (which exists to recover from a cold-start race) bails
                    // out when this param is present, so the menu always
                    // brings the user back to the welcome stage.
                    "go-setup"       => { let _ = app.emit("dev-navigate", "/setup/welcome?force=1"); }
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
                    // Window menu → raise the chosen open project window.
                    id if id.starts_with("focus-window:") => {
                        let label = &id["focus-window:".len()..];
                        if let Some(w) = app.get_webview_window(label) {
                            let _ = w.set_focus();
                        }
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
