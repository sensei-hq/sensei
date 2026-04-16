//! Task handlers — one function per TaskKind.
//! Each handler receives a TaskContext and a Task, performs its work,
//! and may enqueue child tasks.

use super::executor::TaskContext;
use super::{Task, TaskKind};

/// Scan a root directory for git repos, register projects, enqueue process_repo tasks.
pub async fn scan_root(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    // TODO: implement — migrate from routes.rs scan_folder
    tracing::info!("scan_root: {}", task.path);
    Ok(())
}

/// Process a repo: detect subtrees/workspaces, create virtual nodes, enqueue folder tasks.
pub async fn process_repo(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    // TODO: implement — decompose from pipeline.rs Pass 0 + file discovery
    tracing::info!("process_repo: {} at {}", task.repo_id, task.path);
    Ok(())
}

/// Process a folder: create module node, enqueue file tasks.
pub async fn process_folder(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    // TODO: implement — decompose from pipeline.rs Pass 6 module creation
    tracing::info!("process_folder: {}", task.path);
    Ok(())
}

/// Process a single file: adapter-based extraction, create nodes + unresolved refs.
pub async fn process_file(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    // TODO: implement — decompose from pipeline.rs Pass 1 per-file processing
    tracing::info!("process_file: {}", task.path);
    Ok(())
}

/// Delete a file's nodes and edges from the graph.
pub async fn delete_file(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let graph = ctx.graph.lock().await;
    graph.delete_by_file(&task.path, &task.repo_id)?;
    tracing::info!("delete_file: {}", task.path);
    Ok(())
}

/// Delete a folder's nodes (all files under this path) from the graph.
pub async fn delete_folder(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let graph = ctx.graph.lock().await;
    // Delete all nodes whose file path starts with this folder
    let nodes = graph.get_nodes(&task.repo_id)?;
    for node in &nodes {
        if node.file.starts_with(&task.path) {
            graph.delete_node(&node.id)?;
        }
    }
    // Delete the module node for this folder
    let mod_id = format!("mod:{}:{}", task.repo_id, task.path);
    graph.delete_node(&mod_id)?;
    tracing::info!("delete_folder: {} ({} nodes checked)", task.path, nodes.len());
    Ok(())
}

/// Barrier task: resolve CALLS, IMPORTS, HAS_METHOD edges from unresolved references.
pub async fn resolve_edges(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    // TODO: implement — decompose from pipeline.rs Pass 2, 3, 7
    tracing::info!("resolve_edges: {}", task.repo_id);
    Ok(())
}

/// After resolve: build doc↔code traceability, cross-repo links.
pub async fn build_connections(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    // TODO: implement — decompose from pipeline.rs Pass 4 + doc_indexer + cross_repo
    tracing::info!("build_connections: {}", task.repo_id);
    Ok(())
}
