//! Scan phase: discover folders, classify, enqueue ProcessGitFolder.
//! Only emits activity events. Project + folder events come from ProcessGitFolder.

use std::path::Path;
use std::time::Instant;
use super::super::executor::TaskContext;
use super::super::{Task, TaskKind};
use super::scan_logic::{self, FolderKind};
use crate::api::events::*;

// ── Scan Root ──────────────────────────────────────────────────────────────

pub async fn scan_root(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let root = Path::new(&task.path);
    if !root.exists() {
        return Err(format!("Root path does not exist: {}", task.path));
    }

    let start = Instant::now();
    let emit = |evt: StateEvent| { let _ = ctx.app_state.event_tx.send(evt); };

    // 1. Find all git folders
    let git_folders = scan_logic::find_git_folders(root, 3);

    // Emit discover activity per git folder
    for gf in &git_folders {
        emit(StateEvent::activity(ActivityEvent::new(
            ActivityLevel::Discover,
            &format!("{} · git folder", gf.display()),
            start.elapsed().as_secs_f64(),
        )));
    }

    // 2. Classify all directories
    let all_dirs = scan_logic::all_directories(root, 3);
    let classified = scan_logic::classify_folders(root, &git_folders, &all_dirs);

    // Emit discover activity for non-git folders
    for f in &classified {
        match f.kind {
            FolderKind::Sibling => {
                emit(StateEvent::activity(ActivityEvent::new(
                    ActivityLevel::Discover,
                    &format!("{} · non-git sibling", f.path.display()),
                    start.elapsed().as_secs_f64(),
                )));
            }
            FolderKind::Standalone => {
                emit(StateEvent::activity(ActivityEvent::new(
                    ActivityLevel::Discover,
                    &format!("{} · standalone folder", f.path.display()),
                    start.elapsed().as_secs_f64(),
                )));
            }
            _ => {}
        }
    }

    // 3. Register watch root in DB
    let root_name = root.file_name().and_then(|n| n.to_str()).unwrap_or("root");
    let root_id = ctx.pg().add_watch_root(&task.path, root_name, &serde_json::json!([])).await
        .map_err(|e| format!("Failed to register watch root: {}", e))?;

    // 4. Register non-git folders in DB as deferred (no project yet — ProcessGitFolder will create/find projects)
    for f in &classified {
        if matches!(f.kind, FolderKind::Sibling | FolderKind::Standalone) {
            ctx.pg().upsert_repo(&root_id, &f.name, &f.path.to_string_lossy()).await.ok();
        }
    }

    // 5. Register git folders in DB and enqueue ProcessGitFolder
    for f in &classified {
        if matches!(f.kind, FolderKind::Git) {
            ctx.pg().upsert_repo(&root_id, &f.name, &f.path.to_string_lossy()).await.ok();
            let git_task = Task::new(
                TaskKind::ProcessGitFolder,
                &f.path.to_string_lossy(),
                &f.path.to_string_lossy(),
            ).with_parent(task.id);
            ctx.queue.enqueue(git_task).await;
        }
    }

    // 6. Register watcher
    {
        let watcher = crate::watcher::root_watcher::RootWatcher::instance(ctx.queue.clone());
        if let Ok(mut w) = watcher.lock() {
            w.register(std::path::PathBuf::from(&task.path), vec![]);
        }
    }

    // 7. Summary activity
    let git_count = classified.iter().filter(|f| f.kind == FolderKind::Git).count();
    let sibling_count = classified.iter().filter(|f| f.kind == FolderKind::Sibling).count();
    let standalone_count = classified.iter().filter(|f| f.kind == FolderKind::Standalone).count();

    emit(StateEvent::activity(ActivityEvent::new(
        ActivityLevel::Info,
        &format!("{} git · {} sibling · {} standalone folders discovered",
            git_count, sibling_count, standalone_count),
        start.elapsed().as_secs_f64(),
    )));

    tracing::info!("scan_root: {} git, {} sibling, {} standalone in {}",
        git_count, sibling_count, standalone_count, task.path);
    Ok(())
}

// ── Branch Switch ─────────────────────────────────────────────────────────

pub async fn branch_switch(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    let folder_name = task.folder_name();
    let new_branch = task.branch.as_deref().ok_or("branch_switch requires branch field")?;

    let folder = ctx.pg().get_repo_by_name(folder_name).await.ok().flatten();
    let folder_uuid = folder.as_ref()
        .and_then(|f| f["id"].as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok());

    if let Some(ref fid) = folder_uuid {
        // TODO: ctx.pg().snapshot_graph(fid, "branch_switch").await.ok();
        ctx.pg().delete_nodes_by_folder(fid).await.ok();
    }

    // Re-index on the new branch
    let git_task = Task::new(TaskKind::ProcessGitFolder, &task.folder_path, &task.path)
        .with_parent(task.id)
        .with_branch(new_branch);
    ctx.queue.enqueue(git_task).await;

    tracing::info!("branch_switch: {} → {}", folder_name, new_branch);
    Ok(())
}

fn _detect_current_branch(repo_path: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_path)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::tasks::queue::TaskQueue;
    use crate::tasks::Task;
    use crate::api::state::SharedState;
    use super::super::super::executor::TaskContext;

    async fn make_ctx() -> Arc<TaskContext> {
        let queue = Arc::new(TaskQueue::new());
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let (event_tx, _) = tokio::sync::broadcast::channel(256);
        let app_state = Arc::new(SharedState {
            task_queue: queue.clone(),
            pg: crate::db::pg_store::PgStore::connect_test().await.unwrap(),
            gateway,
            event_tx,
        });
        Arc::new(TaskContext {
            queue,
            app_state,
            _graph_path: None,
        })
    }

    async fn make_ctx_with_events() -> (Arc<TaskContext>, tokio::sync::broadcast::Receiver<StateEvent>) {
        let queue = Arc::new(TaskQueue::new());
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let (event_tx, event_rx) = tokio::sync::broadcast::channel(256);
        let app_state = Arc::new(SharedState {
            task_queue: queue.clone(),
            pg: crate::db::pg_store::PgStore::connect_test().await.unwrap(),
            gateway,
            event_tx,
        });
        let ctx = Arc::new(TaskContext {
            queue,
            app_state,
            _graph_path: None,
        });
        (ctx, event_rx)
    }

    #[tokio::test]
    async fn scan_root_errors_on_nonexistent_path() {
        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ScanRoot, "", "/nonexistent/path/xyz");
        let result = scan_root(&ctx, &task).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn scan_root_enqueues_only_git_folders() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("alpha/.git")).unwrap();
        std::fs::create_dir_all(tmp.path().join("beta/.git")).unwrap();
        std::fs::create_dir_all(tmp.path().join("notes")).unwrap(); // non-git

        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ScanRoot, "", &tmp.path().to_string_lossy());
        scan_root(&ctx, &task).await.unwrap();

        let status = ctx.queue.status().await;
        assert_eq!(status.pending, 2, "only git folders should be enqueued");
    }

    #[tokio::test]
    async fn scan_emits_only_activity_events() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("proj/a/.git")).unwrap();
        std::fs::create_dir_all(tmp.path().join("proj/b/.git")).unwrap();
        std::fs::create_dir_all(tmp.path().join("proj/notes")).unwrap();
        std::fs::create_dir_all(tmp.path().join("random")).unwrap();

        let (ctx, mut rx) = make_ctx_with_events().await;
        let task = Task::new(TaskKind::ScanRoot, "", &tmp.path().to_string_lossy());
        scan_root(&ctx, &task).await.unwrap();

        let mut events = vec![];
        while let Ok(evt) = rx.try_recv() { events.push(evt); }

        // ALL events must be activity — no project or folder events from ScanRoot
        for evt in &events {
            assert_eq!(evt.entity, "activity",
                "ScanRoot should only emit activity events, got entity={}", evt.entity);
        }

        // Discover events: 2 git + 1 sibling + 1 standalone = 4
        let discovers: Vec<_> = events.iter()
            .filter(|e| e.data["level"] == "discover")
            .collect();
        assert_eq!(discovers.len(), 4, "expected 4 discover events, got {}", discovers.len());

        // Info summary
        let infos: Vec<_> = events.iter()
            .filter(|e| e.data["level"] == "info")
            .collect();
        assert_eq!(infos.len(), 1);
        let msg = infos[0].data["message"].as_str().unwrap();
        assert!(msg.contains("2 git"), "summary: {}", msg);
        assert!(msg.contains("1 sibling"), "summary: {}", msg);
        assert!(msg.contains("1 standalone"), "summary: {}", msg);
    }
}
