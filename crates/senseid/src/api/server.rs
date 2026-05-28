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

    // Connect to PostgreSQL.
    // All mode-sensitive values come from SenseiConfig — compile-time via Cargo features.
    // Dev builds (--features dev): port 7745, sensei_dev DB, ~/.sensei-dev/
    // Prod builds (no features):  port 7744, sensei DB, ~/.sensei/
    let database_url = sensei_bootstrap::SenseiConfig::from_env().db_url;
    let pg = match crate::db::pg_store::PgStore::connect(&database_url).await {
        Ok(store) => store,
        Err(e) => {
            eprintln!(
                "[senseid] Database unavailable ({}). Run bootstrap to set up the database, then restart.",
                e
            );
            std::process::exit(1);
        }
    };

    // Read max concurrent repos from PgStore config
    if let Ok(Some(max_str)) = pg.get_config("max_concurrent_repos").await
        && let Ok(max) = max_str.parse::<usize>() {
            task_queue.set_max_concurrent_repos(max);
        }

    // Initialize inference gateway (probes Ollama, checks API keys)
    let gateway = super::gateway_init::init_gateway().await;

    let (event_tx, _) = tokio::sync::broadcast::channel(1024);
    let state = Arc::new(SharedState {
        pg,
        task_queue: task_queue.clone(),
        gateway,
        event_tx,
    });

    // Spawn task workers
    let task_logger = sensei_logger::Logger::new(
        sensei_logger::LogWriter::pg(state.pg.pool().clone()),
        sensei_logger::LogLevel::Info,
        "daemon",
        "tasks",
    );
    let task_ctx = Arc::new(TaskContext {
        queue: task_queue.clone(),
        app_state: state.clone(),
        _graph_path: None,
        logger: task_logger,
    });
    spawn_workers(task_ctx, DEFAULT_WORKERS);

    // Spawn the scan progress emitter — translates per-file TaskEvent::Completed
    // into throttled StateEvent::folder_update SSE events for the wizard scan stage.
    crate::tasks::progress_emitter::spawn(
        task_queue.sender().subscribe(),
        state.event_tx.clone(),
        Arc::new(state.pg.clone()),
    );

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
