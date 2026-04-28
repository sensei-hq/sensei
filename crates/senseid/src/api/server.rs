use std::sync::Arc;
use tower_http::cors::{CorsLayer, Any};
use crate::tasks::queue::TaskQueue;
use crate::tasks::executor::{TaskContext, spawn_workers};
use super::routes::create_router;
use super::state::SharedState;

const DEFAULT_WORKERS: usize = 3;

pub async fn start_server(port: u16) -> std::io::Result<()> {
    super::handlers::health::init_uptime();
    let task_queue = Arc::new(TaskQueue::new());

    // Connect to PostgreSQL
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost:5432/sensei".to_string());
    let pg = crate::db::pg_store::PgStore::connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    // Read max concurrent repos from PgStore config
    if let Ok(Some(max_str)) = pg.get_config("max_concurrent_repos").await {
        if let Ok(max) = max_str.parse::<usize>() {
            task_queue.set_max_concurrent_repos(max);
        }
    }

    // Initialize inference gateway (probes Ollama, checks API keys)
    let gateway = super::gateway_init::init_gateway().await;

    let state = Arc::new(SharedState {
        pg,
        task_queue: task_queue.clone(),
        gateway,
    });

    // Spawn task workers
    let task_ctx = Arc::new(TaskContext {
        queue: task_queue.clone(),
        app_state: state.clone(),
        _graph_path: None,
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

    let watcher_queue = task_queue.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("Shutting down...");
            let watcher = crate::watcher::root_watcher::RootWatcher::instance(watcher_queue);
            if let Ok(mut w) = watcher.lock() {
                w.stop();
                tracing::info!("Watcher stopped");
            }
        })
        .await
}

/// Start root watchers for persisted scanned roots.
async fn spawn_root_watchers(state: &Arc<SharedState>, queue: Arc<TaskQueue>) {
    // Get all watch roots from PgStore
    let roots = state.pg.list_watch_roots().await.unwrap_or_default();

    let root_paths: Vec<String> = roots.iter()
        .filter_map(|r| r["path"].as_str().map(String::from))
        .collect();

    if root_paths.is_empty() { return; }

    let watcher = crate::watcher::root_watcher::RootWatcher::instance(queue);
    if let Ok(mut w) = watcher.lock() {
        for root in &root_paths {
            w.register(std::path::PathBuf::from(root), vec![]);
            tracing::info!("Root watcher: registered {}", root);
        }
        w.start().ok();
    }
}
