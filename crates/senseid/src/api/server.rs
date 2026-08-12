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

/// Resolve the daemon's TCP bind host. **Loopback-only (`127.0.0.1`) by
/// default** — the daemon's control-plane routes (`/hook/*`, `/api/runs*`)
/// carry no auth, so a non-loopback bind would expose them to LAN-adjacent
/// hosts (spurious gate prompts / inert run rows — see
/// `docs/plan/decisions.md`). The app, CLI, and MCP all connect over loopback,
/// so this is transparent to them. `SENSEI_BIND_HOST` opts into a specific host
/// (e.g. `0.0.0.0`) for a deliberate non-loopback deployment — which MUST add
/// route auth first. Returns `(host, is_loopback)`.
fn resolve_bind_host(env_host: Option<String>) -> (String, bool) {
    let host = env_host
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let is_loopback = matches!(host.as_str(), "127.0.0.1" | "::1" | "localhost");
    (host, is_loopback)
}

pub async fn start_server(port: u16) -> std::io::Result<()> {
    super::handlers::health::init_uptime();

    // Bind the port FIRST. If the DB is down, we want the daemon to still
    // serve /api/health so the frontend can show the actual cause — old
    // behaviour was to exit, leaving the client to guess "connection
    // refused" with no diagnostic.
    let (bind_host, is_loopback) = resolve_bind_host(std::env::var("SENSEI_BIND_HOST").ok());
    if !is_loopback {
        tracing::warn!(
            "senseid binding non-loopback host '{bind_host}' (SENSEI_BIND_HOST) — /hook/* and /api/runs* have NO auth and are now network-exposed; add route auth before any real deployment (see docs/plan/decisions.md)"
        );
    }
    let listener = match tokio::net::TcpListener::bind(format!("{bind_host}:{port}")).await {
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

    // Connect to the DB, retrying briefly so a cold-boot race — the daemon
    // coming up before Postgres accepts connections (both start together as
    // brew/launchd services) — is absorbed and we reach full mode without ever
    // serving degraded. Branch: full router on success; on a persistent failure
    // serve a hot-swappable degraded router and self-heal in the background
    // (no restart) once the DB returns. See `api::resilience`.
    let (app, watcher_queue): (axum::Router, Option<Arc<TaskQueue>>) =
        match crate::api::resilience::connect_with_retry(
            || {
                let url = database_url.clone();
                async move { crate::db::pg_store::PgStore::connect(&url).await }
            },
            &crate::api::resilience::RetryPolicy::startup(),
        )
        .await
        {
            Ok(pg) => {
                clear_startup_error();
                crate::api::resilience::mark_full();
                tracing::info!("senseid listening on :{} (full mode)", port);
                let (router, queue) = build_full_app(pg).await;
                (router.layer(cors), Some(queue))
            }
            Err(e) => {
                let msg = format!(
                    "[senseid] Database connection failed after startup retries — daemon staying alive in degraded mode; it will self-heal automatically when the DB becomes reachable.\n  URL: {}\n  Error: {}\n  Hint: run `sensei bootstrap` (or `dbd reset`) to (re)provision the database.",
                    database_url, e
                );
                eprintln!("{}", msg);
                write_startup_error(&msg);
                crate::api::resilience::mark_degraded();
                tracing::warn!("senseid listening on :{} (degraded — DB unavailable; self-heal armed)", port);

                // Serve the degraded router through a swappable handle so the
                // background task below can replace it in place once the DB is up.
                let handle = crate::api::resilience::RouterHandle::new(
                    create_degraded_router(database_url.clone(), e.clone()),
                );

                let bg_handle = handle.clone();
                let bg_url = database_url.clone();
                tokio::spawn(async move {
                    let upgraded = crate::api::resilience::reconnect_and_upgrade(
                        bg_handle,
                        || {
                            let url = bg_url.clone();
                            async move { crate::db::pg_store::PgStore::connect(&url).await }
                        },
                        |pg| async move { build_full_app(pg).await.0 },
                        &crate::api::resilience::RetryPolicy::background(),
                    )
                    .await;
                    if upgraded {
                        clear_startup_error();
                        crate::api::resilience::mark_full();
                    }
                });

                (handle.serving_router().layer(cors), None)
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
    let (gateway, provisioning) = super::gateway_init::init_gateway(db_config).await;

    let (event_tx, _) = tokio::sync::broadcast::channel(1024);
    let state = Arc::new(SharedState {
        pg,
        task_queue: task_queue.clone(),
        gateway,
        event_tx,
        breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        provisioning,
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
    // Boot reconcile (D6b/W2): before any worker can create a new
    // task_execution row, terminate rows left `running` by a dead prior
    // session — their in-memory tasks vanished with the process, so they can
    // never complete. `task_id` resets per session, so every `running` row
    // that predates this session start is orphaned. Runs BEFORE spawn_workers
    // so this session's own in-flight rows (all started >= session_start) are
    // never swept. Non-fatal: a failure here must not block startup.
    let session_start = chrono::Utc::now();
    match state.pg.reconcile_orphaned_task_executions(session_start).await {
        Ok(0) => {}
        Ok(n) => tracing::info!("startup: reconciled {n} orphaned task execution(s) from a prior session"),
        Err(e) => tracing::warn!(error = %e, "startup: reconcile_orphaned_task_executions failed"),
    }

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

    // Metrics pipeline (Phase 4): once-daily (watermarked), enqueue the active
    // metric registry per project — one ComputeMetrics per base group + a
    // ComputeHealth barrier. Mirrors the analyzer scheduler's spawn/watermark
    // pattern; the first tick backfills if a day has elapsed since the last run.
    crate::tasks::metrics_scheduler::spawn(
        task_queue.clone(),
        Arc::new(state.pg.clone()),
    );

    // First-install / boot transcript backfill (history recovery): recover MONTHS
    // of metric history, not ~2 weeks. Dispatch the transcript importer (first
    // install = full history; forward = new transcripts, smart-skipped) so the
    // day-keyed sources (sessions/events) exist, then re-attach any orphaned
    // sessions. Per-day metric planning is NOT kicked off here on a boot timer —
    // it is driven by ANALYSIS completion: each `AnalyzeProject` success enqueues an
    // overlap-guarded `PlanMetricDays` for its project, so backfilled sessions are
    // planned only once the analyzer has made them measurable (synthesizer →
    // analyzer → PlanMetricDays), with the daily metrics scheduler as the self-heal
    // backstop. A fixed sleep is not a real dependency, so it was removed. Runs from
    // a spawn so the initial scan + transcript ingestion don't block boot.
    {
        let pg = state.pg.clone();
        let queue = task_queue.clone();
        tokio::spawn(async move {
            let (_seen, dispatched) = crate::transcript::dispatch(&queue).await;
            if dispatched > 0 {
                tracing::info!(dispatched, "startup: dispatched transcript backfill for metric history");
            }
            match pg.repair_orphaned_sessions().await {
                Ok(n) if n > 0 => tracing::info!(repaired = n, "startup: re-attached orphaned sessions"),
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "startup: repair_orphaned_sessions failed"),
            }
        });
    }

    // Relay-engine (P3.2): drive daemon-owned autonomous runs. Each tick (15s)
    // auto-resumes due pauses and enqueues an AdvanceRun per active run. P3.2
    // only heartbeats + logs housekeeping; the agent spawn/drive is P3.3. DB
    // errors (incl. a not-yet-deployed runs table) are logged, never fatal.
    crate::tasks::advance_run_scheduler::spawn(
        task_queue.clone(),
        Arc::new(state.pg.clone()),
    );

    // Relay-engine (P3.6): the run watchdog. Every 60s it sweeps running/stalled
    // runs — a stale-heartbeat running run is marked stalled, a stalled run is
    // bounded-auto-recovered (back to running + AdvanceRun tick), and one past
    // the recovery cap escalates to crashed. Degrade-never-stop: a hung/dead run
    // is recovered or surfaced, never left silently wedged. Tolerates a
    // not-yet-deployed runs table (warn, never fatal).
    crate::tasks::watchdog_scheduler::spawn(
        task_queue.clone(),
        Arc::new(state.pg.clone()),
    );

    // Watcher safety net: frequently (boot + every reconcile.interval_secs,
    // default 300s) re-scan every watch root so the index converges even when
    // the fs-watcher misses events (daemon restarts / stale FSEvents gaps).
    // Reuses the self-healing scan pipeline — re-absorbs mis-scoped roots
    // (Bug 3) + prunes orphan nodes (Bug 2). The two-tier mtime gate makes a
    // no-op re-scan stat-only, so this is cheap to run often. The boot reconcile
    // ALWAYS runs (drift-safety); overlap-guarded so reconciles never stack.
    crate::tasks::reconcile_scheduler::spawn(
        task_queue.clone(),
        Arc::new(state.pg.clone()),
    );

    // Index integrity self-audit (P2): a conservative (daily, watermark-gated)
    // sweep that GENERALIZES the point-fix self-heals into one continuous
    // invariant checker + repairer — orphan nodes, ghost folders, nested
    // standalone roots, duplicate-name phantoms. It stats every indexed
    // file/folder (heavier than the reconcile), so it runs on its own
    // `audit.last_run` cadence, NOT every reconcile tick. Read-only twin behind
    // `GET /api/index/doctor` / `sensei index doctor`.
    crate::tasks::index_audit::spawn(Arc::new(state.pg.clone()));

    // Log retention (#74): periodically prune `public.logs` older than the
    // configured window (default 30d, daily). First tick prunes on startup.
    crate::tasks::log_pruner::spawn(Arc::new(state.pg.clone()));
    // Workstream F: daily library-update detection → 'library_update' recommendations.
    crate::tasks::library_update_scheduler::spawn(task_queue.clone(), Arc::new(state.pg.clone()));

    // Activity-data retention (#74): periodically prune raw activity older
    // than `activity.retention_days` (default 30d, daily), guarded by
    // `analyzed_at IS NOT NULL` so the pruner never drops a session before
    // the analyzer has derived its insights.
    crate::tasks::activity_pruner::spawn(Arc::new(state.pg.clone()));

    // Capture-spool drain: hook events that failed to reach the daemon (daemon
    // down, or a POST slower than the hook's 2s budget) are dead-lettered to
    // ~/.sensei/events.jsonl by the hook fallback. Import them into
    // activity.assistant_events so analysis isn't missing them; runs on boot +
    // every `capture.drain_interval_secs` (default 300s).
    crate::tasks::capture_drain::spawn(Arc::new(state.pg.clone()), crate::paths::sensei_dir());

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

    // Federation: poll registered Dōjō rules sources for applicable rule deltas.
    crate::federation::run_pull_loop(state.pg.clone(), 300);

    // Dōjō upstream contribute cadence (R1): the upstream twin of the pull loop
    // above. On the user's configured cadence (PAUSED by default) it PREPARES an
    // approved memory-share batch into the durable outbox as `pending`, running
    // the same strict anonymise + confidentiality gate as the manual publish.
    // STAGE-ONLY — it never publishes / egresses; the outbox→dojo send stays the
    // explicit manual C6 step. No-op until the user sets a daily/weekly cadence.
    crate::tasks::contribute_scheduler::spawn(
        Arc::new(state.pg.clone()),
        state.gateway.clone(),
    );

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
                // Give the watcher a DB handle so its loop can resolve each changed
                // file to its owning repo (else incremental tasks can't be shaped).
                w.set_store(state.pg.clone());
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

#[cfg(test)]
mod bind_host_tests {
    use super::resolve_bind_host;

    #[test]
    fn defaults_to_loopback_when_unset_or_blank() {
        assert_eq!(resolve_bind_host(None), ("127.0.0.1".to_string(), true));
        assert_eq!(resolve_bind_host(Some(String::new())), ("127.0.0.1".to_string(), true));
        assert_eq!(resolve_bind_host(Some("   ".to_string())), ("127.0.0.1".to_string(), true));
    }

    #[test]
    fn loopback_aliases_are_flagged_loopback() {
        assert!(resolve_bind_host(Some("127.0.0.1".to_string())).1);
        assert!(resolve_bind_host(Some("::1".to_string())).1);
        assert!(resolve_bind_host(Some("localhost".to_string())).1);
    }

    #[test]
    fn non_loopback_override_is_used_and_flagged() {
        assert_eq!(resolve_bind_host(Some("0.0.0.0".to_string())), ("0.0.0.0".to_string(), false));
        let (host, is_loopback) = resolve_bind_host(Some(" 192.168.1.5 ".to_string()));
        assert_eq!(host, "192.168.1.5");
        assert!(!is_loopback, "a LAN host must be flagged non-loopback so the warning fires");
    }
}
