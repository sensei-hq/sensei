use std::sync::Arc;
use tower_http::cors::{CorsLayer, Any};
use axum::http::Method;
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
    // CORS: WKWebView (Safari) does NOT honour `Access-Control-Allow-Methods: *`
    // — it requires an explicit method list, otherwise the preflight for any
    // non-simple request (PUT/DELETE/PATCH, or POST with JSON Content-Type)
    // is treated as blocked and the in-app fetch throws "Load failed" with no
    // server hit. Curl and Chrome accept the wildcard, which is why this only
    // bit in the Tauri app. List the methods we actually serve; same fix for
    // headers since Safari has the same wildcard-rejection there.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
            Method::HEAD,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            axum::http::header::AUTHORIZATION,
        ]);

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
            // `.ok()` is deliberate here — ctrl_c() only errors if signal
            // handler registration fails (rare, non-actionable at runtime).
            // Either way we want the graceful-shutdown future to complete
            // so the server drives its teardown flow. Not a silent-error bug.
            if let Err(e) = tokio::signal::ctrl_c().await {
                tracing::warn!(error = %e, "ctrl_c handler setup failed — shutdown will still run");
            }
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

    // Table-driven gateway config (#76): load routers/models/chains from the
    // `gateway.*` tables. A load error is logged and degrades to the in-code
    // baseline rather than failing daemon startup.
    let db_config = match super::gateway_config_loader::load_gateway_config(pg.pool()).await {
        Ok(Some(cfg)) => {
            tracing::info!(
                "Gateway: loaded table-driven config ({} routers, {} models, {} chains)",
                cfg.routers.len(), cfg.models.len(), cfg.chains.len()
            );
            Some(cfg)
        }
        Ok(None) => {
            tracing::info!("Gateway: no chains in DB — falling back to baseline config");
            None
        }
        Err(e) => {
            tracing::warn!("Gateway: failed to load table-driven config ({e}); using baseline");
            None
        }
    };
    let gateway = super::gateway_init::init_gateway(db_config).await;

    let (event_tx, _) = tokio::sync::broadcast::channel(1024);
    let state = Arc::new(SharedState {
        pg,
        task_queue: task_queue.clone(),
        gateway,
        event_tx,
        breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
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

    // Version-change rebuild (D2): if this binary differs from the one that
    // last scanned the DB, re-scan all indexed roots (rebuilds the code graph
    // under the new binary) and clear the analyzer full-refresh watermark so
    // the scheduler re-analyzes every project. Runs BEFORE the scheduler spawns
    // so the cleared watermark is observed on its first (immediate) tick.
    // Non-fatal: internal failures are logged, never propagated.
    //
    // Crash-safe: the new version is recorded only after the enqueued rescan
    // has drained (arm the commit watcher on trigger). If the daemon aborts
    // mid-rescan — e.g. an over-long embed input tripping a GGML abort — the
    // version stays unrecorded and the next boot re-runs the rebuild.
    if crate::tasks::version_rescan::maybe_rescan_on_version_change(
        &state.pg,
        &task_queue,
        env!("CARGO_PKG_VERSION"),
    ).await {
        crate::tasks::version_rescan::spawn_version_commit_watcher(
            Arc::new(state.pg.clone()),
            task_queue.clone(),
            env!("CARGO_PKG_VERSION").to_string(),
        );
    }

    // Periodically enrich/analyze projects whose sessions changed (#67). First
    // tick fires immediately, so a freshly-started daemon backfills enrichment.
    crate::tasks::analyzer_scheduler::spawn(
        task_queue.clone(),
        Arc::new(state.pg.clone()),
    );

    // Log retention (#74): periodically prune `public.logs` older than the
    // configured window (default 30d, daily). First tick prunes on startup.
    crate::tasks::log_pruner::spawn(Arc::new(state.pg.clone()));

    // Activity-data retention (#74): periodically prune raw activity older
    // than `activity.retention_days` (default 30d, daily), guarded by
    // `analyzed_at IS NOT NULL` so the pruner never drops a session before
    // the analyzer has derived its insights.
    crate::tasks::activity_pruner::spawn(Arc::new(state.pg.clone()));

    // Re-enqueue tasks for folders left in a non-terminal state by a
    // previous daemon session. Must run after workers and the progress
    // emitter are live so resumed tasks get picked up immediately and
    // their progress is broadcast to subscribers.
    let resumed = crate::tasks::resume::resume_pending_scans(&task_queue, &state.pg).await;
    if resumed > 0 {
        tracing::info!("startup: resumed {} pending folder scan(s)", resumed);
    }

    spawn_root_watchers(&state, task_queue.clone()).await;

    // Materialize the global rules file (~/.sensei/rules.md) from the governance
    // plane on startup so the session-start hook injects a fresh, resolved set.
    // Also upsert a durable managed-block pointer into ~/.claude/CLAUDE.md so
    // Claude Code post-compaction states + non-Claude ACPs that read CLAUDE.md
    // see where the resolved global rules live (#13).
    {
        let pg = state.pg.clone();
        tokio::spawn(async move {
            let sensei_dir = crate::paths::sensei_dir();
            match crate::api::handlers::knowledge::materialize_global_rules(&pg, &sensei_dir).await {
                Ok((rules_path, n)) => {
                    tracing::info!("startup: materialized {n} global rule(s) → {}", rules_path.display());
                    let claude_md = crate::paths::home().join(".claude/CLAUDE.md");
                    match crate::api::handlers::knowledge::upsert_pointer_in_claude_md(&claude_md, &rules_path) {
                        Ok(None) => tracing::debug!("startup: {} not present, skipping CLAUDE.md pointer", claude_md.display()),
                        Ok(Some((path, true))) => tracing::info!("startup: updated CLAUDE.md pointer → {}", path.display()),
                        Ok(Some((_, false))) => tracing::debug!("startup: CLAUDE.md pointer already current"),
                        Err(e) => tracing::warn!("startup: CLAUDE.md pointer upsert failed: {e}"),
                    }
                }
                Err(e) => tracing::warn!("startup: global rules materialize failed: {e}"),
            }
        });
    }

    // Tool capture — discover each ACP's MCP servers (incl. Claude Code plugin
    // MCPs), probe them, and rebuild the unified `sensei.assistant_tools`
    // inventory (MCP tools + built-in catalog) so the Instruments · Health share
    // grid (#84) has day-one data without a manual refresh. Runs after the
    // projects table is populated (needs project roots), so it fires from a
    // spawn with a small delay rather than blocking boot.
    {
        let pg = state.pg.clone();
        tokio::spawn(async move {
            // Small delay so the initial scan queue drains and any
            // fresh-boot project rows land first.
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            match crate::tool_discovery::run_capture(&pg).await {
                Ok(c) => tracing::info!(
                    "startup: tool capture — {} servers, {} builtins, probed_ok {}, probed_err {}",
                    c.discovered, c.builtins, c.probed_ok, c.probed_err),
                Err(e) => tracing::warn!("startup: tool capture failed: {e}"),
            }
        });
    }

    // Log retention: prune public.logs rows older than 30 days, on startup and
    // then daily. The task logger writes two rows per task, so large scans add
    // hundreds of thousands of rows; this keeps the table bounded.
    {
        let pg = state.pg.clone();
        tokio::spawn(async move {
            const RETENTION_DAYS: i32 = 30;
            loop {
                match pg.prune_logs(RETENTION_DAYS).await {
                    Ok(n) if n > 0 => tracing::info!("log retention: pruned {n} log rows older than {RETENTION_DAYS}d"),
                    Err(e) => tracing::warn!("log retention prune failed: {e}"),
                    _ => {}
                }
                tokio::time::sleep(std::time::Duration::from_secs(24 * 3600)).await;
            }
        });
    }

    // Federation: poll registered hive-mind sources for applicable rule deltas.
    crate::federation::run_pull_loop(state.pg.clone(), 300);

    // Capture watchdog: hourly sweep over configured ACP adapters. Auto-resolves
    // config-side failures (reinstall), trips a per-adapter breaker on give-up,
    // and notifies the user (the only signal once an adapter is suspended).
    {
        let pg = state.pg.clone();
        let breaker = state.breaker.clone();
        let notifier: std::sync::Arc<dyn crate::notifications::Notifier> =
            std::sync::Arc::new(crate::notifications::DesktopNotifier);
        // DB audit logger (writes to public.logs), mirrors the task_logger above.
        let watchdog_logger = sensei_logger::Logger::new(
            sensei_logger::LogWriter::pg(state.pg.pool().clone()),
            sensei_logger::LogLevel::Info,
            "daemon",
            "watchdog",
        );
        tokio::spawn(async move {
            // Small initial delay so startup churn settles before the first sweep.
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            loop {
                let now_ms = chrono::Utc::now().timestamp_millis();
                crate::assistants::run_sweep(&pg, &notifier, &breaker, &watchdog_logger, now_ms).await;
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });
    }

    (create_router(state), task_queue)
}

/// Start root watchers for persisted scanned roots.
async fn spawn_root_watchers(state: &Arc<SharedState>, queue: Arc<TaskQueue>) {
    // Get all watch roots from PgStore — (id, path) for roots that still exist
    // on disk (skip stale rows pointing at deleted dirs).
    let roots = state.pg.list_watch_roots().await.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "spawn_root_watchers: list_watch_roots failed; no roots will be watched");
        Vec::new()
    });
    let live: Vec<(uuid::Uuid, String)> = roots.iter()
        .filter_map(|r| {
            let path = r["path"].as_str()?.to_string();
            let id = crate::api::util::json_uuid(&r["id"])?;
            Some((id, path))
        })
        .filter(|(_, p)| std::path::Path::new(p).exists())
        .collect();

    if live.is_empty() { return; }

    // Register + start inside the lock; capture whether the watcher is live.
    // (Don't hold the std Mutex across an await.)
    let started = {
        let watcher = crate::watcher::root_watcher::RootWatcher::instance(queue);
        match watcher.lock() {
            Ok(mut w) => {
                for (_, root) in &live {
                    w.register(std::path::PathBuf::from(root), vec![]);
                    tracing::info!("Root watcher: registered {}", root);
                }
                w.start().is_ok()
                    && *w.status() == crate::watcher::root_watcher::WatcherStatus::Watching
            }
            Err(_) => false,
        }
    };

    // Reflect the live watcher state in folders_to_watch so the DB/UI shows
    // 'watching' instead of being stuck at 'scanning' (the restart-watch gap).
    if started {
        for (id, _) in &live {
            if let Err(e) = state.pg.update_watch_status(id, "watching").await {
                tracing::warn!(error = %e, %id, "failed to update watch status to 'watching'");
            }
        }
    }
}
