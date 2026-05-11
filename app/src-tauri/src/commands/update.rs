//! Update commands — post-restart upgrade steps and update checking.
//!
//! Upgrade flow:
//!   1. Tauri updater downloads new app version, writes `sensei:app-version` to
//!      localStorage, then calls `install()` which triggers a restart.
//!   2. On next launch, the frontend detects the flag and navigates to /upgrade.
//!   3. /upgrade calls `run_upgrade_steps` which runs brew bundle + db deploy
//!      and streams progress on the "upgrade" event channel.
//!   4. /upgrade clears the flag and redirects to /health.

use crate::flog;
use sensei_bootstrap::{
    prereq::fixer::{BrewBundleFixer, Fixer},
    util, database, HOMEBREW_BREWFILE_URL,
};
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
#[tauri::command]
pub fn run_upgrade_steps(app: tauri::AppHandle) -> Result<(), String> {
    let version = app.package_info().version.to_string();
    flog::log(&format!("=== run_upgrade_steps v={version} ==="));

    std::thread::spawn(move || {
        let mut any_failed = false;

        // ── Step 1: brew bundle --upgrade ────────────────────────────────────
        flog::log("upgrade: step brew_bundle — running");
        let _ = app.emit("upgrade", serde_json::json!({
            "step": "brew_bundle", "status": "running"
        }));

        let brew_path = util::which_binary("brew")
            .unwrap_or_else(|| "brew".to_string());

        match BrewBundleFixer::new(&brew_path, HOMEBREW_BREWFILE_URL).fix() {
            Ok(r) => {
                flog::log(&format!("upgrade: brew_bundle done — {}", r.approach));
                let _ = app.emit("upgrade", serde_json::json!({
                    "step": "brew_bundle", "status": "done"
                }));
            }
            Err(e) => {
                flog::log(&format!("upgrade: brew_bundle failed — {e}"));
                let _ = app.emit("upgrade", serde_json::json!({
                    "step": "brew_bundle", "status": "failed", "error": e
                }));
                any_failed = true;
            }
        }

        // ── Step 2: database deploy (idempotent) ─────────────────────────────
        flog::log("upgrade: step db_deploy — running");
        let _ = app.emit("upgrade", serde_json::json!({
            "step": "db_deploy", "status": "running"
        }));

        match database::deploy(&version) {
            Ok(status) => {
                flog::log(&format!(
                    "upgrade: db_deploy done — {}",
                    status.version.as_deref().unwrap_or("unknown")
                ));
                let _ = app.emit("upgrade", serde_json::json!({
                    "step": "db_deploy", "status": "done"
                }));
            }
            Err(e) => {
                flog::log(&format!("upgrade: db_deploy failed — {e}"));
                let _ = app.emit("upgrade", serde_json::json!({
                    "step": "db_deploy", "status": "failed", "error": e
                }));
                any_failed = true;
            }
        }

        // ── Final: signal completion ─────────────────────────────────────────
        let outcome = if any_failed { "partial" } else { "ok" };
        flog::log(&format!("upgrade: complete outcome={outcome}"));
        let _ = app.emit("upgrade", serde_json::json!({
            "step": "complete", "status": outcome
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
