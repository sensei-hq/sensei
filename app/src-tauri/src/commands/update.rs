//! Update commands — post-restart upgrade steps and update checking.
//!
//! Upgrade flow:
//!   1. Tauri updater downloads new app version, writes `sensei:app-version` to
//!      localStorage, then calls `install()` which triggers a restart.
//!   2. On next launch, the frontend detects the flag and navigates to /upgrade.
//!   3. /upgrade calls `run_upgrade_steps` which runs brew bundle + db deploy
//!      and streams progress on the "upgrade" event channel.
//!   4. /upgrade clears the flag and redirects to /health.
//!
//! NOTE: `run_upgrade_steps` is currently stubbed. The legacy implementation
//! depended on `sensei_bootstrap::prereq::fixer::BrewBundleFixer` and
//! `sensei_bootstrap::database` — both removed in the health rewrite (Section 0).
//! A proper implementation using the new health resolver layer will be provided
//! in a follow-up task (Section H).

use crate::flog;
use tauri::Emitter;

// ---------------------------------------------------------------------------
// Upgrade steps
// ---------------------------------------------------------------------------

/// Run post-restart upgrade steps: brew bundle upgrade + database deploy.
///
/// Streams progress on the "upgrade" Tauri event channel:
///   { step: "brew_bundle" | "db_deploy", status: "running" | "done" | "failed", error?: string }
///   { step: "complete", status: "ok" | "partial" }
///
/// Returns immediately — steps run on a background thread.
///
/// STUB: emits a "partial" outcome until Section H reimplements using the new
/// resolver layer.
#[tauri::command]
pub fn run_upgrade_steps(app: tauri::AppHandle) -> Result<(), String> {
    let version = app.package_info().version.to_string();
    flog::log(&format!("=== run_upgrade_steps v={version} (stubbed) ==="));

    std::thread::spawn(move || {
        flog::log("upgrade: stubbed — legacy prereq/database removed; Section H will reimplement");
        let _ = app.emit("upgrade", serde_json::json!({
            "step": "complete", "status": "partial",
            "error": "upgrade steps not yet implemented in this release"
        }));
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
