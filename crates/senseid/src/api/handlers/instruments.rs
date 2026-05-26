//! `GET /api/instruments` — list known MCP servers for the setup wizard.
//!
//! Recommendation is computed daemon-side by intersecting the registry's
//! stack-affinity keywords with the detected stack across all projects.
//! Letting the daemon do this keeps the wire shape stable and means a new
//! recommendation rule lands without a frontend release.

use axum::{extract::State, http::StatusCode, response::Json};
use crate::api::state::AppState;

/// Aggregate every project's stack into a single list. Used as the
/// recommendation input — a keyword needs to match anywhere to count.
async fn detected_stack(state: &AppState) -> Vec<String> {
    let projects = match state.pg.list_projects().await {
        Ok(p) => p,
        Err(_) => return vec![],
    };
    let mut stack: Vec<String> = vec![];
    for project in &projects {
        let project_id = project["id"].as_str()
            .and_then(|s| uuid::Uuid::parse_str(s).ok());
        let Some(pid) = project_id else { continue };
        let folders = state.pg.list_folders_by_project(&pid).await.unwrap_or_default();
        for folder in &folders {
            if let Some(s) = folder["stack"].as_array() {
                for tag in s {
                    if let Some(t) = tag.as_str() {
                        stack.push(t.to_string());
                    }
                }
            }
        }
    }
    stack
}

pub(crate) async fn list_instruments(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let stack = detected_stack(&state).await;
    let mcps = crate::instruments::list_for_stack(&stack);
    Ok(Json(serde_json::json!({
        "total": mcps.len(),
        "mcps": mcps,
        "stack": stack,
    })))
}
