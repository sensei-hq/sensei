//! Assistant detection and configuration — thin proxy to the daemon API.
//!
//! The Tauri sidecar owns bootstrap/health/upgrade only.
//! All assistant logic (detection, configuration, removal) is daemon-owned.
//! These commands are pure passthroughs — no types are duplicated here.

use serde_json::{json, Value};

/// Daemon base URL — derived from `SenseiConfig::from_env()` (the
/// single source of truth in `sensei-bootstrap`). Never hardcode the
/// port or the prod/dev distinction here: previously this file used
/// `cfg!(debug_assertions)` to talk to port 7745 in debug builds,
/// which broke silently when the dev/prod split was ripped out (port
/// is 7744 unconditionally now and nothing listens on 7745).
fn daemon_url() -> String {
    sensei_bootstrap::SenseiConfig::from_env().daemon_url()
}

/// Returns the daemon's full assistant list. Caller receives raw daemon JSON.
/// Fails closed: a daemon-unreachable or unparseable response is an `Err`, never
/// an empty list (which would read as "no assistants detected"). See #109 audit.
#[tauri::command]
pub fn detect_assistants() -> Result<Vec<Value>, String> {
    match ureq::get(&format!("{}/api/assistants/detect", daemon_url())).call() {
        Ok(resp) => resp.into_json().map_err(|e| format!("parse assistants: {e}")),
        Err(e) => Err(format!("Daemon not available: {e}")),
    }
}

#[tauri::command]
pub fn configure_mcp(assistants: Vec<String>) -> Result<Vec<String>, String> {
    let body = json!({"acps": assistants});
    match ureq::post(&format!("{}/api/assistants/configure", daemon_url()))
        .send_json(&body)
    {
        Ok(resp) => {
            // A malformed 200 body must error, not silently become {} → an empty
            // "configured nothing, no errors" success (#109 audit).
            let result: Value = resp.into_json()
                .map_err(|e| format!("parse configure response: {e}"))?;
            let configured: Vec<String> = result["configured"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            if let Some(errors) = result["errors"].as_array() {
                if !errors.is_empty() {
                    let err_msgs: Vec<&str> = errors.iter().filter_map(|e| e.as_str()).collect();
                    if configured.is_empty() {
                        return Err(err_msgs.join(", "));
                    }
                }
            }
            Ok(configured)
        }
        Err(e) => Err(format!("Daemon not available: {e}")),
    }
}

/// Returns the daemon's full assistant status list. Caller receives raw daemon JSON.
/// Fails closed like [`detect_assistants`]: unreachable/unparseable → `Err`, not [].
#[tauri::command]
pub fn check_assistant_configs() -> Result<Vec<Value>, String> {
    match ureq::get(&format!("{}/api/assistants/detect", daemon_url())).call() {
        Ok(resp) => resp.into_json().map_err(|e| format!("parse assistant configs: {e}")),
        Err(e) => Err(format!("Daemon not available: {e}")),
    }
}
