//! `GET /api/instruments` — list known MCP servers for the setup wizard.
//!
//! Recommendation is computed daemon-side by intersecting the registry's
//! stack-affinity keywords with the detected stack across all workspaces.
//! Letting the daemon do this keeps the wire shape stable and means a new
//! recommendation rule lands without a frontend release.

use crate::api::state::AppState;
use crate::instruments::{McpEntry, REGISTRY};
use axum::{extract::State, http::StatusCode, response::Json};

/// Per-workspace stack — `workspace_count` for each MCP is the number of these
/// workspaces whose stack matches any of the MCP's keywords.
async fn per_workspace_stacks(state: &AppState) -> Result<Vec<Vec<String>>, StatusCode> {
    // Fail closed: a DB error must not read as "no workspaces / no stacks" — that
    // would silently zero every recommendation + workspace_count as if the user's
    // stack matched nothing (the #109 audit).
    let workspaces =
        state.pg.list_workspaces().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut stacks = Vec::with_capacity(workspaces.len());
    for workspace in &workspaces {
        let Some(pid) = workspace["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) else {
            continue;
        };
        let folders = state
            .pg
            .list_folders_by_workspace(&pid)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    Ok(stacks)
}

/// True when any of this MCP's keywords appears as a substring in any tag
/// of the given (already-lowercased) workspace stack.
fn entry_matches_workspace(entry: &McpEntry, workspace_stack: &[String]) -> bool {
    if entry.stack_keywords.is_empty() {
        return false;
    }
    entry.stack_keywords.iter().any(|kw| workspace_stack.iter().any(|s| s.contains(kw)))
}

pub(crate) async fn list_instruments(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let workspace_stacks = per_workspace_stacks(&state).await?;

    // Flat union of every workspace's stack, used for the top-level
    // `recommended` flag (a single workspace that uses Postgres is enough
    // for Postgres MCP to show up in the recommendation set).
    let flat_stack: Vec<String> = workspace_stacks.iter().flat_map(|s| s.iter().cloned()).collect();

    // Probing config files is cheap (~handful of small JSON reads) but it's
    // pure synchronous IO — move it off the runtime so a slow disk doesn't
    // stall axum's executor.
    let installed = tokio::task::spawn_blocking(crate::assistants::installed_mcp_keys)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut mcps = crate::instruments::list_for_stack(&flat_stack, &installed);

    // Overlay per-entry workspace_count using the per-workspace stacks.
    for value in mcps.iter_mut() {
        let id = value["id"].as_str().unwrap_or("");
        if let Some(entry) = REGISTRY.iter().find(|e| e.id == id) {
            let count = workspace_stacks
                .iter()
                .filter(|stack| entry_matches_workspace(entry, stack))
                .count();
            value["workspace_count"] = serde_json::json!(count);
        }
    }

    Ok(Json(serde_json::json!({
        "total": mcps.len(),
        "mcps": mcps,
        "stack": flat_stack,
        "workspace_count": workspace_stacks.len(),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_matches_workspace_respects_substring() {
        let entry = REGISTRY.iter().find(|e| e.id == "postgres-mcp").unwrap();
        assert!(entry_matches_workspace(entry, &["postgresql".to_string()]));
        assert!(entry_matches_workspace(entry, &["supabase".to_string()]));
        assert!(!entry_matches_workspace(entry, &["rust".to_string()]));
        assert!(!entry_matches_workspace(entry, &[]));
    }

    #[test]
    fn entry_without_keywords_never_matches() {
        let fs = REGISTRY.iter().find(|e| e.id == "filesystem-mcp").unwrap();
        assert!(!entry_matches_workspace(fs, &["anything".to_string(), "rust".to_string()]));
    }
}
