//! `GET /api/instruments` — list known MCP servers for the setup wizard.
//!
//! Recommendation is computed daemon-side by intersecting the registry's
//! stack-affinity keywords with the detected stack across all projects.
//! Letting the daemon do this keeps the wire shape stable and means a new
//! recommendation rule lands without a frontend release.

use axum::{extract::State, http::StatusCode, response::Json};
use crate::api::state::AppState;
use crate::instruments::{McpEntry, REGISTRY};

/// Per-project stack — `project_count` for each MCP is the number of these
/// projects whose stack matches any of the MCP's keywords.
async fn per_project_stacks(state: &AppState) -> Vec<Vec<String>> {
    let projects = match state.pg.list_projects().await {
        Ok(p) => p,
        Err(_) => return vec![],
    };
    let mut stacks = Vec::with_capacity(projects.len());
    for project in &projects {
        let Some(pid) = project["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok())
        else { continue };
        let folders = state.pg.list_folders_by_project(&pid).await.unwrap_or_default();
        let mut stack: Vec<String> = vec![];
        for folder in &folders {
            if let Some(s) = folder["stack"].as_array() {
                for tag in s {
                    if let Some(t) = tag.as_str() {
                        stack.push(t.to_lowercase());
                    }
                }
            }
        }
        stacks.push(stack);
    }
    stacks
}

/// True when any of this MCP's keywords appears as a substring in any tag
/// of the given (already-lowercased) project stack.
fn entry_matches_project(entry: &McpEntry, project_stack: &[String]) -> bool {
    if entry.stack_keywords.is_empty() { return false; }
    entry.stack_keywords.iter().any(|kw|
        project_stack.iter().any(|s| s.contains(kw))
    )
}

pub(crate) async fn list_instruments(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_stacks = per_project_stacks(&state).await;

    // Flat union of every project's stack, used for the top-level
    // `recommended` flag (a single project that uses Postgres is enough
    // for Postgres MCP to show up in the recommendation set).
    let flat_stack: Vec<String> = project_stacks.iter()
        .flat_map(|s| s.iter().cloned())
        .collect();

    // Probing config files is cheap (~handful of small JSON reads) but it's
    // pure synchronous IO — move it off the runtime so a slow disk doesn't
    // stall axum's executor.
    let installed = tokio::task::spawn_blocking(crate::assistants::installed_mcp_keys)
        .await
        .unwrap_or_default();

    let mut mcps = crate::instruments::list_for_stack(&flat_stack, &installed);

    // Overlay per-entry project_count using the per-project stacks.
    for value in mcps.iter_mut() {
        let id = value["id"].as_str().unwrap_or("");
        if let Some(entry) = REGISTRY.iter().find(|e| e.id == id) {
            let count = project_stacks.iter()
                .filter(|stack| entry_matches_project(entry, stack))
                .count();
            value["project_count"] = serde_json::json!(count);
        }
    }

    Ok(Json(serde_json::json!({
        "total": mcps.len(),
        "mcps": mcps,
        "stack": flat_stack,
        "project_count": project_stacks.len(),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_matches_project_respects_substring() {
        let entry = REGISTRY.iter().find(|e| e.id == "postgres-mcp").unwrap();
        assert!(entry_matches_project(entry, &["postgresql".to_string()]));
        assert!(entry_matches_project(entry, &["supabase".to_string()]));
        assert!(!entry_matches_project(entry, &["rust".to_string()]));
        assert!(!entry_matches_project(entry, &[]));
    }

    #[test]
    fn entry_without_keywords_never_matches() {
        let fs = REGISTRY.iter().find(|e| e.id == "filesystem-mcp").unwrap();
        assert!(!entry_matches_project(fs, &["anything".to_string(), "rust".to_string()]));
    }
}
