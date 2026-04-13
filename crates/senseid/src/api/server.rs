use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{CorsLayer, Any};
use crate::db::Store;
use crate::indexer::graph::GraphDb;
use crate::indexer::queue::IndexQueue;
use crate::watcher::DirtyTracker;
use super::routes::{create_router, SharedState, AppState};

pub async fn start_server(store: Store, graph: GraphDb, port: u16) -> std::io::Result<()> {
    let (index_queue, _rx) = IndexQueue::new();
    let dirty_tracker = DirtyTracker::new();

    let state: AppState = Arc::new(SharedState {
        store: Mutex::new(store),
        graph: Mutex::new(graph),
        index_queue: index_queue.clone(),
        dirty_tracker: dirty_tracker.clone(),
    });

    // Spawn background index worker
    crate::indexer::worker::spawn_worker(index_queue.clone(), state.clone()).await;

    // Spawn file watchers for all registered projects
    spawn_watchers(state.clone()).await;

    // Spawn dirty flusher — periodically checks for dirty repos and enqueues re-index
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

/// Spawn file watchers for all registered projects.
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

        // Register in dirty tracker
        state.dirty_tracker.register_repo(&repo_id, &repo_path).await;

        let tracker = state.dirty_tracker.clone();
        let rid = repo_id.clone();

        // Spawn watcher thread
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
                                // Bridge sync→async: use tokio's handle
                                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                                    handle.block_on(async {
                                        tracker.mark_dirty(&rid, batch).await;
                                    });
                                }
                            }
                            Err(_) => break, // channel closed
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to watch {}: {}", rid, e);
                }
            }
        });
    }
}

/// Periodically flush dirty repos into the index queue.
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
