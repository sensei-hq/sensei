//! Process phase: index repos, folders, and files; handle deletions.

use std::path::Path;
use super::super::executor::TaskContext;
use super::super::{Task, TaskKind};
use super::helpers::{is_binary_ext, build_globset};

// ── Process Repo ──────────────────────────────────────────────────────────

/// Process a git folder: detect stack, count files, create/find project, emit events, enqueue file tasks.
pub async fn process_git_folder(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let repo_path = Path::new(&task.path);
    if !repo_path.exists() {
        return Err(format!("Repo path does not exist: {}", task.path));
    }

    let folder_path = &task.folder_path;
    let emit = |evt: crate::api::events::StateEvent| { let _ = ctx.app_state.event_tx.send(evt); };

    // Display name comes from the DB row (looked up by abs_path) so subtree
    // labels like "sensei:homebrew" survive — task.folder_name() would
    // return the basename only.
    let pre_registered = ctx.pg().get_repo_by_path(&task.path).await.ok().flatten();
    let folder_name_owned: String = pre_registered
        .as_ref()
        .and_then(|r| r["name"].as_str().map(String::from))
        .unwrap_or_else(|| task.folder_name().to_string());
    let folder_name: &str = &folder_name_owned;

    // ── 1. Detect stack ──────────────────────────────────────────────
    let stack = super::scan_logic::detect_stack(repo_path);

    // ── 2. Count indexable files ─────────────────────────────────────
    let (_indexable_files, files_total) = super::scan_logic::count_indexable_files(repo_path);

    // ── 3. Find or create project ────────────────────────────────────
    let parent_name = repo_path.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or(folder_name);

    // Check if a project with this parent name exists
    let project_id = match ctx.pg().get_project_by_name(parent_name).await {
        Ok(Some(proj)) => {
            // Project exists — use it
            proj["id"].as_str().unwrap_or("").to_string()
        }
        _ => {
            // Create new project
            let id = ctx.pg().create_project(parent_name, None, None).await
                .map(|id| id.to_string())
                .unwrap_or_else(|_| format!("p-{}", parent_name));

            // Emit: project add
            emit(crate::api::events::StateEvent::project_add(crate::api::events::ScanProject {
                id: id.clone(),
                name: parent_name.to_string(),
                status: crate::api::events::ProjectStatus::Indexing,
                folders: vec![],
                auto_detected: true,
                confidence: crate::api::events::Confidence::High,
            }));

            id
        }
    };

    // ── 4. Emit: folder add with stack + file count ──────────────────
    // Reuse the lookup we already did to derive folder_name. abs_path is
    // unique on sensei.folders so the row identifies this exact repo
    // (vs name which can collide across roots).
    let folder_by_path = pre_registered;
    let folder_uuid_str = folder_by_path.as_ref()
        .and_then(|f| f["id"].as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| format!("f-{}", folder_name));

    emit(crate::api::events::StateEvent::folder_add(crate::api::events::ScanFolder {
        id: folder_uuid_str.clone(),
        project_id: project_id.clone(),
        name: folder_name.to_string(),
        path: task.path.clone(),
        kind: crate::api::events::FolderKind::Git,
        stack: stack.clone(),
        files_total,
        files_completed: 0,
        status: crate::api::events::FolderStatus::Queued,
    }));

    // ── 5. Emit: activity queue ──────────────────────────────────────
    emit(crate::api::events::StateEvent::activity(crate::api::events::ActivityEvent::new(
        crate::api::events::ActivityLevel::Queue,
        &format!("{} · {} files queued · {}", folder_name, files_total, stack.join(", ")),
        0.0,
    )));

    // ── Existing logic: look up folder by path, clear stale data ─────
    // Use the path-based row (same as folder_by_path above) to avoid
    // name collisions with identically-named repos from prior runs.
    let folder = folder_by_path;
    let folder_uuid = folder.as_ref()
        .and_then(|f| crate::api::util::json_uuid(&f["id"]));

    // ── Persist project_id on the folder record ──────────────────────
    // upsert_repo does not set project_id; do it now so that
    // progress_emitter::build_tracker can read it via get_repo_by_path.
    if let (Some(fid), Ok(pid)) = (&folder_uuid, uuid::Uuid::parse_str(&project_id)) {
        ctx.pg().set_folder_project(fid, &pid, "primary", None).await.ok();
    }

    // ── 6. Attach deferred siblings to this project ──────────────────
    if let Ok(project_id) = uuid::Uuid::parse_str(&project_id) {
        let parent = std::path::Path::new(&task.folder_path)
            .parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        if !parent.is_empty() {
            let assigned = ctx.pg().assign_deferred_siblings_to_project(&parent, &project_id).await.unwrap_or(0);
            if assigned > 0 {
                tracing::info!("process_git_folder: {} — assigned {} deferred siblings to project {}", folder_name, assigned, project_id);
            }
        }
    }

    if let Some(ref fid) = folder_uuid {
        ctx.pg().delete_nodes_by_folder(fid).await.ok();
    }

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
    let root_pkg_id = format!("pkg:{}:(root)", folder_name);
    for dir in &dirs {
        let rel_dir = dir.strip_prefix(repo_path).unwrap_or(dir)
            .to_string_lossy().to_string();
        let abs_dir = dir.to_string_lossy().to_string();

        // Determine parent package
        let pkg_id = workspace_members.iter()
            .find(|pkg| rel_dir.starts_with(&pkg.path))
            .map(|pkg| format!("pkg:{}:{}", folder_name, pkg.name))
            .unwrap_or_else(|| root_pkg_id.clone());

        let folder_task = Task::new(TaskKind::ProcessFolder, folder_path, &abs_dir)
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
                let mod_id = format!("mod:{}:{}", folder_name, rel_dir_name);

                let file_task = Task::new(TaskKind::ProcessFile, folder_path, &entry.path().to_string_lossy())
                    .with_parent(folder_id)
                    .with_module(&mod_id);
                let file_id = ctx.queue.enqueue(file_task).await;
                all_file_task_ids.push(file_id);

                // file_id collected in all_file_task_ids for barrier
            }
        }
    }

    // Emit FolderQueued event with file count so UI can show accurate progress
    let _ = ctx.queue.sender().send(
        crate::tasks::progress::TaskEvent::FolderQueued {
            folder_path: task.folder_path.clone(),
            files_total: all_file_task_ids.len() as u32,
        }
    );

    // Remove the sentinel dependency from resolve_edges now that real deps are wired
    // (the sentinel u64::MAX will never complete, so we need to remove it)
    // Simplest: if we have real deps, the sentinel is harmless — it just means
    // resolve won't run until all file tasks AND the sentinel complete.
    // Now create barrier tasks with REAL dependencies (no sentinel)
    let resolve_id = ctx.queue.enqueue(
        Task::new(TaskKind::ResolveEdges, folder_path, "")
            .with_parent(task.id)
            .blocked_by(all_file_task_ids.clone())
    ).await;

    let libs_id = ctx.queue.enqueue(
        Task::new(TaskKind::ResolveLibs, folder_path, "")
            .with_parent(task.id)
            .blocked_by(vec![resolve_id])
    ).await;

    ctx.queue.enqueue(
        Task::new(TaskKind::BuildConnections, folder_path, "")
            .with_parent(task.id)
            .blocked_by(vec![libs_id])
    ).await;

    // Detect subtrees → register as separate repos
    {
        let folder = ctx.pg().get_repo_by_path(&task.path).await.ok().flatten();
        if folder.is_some() {
            // Detect git subtrees
            let subtrees = crate::indexer::cross_repo::detect_git_subtrees_pub(repo_path);
            if !subtrees.is_empty() {
                // Register each subtree as a separate repo via PgStore
                // Look up the root_id for upsert_repo
                let root_id = folder.as_ref()
                    .and_then(|f| crate::api::util::json_uuid(&f["root_id"]));

                if let Some(root_id) = root_id {
                    for (name, subtree_path) in &subtrees {
                        let subtree_folder_name = format!("{}:{}", folder_name, name);
                        ctx.pg().upsert_repo(&root_id, &subtree_folder_name, subtree_path).await.ok();
                    }
                }

                for (name, subtree_path) in &subtrees {
                    // folder_path is an abs_path per the Task struct contract;
                    // the composite display name "{folder_name}:{name}" lives
                    // on sensei.folders.name (upserted above) and is read
                    // back by handlers via get_repo_by_path.
                    let sub_task = Task::new(TaskKind::ProcessGitFolder, subtree_path, subtree_path)
                        .with_parent(task.id);
                    ctx.queue.enqueue(sub_task).await;
                    let subtree_folder_name = format!("{}:{}", folder_name, name);
                    tracing::info!("process_git_folder: enqueued subtree {} at {}", subtree_folder_name, subtree_path);
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
        let folder = ctx.pg().get_repo_by_path(&task.path).await.ok().flatten();
        if let Some(folder) = folder
            && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
                let meta = serde_json::json!({
                    "icon": icon,
                    "external_links": links.links,
                    "summary": summary,
                });
                ctx.pg().set_folder_props(&folder_id, &meta).await.ok();
            }
    }

    tracing::info!("process_git_folder: {} — {} dirs, {} file tasks, barrier=#{}", folder_name, dirs.len(), all_file_task_ids.len(), resolve_id);
    Ok(all_file_task_ids.len() as u32)
}

// ── Process Folder ────────────────────────────────────────────────────────

/// Create module node for a folder.
pub async fn process_folder(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    // folder_path is the repo's abs_path by contract — look the row up
    // directly instead of round-tripping through name (which can collide
    // across roots and breaks for subtrees whose DB name is a composite
    // like "sensei:homebrew").
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await.ok().flatten();
    let folder_id = folder.as_ref()
        .and_then(|f| crate::api::util::json_uuid(&f["id"]));

    let rel_dir = Path::new(&task.path).strip_prefix(Path::new(&task.folder_path))
        .unwrap_or(Path::new(&task.path))
        .to_string_lossy().to_string();

    // Write module node to PG
    if let Some(ref fid) = folder_id {
        let mod_name = if rel_dir.is_empty() { "(root)".to_string() } else { rel_dir.replace('\\', "/") };
        ctx.pg().upsert_node(fid, "module", &mod_name, &task.path, None, None, None, None).await.ok();
    }

    Ok(0)
}

// ── Process File ──────────────────────────────────────────────────────────

/// Parse a single file using file_processor, then write results to graph.
pub async fn process_file(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let abs_path = &task.path;

    // Lookup once by abs_path; folder name comes from the DB row so subtree
    // composite names ("sensei:homebrew") survive as the repo_id passed to
    // downstream processors that namespace symbol IDs by repo.
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await.ok().flatten();
    let folder_name = folder.as_ref()
        .and_then(|r| r["name"].as_str())
        .unwrap_or_else(|| task.folder_name());

    // Parse the file
    let result = crate::tasks::processors::process_file(abs_path, &task.folder_path, folder_name)?;

    // Write parsed symbols to PG
    let symbols_count = result.symbols.len();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
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
            if result.kind == "doc"
                && let Some(ref fid) = file_node_id {
                    for file_ref in &result.file_refs {
                        ctx.pg().insert_edge(&folder_id, fid, None, Some(file_ref), "covers").await.ok();
                    }
                    for fn_ref in &result.fn_mentions {
                        ctx.pg().insert_edge(&folder_id, fid, None, Some(fn_ref), "references").await.ok();
                    }
                }
        }

    Ok(symbols_count as u32)
}

// ── Delete File / Folder ──────────────────────────────────────────────────

pub async fn delete_file(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    // folder_path is the repo abs_path (Task contract).
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            ctx.pg().delete_nodes_by_file(&folder_id, &task.path).await.ok();
        }
    tracing::info!("delete_file: {}", task.path);
    Ok(0)
}

pub async fn delete_folder(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await.ok().flatten();
    if let Some(folder) = folder
        && let Some(folder_id) = crate::api::util::json_uuid(&folder["id"]) {
            ctx.pg().delete_nodes_by_path_prefix(&folder_id, &task.path).await.ok();
        }
    tracing::info!("delete_folder: {}", task.path);
    Ok(0)
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
            logger: sensei_logger::Logger::noop(),
        })
    }

    #[tokio::test]
    async fn process_git_folder_errors_on_nonexistent_path() {
        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ProcessGitFolder, "/nonexistent/repo", "/nonexistent/repo");
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
        let folder_name = "test-repo";
        let repo_path = tmp.path().to_string_lossy().to_string();

        // Register the project so process_folder can look up its path
        {
            let root_id = ctx.pg().add_watch_root(&repo_path, "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, folder_name, &repo_path).await.unwrap();
        }

        let pkg_id = format!("pkg:{}:(root)", folder_name);

        let mut task = Task::new(TaskKind::ProcessFolder, &repo_path, &src_dir.to_string_lossy());
        task.module_id = Some(pkg_id.clone());

        process_folder(&ctx, &task).await.unwrap();

        // TODO: verify module node once module writes are implemented
    }

    #[tokio::test]
    async fn delete_file_succeeds() {
        let ctx = make_ctx().await;
        let folder_name = "test-repo";

        // Register a project
        {
            let root_id = ctx.pg().add_watch_root("/tmp/test", "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, folder_name, "/tmp/test").await.unwrap();
        }

        let task = Task::new(TaskKind::DeleteFile, "/tmp/test", "/tmp/a.rs");
        let result = delete_file(&ctx, &task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_folder_removes_module_and_child_nodes() {
        let ctx = make_ctx().await;
        let folder_name = "test-repo";
        let repo_path = "/tmp/myrepo";

        // Register project
        {
            let root_id = ctx.pg().add_watch_root(repo_path, "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, folder_name, repo_path).await.unwrap();
        }

        let task = Task::new(TaskKind::DeleteFolder, repo_path, "/tmp/myrepo/src");
        delete_folder(&ctx, &task).await.unwrap();
    }
}
