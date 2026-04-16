use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{CorsLayer, Any};
use crate::db::Store;
use crate::indexer::graph::GraphDb;
use crate::indexer::queue::IndexQueue;
use crate::watcher::DirtyTracker;
use crate::tasks::queue::TaskQueue;
use crate::tasks::executor::{TaskContext, spawn_workers};
use super::routes::{create_router, SharedState, AppState};

const DEFAULT_WORKERS: usize = 3;

pub async fn start_server(store: Store, graph: GraphDb, port: u16) -> std::io::Result<()> {
    // Old queue (kept for backward compat during migration)
    let (index_queue, _rx) = IndexQueue::new();
    let dirty_tracker = DirtyTracker::new();

    // New task queue
    let task_queue = Arc::new(TaskQueue::new());

    // Read max concurrent repos from config
    if let Ok(Some(max_str)) = store.get_config("max_concurrent_repos") {
        if let Ok(max) = max_str.parse::<usize>() {
            task_queue.set_max_concurrent_repos(max);
        }
    }

    let graph_path = graph.db_path().map(|p| p.parent().unwrap_or(p).to_path_buf());

    let state: AppState = Arc::new(SharedState {
        store: Mutex::new(store),
        graph: Mutex::new(graph),
        index_queue: index_queue.clone(),
        dirty_tracker: dirty_tracker.clone(),
        task_queue: task_queue.clone(),
    });

    // Spawn old worker (kept for backward compat)
    crate::indexer::worker::spawn_worker(index_queue.clone(), state.clone()).await;

    // Spawn new task workers — share AppState directly
    let task_ctx = Arc::new(TaskContext {
        queue: task_queue.clone(),
        app_state: state.clone(),
        graph_path,
    });
    spawn_workers(task_ctx.clone(), DEFAULT_WORKERS);

    // Spawn old file watchers (kept for backward compat)
    spawn_watchers(state.clone()).await;

    // Start root watchers for scanned roots
    spawn_root_watchers(state.clone(), task_queue.clone()).await;

    // Spawn old dirty flusher (kept for backward compat)
    spawn_dirty_flusher(index_queue, dirty_tracker, state.clone());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = create_router(state).layer(cors);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("senseid listening on :{}", port);

    axum::serve(listener, app).await
}

/// Spawn file watchers for all registered projects (OLD — backward compat).
async fn spawn_watchers(state: AppState) {
    let projects = {
        let s = state.store.lock().await;
        s.list_projects().unwrap_or_default()
    };

    for project in projects {
        let path_buf = std::path::PathBuf::from(&project.path);
        if !path_buf.exists() { continue; }

        let repo_id = project.repo_id.clone();
        let repo_path = project.path.clone();

        state.dirty_tracker.register_repo(&repo_id, &repo_path).await;

        let tracker = state.dirty_tracker.clone();
        let rid = repo_id.clone();

        tokio::task::spawn_blocking(move || {
            let watcher = crate::watcher::RepoWatcher::new(&rid, &path_buf);
            match watcher.watch() {
                Ok(rx) => {
                    tracing::info!("Watching {} at {}", rid, repo_path);
                    loop {
                        match rx.recv() {
                            Ok(batch) => {
                                let tracker = tracker.clone();
                                let rid = rid.clone();
                                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                                    handle.block_on(async {
                                        tracker.mark_dirty(&rid, batch).await;
                                    });
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }
                Err(e) => tracing::warn!("Failed to watch {}: {}", rid, e),
            }
        });
    }
}

/// Start root watchers for persisted scanned roots (NEW task-based).
async fn spawn_root_watchers(state: AppState, queue: Arc<TaskQueue>) {
    let store = state.store.lock().await;

    // Get scanned roots from config
    let roots: Vec<String> = store.execute_raw("CREATE TABLE IF NOT EXISTS scanned_roots(path TEXT PRIMARY KEY, created_at TEXT DEFAULT (datetime('now')))")
        .ok()
        .and_then(|_| {
            // Query all roots
            None // roots are populated via scan API — will start watchers then
        })
        .unwrap_or_default();

    if roots.is_empty() { return; }

    let projects: std::collections::HashMap<String, String> = store.list_projects()
        .unwrap_or_default()
        .into_iter()
        .map(|p| (p.repo_id, p.path))
        .collect();

    drop(store);

    for root in roots {
        crate::watcher::root_watcher::start_root_watcher(
            std::path::PathBuf::from(&root),
            queue.clone(),
            projects.clone(),
        ).ok();
    }
}

/// Periodically flush dirty repos into the index queue (OLD — backward compat).
fn spawn_dirty_flusher(
    queue: Arc<IndexQueue>,
    tracker: DirtyTracker,
    state: AppState,
) {
    tokio::spawn(async move {
        let flush_interval = std::time::Duration::from_secs(5);
        loop {
            tokio::time::sleep(flush_interval).await;

            let dirty = tracker.dirty_repos().await;
            for (repo_id, repo_path, count) in dirty {
                if count > 0 {
                    tracing::info!("Auto-enqueuing re-index for {} ({} dirty files)", repo_id, count);
                    queue.enqueue(repo_id, repo_path, false).await;
                }
            }
        }
    });
}
