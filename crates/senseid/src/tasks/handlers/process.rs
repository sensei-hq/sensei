//! Process phase: index repos, folders, and files; handle deletions.

use std::path::Path;
use super::super::executor::TaskContext;
use super::super::{Task, TaskKind};
use crate::types::{NodeKind, HierarchyNode};
use super::helpers::{is_binary_ext, build_globset};

// ── Process Repo ──────────────────────────────────────────────────────────

/// Process a repo: create virtual nodes, detect workspaces, enqueue folder + barrier tasks.
pub async fn process_repo(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_path = Path::new(&task.path);
    if !repo_path.exists() {
        return Err(format!("Repo path does not exist: {}", task.path));
    }

    let repo_id = &task.repo_id;
    let graph = ctx.graph().await;

    // Clear stale hierarchy
    graph.clear_hierarchy(repo_id).ok();
    graph.clear_unresolved_refs(repo_id).ok();

    // Create repo node — packages and docs wire directly to it (no code/docs grouping layer)
    let repo_node_id = format!("repo:{}", repo_id);
    graph.merge_node(&HierarchyNode::group(repo_node_id.clone(), repo_id.to_string(), NodeKind::Repo, repo_id.to_string())).ok();

    // Detect workspace members → package nodes (parent = repo, tagged as code)
    let workspace_members = crate::config::detector::detect_workspace_members(repo_path);
    for pkg in &workspace_members {
        let pkg_id = format!("pkg:{}:{}", repo_id, pkg.name);
        let mut node = HierarchyNode::group(pkg_id.clone(), pkg.name.clone(), NodeKind::Package, repo_id.to_string());
        node.level = Some(pkg.pkg_type.clone());
        node.file = Some(pkg.path.clone());
        node.parent_id = Some(repo_node_id.clone());
        node.tags = Some("src".into());
        graph.merge_node(&node).ok();
        graph.merge_edge(&repo_node_id, &pkg_id, "CONTAINS_PKG").ok();
    }

    // Virtual root package for files not under any workspace member
    let root_pkg_id = format!("pkg:{}:(root)", repo_id);
    let mut root_pkg = HierarchyNode::group(root_pkg_id.clone(), "(root)".into(), NodeKind::Package, repo_id.to_string());
    root_pkg.level = Some("root".into());
    root_pkg.parent_id = Some(repo_node_id.clone());
    root_pkg.tags = Some("src".into());
    graph.merge_node(&root_pkg).ok();
    graph.merge_edge(&repo_node_id, &root_pkg_id, "CONTAINS_PKG").ok();

    drop(graph); // release lock before enqueuing

    // Discover directories and enqueue folder tasks
    let exclude = build_globset();
    let mut dirs = std::collections::HashSet::new();

    // Walk all files to discover directories
    let walker = ignore::WalkBuilder::new(repo_path)
        .hidden(true).git_ignore(true).git_global(true).git_exclude(true)
        .build();

    for entry in walker.flatten() {
        if !entry.path().is_file() { continue; }
        let rel = entry.path().strip_prefix(repo_path).unwrap_or(entry.path());
        let rel_str = rel.to_string_lossy();
        if exclude.is_match(&*rel_str) { continue; }

        // Skip binary files and files without extensions
        let ext = entry.path().extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();
        if ext.is_empty() { continue; }
        if is_binary_ext(&ext) { continue; }

        if let Some(parent) = entry.path().parent() {
            dirs.insert(parent.to_path_buf());
        }
    }

    // Enqueue folder + file tasks first, collecting all file task IDs
    let mut all_file_task_ids: Vec<u64> = Vec::new();
    for dir in &dirs {
        let rel_dir = dir.strip_prefix(repo_path).unwrap_or(dir)
            .to_string_lossy().to_string();
        let abs_dir = dir.to_string_lossy().to_string();

        // Determine parent package
        let pkg_id = workspace_members.iter()
            .find(|pkg| rel_dir.starts_with(&pkg.path))
            .map(|pkg| format!("pkg:{}:{}", repo_id, pkg.name))
            .unwrap_or_else(|| root_pkg_id.clone());

        let folder_task = Task::new(TaskKind::ProcessFolder, repo_id, &abs_dir)
            .with_parent(task.id);
        // Store pkg_id in module_id field for folder processing
        let mut ft = folder_task;
        ft.module_id = Some(pkg_id);
        let folder_id = ctx.queue.enqueue(ft).await;

        // Enumerate files in this directory (non-recursive)
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if !entry.path().is_file() { continue; }
                let ext = entry.path().extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_string();
                if ext.is_empty() { continue; }
                if is_binary_ext(&ext) { continue; }

                let rel_dir_name = if rel_dir.is_empty() { "(root)".to_string() } else { rel_dir.replace('\\', "/") };
                let mod_id = format!("mod:{}:{}", repo_id, rel_dir_name);

                let file_task = Task::new(TaskKind::ProcessFile, repo_id, &entry.path().to_string_lossy())
                    .with_parent(folder_id)
                    .with_module(&mod_id);
                let file_id = ctx.queue.enqueue(file_task).await;
                all_file_task_ids.push(file_id);

                // file_id collected in all_file_task_ids for barrier
            }
        }
    }

    // Emit RepoQueued event with file count so UI can show accurate progress
    let _ = ctx.queue.sender().send(
        crate::tasks::progress::TaskEvent::RepoQueued {
            repo_id: repo_id.to_string(),
            files_total: all_file_task_ids.len() as u32,
        }
    );

    // Remove the sentinel dependency from resolve_edges now that real deps are wired
    // (the sentinel u64::MAX will never complete, so we need to remove it)
    // Simplest: if we have real deps, the sentinel is harmless — it just means
    // resolve won't run until all file tasks AND the sentinel complete.
    // Now create barrier tasks with REAL dependencies (no sentinel)
    let resolve_id = ctx.queue.enqueue(
        Task::new(TaskKind::ResolveEdges, repo_id, "")
            .with_parent(task.id)
            .blocked_by(all_file_task_ids.clone())
    ).await;

    let libs_id = ctx.queue.enqueue(
        Task::new(TaskKind::ResolveLibs, repo_id, "")
            .with_parent(task.id)
            .blocked_by(vec![resolve_id])
    ).await;

    ctx.queue.enqueue(
        Task::new(TaskKind::BuildConnections, repo_id, "")
            .with_parent(task.id)
            .blocked_by(vec![libs_id])
    ).await;

    // Detect subtrees → register as separate repos, create project
    {
        let store = ctx.store().await;
        if let Ok(Some(repo)) = store.get_repo(repo_id) {
            // Detect git subtrees
            let subtrees = crate::indexer::cross_repo::detect_git_subtrees_pub(repo_path);
            if !subtrees.is_empty() {
                // Create project for monorepo
                crate::indexer::cross_repo::auto_project_for_monorepo(&store, &repo).ok();

                // Register and index each subtree as a separate repo
                for (name, subtree_path) in &subtrees {
                    let subtree_repo_id = format!("{}:{}", repo_id, name);
                    store.upsert_repo_basic(&subtree_repo_id, name, subtree_path).ok();
                }
                drop(store); // release lock before enqueuing

                for (name, subtree_path) in &subtrees {
                    let subtree_repo_id = format!("{}:{}", repo_id, name);
                    let sub_task = Task::new(TaskKind::ProcessRepo, &subtree_repo_id, subtree_path)
                        .with_parent(task.id);
                    ctx.queue.enqueue(sub_task).await;
                    tracing::info!("process_repo: enqueued subtree {} at {}", subtree_repo_id, subtree_path);
                }
            }
        }
    }

    // ── Repo-level metadata scanners (fast, filesystem-only) ──────────
    {
        use crate::tasks::processors::metadata;

        let icon = metadata::scan_icons(repo_path);
        let links = metadata::scan_external_links(repo_path);
        let summary = metadata::extract_summary(repo_path);

        // Persist metadata on the project record
        let store = ctx.store().await;
        let meta = serde_json::json!({
            "icon": icon,
            "external_links": links.links,
            "summary": summary,
        });
        store.set_repo_metadata(repo_id, &meta).ok();
    }

    tracing::info!("process_repo: {} — {} dirs, {} file tasks, barrier=#{}", repo_id, dirs.len(), all_file_task_ids.len(), resolve_id);
    Ok(())
}

// ── Process Folder ────────────────────────────────────────────────────────

/// Create module node for a folder.
pub async fn process_folder(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let repo_path_str = {
        let store = ctx.store().await;
        store.get_repo(repo_id).ok().flatten()
            .map(|p| p.path.clone())
            .unwrap_or_default()
    };
    let repo_path = Path::new(&repo_path_str);

    let rel_dir = Path::new(&task.path).strip_prefix(repo_path)
        .unwrap_or(Path::new(&task.path))
        .to_string_lossy().to_string();
    let mod_name = if rel_dir.is_empty() { "(root)".to_string() } else { rel_dir.replace('\\', "/") };
    let mod_id = format!("mod:{}:{}", repo_id, mod_name);

    let pkg_id = task.module_id.clone(); // parent package ID stored here by process_repo

    let graph = ctx.graph().await;
    let mut mod_node = HierarchyNode::group(mod_id.clone(), mod_name, NodeKind::Module, repo_id.to_string());
    mod_node.file = Some(rel_dir);
    mod_node.parent_id = pkg_id.clone();
    graph.merge_node(&mod_node).ok();

    if let Some(ref pid) = pkg_id {
        graph.merge_edge(pid, &mod_id, "CONTAINS_MOD").ok();
    }

    Ok(())
}

// ── Process File ──────────────────────────────────────────────────────────

/// Parse a single file using file_processor, then write results to graph.
pub async fn process_file(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let abs_path = &task.path;

    let repo_path_str = {
        let store = ctx.store().await;
        store.get_repo(repo_id).ok().flatten()
            .map(|p| p.path.clone())
            .unwrap_or_default()
    };

    // Use the testable file processor — no DB dependency in extraction
    let result = crate::tasks::processors::process_file(abs_path, &repo_path_str, repo_id)?;

    // Write to graph via the separated graph writer
    let graph = ctx.graph().await;
    crate::tasks::processors::write_to_graph(&graph, &result, repo_id, task.module_id.as_deref())?;

    Ok(())
}

// ── Delete File / Folder ──────────────────────────────────────────────────

pub async fn delete_file(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let graph = ctx.graph().await;
    graph.delete_by_file(&task.path, &task.repo_id)?;
    graph.clear_unresolved_refs_from(&format!("file:{}", task.path), &task.repo_id).ok();
    // file: prefix used for both code and doc leaf files
    tracing::info!("delete_file: {}", task.path);
    Ok(())
}

pub async fn delete_folder(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let graph = ctx.graph().await;
    let nodes = graph.get_nodes(&task.repo_id)?;
    for node in &nodes {
        if node.file.starts_with(&task.path) {
            graph.delete_node(&node.id)?;
        }
    }
    // Delete the module node
    let repo_path_str = {
        let store = ctx.store().await;
        store.get_repo(&task.repo_id).ok().flatten()
            .map(|p| p.path.clone())
            .unwrap_or_default()
    };
    let rel_dir = Path::new(&task.path).strip_prefix(Path::new(&repo_path_str))
        .unwrap_or(Path::new(&task.path))
        .to_string_lossy().replace('\\', "/");
    let mod_id = format!("mod:{}:{}", task.repo_id, if rel_dir.is_empty() { "(root)" } else { &rel_dir });
    graph.delete_node(&mod_id)?;
    tracing::info!("delete_folder: {} ({} nodes checked)", task.path, nodes.len());
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
    async fn process_repo_errors_on_nonexistent_path() {
        let ctx = make_ctx();
        let task = Task::new(TaskKind::ProcessRepo, "test-repo", "/nonexistent/repo");
        let result = process_repo(&ctx, &task).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn process_folder_creates_module_node() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();

        let ctx = make_ctx();
        let repo_id = "test-repo";
        let repo_path = tmp.path().to_string_lossy().to_string();

        // Register the project so process_folder can look up its path
        {
            let store = ctx.store().await;
            store.upsert_repo_basic(repo_id, repo_id, &repo_path).unwrap();
        }

        let pkg_id = format!("pkg:{}:(root)", repo_id);

        // Create the parent package node so get_edges can find it
        {
            let graph = ctx.graph().await;
            let pkg_node = HierarchyNode::group(
                pkg_id.clone(), "(root)".to_string(),
                NodeKind::Package, repo_id.to_string(),
            );
            graph.merge_node(&pkg_node).unwrap();
        }

        let mut task = Task::new(TaskKind::ProcessFolder, repo_id, &src_dir.to_string_lossy());
        task.module_id = Some(pkg_id.clone());

        process_folder(&ctx, &task).await.unwrap();

        // Verify module node was created (package + module = 2 nodes)
        let graph = ctx.graph().await;
        let nodes = graph.get_nodes(repo_id).unwrap();
        assert_eq!(nodes.len(), 2);
        let module_node = nodes.iter().find(|n| n.kind == "module").expect("module node");
        assert_eq!(module_node.name, "src");

        // Verify edge from package to module
        let edges = graph.get_edges(repo_id).unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].edge_type, "CONTAINS_MOD");
        assert_eq!(edges[0].source, pkg_id);
    }

    #[tokio::test]
    async fn delete_file_removes_nodes_and_edges() {
        let ctx = make_ctx();
        let repo_id = "test-repo";

        // Seed graph with two file nodes and an edge
        {
            let graph = ctx.graph().await;
            graph.merge_file("file:/tmp/a.rs", "/tmp/a.rs", "mod_a", "rust", repo_id).unwrap();
            graph.merge_function("fn:/tmp/a.rs:foo:1", "foo", "/tmp/a.rs", 1, "", "", "", 1, repo_id).unwrap();
            graph.merge_file("file:/tmp/b.rs", "/tmp/b.rs", "mod_b", "rust", repo_id).unwrap();
            graph.merge_edge("file:/tmp/a.rs", "file:/tmp/b.rs", "IMPORTS").unwrap();
        }

        let task = Task::new(TaskKind::DeleteFile, repo_id, "/tmp/a.rs");
        delete_file(&ctx, &task).await.unwrap();

        let graph = ctx.graph().await;
        let nodes = graph.get_nodes(repo_id).unwrap();
        // Only file b should remain; a.rs and its function were deleted
        let remaining_files: Vec<&str> = nodes.iter()
            .filter(|n| n.kind == "file")
            .map(|n| n.file.as_str())
            .collect();
        assert_eq!(remaining_files, vec!["/tmp/b.rs"]);
    }

    #[tokio::test]
    async fn delete_folder_removes_module_and_child_nodes() {
        let ctx = make_ctx();
        let repo_id = "test-repo";
        let repo_path = "/tmp/myrepo";

        // Register project
        {
            let store = ctx.store().await;
            store.upsert_repo_basic(repo_id, repo_id, repo_path).unwrap();
        }

        // Seed graph: module node + file node under /tmp/myrepo/src/
        {
            let graph = ctx.graph().await;
            graph.merge_module("mod:test-repo:src", "src", "src", None, repo_id).unwrap();
            graph.merge_file("file:src/main.rs", "/tmp/myrepo/src/main.rs", "main", "rust", repo_id).unwrap();
            graph.merge_function("fn:src/main.rs:main:1", "main", "/tmp/myrepo/src/main.rs", 1, "", "", "", 1, repo_id).unwrap();
        }

        let task = Task::new(TaskKind::DeleteFolder, repo_id, "/tmp/myrepo/src");
        delete_folder(&ctx, &task).await.unwrap();

        let graph = ctx.graph().await;
        let nodes = graph.get_nodes(repo_id).unwrap();
        // All nodes under /tmp/myrepo/src should be deleted, plus the module node
        assert!(nodes.is_empty(), "all nodes under deleted folder should be removed, got: {:?}",
            nodes.iter().map(|n| &n.id).collect::<Vec<_>>());
    }
}
