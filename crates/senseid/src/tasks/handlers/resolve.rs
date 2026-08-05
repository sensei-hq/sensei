//! Resolve phase: resolve edges, build connections, reconcile cross-repo links.

use super::super::executor::TaskContext;
use super::super::Task;

// ── Resolve Edges (barrier) ──────────────────────────────────────────────

/// Resolve unresolved edges by matching target_name against existing nodes.
pub async fn resolve_edges(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let folder = match ctx.pg().get_repo_by_path(&task.folder_path).await {
        Ok(f) => f,
        Err(e) => { tracing::warn!(error = %e, path = %task.folder_path, "resolve_edges: get_repo_by_path failed"); None }
    };
    let folder_name = folder.as_ref()
        .and_then(|f| f["name"].as_str())
        .unwrap_or_else(|| task.folder_name());
    let folder_id = match folder.as_ref()
        .and_then(|f| crate::api::util::json_uuid(&f["id"])) {
        Some(id) => id,
        None => { tracing::warn!("resolve_edges: {} — folder not found", task.folder_path); return Ok(0); }
    };

    // Get all unresolved edges (target_id IS NULL, target_name IS NOT NULL)
    let unresolved: Vec<serde_json::Value> = ctx.pg().execute_raw_query(
        "SELECT id, source_id, target_name, kind::text FROM sensei.edges WHERE folder_id = $1 AND target_id IS NULL AND target_name IS NOT NULL",
        &folder_id,
    ).await.unwrap_or_else(|e| { tracing::warn!(error = %e, folder = %folder_name, "resolve_edges: query unresolved edges failed"); Vec::new() });

    // Get all nodes for name matching
    let nodes = ctx.pg().get_nodes_by_folder(&folder_id).await.unwrap_or_else(|e| { tracing::warn!(error = %e, folder = %folder_name, "resolve_edges: get_nodes_by_folder failed"); Vec::new() });

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
        let edge_id = match crate::api::util::json_uuid(&edge["id"]) { Some(id) => id, None => continue };
        let kind = edge["kind"].as_str().unwrap_or("calls");

        let matched_id = match kind {
            "imports" => {
                // Resolve relative import: match against file paths
                file_by_path.iter()
                    .find(|(fp, _)| fp.contains(target_name))
                    .and_then(|(_, n)| crate::api::util::json_uuid(&n["id"]))
            }
            "calls" => {
                // Resolve function call by name
                node_by_name.get(target_name)
                    .and_then(|n| crate::api::util::json_uuid(&n["id"]))
            }
            _ => {
                node_by_name.get(target_name)
                    .and_then(|n| crate::api::util::json_uuid(&n["id"]))
            }
        };

        if let Some(target_id) = matched_id {
            if let Err(e) = ctx.pg().resolve_edge(&edge_id, &target_id).await {
                tracing::warn!(error = %e, edge_id = %edge_id, "resolve_edges: resolve_edge failed");
            }
            resolved += 1;
        }
    }

    tracing::info!("resolve_edges: {} — {} unresolved, {} resolved", folder_name, unresolved.len(), resolved);
    Ok(resolved)
}

// ── Build Connections ─────────────────────────────────────────────────────

/// Build doc↔code traceability edges and mark as indexed.
pub async fn build_connections(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let folder_path = &task.folder_path;
    // abs_path lookup avoids name collisions across roots.
    let folder = match ctx.pg().get_repo_by_path(folder_path).await {
        Ok(f) => f,
        Err(e) => { tracing::warn!(error = %e, path = %folder_path, "build_connections: get_repo_by_path failed"); None }
    };
    let folder_name = folder.as_ref()
        .and_then(|f| f["name"].as_str())
        .unwrap_or_else(|| task.folder_name());
    let folder_id = match folder.as_ref()
        .and_then(|f| crate::api::util::json_uuid(&f["id"])) {
        Some(id) => id,
        None => { tracing::info!("build_connections: {} — folder not found", folder_path); return Ok(0); }
    };

    let nodes = ctx.pg().get_nodes_by_folder(&folder_id).await.unwrap_or_else(|e| { tracing::warn!(error = %e, folder = %folder_name, "build_connections: get_nodes_by_folder failed"); Vec::new() });

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
        let doc_id = match crate::api::util::json_uuid(&doc["id"]) { Some(id) => id, None => continue };
        let doc_path = doc["file_path"].as_str().unwrap_or("");

        // Check if doc covers a code file by path proximity
        // e.g., docs/api/auth.md → src/api/auth.ts
        let doc_stem = std::path::Path::new(doc_path)
            .file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if doc_stem.is_empty() { continue; }

        for (file_path, file_node) in &files {
            let file_stem = std::path::Path::new(file_path)
                .file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if file_stem == doc_stem && file_path != &doc_path
                && let Some(file_id) = crate::api::util::json_uuid(&file_node["id"]) {
                    if let Err(e) = ctx.pg().insert_edge(&folder_id, &doc_id, Some(&file_id), None, None, "covers").await {
                        tracing::warn!(error = %e, doc_id = %doc_id, file_id = %file_id, "build_connections: insert covers edge failed");
                    }
                    edges_created += 1;
                }
        }
    }

    // Collect libs from detected import targets
    let edges = ctx.pg().get_edges_by_kind(&folder_id, "imports").await.unwrap_or_else(|e| { tracing::warn!(error = %e, folder = %folder_name, "build_connections: get_edges_by_kind failed"); Vec::new() });
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

    // D6d — mark indexed, fail-closed: a ProcessFile that recorded a fatal
    // failure left the folder `failed`; don't advance it to `indexed`. The guard
    // is shared with resolve_libs (the other barrier writer) so it can't be
    // bypassed at one site.
    super::helpers::mark_folder_indexed_fail_closed(ctx, &folder_id, folder_name, &libs).await;

    tracing::info!("build_connections: {} — {} traceability edges, {} libs detected", folder_name, edges_created, libs.len());
    Ok(edges_created)
}

// ── Reconcile Connections ──────────────────────────────────────────────────

/// Re-evaluate cross-repo edges after a branch switch or repo update.
/// Detects shared symbols across repos in the same project.
pub async fn reconcile_connections(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let folder = match ctx.pg().get_repo_by_path(&task.folder_path).await {
        Ok(f) => f,
        Err(e) => { tracing::warn!(error = %e, path = %task.folder_path, "reconcile_connections: get_repo_by_path failed"); None }
    };
    let folder_name = folder.as_ref()
        .and_then(|f| f["name"].as_str())
        .unwrap_or_else(|| task.folder_name());
    let folder_id = folder.as_ref()
        .and_then(|f| crate::api::util::json_uuid(&f["id"]));
    let project_id = folder.as_ref()
        .and_then(|f| crate::api::util::json_uuid(&f["project_id"]));

    // Rebuild doc↔code traceability for this repo
    let mut edges = 0u32;
    if let Some(ref fid) = folder_id {
        let nodes = ctx.pg().get_nodes_by_folder(fid).await.unwrap_or_else(|e| { tracing::warn!(error = %e, folder = %folder_name, "reconcile_connections: get_nodes_by_folder failed"); Vec::new() });
        let docs: Vec<_> = nodes.iter().filter(|n| n["kind"].as_str() == Some("doc")).collect();
        let code_files: Vec<_> = nodes.iter().filter(|n| n["kind"].as_str() == Some("file")).collect();

        for doc in &docs {
            let doc_id = match crate::api::util::json_uuid(&doc["id"]) { Some(id) => id, None => continue };
            let doc_stem = std::path::Path::new(doc["file_path"].as_str().unwrap_or(""))
                .file_stem().and_then(|s| s.to_str()).unwrap_or("");
            for code in &code_files {
                let code_stem = std::path::Path::new(code["file_path"].as_str().unwrap_or(""))
                    .file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if !doc_stem.is_empty() && doc_stem == code_stem
                    && let Some(code_id) = crate::api::util::json_uuid(&code["id"]) {
                        if let Err(e) = ctx.pg().insert_edge(fid, &doc_id, Some(&code_id), None, None, "covers").await {
                            tracing::warn!(error = %e, doc_id = %doc_id, code_id = %code_id, "reconcile_connections: insert covers edge failed");
                        }
                        edges += 1;
                    }
            }
        }
        tracing::info!("reconcile_connections: {} — {} traceability edges", folder_name, edges);
    }

    // Cross-repo analysis requires a project with 2+ repos
    if project_id.is_none() {
        tracing::info!("reconcile_connections: {} not in any project", folder_name);
        return Ok(edges);
    }

    let project_id = project_id.unwrap();
    tracing::info!("reconcile_connections: {} — project {}", folder_name, project_id);
    Ok(edges)
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
            breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            provisioning: None,
        });
        Arc::new(TaskContext {
            queue,
            app_state,
            _graph_path: None,
            logger: sensei_logger::Logger::noop(),
        })
    }

    #[tokio::test]
    async fn resolve_edges_succeeds() {
        let ctx = make_ctx().await;
        let folder_name = "test-repo";
        let folder_path = "/tmp/repo";

        {
            let root_id = ctx.pg().add_watch_root(folder_path, "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, folder_name, folder_path).await.unwrap();
        }

        let task = Task::new(TaskKind::ResolveEdges, folder_path, folder_path);
        resolve_edges(&ctx, &task).await.unwrap();
    }

    #[tokio::test]
    async fn resolve_edges_with_no_refs_is_noop() {
        let ctx = make_ctx().await;
        let folder_name = "test-repo";
        let folder_path = "/tmp/repo";

        {
            let root_id = ctx.pg().add_watch_root(folder_path, "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, folder_name, folder_path).await.unwrap();
        }

        let task = Task::new(TaskKind::ResolveEdges, folder_path, folder_path);
        resolve_edges(&ctx, &task).await.unwrap();
    }

    #[tokio::test]
    async fn build_connections_is_fail_closed_on_a_failed_folder() {
        // D6d: a folder a ProcessFile marked `failed` (D6c-trigger) must NOT be
        // advanced to `indexed` by the terminal barrier — leave it `failed` so
        // boot-reconcile / bounded-retry re-drives it.
        let ctx = make_ctx().await;
        let folder_path = format!("/tmp/failclosed_{}", uuid::Uuid::new_v4());
        let root_id = ctx.pg().add_watch_root(&folder_path, "fc", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "fc-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "failed").await.unwrap();

        let task = Task::new(TaskKind::BuildConnections, &folder_path, &folder_path);
        build_connections(&ctx, &task).await.unwrap();

        assert_eq!(ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(), Some("failed"),
            "the barrier must not mark a failed folder indexed");
        ctx.pg().remove_watch_root(&root_id).await.ok();
    }

    #[tokio::test]
    async fn build_connections_marks_a_clean_folder_indexed() {
        // The happy path: a folder with no recorded failure is advanced to
        // `indexed` by the barrier (regression guard for the fail-closed check).
        let ctx = make_ctx().await;
        let folder_path = format!("/tmp/clean_{}", uuid::Uuid::new_v4());
        let root_id = ctx.pg().add_watch_root(&folder_path, "cl", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "cl-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        let task = Task::new(TaskKind::BuildConnections, &folder_path, &folder_path);
        build_connections(&ctx, &task).await.unwrap();

        assert_eq!(ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(), Some("indexed"),
            "a clean folder is marked indexed by the barrier");
        ctx.pg().remove_watch_root(&root_id).await.ok();
    }

    #[tokio::test]
    async fn resolve_libs_is_fail_closed_on_a_failed_folder() {
        // D6d regression: resolve_libs runs BEFORE build_connections in the
        // barrier chain and ALSO marks a folder indexed — so it must honour the
        // same fail-closed guard, else a `failed` folder is flipped to `indexed`
        // here and build_connections' guard never sees it (the real-pipeline
        // bypass the isolated build_connections test misses).
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap(); // empty dir → no libs to walk
        let folder_path = tmp.path().to_string_lossy().to_string();
        let root_id = ctx.pg().add_watch_root(&folder_path, "rl_fc", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "rl-fc-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "failed").await.unwrap();

        let task = Task::new(TaskKind::ResolveLibs, &folder_path, &folder_path);
        super::super::resolve_libs(&ctx, &task).await.unwrap();

        assert_eq!(ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(), Some("failed"),
            "resolve_libs must not mark a failed folder indexed (fail-closed)");
        ctx.pg().remove_watch_root(&root_id).await.ok();
    }
}
