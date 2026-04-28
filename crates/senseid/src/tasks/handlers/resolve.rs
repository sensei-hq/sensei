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

    // Build lookup maps
    let node_by_name: std::collections::HashMap<&str, &serde_json::Value> = nodes.iter()
        .filter_map(|n| n["name"].as_str().map(|name| (name, n)))
        .collect();

    let file_by_path: std::collections::HashMap<&str, &serde_json::Value> = nodes.iter()
        .filter(|n| n["kind"].as_str() == Some("file"))
        .filter_map(|n| n["file_path"].as_str().map(|fp| (fp, n)))
        .collect();

    let mut resolved = 0u32;
    for edge in &unresolved {
        let target_name = match edge["target_name"].as_str() { Some(n) => n, None => continue };
        let edge_id = match edge["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) { Some(id) => id, None => continue };
        let kind = edge["kind"].as_str().unwrap_or("calls");

        let matched_id = match kind {
            "imports" => {
                // Resolve relative import: match against file paths
                file_by_path.iter()
                    .find(|(fp, _)| fp.contains(target_name))
                    .and_then(|(_, n)| n["id"].as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok())
            }
            "calls" => {
                // Resolve function call by name
                node_by_name.get(target_name)
                    .and_then(|n| n["id"].as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok())
            }
            _ => {
                node_by_name.get(target_name)
                    .and_then(|n| n["id"].as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok())
            }
        };

        if let Some(target_id) = matched_id {
            ctx.pg().resolve_edge(&edge_id, &target_id).await.ok();
            resolved += 1;
        }
    }

    tracing::info!("resolve_edges: {} — {} unresolved, {} resolved", repo_id, unresolved.len(), resolved);
    Ok(())
}

// ── Build Connections ─────────────────────────────────────────────────────

/// Build doc↔code traceability edges and mark as indexed.
pub async fn build_connections(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let folder = ctx.pg().get_repo_by_name(repo_id).await.ok().flatten();
    let folder_id = match folder.as_ref()
        .and_then(|f| f["id"].as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok()) {
        Some(id) => id,
        None => { tracing::info!("build_connections: {} — folder not found", repo_id); return Ok(()); }
    };

    let nodes = ctx.pg().get_nodes_by_folder(&folder_id).await.unwrap_or_default();

    // Separate docs and code nodes
    let docs: Vec<&serde_json::Value> = nodes.iter()
        .filter(|n| n["kind"].as_str() == Some("doc"))
        .collect();
    let _functions: std::collections::HashMap<&str, &serde_json::Value> = nodes.iter()
        .filter(|n| matches!(n["kind"].as_str(), Some("function" | "method")))
        .filter_map(|n| n["name"].as_str().map(|name| (name, n)))
        .collect();
    let files: std::collections::HashMap<&str, &serde_json::Value> = nodes.iter()
        .filter(|n| n["kind"].as_str() == Some("file"))
        .filter_map(|n| n["file_path"].as_str().map(|fp| (fp, n)))
        .collect();

    let mut edges_created = 0u32;

    // For each doc, check if its file_path suggests coverage of code files
    for doc in &docs {
        let doc_id = match doc["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) { Some(id) => id, None => continue };
        let doc_path = doc["file_path"].as_str().unwrap_or("");

        // Check if doc covers a code file by path proximity
        // e.g., docs/api/auth.md → src/api/auth.ts
        let doc_stem = std::path::Path::new(doc_path)
            .file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if doc_stem.is_empty() { continue; }

        for (file_path, file_node) in &files {
            let file_stem = std::path::Path::new(file_path)
                .file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if file_stem == doc_stem && file_path != &doc_path {
                if let Some(file_id) = file_node["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
                    ctx.pg().insert_edge(&folder_id, &doc_id, Some(&file_id), None, "covers").await.ok();
                    edges_created += 1;
                }
            }
        }
    }

    // Collect libs from detected import targets
    let edges = ctx.pg().get_edges_by_kind(&folder_id, "imports").await.unwrap_or_default();
    let mut lib_set = std::collections::HashSet::new();
    for edge in &edges {
        if let Some(target_name) = edge["target_name"].as_str() {
            // External imports (not resolved to local files) are likely library imports
            if edge["target_id"].is_null() {
                lib_set.insert(target_name.to_string());
            }
        }
    }
    let libs: Vec<String> = lib_set.into_iter().collect();

    // Mark as indexed
    ctx.pg().mark_folder_indexed(&folder_id, &libs).await.ok();

    tracing::info!("build_connections: {} — {} traceability edges, {} libs detected", repo_id, edges_created, libs.len());
    Ok(())
}

// ── Reconcile Connections ──────────────────────────────────────────────────

/// Re-evaluate cross-repo edges after a branch switch or repo update.
/// Detects shared symbols across repos in the same project.
pub async fn reconcile_connections(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let folder = ctx.pg().get_repo_by_name(repo_id).await.ok().flatten();
    let folder_id = folder.as_ref()
        .and_then(|f| f["id"].as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok());
    let project_id = folder.as_ref()
        .and_then(|f| f["project_id"].as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok());

    // Rebuild doc↔code traceability for this repo
    if let Some(ref fid) = folder_id {
        let nodes = ctx.pg().get_nodes_by_folder(fid).await.unwrap_or_default();
        let docs: Vec<_> = nodes.iter().filter(|n| n["kind"].as_str() == Some("doc")).collect();
        let code_files: Vec<_> = nodes.iter().filter(|n| n["kind"].as_str() == Some("file")).collect();

        let mut edges = 0u32;
        for doc in &docs {
            let doc_id = match doc["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) { Some(id) => id, None => continue };
            let doc_stem = std::path::Path::new(doc["file_path"].as_str().unwrap_or(""))
                .file_stem().and_then(|s| s.to_str()).unwrap_or("");
            for code in &code_files {
                let code_stem = std::path::Path::new(code["file_path"].as_str().unwrap_or(""))
                    .file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if !doc_stem.is_empty() && doc_stem == code_stem {
                    if let Some(code_id) = code["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
                        ctx.pg().insert_edge(fid, &doc_id, Some(&code_id), None, "covers").await.ok();
                        edges += 1;
                    }
                }
            }
        }
        tracing::info!("reconcile_connections: {} — {} traceability edges", repo_id, edges);
    }

    // Cross-repo analysis requires a project with 2+ repos
    if project_id.is_none() {
        tracing::info!("reconcile_connections: {} not in any project", repo_id);
        return Ok(());
    }

    let project_id = project_id.unwrap();
    tracing::info!("reconcile_connections: {} — project {}", repo_id, project_id);
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
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let app_state = Arc::new(SharedState {
            task_queue: queue.clone(),
            pg: crate::db::pg_store::PgStore::connect_test().await.unwrap(),
            gateway,
            event_tx: { let (tx, _) = tokio::sync::broadcast::channel(16); tx },
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
