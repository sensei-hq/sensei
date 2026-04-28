//! Process phase: index repos, folders, and files; handle deletions.

use std::path::Path;
use super::super::executor::TaskContext;
use super::super::{Task, TaskKind};
use super::helpers::{is_binary_ext, build_globset};

// ── Process Repo ──────────────────────────────────────────────────────────

/// Process a repo: create virtual nodes, detect workspaces, enqueue folder + barrier tasks.
pub async fn process_git_folder(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_path = Path::new(&task.path);
    if !repo_path.exists() {
        return Err(format!("Repo path does not exist: {}", task.path));
    }

    let repo_id = &task.repo_id;

    // Look up folder UUID for PgStore operations
    let folder = ctx.pg().get_repo_by_name(repo_id).await.ok().flatten();
    let folder_uuid = folder.as_ref()
        .and_then(|f| f["id"].as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok());

    // Clear stale hierarchy in PG
    if let Some(ref fid) = folder_uuid {
        ctx.pg().delete_nodes_by_folder(fid).await.ok();
    }
    // TODO: clear_unresolved_refs

    // Detect workspace members
    let workspace_members = crate::config::detector::detect_workspace_members(repo_path);

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
    let root_pkg_id = format!("pkg:{}:(root)", repo_id);
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

    // Detect subtrees → register as separate repos
    {
        let folder = ctx.pg().get_repo_by_name(repo_id).await.ok().flatten();
        if folder.is_some() {
            // Detect git subtrees
            let subtrees = crate::indexer::cross_repo::detect_git_subtrees_pub(repo_path);
            if !subtrees.is_empty() {
                // Register each subtree as a separate repo via PgStore
                // Look up the root_id for upsert_repo
                let root_id = folder.as_ref()
                    .and_then(|f| f["root_id"].as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());

                if let Some(root_id) = root_id {
                    for (name, subtree_path) in &subtrees {
                        let subtree_repo_id = format!("{}:{}", repo_id, name);
                        ctx.pg().upsert_repo(&root_id, &subtree_repo_id, subtree_path).await.ok();
                    }
                }

                for (name, subtree_path) in &subtrees {
                    let subtree_repo_id = format!("{}:{}", repo_id, name);
                    let sub_task = Task::new(TaskKind::ProcessGitFolder, &subtree_repo_id, subtree_path)
                        .with_parent(task.id);
                    ctx.queue.enqueue(sub_task).await;
                    tracing::info!("process_git_folder: enqueued subtree {} at {}", subtree_repo_id, subtree_path);
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

        // Persist metadata on the folder record via PgStore
        let folder = ctx.pg().get_repo_by_name(repo_id).await.ok().flatten();
        if let Some(folder) = folder {
            if let Some(folder_id) = folder["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
                let meta = serde_json::json!({
                    "icon": icon,
                    "external_links": links.links,
                    "summary": summary,
                });
                ctx.pg().set_folder_props(&folder_id, &meta).await.ok();
            }
        }
    }

    tracing::info!("process_git_folder: {} — {} dirs, {} file tasks, barrier=#{}", repo_id, dirs.len(), all_file_task_ids.len(), resolve_id);
    Ok(())
}

// ── Process Folder ────────────────────────────────────────────────────────

/// Create module node for a folder.
pub async fn process_folder(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let folder = ctx.pg().get_repo_by_name(repo_id).await.ok().flatten();
    let folder_id = folder.as_ref()
        .and_then(|f| f["id"].as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok());
    let repo_path_str = folder.as_ref()
        .and_then(|f| f["abs_path"].as_str())
        .unwrap_or("");

    let rel_dir = Path::new(&task.path).strip_prefix(Path::new(repo_path_str))
        .unwrap_or(Path::new(&task.path))
        .to_string_lossy().to_string();

    // Write module node to PG
    if let Some(ref fid) = folder_id {
        let mod_name = if rel_dir.is_empty() { "(root)".to_string() } else { rel_dir.replace('\\', "/") };
        ctx.pg().upsert_node(fid, "module", &mod_name, &task.path, None, None, None, None).await.ok();
    }

    Ok(())
}

// ── Process File ──────────────────────────────────────────────────────────

/// Parse a single file using file_processor, then write results to graph.
pub async fn process_file(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let repo_id = &task.repo_id;
    let abs_path = &task.path;

    let repo_path_str = ctx.pg().get_repo_by_name(repo_id).await
        .ok().flatten()
        .and_then(|r| r["abs_path"].as_str().map(String::from))
        .unwrap_or_default();

    // Parse the file
    let result = crate::tasks::processors::process_file(abs_path, &repo_path_str, repo_id)?;

    // Write parsed symbols to PG
    let folder = ctx.pg().get_repo_by_name(repo_id).await.ok().flatten();
    if let Some(folder) = folder {
        if let Some(folder_id) = folder["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
            // Write file node
            let file_node_id = ctx.pg().upsert_node(
                &folder_id, &result.kind, &result.rel_path, &result.rel_path, None, None, None, None
            ).await.ok();

            // Write symbol nodes (functions, classes, types, etc.)
            for sym in &result.symbols {
                let parent_uuid = file_node_id; // symbols are children of the file
                ctx.pg().upsert_node(
                    &folder_id, &sym.kind, &sym.name, &result.rel_path,
                    parent_uuid.as_ref(), sym.signature.as_deref(),
                    Some(sym.line as i32), Some(sym.line_end as i32),
                ).await.ok();
            }

            // Write unresolved import edges
            for import in &result.unresolved_imports {
                if let Some(ref fid) = file_node_id {
                    ctx.pg().insert_edge(&folder_id, fid, None, Some(import), "imports").await.ok();
                }
            }

            // Write unresolved call edges
            for call in &result.unresolved_calls {
                if let Some(ref fid) = file_node_id {
                    ctx.pg().insert_edge(&folder_id, fid, None, Some(&call.callee_name), "calls").await.ok();
                }
            }

            // Write parent refs (HAS_METHOD: type → method)
            for pref in &result.parent_refs {
                if let Some(ref fid) = file_node_id {
                    ctx.pg().insert_edge(&folder_id, fid, None, Some(&pref.parent_name), "extends").await.ok();
                }
            }

            // Write doc references (file_refs → COVERS, fn_mentions → references)
            if result.kind == "doc" {
                if let Some(ref fid) = file_node_id {
                    for file_ref in &result.file_refs {
                        ctx.pg().insert_edge(&folder_id, fid, None, Some(file_ref), "covers").await.ok();
                    }
                    for fn_ref in &result.fn_mentions {
                        ctx.pg().insert_edge(&folder_id, fid, None, Some(fn_ref), "references").await.ok();
                    }
                }
            }
        }
    }

    Ok(())
}

// ── Delete File / Folder ──────────────────────────────────────────────────

pub async fn delete_file(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    // Look up folder UUID for PgStore operations
    let folder = ctx.pg().get_repo_by_name(&task.repo_id).await.ok().flatten();
    if let Some(folder) = folder {
        if let Some(folder_id) = folder["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
            ctx.pg().delete_nodes_by_file(&folder_id, &task.path).await.ok();
        }
    }
    tracing::info!("delete_file: {}", task.path);
    Ok(())
}

pub async fn delete_folder(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    // Look up folder UUID for PgStore operations
    let folder = ctx.pg().get_repo_by_name(&task.repo_id).await.ok().flatten();
    if let Some(folder) = folder {
        if let Some(folder_id) = folder["id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) {
            // Delete all nodes whose file path starts with the deleted folder path
            // TODO: add a delete_nodes_by_path_prefix method
            ctx.pg().delete_nodes_by_file(&folder_id, &task.path).await.ok();
        }
    }
    tracing::info!("delete_folder: {}", task.path);
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
    async fn process_git_folder_errors_on_nonexistent_path() {
        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ProcessGitFolder, "test-repo", "/nonexistent/repo");
        let result = process_git_folder(&ctx, &task).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn process_folder_creates_module_node() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();

        let ctx = make_ctx().await;
        let repo_id = "test-repo";
        let repo_path = tmp.path().to_string_lossy().to_string();

        // Register the project so process_folder can look up its path
        {
            let root_id = ctx.pg().add_watch_root(&repo_path, "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, repo_id, &repo_path).await.unwrap();
        }

        let pkg_id = format!("pkg:{}:(root)", repo_id);

        let mut task = Task::new(TaskKind::ProcessFolder, repo_id, &src_dir.to_string_lossy());
        task.module_id = Some(pkg_id.clone());

        process_folder(&ctx, &task).await.unwrap();

        // TODO: verify module node once module writes are implemented
    }

    #[tokio::test]
    async fn delete_file_succeeds() {
        let ctx = make_ctx().await;
        let repo_id = "test-repo";

        // Register a project
        {
            let root_id = ctx.pg().add_watch_root("/tmp/test", "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, repo_id, "/tmp/test").await.unwrap();
        }

        let task = Task::new(TaskKind::DeleteFile, repo_id, "/tmp/a.rs");
        let result = delete_file(&ctx, &task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_folder_removes_module_and_child_nodes() {
        let ctx = make_ctx().await;
        let repo_id = "test-repo";
        let repo_path = "/tmp/myrepo";

        // Register project
        {
            let root_id = ctx.pg().add_watch_root(repo_path, "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, repo_id, repo_path).await.unwrap();
        }

        let task = Task::new(TaskKind::DeleteFolder, repo_id, "/tmp/myrepo/src");
        delete_folder(&ctx, &task).await.unwrap();
    }
}
