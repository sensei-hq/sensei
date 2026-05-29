use std::sync::Arc;
use tower_http::cors::{CorsLayer, Any};
use crate::tasks::queue::TaskQueue;
use crate::tasks::executor::{TaskContext, spawn_workers};
use super::routes::{create_router, create_degraded_router};
use super::state::SharedState;

/// Write a single-line startup error to `<sensei_dir>/startup-error.log` so
/// users can find it without scraping launchd / brew-services log paths.
fn write_startup_error(msg: &str) {
    let dir = sensei_bootstrap::SenseiConfig::from_env().sensei_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("startup-error.log"), msg);
}

fn clear_startup_error() {
    let dir = sensei_bootstrap::SenseiConfig::from_env().sensei_dir();
    let _ = std::fs::remove_file(dir.join("startup-error.log"));
}

const DEFAULT_WORKERS: usize = 3;

pub async fn start_server(port: u16) -> std::io::Result<()> {
    super::handlers::health::init_uptime();

    // Bind the port FIRST. If the DB is down, we want the daemon to still
    // serve /api/health so the frontend can show the actual cause — old
    // behaviour was to exit, leaving the client to guess "connection
    // refused" with no diagnostic.
    let listener = match tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            let msg = format!(
                "[senseid] Port {0} is already in use. Another senseid process is likely running.\n  Investigate: lsof -i :{0}\n  If launchd / brew services restarts it: brew services stop sensei-dev (or sensei).\n  Or set a different port via env (e.g., SENSEI_PORT=...).",
                port
            );
            eprintln!("{}", msg);
            write_startup_error(&msg);
            return Err(e);
        }
        Err(e) => {
            let msg = format!("[senseid] Failed to bind to :{}: {}", port, e);
            eprintln!("{}", msg);
            write_startup_error(&msg);
            return Err(e);
        }
    };

    let cfg = sensei_bootstrap::SenseiConfig::from_env();
    let database_url = cfg.db_url.clone();
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Try DB connect. Branch on result: full router on success, degraded
    // router (health endpoint + 503 catch-all) on failure.
    let (app, watcher_queue): (axum::Router, Option<Arc<TaskQueue>>) =
        match crate::db::pg_store::PgStore::connect(&database_url).await {
            Ok(pg) => {
                clear_startup_error();
                tracing::info!("senseid listening on :{} (full mode)", port);
                let (router, queue) = build_full_app(pg).await;
                (router.layer(cors), Some(queue))
            }
            Err(e) => {
                let msg = format!(
                    "[senseid] Database connection failed — daemon staying alive in degraded mode.\n  URL: {}\n  Error: {}\n  Hint: run `sensei bootstrap` (or `dbd reset`) to (re)provision the database, then restart.",
                    database_url, e
                );
                eprintln!("{}", msg);
                write_startup_error(&msg);
                tracing::warn!("senseid listening on :{} (degraded — DB unavailable)", port);
                let router = create_degraded_router(database_url.clone(), e.clone()).layer(cors);
                (router, None)
            }
        };

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("Shutting down...");
            if let Some(q) = watcher_queue {
                let watcher = crate::watcher::root_watcher::RootWatcher::instance(q);
                if let Ok(mut w) = watcher.lock() {
                    w.stop();
                    tracing::info!("Watcher stopped");
                }
            }
        })
        .await
}

/// Build the full-mode router and supporting infrastructure (task queue,
/// workers, progress emitter, root watchers). Only called once the DB
/// connect has succeeded — every component below assumes `PgStore` works.
async fn build_full_app(pg: crate::db::pg_store::PgStore) -> (axum::Router, Arc<TaskQueue>) {
    let task_queue = Arc::new(TaskQueue::new());

    if let Ok(Some(max_str)) = pg.get_config("max_concurrent_repos").await
        && let Ok(max) = max_str.parse::<usize>() {
            task_queue.set_max_concurrent_repos(max);
        }

    let gateway = super::gateway_init::init_gateway().await;

    let (event_tx, _) = tokio::sync::broadcast::channel(1024);
    let state = Arc::new(SharedState {
        pg,
        task_queue: task_queue.clone(),
        gateway,
        event_tx,
    });

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

    crate::tasks::progress_emitter::spawn(
        task_queue.sender().subscribe(),
        state.event_tx.clone(),
        Arc::new(state.pg.clone()),
    );

    spawn_root_watchers(&state, task_queue.clone()).await;

    (create_router(state), task_queue)
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
