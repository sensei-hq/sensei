//! Update commands — post-restart upgrade steps and update checking.
//!
//! Upgrade flow:
//!   1. Tauri updater downloads new app version, writes `sensei:app-version`
//!      to localStorage, then calls `install()` which triggers a restart.
//!   2. On next launch, the frontend detects the flag (and that the stored
//!      version differs from the running app version, per L10) and navigates
//!      to /upgrade.
//!   3. /upgrade calls `run_upgrade_steps`, which delegates to
//!      `sensei_bootstrap::upgrade::run()` — brew upgrade + dbd deploy — and
//!      streams progress on the "upgrade" event channel.
//!   4. /upgrade clears the flag and redirects to /health.

use crate::flog;
use sensei_bootstrap::upgrade::{self, UpgradeEvent};
use tauri::Emitter;

// ---------------------------------------------------------------------------
// Upgrade steps
// ---------------------------------------------------------------------------

/// Run post-restart upgrade steps: brew upgrade + database schema deploy.
///
/// Streams progress on the "upgrade" Tauri event channel:
///   { step: "prereqs" | "db_deploy", status: "running" | "done" | "failed", error?: string }
///   { step: "complete", status: "ok" | "partial" }
///
/// Returns immediately — steps run on a background thread.
#[tauri::command]
pub fn run_upgrade_steps(app: tauri::AppHandle) -> Result<(), String> {
    let version = app.package_info().version.to_string();
    flog::log(&format!("=== run_upgrade_steps v={version} ==="));

    std::thread::spawn(move || {
        upgrade::run(|ev: UpgradeEvent| {
            let payload = match ev.error {
                Some(err) => serde_json::json!({
                    "step": ev.step, "status": ev.status, "error": err,
                }),
                None => serde_json::json!({
                    "step": ev.step, "status": ev.status,
                }),
            };
            let _ = app.emit("upgrade", payload);
        });
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// Update check (stub — requires signing key + latest.json in CI)
// ---------------------------------------------------------------------------

/// Emit a "check-for-update" event so the frontend can show update status.
/// The actual download/install requires tauri-plugin-updater with a signing key.
#[tauri::command]
pub fn check_for_update(app: tauri::AppHandle) -> Result<(), String> {
    flog::log("check_for_update called");
    let _ = app.emit("update-check-requested", ());
    Ok(())
}
