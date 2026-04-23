use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{CorsLayer, Any};
use crate::db::Store;
use crate::indexer::graph::GraphDb;
use crate::tasks::queue::TaskQueue;
use crate::tasks::executor::{TaskContext, spawn_workers};
use super::routes::create_router;
use super::state::SharedState;

const DEFAULT_WORKERS: usize = 3;

pub async fn start_server(store: Store, graph: GraphDb, port: u16) -> std::io::Result<()> {
    let task_queue = Arc::new(TaskQueue::new());

    // Read max concurrent repos from config
    if let Ok(Some(max_str)) = store.get_config("max_concurrent_repos")
        && let Ok(max) = max_str.parse::<usize>() {
            task_queue.set_max_concurrent_repos(max);
        }

    let graph_path = graph.db_path().map(|p| p.parent().unwrap_or(p).to_path_buf());

    let state = Arc::new(SharedState {
        store: Mutex::new(store),
        graph: Mutex::new(graph),
        task_queue: task_queue.clone(),
    });

    // Spawn task workers
    let task_ctx = Arc::new(TaskContext {
        queue: task_queue.clone(),
        app_state: state.clone(),
        _graph_path: graph_path,
    });
    spawn_workers(task_ctx, DEFAULT_WORKERS);

    // Start root watchers for persisted scanned roots
    spawn_root_watchers(&state, task_queue.clone()).await;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = create_router(state).layer(cors);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("senseid listening on :{}", port);

    axum::serve(listener, app).await
}

/// Start root watchers for persisted scanned roots.
async fn spawn_root_watchers(state: &Arc<SharedState>, queue: Arc<TaskQueue>) {
    let store = state.store.lock().await;

    // Ensure scanned_roots table exists
    store.execute_raw("CREATE TABLE IF NOT EXISTS scanned_roots(path TEXT PRIMARY KEY, created_at TEXT DEFAULT (datetime('now')))").ok();

    // Get all scanned roots
    let roots: Vec<String> = {
        let mut stmt = match store.conn_ref().prepare("SELECT path FROM scanned_roots") {
            Ok(s) => s,
            Err(_) => return,
        };
        stmt.query_map([], |row| row.get::<_, String>(0))
            .unwrap_or_else(|_| unreachable!())
            .filter_map(|r| r.ok())
            .collect()
    };

    if roots.is_empty() { return; }

    let projects: std::collections::HashMap<String, String> = store.list_repos()
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
        tracing::info!("Root watcher: {}", root);
    }
}
