//! Resolve phase: resolve edges, build connections, reconcile cross-repo links.

use std::path::Path;
use super::super::executor::TaskContext;
use super::super::Task;
use crate::types::Project;

// ── Resolve Edges (barrier) ───────────────────────────────────────────────

/// Resolve all unresolved references for a repo: IMPORTS, CALLS, HAS_METHOD, COVERS, MENTIONS_FN.
pub async fn resolve_edges(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let graph = ctx.graph().await;

    let refs = graph.get_unresolved_refs(repo_id)?;
    let nodes = graph.get_nodes(repo_id)?;

    // Build lookup maps
    let file_by_path: std::collections::HashMap<String, String> = nodes.iter()
        .filter(|n| n.kind == "file")
        .map(|n| (n.file.clone(), n.id.clone()))
        .collect();

    let fn_by_name: std::collections::HashMap<String, String> = nodes.iter()
        .filter(|n| ["function", "method", "component", "hook"].contains(&n.kind.as_str()))
        .map(|n| (n.name.clone(), n.id.clone()))
        .collect();

    let type_by_name: std::collections::HashMap<String, String> = nodes.iter()
        .filter(|n| ["class", "struct", "interface", "enum", "type"].contains(&n.kind.as_str()))
        .map(|n| (n.name.clone(), n.id.clone()))
        .collect();

    let _repo_path_str = {
        let store = ctx.store().await;
        store.get_repo(repo_id).ok().flatten()
            .map(|p| p.path.clone())
            .unwrap_or_default()
    };

    let mut edges_created = 0u32;

    for (source_id, ref_kind, ref_target) in &refs {
        match ref_kind.as_str() {
            "imports" => {
                // Resolve relative import: ./bar → find file:abs_path/bar.ts
                if !ref_target.starts_with('.') { continue; }
                let source_file = nodes.iter()
                    .find(|n| n.id == *source_id)
                    .map(|n| n.file.clone())
                    .unwrap_or_default();
                let source_dir = Path::new(&source_file).parent().unwrap_or(Path::new(""));
                let resolved = source_dir.join(ref_target);
                let resolved_str = resolved.to_string_lossy().to_string();

                for suffix in &["", ".ts", ".tsx", ".js", ".jsx", ".py", ".rs"] {
                    let candidate = format!("file:{}{}", resolved_str, suffix);
                    if file_by_path.values().any(|id| *id == candidate) {
                        graph.merge_edge(source_id, &candidate, "IMPORTS").ok();
                        edges_created += 1;
                        break;
                    }
                }
            }
            "calls" => {
                // Resolve function call by name
                if let Some(callee_id) = fn_by_name.get(ref_target) {
                    // Find the actual caller fn ID (source_id is a prefix like fn:path:name:)
                    let caller = fn_by_name.iter()
                        .find(|(_, id)| source_id.starts_with(id.as_str()) || id.starts_with(source_id.as_str()))
                        .or_else(|| fn_by_name.iter().find(|(name, _)| source_id.contains(name.as_str())));
                    if let Some((_, caller_id)) = caller
                        && caller_id != callee_id {
                            graph.merge_edge(caller_id, callee_id, "CALLS").ok();
                            edges_created += 1;
                        }
                }
            }
            "parent" => {
                // Resolve HAS_METHOD: find type by name
                if let Some(type_id) = type_by_name.get(ref_target) {
                    graph.merge_edge(type_id, source_id, "HAS_METHOD").ok();
                    edges_created += 1;
                }
            }
            "covers" => {
                // Resolve doc COVERS file
                let file_id = format!("file:{}", ref_target);
                if file_by_path.values().any(|id| *id == file_id) {
                    graph.merge_edge(source_id, &file_id, "COVERS").ok();
                    edges_created += 1;
                }
            }
            "mentions_fn" => {
                // Resolve doc MENTIONS_FN
                if let Some(fn_id) = fn_by_name.get(ref_target) {
                    graph.merge_edge(source_id, fn_id, "MENTIONS_FN").ok();
                    edges_created += 1;
                }
            }
            _ => {}
        }
    }

    // Framework tagging
    // TODO: migrate framework_tagger to work with hierarchy_nodes

    graph.clear_unresolved_refs(repo_id).ok();
    tracing::info!("resolve_edges: {} — {} refs processed, {} edges created", repo_id, refs.len(), edges_created);
    Ok(())
}

// ── Build Connections ─────────────────────────────────────────────────────

/// Build doc<>code traceability and cross-repo links.
pub async fn build_connections(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let graph = ctx.graph().await;

    // Doc<>code traceability (SPECIFIES, IMPLEMENTS, DOCUMENTS)
    crate::indexer::doc_indexer::create_traceability_edges_pub(&graph, repo_id)?;

    // Mark project as indexed
    let store = ctx.store().await;
    // Collect libs from unresolved import targets
    let nodes = graph.get_nodes(repo_id)?;
    let libs = std::collections::HashSet::new();
    // Simple lib detection from file-level imports
    for node in &nodes {
        if node.kind == "file" {
            // Libs are detected from imports during resolve, but we can also check here
        }
    }
    let lib_vec: Vec<String> = libs.into_iter().collect();
    store.mark_indexed(repo_id, &lib_vec).ok();

    tracing::info!("build_connections: {} complete", repo_id);
    Ok(())
}

// ── Reconcile Connections ──────────────────────────────────────────────────

/// Re-evaluate cross-repo edges after a branch switch or repo update.
pub async fn reconcile_connections(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let store = ctx.store().await;
    let graph = ctx.graph().await;

    // Find the project this repo belongs to
    let repo_data = store.get_repo(repo_id).ok().flatten();
    let my_projects: Vec<Project> = if let Some(pid) = repo_data.as_ref().and_then(|r| r.project_id.as_ref()) {
        store.list_projects().unwrap_or_default().into_iter().filter(|p| p.id == *pid).collect()
    } else {
        vec![]
    };

    if my_projects.is_empty() {
        tracing::info!("reconcile_connections: {} not in any project", repo_id);
        return Ok(());
    }

    for proj in &my_projects {
        match crate::indexer::cross_repo::analyze_project(&store, &graph, proj) {
            Ok(analysis) => {
                tracing::info!(
                    "reconcile_connections: project {} — {} links, {} shared libs",
                    proj.id, analysis.links.len(), analysis.shared_libs.len()
                );
            }
            Err(e) => tracing::warn!("reconcile failed for {}: {}", proj.id, e),
        }
    }

    // Rebuild doc<>code traceability
    crate::indexer::doc_indexer::create_traceability_edges_pub(&graph, repo_id)?;

    tracing::info!("reconcile_connections: {} — {} projects", repo_id, my_projects.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use crate::db::Store;
    use crate::indexer::graph::GraphDb;
    use crate::tasks::queue::TaskQueue;
    use crate::tasks::{Task, TaskKind};
    use crate::api::state::SharedState;
    use super::super::super::executor::TaskContext;

    /// Build a TaskContext backed by in-memory Store + GraphDb and a fresh TaskQueue.
    fn make_ctx() -> Arc<TaskContext> {
        let store = Store::open_memory().unwrap();
        let graph = GraphDb::open_memory().unwrap();
        let queue = Arc::new(TaskQueue::new());
        let app_state = Arc::new(SharedState {
            store: Mutex::new(store),
            graph: Mutex::new(graph),
            task_queue: queue.clone(),
        });
        Arc::new(TaskContext {
            queue,
            app_state,
            _graph_path: None,
        })
    }

    #[tokio::test]
    async fn resolve_edges_resolves_calls_and_parent_refs() {
        let ctx = make_ctx();
        let repo_id = "test-repo";

        // Seed graph with functions and types
        {
            let graph = ctx.graph().await;
            graph.merge_function("fn:a:caller:1", "caller", "a.rs", 1, "", "", "", 1, repo_id).unwrap();
            graph.merge_function("fn:a:callee:10", "callee", "a.rs", 10, "", "", "", 1, repo_id).unwrap();
            graph.merge_type("type:a:MyStruct:1", "MyStruct", "a.rs", 1, "struct", repo_id).unwrap();
            graph.merge_function("fn:a:my_method:20", "my_method", "a.rs", 20, "", "", "", 1, repo_id).unwrap();

            // Add unresolved refs: a call and a parent (HAS_METHOD)
            graph.add_unresolved_ref("fn:a:caller:1", "calls", "callee", repo_id).unwrap();
            graph.add_unresolved_ref("fn:a:my_method:20", "parent", "MyStruct", repo_id).unwrap();
        }

        // Register project (resolve_edges reads project path)
        {
            let store = ctx.store().await;
            store.upsert_repo_basic(repo_id, repo_id, "/tmp/repo").unwrap();
        }

        let task = Task::new(TaskKind::ResolveEdges, repo_id, "");
        resolve_edges(&ctx, &task).await.unwrap();

        let graph = ctx.graph().await;
        let edges = graph.get_edges(repo_id).unwrap();
        let edge_types: Vec<&str> = edges.iter().map(|e| e.edge_type.as_str()).collect();
        assert!(edge_types.contains(&"CALLS"), "expected a CALLS edge, got: {:?}", edge_types);
        assert!(edge_types.contains(&"HAS_METHOD"), "expected a HAS_METHOD edge, got: {:?}", edge_types);

        // Unresolved refs should be cleared
        let refs = graph.get_unresolved_refs(repo_id).unwrap();
        assert!(refs.is_empty(), "unresolved refs should be cleared after resolve_edges");
    }

    #[tokio::test]
    async fn resolve_edges_resolves_covers_refs() {
        let ctx = make_ctx();
        let repo_id = "test-repo";

        {
            let graph = ctx.graph().await;
            graph.merge_file("file:src/main.rs", "src/main.rs", "main", "rust", repo_id).unwrap();
            graph.merge_doc("doc:docs/arch.md", "docs/arch.md", "Architecture", "design", repo_id).unwrap();
            // Unresolved ref: doc covers file
            graph.add_unresolved_ref("doc:docs/arch.md", "covers", "src/main.rs", repo_id).unwrap();
        }

        {
            let store = ctx.store().await;
            store.upsert_repo_basic(repo_id, repo_id, "/tmp/repo").unwrap();
        }

        let task = Task::new(TaskKind::ResolveEdges, repo_id, "");
        resolve_edges(&ctx, &task).await.unwrap();

        let graph = ctx.graph().await;
        let edges = graph.get_edges(repo_id).unwrap();
        let covers: Vec<_> = edges.iter().filter(|e| e.edge_type == "COVERS").collect();
        assert_eq!(covers.len(), 1);
        assert_eq!(covers[0].source, "doc:docs/arch.md");
        assert_eq!(covers[0].target, "file:src/main.rs");
    }

    #[tokio::test]
    async fn resolve_edges_resolves_mentions_fn_refs() {
        let ctx = make_ctx();
        let repo_id = "test-repo";

        {
            let graph = ctx.graph().await;
            graph.merge_function("fn:a:process:5", "process", "a.rs", 5, "", "", "", 1, repo_id).unwrap();
            graph.merge_doc("doc:docs/api.md", "docs/api.md", "API", "design", repo_id).unwrap();
            graph.add_unresolved_ref("doc:docs/api.md", "mentions_fn", "process", repo_id).unwrap();
        }

        {
            let store = ctx.store().await;
            store.upsert_repo_basic(repo_id, repo_id, "/tmp/repo").unwrap();
        }

        let task = Task::new(TaskKind::ResolveEdges, repo_id, "");
        resolve_edges(&ctx, &task).await.unwrap();

        let graph = ctx.graph().await;
        let edges = graph.get_edges(repo_id).unwrap();
        let mentions: Vec<_> = edges.iter().filter(|e| e.edge_type == "MENTIONS_FN").collect();
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].target, "fn:a:process:5");
    }

    #[tokio::test]
    async fn resolve_edges_skips_non_relative_imports() {
        let ctx = make_ctx();
        let repo_id = "test-repo";

        {
            let graph = ctx.graph().await;
            graph.merge_file("file:src/main.rs", "src/main.rs", "main", "rust", repo_id).unwrap();
            // Non-relative import: should be skipped
            graph.add_unresolved_ref("file:src/main.rs", "imports", "react", repo_id).unwrap();
        }

        {
            let store = ctx.store().await;
            store.upsert_repo_basic(repo_id, repo_id, "/tmp/repo").unwrap();
        }

        let task = Task::new(TaskKind::ResolveEdges, repo_id, "");
        resolve_edges(&ctx, &task).await.unwrap();

        let graph = ctx.graph().await;
        let edges = graph.get_edges(repo_id).unwrap();
        let imports: Vec<_> = edges.iter().filter(|e| e.edge_type == "IMPORTS").collect();
        assert!(imports.is_empty(), "non-relative imports should not produce edges");
    }

    #[tokio::test]
    async fn resolve_edges_with_no_refs_is_noop() {
        let ctx = make_ctx();
        let repo_id = "test-repo";

        {
            let graph = ctx.graph().await;
            graph.merge_file("file:a.rs", "a.rs", "a", "rust", repo_id).unwrap();
        }
        {
            let store = ctx.store().await;
            store.upsert_repo_basic(repo_id, repo_id, "/tmp/repo").unwrap();
        }

        let task = Task::new(TaskKind::ResolveEdges, repo_id, "");
        resolve_edges(&ctx, &task).await.unwrap();

        let graph = ctx.graph().await;
        let edges = graph.get_edges(repo_id).unwrap();
        assert!(edges.is_empty());
    }
}
