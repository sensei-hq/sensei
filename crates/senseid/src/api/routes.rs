use axum::{
    routing::{get, post, put, delete},
    Router,
    response::Json,
    http::StatusCode,
};

use crate::api::state::AppState;

use crate::api::handlers::health;
use crate::api::handlers::workspace;
use crate::api::handlers::observatory;
use crate::api::handlers::sessions;
use crate::api::handlers::codebase;
use crate::api::handlers::libraries;
use crate::api::handlers::config;
use crate::api::handlers::query;
use crate::api::handlers::mcp;
use crate::api::handlers::logs;
use crate::api::handlers::gateway;
use crate::api::handlers::scan_events;
use crate::api::handlers::project_detail;
use crate::api::handlers::instruments;
use crate::api::handlers::verdicts;
use crate::api::handlers::mcp_servers as mcp_servers_handler;
use crate::api::handlers::gateway_routers;
use crate::api::handlers::gateway_chains;
use crate::api::handlers::gateway_image;
use crate::api::handlers::knowledge;
use crate::api::handlers::dojo;
use crate::api::handlers::corrections;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health
        .route("/health", get(health::health))
        .route("/api/watcher/status", get(health::watcher_status))
        .route("/api/watcher/unregister", axum::routing::post(health::watcher_unregister))
        // Scan events (SSE)
        .route("/api/scan/events", get(scan_events::scan_events_sse))
        // Gateway
        .route("/api/gateway/status", get(gateway::gateway_status))
        .route("/api/gateway/infer", post(gateway::infer))
        .route("/api/gateway/embed", post(gateway::embed))
        .route("/api/gateway/consensus", post(gateway::consensus))
        .route("/api/gateway/routers",                       get(gateway_routers::list_routers))
        .route("/api/gateway/routers/{id}/providers",        get(gateway_routers::router_providers))
        .route("/api/gateway/routers/{id}/models",           get(gateway_routers::router_models))
        .route("/api/gateway/routers/{id}/key",              post(gateway_routers::set_router_key).delete(gateway_routers::clear_router_key))
        .route("/api/gateway/models",                        get(gateway_routers::list_all_models))
        .route("/api/gateway/chains",                        get(gateway_chains::list_chains))
        .route("/api/gateway/chains/{id}/role",              put(gateway_chains::set_chain_role))
        .route("/api/gateway/chains/{id}/available-models",  get(gateway_chains::list_available_models))
        .route("/api/gateway/chains/{id}/models",            post(gateway_chains::add_chain_model))
        .route("/api/gateway/chains/{id}/models/{member_id}",           delete(gateway_chains::remove_chain_model))
        .route("/api/gateway/chains/{id}/models/{member_id}/move",      put(gateway_chains::move_chain_model))
        .route("/api/gateway/image/generate",                post(gateway_image::image_generate))
        // Repos (individual git repos)
        .route("/api/repos", get(workspace::list_projects).post(workspace::create_project))
        .route("/api/repos/sync-frontmatter", post(workspace::sync_readme_frontmatter))
        .route("/api/repos/{repo_id}", put(workspace::update_project).delete(workspace::delete_project))
        .route("/api/repos/{repo_id}/tags", post(workspace::add_project_tag))
        .route("/api/repos/{repo_id}/tags/{tag}", delete(workspace::remove_project_tag))
        .route("/api/repos/{repo_id}/summary", get(observatory::project_summary))
        .route("/api/repos/{repo_id}/exclude", post(workspace::exclude_project))
        // Exclusions
        .route("/api/exclusions", get(workspace::list_exclusions))
        .route("/api/exclusions/{path}", delete(workspace::remove_exclusion))
        // Projects (groups of 1+ repos)
        .route("/api/projects", get(observatory::list_solutions).post(observatory::create_solution))
        .route("/api/projects/merge", post(observatory::merge_projects))
        .route("/api/projects/{id}", put(observatory::update_solution).delete(observatory::delete_solution))
        .route("/api/projects/{id}/repos", get(project_detail::get_project_repos).post(observatory::add_solution_repo))
        .route("/api/projects/{id}/repos/{repo_id}", delete(observatory::remove_solution_repo))
        .route("/api/projects/{id}/tags", post(observatory::add_solution_tag))
        .route("/api/projects/{id}/tags/{tag}", delete(observatory::remove_solution_tag))
        // Project detail endpoints (multi-window)
        .route("/api/projects/{id}/ftr",             get(project_detail::get_project_ftr))
        .route("/api/projects/{id}/overview",        get(project_detail::get_project_overview))
        .route("/api/projects/{id}/drift",           get(project_detail::get_project_drift))
        .route("/api/projects/{id}/drift/scan",      post(project_detail::scan_project_doc_drift))
        .route("/api/projects/{id}/patterns",        get(project_detail::get_project_patterns))
        .route("/api/projects/{id}/libraries",       get(project_detail::get_project_libraries))
        .route("/api/projects/{id}/instruments",     get(project_detail::get_project_instruments))
        .route("/api/projects/{id}/mcp-tool-stats",  get(project_detail::get_project_mcp_tool_stats))
        .route("/api/projects/{id}/services",        get(project_detail::list_project_services))
        .route("/api/projects/{id}/services/{service_id}/scope",
               put(project_detail::set_project_service_scope))
        .route("/api/projects/{id}/memories",        get(project_detail::get_project_memories))
        .route("/api/projects/{id}/memory-batches",
               get(project_detail::list_memory_share_batches)
                 .post(project_detail::create_memory_share_batch))
        .route("/api/projects/{id}/memory-batches/{batch_id}",
               put(project_detail::decide_memory_share_batch))
        .route("/api/projects/{id}/recommendations", get(project_detail::get_project_recommendations))
        .route("/api/projects/{id}/recommendations/{rec_id}/accept",
               post(project_detail::accept_project_recommendation))
        .route("/api/projects/{id}/recommendations/{rec_id}/reject",
               post(project_detail::reject_project_recommendation))
        .route("/api/projects/{id}/impact",          get(project_detail::get_project_impact))
        .route("/api/projects/{id}/impact-verdicts",
               get(project_detail::list_impact_verdicts)
                 .post(project_detail::create_impact_verdict))
        .route("/api/projects/{id}/impact-verdicts/{verdict_id}",
               put(project_detail::decide_impact_verdict))
        .route("/api/projects/{id}/sessions",        get(project_detail::get_project_sessions))
        .route("/api/projects/{id}/project-deps",    get(project_detail::get_project_project_deps))
        .route("/api/projects/{id}/commands",        get(project_detail::get_project_commands))
        .route("/api/projects/{id}/library-version-conflicts", get(project_detail::get_project_library_version_conflicts))
        // Observatory chart data
        .route("/api/observatory/ftr-daily",             get(observatory::holistic_ftr_daily))
        .route("/api/observatory/tool-usage",            get(observatory::tool_usage))
        .route("/api/observatory/tool-signals",          get(observatory::tool_signals))
        .route("/api/observatory/tool-insights",         get(observatory::tool_insights))
        .route("/api/observatory/model-effectiveness",   get(observatory::model_effectiveness))
        .route("/api/observatory/today",                 get(observatory::observatory_today))
        .route("/api/observatory/ftr",                   get(observatory::observatory_ftr))
        .route("/api/insights",                          get(observatory::get_insights))
        .route("/api/projects/{id}/ftr-daily",           get(observatory::project_ftr_daily))
        .route("/api/projects/{id}/hotspots",            get(observatory::project_hotspots))
        .route("/api/projects/{id}/quality-signals",     get(observatory::project_quality_signals))
        .route("/api/projects/{id}/maturity",            get(observatory::project_maturity))
        .route("/api/corrections", get(corrections::list_corrections))
        .route("/api/projects/{id}/corrections", get(corrections::project_corrections))
        .route("/api/projects/{id}/teachings",           get(observatory::project_teachings))
        .route("/api/libs/{id}/usage",                   get(observatory::library_usage))
        // Indexing
        .route("/api/index", post(workspace::index_project))
        .route("/api/index/status", get(workspace::task_status))
        .route("/api/index/progress", get(workspace::index_progress_sse))
        // dirty_status removed — task queue handles incremental
        .route("/api/index/errors", get(workspace::list_index_errors))
        .route("/api/index/errors/{repo_id}", get(workspace::list_repo_index_errors))
        // Task queue (new)
        .route("/api/tasks/status", get(workspace::task_status))
        .route("/api/tasks/progress", get(workspace::task_progress_sse))
        // Graph
        .route("/api/graph/nodes", get(codebase::graph_nodes))
        .route("/api/graph/functions", get(codebase::search_functions))
        .route("/api/graph/types", get(codebase::search_types))
        .route("/api/graph/callers", get(codebase::fn_callers))
        .route("/api/graph/callees", get(codebase::fn_callees))
        .route("/api/graph/files", get(codebase::files_by_tag))
        .route("/api/graph/communities", post(codebase::detect_communities))
        .route("/api/graph/communities/info", get(codebase::community_info))
        .route("/api/graph/doc-drift", get(codebase::doc_drift))
        .route("/api/graph/call-flow", get(codebase::call_flow))
        // Project analysis
        .route("/api/projects/{id}/analyze", post(observatory::analyze_solution))
        .route("/api/transcripts/backfill", post(observatory::backfill_transcripts))
        .route("/api/projects/{id}/graph", get(observatory::solution_graph))
        .route("/api/projects/{id}/roles", get(observatory::solution_roles))
        // Folder mutations (setup wizard — Projects stage)
        .route("/api/folders/{id}", put(workspace::update_folder))
        // Libraries
        .route("/api/libs", get(libraries::list_libs))
        .route("/api/libs/index", post(libraries::index_lib))
        .route("/api/libs/docs", get(libraries::search_lib_docs))
        .route("/api/libs/{name}/docs", get(libraries::get_lib_docs))
        .route("/api/libs/versions", get(libraries::get_dep_versions))
        // Instruments (MCP registry — setup wizard Instruments stage)
        .route("/api/instruments", get(instruments::list_instruments))
        // Instruments Replay — tool-call usage verdicts (#90)
        .route("/api/instruments/verdicts/classify", post(verdicts::classify))
        .route("/api/instruments/verdicts", get(verdicts::list))
        .route("/api/instruments/verdicts/summary", get(verdicts::summary))
        // Instruments — discovered MCP servers (#84 T2 Slice A)
        .route("/api/instruments/mcp-servers", get(mcp_servers_handler::list))
        .route("/api/instruments/mcp-servers/{id}/enabled", put(mcp_servers_handler::set_enabled))
        .route("/api/instruments/mcp-servers/{id}/tools", get(mcp_servers_handler::get_tools))
        .route("/api/instruments/mcp-servers/refresh", post(mcp_servers_handler::refresh))
        .route("/api/instruments/tools-health", get(crate::api::handlers::tools_health::grid))
        .route("/api/instruments/tools/refresh", post(crate::api::handlers::tools_health::refresh))
        // Unified query (desktop/MCP)
        .route("/api/query", post(query::unified_query))
        // MCP tool proxy
        .route("/api/mcp/tools", get(mcp::mcp_list_tools))
        .route("/api/mcp/call", post(mcp::mcp_call_tool))
        // Marketplace install (legacy — prefer /api/install endpoints)
        .route("/api/marketplace/install", post(config::marketplace_install))
        // Assistants detection & configuration
        .route("/api/assistants/detect", get(config::assistant_detect))
        .route("/api/assistants/families", get(config::assistant_detect_families))
        .route("/api/assistants/configure", post(config::assistant_configure))
        .route("/api/assistants/remove", post(config::assistant_remove))
        .route("/api/assistants/health", get(config::assistants_health))
        .route("/api/assistants/resolve", post(config::assistants_resolve))
        // Installer — hooks, skills, commands, install/remove
        .route("/api/install", post(config::install_all))
        .route("/api/install/hooks", post(config::install_hooks))
        .route("/api/install/item", post(config::install_single_item))
        .route("/api/install/item/remove", post(config::remove_single_item))
        .route("/api/install/catalog", get(config::get_catalog))
        .route("/api/install/installed", get(config::list_installed_items))
        .route("/api/install/installed/{name}/enabled", put(config::set_installed_enabled))
        .route("/api/remove", post(config::remove_all))
        // Config (user preferences)
        .route("/api/config", get(config::get_config).put(config::set_config_handler))
        .route("/api/config/{key}", get(config::get_config_key).delete(config::delete_config_key))
        // Sessions
        .route("/api/sessions", get(sessions::get_sessions_stub).post(sessions::create_session))
        .route("/api/sessions/{id}", get(sessions::get_session).put(sessions::update_session_handler))
        .route("/api/sessions/{id}/tool-timeline", get(sessions::get_session_tool_timeline))
        .route("/api/sessions/{id}/replay", get(sessions::get_session_replay))
        // Patterns
        .route("/api/patterns/{project}/detect", post(codebase::detect_patterns))
        .route("/api/patterns/{project}", get(codebase::list_patterns))
        .route("/api/patterns/{project}/match", get(codebase::match_pattern_handler))
        .route("/api/patterns/{project}/for/{symbol}", get(codebase::pattern_for_symbol))
        .route("/api/patterns/{project}/duplicates", get(codebase::find_duplicates_handler))
        .route("/api/patterns/{project}/conventions", get(codebase::project_conventions_handler))
        // Hook event ingestion (from sensei-hook.ts)
        .route("/hook/event", post(sessions::ingest_hook_event))
        // Structured logging (remote writers: CLI, MCP, app)
        .route("/api/logs", post(logs::ingest_log))
        // Metrics
        .route("/api/metrics/{project}", get(observatory::get_metrics))
        // Workflow state
        .route("/api/state/{project}", get(sessions::get_workflow_state).put(sessions::update_workflow_state))
        // Scan
        .route("/api/scan", post(workspace::scan_folder))
        .route("/api/scan/suggestions", get(workspace::scan_suggestions))
        .route("/api/scan/roots", get(workspace::scan_roots).post(workspace::add_watch_root))
        .route("/api/scan/roots/{id}", put(workspace::update_watch_root).delete(workspace::delete_watch_root))
        // Backfill embeddings for already-indexed nodes (EmbedNodes per folder)
        .route("/api/embed/backfill", post(workspace::backfill_embeddings))
        // Knowledge plane
        .route("/api/knowledge/memories",                get(knowledge::list_memories).post(knowledge::save_memory))
        .route("/api/knowledge/memories/{id}",           get(knowledge::get_memory))
        .route("/api/knowledge/memories/{id}/promote",   post(knowledge::promote_memory))
        .route("/api/knowledge/memories/{id}/generalise", post(knowledge::generalise_memory))
        .route("/api/knowledge/promotion-candidates",    get(knowledge::promotion_candidates))
        .route("/api/knowledge/proposals",               post(knowledge::propose_memory))
        .route("/api/knowledge/proposals/{id}/accept",   post(knowledge::accept_proposal))
        .route("/api/knowledge/proposals/{id}/reject",   post(knowledge::reject_proposal))
        .route("/api/knowledge/outcomes",                post(knowledge::record_outcomes))
        .route("/api/knowledge/context",                 get(knowledge::get_context))
        .route("/api/knowledge/rules",                   get(knowledge::get_rules))
        .route("/api/knowledge/rules/materialize",       post(knowledge::materialize_rules))
        .route("/api/knowledge/rules/consolidate",       post(knowledge::consolidate_rules))
        .route("/api/knowledge/rules/consolidated",      get(knowledge::get_consolidated))
        .route("/api/knowledge/rules/consolidate/{id}/approve", post(knowledge::approve_consolidated))
        // Federation sources (hive-mind)
        .route("/api/knowledge/sources",                 get(knowledge::list_sources).post(knowledge::create_source))
        .route("/api/knowledge/sources/{id}",            delete(knowledge::delete_source))
        .route("/api/knowledge/sources/{id}/sync",       post(knowledge::sync_source))
        .route("/api/knowledge/sources/{id}/status",     get(knowledge::source_status))
        // Dōjō connections (memberships)
        .route("/api/dojo/memberships",                  get(dojo::list_memberships).post(dojo::create_membership))
        // Stop
        .route("/stop", post(workspace::stop))
        .with_state(state)
}

/// Build the **degraded-mode** router used when the daemon can boot but
/// can't connect to PostgreSQL.
///
/// The full router requires `AppState` (which carries `PgStore`); if the
/// connect fails we still want to keep the port open so the frontend's
/// health page can surface the failure rather than seeing connection
/// refused. This router:
///
///   * Serves `/health` and `/api/health` via the same probe as full mode
///     (`sensei_bootstrap::check`) — the DB component will report its
///     real state (not_ok with detail) on every poll.
///   * Returns 503 with a structured payload on every other route so a
///     misguided client (e.g. a wizard mid-step) sees a clear "daemon
///     degraded" error instead of confusing partial responses.
///
/// The daemon exits this mode only on restart — once DB is fixed and the
/// daemon comes back up, `start_server` takes the full-mode path.
pub fn create_degraded_router(db_url: String, error: String) -> Router {
    let fallback_msg = format!(
        "Daemon is running in degraded mode — PostgreSQL is unreachable. URL: {}. Error: {}. Fix the database and restart the daemon.",
        db_url, error
    );
    Router::new()
        .route("/health", get(health::health))
        .route("/api/health", get(health::health))
        .fallback(move || {
            let msg = fallback_msg.clone();
            async move {
                (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({
                    "ok": false,
                    "reason": "db_unavailable",
                    "message": msg,
                })))
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    use crate::tasks::queue::TaskQueue;
    use crate::api::state::SharedState;

    async fn test_app() -> (Router, AppState) {
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let (event_tx, _) = tokio::sync::broadcast::channel(256);
        let state = Arc::new(SharedState {
            task_queue: Arc::new(TaskQueue::new()),
            pg: crate::db::pg_store::PgStore::connect_test().await.unwrap(),
            gateway,
            event_tx,
            breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        });
        let router = create_router(state.clone());
        (router, state)
    }

    #[tokio::test]
    async fn health_check() {
        let (app, _) = test_app().await;
        let resp = app.oneshot(
            Request::builder().uri("/health").body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // /health now returns sensei_bootstrap::HealthPayload (Phase 1b).
        assert!(json["version"].is_string());
        assert_eq!(json["packageManager"]["id"], "homebrew");
        assert!(json["components"].is_array());
        assert_eq!(json["components"].as_array().unwrap().len(), 5);
        let status = json["status"].as_str().unwrap();
        assert!(
            matches!(status, "ok" | "needs-action" | "checking" | "resolving"),
            "unexpected status {status}"
        );
    }

    #[tokio::test]
    async fn create_and_list_repos() {
        let (app, _) = test_app().await;

        // Create
        let resp = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/repos")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"repoId":"test","path":"/tmp/test"}"#))
                .unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // List
        let resp = app.oneshot(
            Request::builder().uri("/api/repos").body(Body::empty()).unwrap()
        ).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let repos: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(repos.iter().any(|r| r["name"] == "test"), "created repo should be in list");
    }

    #[tokio::test]
    async fn delete_project_returns_ok() {
        let (app, state) = test_app().await;
        // Register a repo via PgStore
        let root_id = state.pg.add_watch_root("/_test/del_proj", "test", &serde_json::json!([])).await.unwrap();
        state.pg.upsert_repo(&root_id, "x", "/_test/del_proj/x").await.unwrap();
        let resp = app.oneshot(
            Request::builder().method("DELETE").uri("/api/repos/x").body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn create_and_list_solutions() {
        let (app, _) = test_app().await;

        let resp = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"Acme","repos":[]}"#))
                .unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["id"].is_string());

        let resp = app.oneshot(
            Request::builder().uri("/api/projects").body(Body::empty()).unwrap()
        ).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let solutions: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(solutions.iter().any(|s| s["name"] == "Acme"), "Acme project should be in list");
    }

    #[tokio::test]
    async fn index_project_via_api() {
        let (app, _) = test_app().await;

        // Create a temp repo with a Python file
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("hello.py"), "def greet(name):\n    return f'hi {name}'\n").unwrap();

        let body = serde_json::json!({
            "repoId": "test-repo",
            "repoPath": dir.path().to_string_lossy(),
        });

        let resp = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/index")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["queued"], true);

        // Note: indexing happens async via worker — graph won't have data in unit test
        // (no worker spawned in test_app). The e2e_server test covers the full flow.

        // Verify queue status endpoint works
        let resp = app.oneshot(
            Request::builder()
                .uri("/api/index/status")
                .body(Body::empty())
                .unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["queue"].is_object());
    }

    /// #60 Part A — handler-level scoping: index a 2-folder project (root + child
    /// with a function in the child) and assert that search_functions and
    /// project_summary scoped to the PROJECT NAME include the child-folder data.
    #[tokio::test]
    async fn scoped_search_includes_child_folders() {
        let (app, state) = test_app().await;

        // Use a unique suffix to avoid conflicts with parallel test runs on the shared DB.
        let uniq = uuid::Uuid::new_v4().simple().to_string();
        let proj_name = format!("ScopeTestProject-{}", &uniq[..8]);
        let root_path = format!("/_test/scope_{}", &uniq[..8]);
        let root_name = format!("scope-root-{}", &uniq[..8]);
        let child_name = format!("scope-child-{}", &uniq[..8]);
        let fn_name = format!("scope_child_fn_{}", &uniq[..8]);

        // Setup: root + child folders registered under a project.
        let root_id = state.pg.add_watch_root(&root_path, "test", &serde_json::json!([])).await.unwrap();
        let root_fid = state.pg.upsert_repo(&root_id, &root_name, &format!("{}/{}", root_path, root_name)).await.unwrap();
        let child_fid = state.pg.upsert_subfolder(
            &root_id, &child_name,
            &format!("{}/{}", root_name, child_name),
            &format!("{}/{}/{}", root_path, root_name, child_name),
            Some(&root_fid), None,
        ).await.unwrap();

        let proj_id = state.pg.create_project(&proj_name, None, None).await.unwrap();
        state.pg.set_folder_project(&root_fid, &proj_id, "backend", None).await.unwrap();
        state.pg.set_folder_project(&child_fid, &proj_id, "backend", None).await.unwrap();

        // Insert a function only in the child folder.
        state.pg.upsert_node(
            &child_fid, "function", &fn_name, "src/lib.rs",
            None, Some(&format!("fn {}()", fn_name)), Some(1), Some(5),
        ).await.unwrap();

        // GET /api/graph/functions?repoId=<proj>&q=<fn>
        let resp = app.clone().oneshot(
            Request::builder()
                .uri(format!("/api/graph/functions?repoId={}&q={}", proj_name, fn_name))
                .body(Body::empty())
                .unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "search_functions should succeed");
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let results: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(
            results.iter().any(|r| r["name"] == fn_name),
            "child-folder function must appear when searching by project name; got: {:?}", results
        );

        // GET /api/repos/{repo_id}/summary — scoped counts should include child.
        let resp = app.clone().oneshot(
            Request::builder()
                .uri(format!("/api/repos/{}/summary", proj_name))
                .body(Body::empty())
                .unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "project_summary should succeed");
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let summary: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let fn_count = summary["functions"].as_i64().unwrap_or(0);
        assert!(fn_count >= 1, "summary must count child-folder function; got functions={}", fn_count);
    }

    #[tokio::test]
    async fn assistants_health_endpoint_returns_status_and_adapters() {
        let (app, _) = test_app().await;
        let resp = app.oneshot(
            Request::builder().uri("/api/assistants/health").body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["status"].is_string());
        assert!(json["adapters"].is_array());
    }

    #[tokio::test]
    async fn scan_folder_finds_repos() {
        let (app, _) = test_app().await;

        // Create a temp dir with a "repo" (has .git)
        let root = tempfile::TempDir::new().unwrap();
        let repo = root.path().join("my-project");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::write(repo.join("package.json"), r#"{"name":"test","dependencies":{"express":"4"}}"#).unwrap();

        let resp = app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/scan")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"root": root.path().to_string_lossy()}).to_string()))
                .unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        // scan_folder returns {"ok": true, "scanning": true} — scan runs async in background
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(result["ok"], true);
        assert_eq!(result["scanning"], true);
    }
}
