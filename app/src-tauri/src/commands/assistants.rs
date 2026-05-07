//! Assistant detection and configuration — delegates to daemon API.

use serde_json::{json, Value};

/// Debug builds talk to the dev daemon (port 7745 / senseid-dev).
/// Release builds talk to the release daemon (port 7744 / senseid).
fn daemon_url() -> &'static str {
    if cfg!(debug_assertions) {
        "http://127.0.0.1:7745"
    } else {
        "http://127.0.0.1:7744"
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AssistantStatus {
    id: String,
    name: String,
    family: String,
    installed: bool,
    mcp_configured: bool,
    config_path: String,
}

#[tauri::command]
pub fn detect_assistants() -> Vec<String> {
    match ureq::get(&format!("{daemon_url()}/api/assistants/detect")).call() {
        Ok(resp) => {
            let assistants: Vec<Value> = resp.into_json().unwrap_or_default();
            assistants.iter()
                .filter(|a| a["installed"].as_bool() == Some(true))
                .filter_map(|a| a["id"].as_str().map(String::from))
                .collect()
        }
        Err(_) => vec![],
    }
}

#[tauri::command]
pub fn configure_mcp(assistants: Vec<String>) -> Result<Vec<String>, String> {
    let body = json!({"acps": assistants});
    match ureq::post(&format!("{daemon_url()}/api/assistants/configure"))
        .send_json(&body)
    {
        Ok(resp) => {
            let result: Value = resp.into_json().unwrap_or(json!({}));
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

#[tauri::command]
pub fn check_assistant_configs() -> Vec<AssistantStatus> {
    match ureq::get(&format!("{daemon_url()}/api/assistants/detect")).call() {
        Ok(resp) => resp.into_json().unwrap_or_default(),
        Err(_) => vec![],
    }
}
