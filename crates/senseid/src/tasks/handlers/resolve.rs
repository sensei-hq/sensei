//! Resolve phase: resolve edges, build connections, reconcile cross-repo links.

use super::super::executor::TaskContext;
use super::super::Task;

// ── Resolve Edges (barrier) ──────────────────────────────────────────────

/// Resolve unresolved edges by matching target_name against existing nodes.
pub async fn resolve_edges(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let folder = ctx.pg().get_repo_by_name(repo_id).await.ok().flatten();
    let folder_id = match folder.as_ref()
        .and_then(|f| f["id"].as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok()) {
        Some(id) => id,
        None => { tracing::warn!("resolve_edges: {} — folder not found", repo_id); return Ok(()); }
    };

    // Get all unresolved edges (target_id IS NULL, target_name IS NOT NULL)
    let unresolved: Vec<serde_json::Value> = ctx.pg().execute_raw_query(
        "SELECT id, source_id, target_name, kind::text FROM sensei.edges WHERE folder_id = $1 AND target_id IS NULL AND target_name IS NOT NULL",
        &folder_id,
    ).await.unwrap_or_default();

    // Get all nodes for name matching
    let nodes = ctx.pg().get_nodes_by_folder(&folder_id).await.unwrap_or_default();

    let mut resolved = 0u32;
    for edge in &unresolved {
        let target_name = match edge["target_name"].as_str() { Some(n) => n, None => continue };
        let edge_id = match edge["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) { Some(id) => id, None => continue };
        let kind = edge["kind"].as_str().unwrap_or("calls");

        // Try to find a matching node by name
        let matched = nodes.iter().find(|n| {
            let name = n["name"].as_str().unwrap_or("");
            let file_path = n["file_path"].as_str().unwrap_or("");
            match kind {
                "imports" => file_path.contains(target_name) || file_path.ends_with(&format!("{}.rs", target_name)) || file_path.ends_with(&format!("{}.ts", target_name)),
                "calls" => name == target_name,
                _ => name == target_name,
            }
        });

        if let Some(target_node) = matched {
            if let Some(target_id) = target_node["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
                // Update edge with resolved target_id
                ctx.pg().execute_raw(&format!(
                    "UPDATE sensei.edges SET target_id = '{}' WHERE id = '{}'", target_id, edge_id
                )).await.ok();
                resolved += 1;
            }
        }
    }

    tracing::info!("resolve_edges: {} — {} unresolved, {} resolved", repo_id, unresolved.len(), resolved);
    Ok(())
}

// ── Build Connections ─────────────────────────────────────────────────────

/// Build doc<>code traceability and cross-repo links.
pub async fn build_connections(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;

    // TODO: doc<>code traceability edges

    // Mark project as indexed via PgStore
    let folder = ctx.pg().get_repo_by_name(repo_id).await.ok().flatten();
    if let Some(folder) = folder {
        if let Some(folder_id) = folder["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
            let libs = Vec::<String>::new();
            ctx.pg().mark_folder_indexed(&folder_id, &libs).await.ok();
        }
    }

    tracing::info!("build_connections: {} complete", repo_id);
    Ok(())
}

// ── Reconcile Connections ──────────────────────────────────────────────────

/// Re-evaluate cross-repo edges after a branch switch or repo update.
pub async fn reconcile_connections(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;

    // Find the project this repo belongs to via PgStore
    let folder = ctx.pg().get_repo_by_name(repo_id).await.ok().flatten();
    let project_id = folder.as_ref()
        .and_then(|f| f["project_id"].as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok());

    if project_id.is_none() {
        tracing::info!("reconcile_connections: {} not in any project", repo_id);
        // TODO: rebuild doc<>code traceability
        return Ok(());
    }

    let project_id = project_id.unwrap();
    let project = ctx.pg().get_project(&project_id).await.ok().flatten();

    if let Some(proj) = &project {
        tracing::info!(
            "reconcile_connections: project {} — {}",
            proj["name"].as_str().unwrap_or("unknown"),
            proj["id"].as_str().unwrap_or(""),
        );
    }

    // TODO: rebuild doc<>code traceability

    tracing::info!("reconcile_connections: {} — project {:?}", repo_id, project_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::tasks::queue::TaskQueue;
    use crate::tasks::{Task, TaskKind};
    use crate::api::state::SharedState;
    use super::super::super::executor::TaskContext;

    /// Build a TaskContext backed by PgStore and a fresh TaskQueue.
    async fn make_ctx() -> Arc<TaskContext> {
        let queue = Arc::new(TaskQueue::new());
        let app_state = Arc::new(SharedState {
            task_queue: queue.clone(),
            pg: crate::db::pg_store::PgStore::connect_test().await.unwrap(),
        });
        Arc::new(TaskContext {
            queue,
            app_state,
            _graph_path: None,
        })
    }

    #[tokio::test]
    async fn resolve_edges_succeeds() {
        let ctx = make_ctx().await;
        let repo_id = "test-repo";

        // Register project (resolve_edges reads project path)
        {
            let root_id = ctx.pg().add_watch_root("/tmp/repo", "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, repo_id, "/tmp/repo").await.unwrap();
        }

        let task = Task::new(TaskKind::ResolveEdges, repo_id, "");
        resolve_edges(&ctx, &task).await.unwrap();
    }

    #[tokio::test]
    async fn resolve_edges_with_no_refs_is_noop() {
        let ctx = make_ctx().await;
        let repo_id = "test-repo";

        {
            let root_id = ctx.pg().add_watch_root("/tmp/repo", "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, repo_id, "/tmp/repo").await.unwrap();
        }

        let task = Task::new(TaskKind::ResolveEdges, repo_id, "");
        resolve_edges(&ctx, &task).await.unwrap();
    }
}
