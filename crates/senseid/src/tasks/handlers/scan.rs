//! Scan phase: discover folders, classify, group into projects, enqueue tasks.
//! Emits StateEvent (activity + project + folder) via event_tx for SSE consumers.

use std::path::Path;
use std::time::Instant;
use super::super::executor::TaskContext;
use super::super::{Task, TaskKind};
use super::scan_logic::{self, FolderKind, Confidence};
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
            _ => {} // git folders already emitted above
        }
    }

    // 3. Group into projects
    let projects = scan_logic::group_into_projects(&classified);

    // 4. Register watch root in DB
    let root_name = root.file_name().and_then(|n| n.to_str()).unwrap_or("root");
    let root_id = ctx.pg().add_watch_root(&task.path, root_name, &serde_json::json!([])).await
        .map_err(|e| format!("Failed to register watch root: {}", e))?;

    // 5. Register projects and folders in DB, emit events, enqueue tasks
    for proj in &projects {
        // Create project in DB
        let project_id = ctx.pg().create_project(&proj.name, None, None).await
            .map(|id| id.to_string())
            .unwrap_or_else(|_| format!("p-{}", proj.name));

        // Emit: project add
        let confidence = match proj.confidence {
            Confidence::High => crate::api::events::Confidence::High,
            Confidence::Medium => crate::api::events::Confidence::Medium,
            Confidence::Low => crate::api::events::Confidence::Low,
        };
        emit(StateEvent::project_add(ScanProject {
            id: project_id.clone(),
            name: proj.name.clone(),
            status: ProjectStatus::Scanning,
            folders: vec![], // folders emitted separately
            auto_detected: true,
            confidence,
        }));

        // Register + emit each folder
        for f in &proj.folders {
            let folder_kind_str = match f.kind {
                FolderKind::Git => "git",
                FolderKind::WorkspaceMember => "workspace_member",
                FolderKind::Subtree => "subtree",
                FolderKind::Sibling => "sibling",
                FolderKind::Standalone => "standalone",
            };
            let status_str = match f.kind {
                FolderKind::Git | FolderKind::WorkspaceMember | FolderKind::Subtree => "discovered",
                FolderKind::Sibling | FolderKind::Standalone => "deferred",
            };

            // Register folder in DB
            ctx.pg().upsert_repo(&root_id, &f.name, &f.path.to_string_lossy()).await.ok();
            // TODO: set kind, status, project_id on folder record via PgStore

            // Emit: folder add
            let folder_status = match f.kind {
                FolderKind::Sibling | FolderKind::Standalone => FolderStatus::Discovered, // will use Deferred when added to events.rs
                _ => FolderStatus::Discovered,
            };
            emit(StateEvent::folder_add(ScanFolder {
                id: format!("f-{}", f.name),
                project_id: project_id.clone(),
                name: f.name.clone(),
                path: f.path.to_string_lossy().to_string(),
                kind: match f.kind {
                    FolderKind::Git => crate::api::events::FolderKind::Git,
                    FolderKind::WorkspaceMember => crate::api::events::FolderKind::WorkspaceMember,
                    FolderKind::Subtree => crate::api::events::FolderKind::Subtree,
                    FolderKind::Sibling => crate::api::events::FolderKind::Sibling,
                    FolderKind::Standalone => crate::api::events::FolderKind::Standalone,
                },
                stack: vec![],
                files_total: 0,
                files_completed: 0,
                status: folder_status,
            }));

            // Enqueue ProcessGitFolder for indexable folders
            if matches!(f.kind, FolderKind::Git | FolderKind::Subtree) {
                let git_task = Task::new(TaskKind::ProcessGitFolder, &f.path.to_string_lossy(), &f.path.to_string_lossy())
                    .with_parent(task.id);
                ctx.queue.enqueue(git_task).await;
            }
        }
    }

    // 6. Summary activity
    let git_count = classified.iter().filter(|f| f.kind == FolderKind::Git).count();
    let sibling_count = classified.iter().filter(|f| f.kind == FolderKind::Sibling).count();
    let standalone_count = classified.iter().filter(|f| f.kind == FolderKind::Standalone).count();

    emit(StateEvent::activity(ActivityEvent::new(
        ActivityLevel::Info,
        &format!("{} git · {} sibling · {} standalone · {} projects",
            git_count, sibling_count, standalone_count, projects.len()),
        start.elapsed().as_secs_f64(),
    )));

    // Register with watcher
    {
        let watcher = crate::watcher::root_watcher::RootWatcher::instance(ctx.queue.clone());
        if let Ok(mut w) = watcher.lock() {
            w.register(std::path::PathBuf::from(&task.path), vec![]);
        }
    }

    tracing::info!("scan_root: {} git, {} sibling, {} standalone, {} projects in {}",
        git_count, sibling_count, standalone_count, projects.len(), task.path);
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
    async fn scan_root_discovers_and_enqueues() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("alpha/.git")).unwrap();
        std::fs::create_dir_all(tmp.path().join("beta/.git")).unwrap();

        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ScanRoot, "", &tmp.path().to_string_lossy());
        scan_root(&ctx, &task).await.unwrap();

        let status = ctx.queue.status().await;
        assert_eq!(status.pending, 2, "expected 2 ProcessGitFolder tasks");
    }

    #[tokio::test]
    async fn scan_emits_correct_events() {
        let tmp = tempfile::tempdir().unwrap();
        // 2 git siblings + 1 non-git sibling
        std::fs::create_dir_all(tmp.path().join("proj/a/.git")).unwrap();
        std::fs::create_dir_all(tmp.path().join("proj/b/.git")).unwrap();
        std::fs::create_dir_all(tmp.path().join("proj/notes")).unwrap();
        // 1 standalone non-git
        std::fs::create_dir_all(tmp.path().join("random")).unwrap();

        let (ctx, mut rx) = make_ctx_with_events().await;
        let task = Task::new(TaskKind::ScanRoot, "", &tmp.path().to_string_lossy());
        scan_root(&ctx, &task).await.unwrap();

        // Drain events
        let mut events = vec![];
        while let Ok(evt) = rx.try_recv() { events.push(evt); }

        // Activity: discover events
        let discovers: Vec<_> = events.iter()
            .filter(|e| e.entity == "activity" && e.data["level"] == "discover")
            .collect();
        assert!(discovers.len() >= 4, "expected 4+ discover events (2 git + 1 sibling + 1 standalone), got {}", discovers.len());

        // Project events
        let proj_events: Vec<_> = events.iter().filter(|e| e.entity == "project").collect();
        assert!(proj_events.len() >= 2, "expected 2+ projects (proj group + random standalone), got {}", proj_events.len());

        // Folder events
        let folder_events: Vec<_> = events.iter().filter(|e| e.entity == "folder").collect();
        assert!(folder_events.len() >= 4, "expected 4+ folders (2 git + 1 sibling + 1 standalone), got {}", folder_events.len());

        // Check kinds
        let git_folders: Vec<_> = folder_events.iter()
            .filter(|e| e.data["kind"] == "git")
            .collect();
        assert_eq!(git_folders.len(), 2);

        let sibling_folders: Vec<_> = folder_events.iter()
            .filter(|e| e.data["kind"] == "sibling")
            .collect();
        assert_eq!(sibling_folders.len(), 1);

        // Info summary
        let infos: Vec<_> = events.iter()
            .filter(|e| e.entity == "activity" && e.data["level"] == "info")
            .collect();
        assert_eq!(infos.len(), 1);

        // Log events
        let log_path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".sensei/logs/scan-rewired-events.jsonl");
        std::fs::create_dir_all(log_path.parent().unwrap()).ok();
        let mut log = String::new();
        for evt in &events {
            log.push_str(&serde_json::to_string_pretty(evt).unwrap());
            log.push('\n');
        }
        std::fs::write(&log_path, &log).unwrap();
        eprintln!("Events: {}", log_path.display());
    }
}
