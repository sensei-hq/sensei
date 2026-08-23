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
use crate::api::handlers::model_provisioning;
use crate::api::handlers::knowledge;
use crate::api::handlers::planner;
use crate::api::handlers::checker;
use crate::api::handlers::dojo;
use crate::api::handlers::preferences;
use crate::api::handlers::share_review;
use crate::api::handlers::review;
use crate::api::handlers::upgrades;
use crate::api::handlers::corrections;
use crate::api::handlers::runs;
use crate::api::handlers::identity;
use crate::api::handlers::stance;
use crate::api::handlers::playbook;
use crate::api::handlers::metrics;

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
        // On-demand model provisioning: pull + coldboot a local GGUF chat model
        // behind the embedded-llama router. POST starts (or joins) a pull; GET
        // snapshots every tracked model's phase. Both degrade cleanly when the
        // daemon lacks the embedded engine (501 / empty list).
        .route("/api/gateway/models/provision/status",       get(model_provisioning::provision_status))
        .route("/api/gateway/models/{id}/provision",         post(model_provisioning::provision_model))
        // Repos (individual git repos)
        .route("/api/repos", get(workspace::list_projects).post(workspace::create_project))
        .route("/api/repos/sync-frontmatter", post(workspace::sync_readme_frontmatter))
        .route("/api/repos/{repo_id}", put(workspace::update_project).delete(workspace::delete_project))
        .route("/api/repos/{repo_id}/tags", post(workspace::add_project_tag))
        .route("/api/repos/{repo_id}/tags/{tag}", delete(workspace::remove_project_tag))
        .route("/api/repos/{repo_id}/summary", get(observatory::project_summary))
        .route("/api/repos/{repo_id}/exclude", post(workspace::exclude_project))
        // Exclusions live per watch root in `folders_to_watch.excluded` — set them
        // via PUT /api/scan/roots/{id} { excluded:[...] } (prunes added subtrees,
        // re-scans removed ones). No standalone /api/exclusions endpoint.
        // Projects (groups of 1+ repos)
        .route("/api/projects", get(observatory::list_solutions).post(observatory::create_solution))
        .route("/api/projects/merge", post(observatory::merge_projects))
        .route("/api/projects/{id}", put(observatory::update_solution).delete(observatory::delete_solution))
        .route("/api/projects/{id}/repos", get(project_detail::get_project_repos).post(observatory::add_solution_repo))
        .route("/api/projects/{id}/repos/{repo_id}", delete(observatory::remove_solution_repo))
        .route("/api/projects/{id}/tags", post(observatory::add_solution_tag))
        .route("/api/projects/{id}/tags/{tag}", delete(observatory::remove_solution_tag))
        // Git author identity for a folder + its owning project (MCP
        // get_user_for_project) — who is doing the work, resolved local→global.
        .route("/api/user", get(identity::get_user))
        // Effective behavioural stance for a folder (autonomy · sharing · review)
        // — HOW a run behaves, resolved on the sensei.scopes ladder for the git
        // user. Complements /api/knowledge/rules (WHAT a run may do).
        .route("/api/stance", get(stance::get_stance).post(stance::set_stance))
        // Project detail endpoints (multi-window)
        .route("/api/projects/{id}/ftr",             get(project_detail::get_project_ftr))
        .route("/api/projects/{id}/icon",            get(project_detail::get_project_icon))
        .route("/api/projects/{id}/overview",        get(project_detail::get_project_overview))
        .route("/api/projects/{id}/drift",           get(project_detail::get_project_drift))
        .route("/api/projects/{id}/drift/scan",      post(project_detail::scan_project_doc_drift))
        .route("/api/projects/{id}/coverage/backfill", post(project_detail::coverage_backfill))
        .route("/api/projects/{id}/health",           get(project_detail::get_project_health))
        .route("/api/projects/{id}/correlations",     get(project_detail::get_metric_correlations))
        .route("/api/metrics/correlations",          get(project_detail::get_portfolio_correlations))
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
        // P-A: rule-class accept that MATERIALIZES a governance rule (+ preview).
        .route("/api/projects/{id}/recommendations/{rec_id}/preview",
               get(project_detail::preview_recommendation_materialization))
        .route("/api/projects/{id}/recommendations/{rec_id}/materialize",
               post(project_detail::materialize_recommendation))
        .route("/api/projects/{id}/impact",          get(project_detail::get_project_impact))
        .route("/api/projects/{id}/impact-verdicts",
               get(project_detail::list_impact_verdicts)
                 .post(project_detail::create_impact_verdict))
        .route("/api/projects/{id}/impact-verdicts/{verdict_id}",
               put(project_detail::decide_impact_verdict))
        .route("/api/projects/{id}/sessions",        get(project_detail::get_project_sessions))
        .route("/api/projects/{id}/project-deps",    get(project_detail::get_project_project_deps))
        .route("/api/projects/{id}/commands",        get(project_detail::get_project_commands))
        // Metrics (Phase 7): latest-per-metric + trend + health for a project, and
        // one metric's series at a grain (?grain=daily|weekly|monthly|quarterly).
        .route("/api/projects/{id}/metrics",         get(metrics::get_project_metrics))
        .route("/api/projects/{id}/metrics/{key}",   get(metrics::get_project_metric_series))
        // Datapoint→sessions drill-down: the measurable sessions behind one daily
        // metric point (?day=YYYY-MM-DD).
        .route("/api/projects/{id}/metrics/{key}/sessions",
               get(metrics::get_project_metric_day_sessions))
        .route("/api/projects/{id}/tools",           get(metrics::get_project_tools))
        // G10: user-scope capability→preferred-tool bias for get_commands.
        .route("/api/preferences/commands",          get(project_detail::get_command_preferences).put(project_detail::set_command_preference))
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
        .route("/api/index/doctor", get(workspace::index_doctor))
        .route("/api/index/progress", get(workspace::index_progress_sse))
        // dirty_status removed — task queue handles incremental
        .route("/api/index/errors", get(workspace::list_index_errors))
        .route("/api/index/errors/{repo_id}", get(workspace::list_repo_index_errors))
        // Task queue (new)
        .route("/api/tasks/status", get(workspace::task_status))
        .route("/api/tasks/progress", get(workspace::task_progress_sse))
        // Background-task visibility (#96): scheduler registry + last-run times
        .route("/api/tasks/scheduled", get(crate::api::handlers::scheduled_tasks::scheduled))
        // Graph
        .route("/api/graph/nodes", get(codebase::graph_nodes))
        .route("/api/graph/functions", get(codebase::search_functions))
        .route("/api/graph/types", get(codebase::search_types))
        .route("/api/graph/callers", get(codebase::fn_callers))
        .route("/api/graph/callees", get(codebase::fn_callees))
        .route("/api/graph/files", get(codebase::files_by_tag))
        .route("/api/graph/communities", post(codebase::detect_communities))
        .route("/api/graph/communities/info", get(codebase::community_info))
        .route("/api/graph/{repoId}/tree", get(codebase::graph_tree))
        .route("/api/graph/doc-drift", get(codebase::doc_drift))
        .route("/api/graph/call-flow", get(codebase::call_flow))
        // Project analysis
        .route("/api/projects/{id}/analyze", post(observatory::analyze_solution))
        .route("/api/projects/{id}/process/analyze", post(observatory::analyze_process))
        .route("/api/projects/{id}/backfill", post(observatory::backfill_project_sessions))
        .route("/api/transcripts/backfill", post(observatory::backfill_transcripts))
        .route("/api/metrics/backfill", post(observatory::backfill_metrics))
        .route("/api/projects/{id}/graph", get(observatory::solution_graph))
        .route("/api/projects/{id}/roles", get(observatory::solution_roles))
        // Folder mutations (setup wizard — Projects stage)
        .route("/api/folders/{id}", put(workspace::update_folder))
        // Manual rename repair — `sensei folder remap <old> <new>`
        .route("/api/folders/remap", post(workspace::remap_folder_endpoint))
        // Review-depth gate (E1) — classify a change's required review depth
        .route("/api/review/risk-class", post(review::risk_class))
        // Libraries
        .route("/api/libs", get(libraries::list_libs))
        .route("/api/libs/index", post(libraries::index_lib))
        .route("/api/libs/docs", get(libraries::search_lib_docs))
        .route("/api/libs/{name}/docs", get(libraries::get_lib_docs))
        // Library-provided capabilities (workstream D)
        .route("/api/libs/{name}/skills", get(libraries::list_library_skills))
        .route("/api/libs/{name}/skills/{focus}", get(libraries::get_library_skill))
        .route("/api/libs/{name}/agents", get(libraries::list_library_agents))
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
        .route("/api/assistants/upgrade", post(config::assistant_upgrade))
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
        // Relay runs (P3.2 observability + P3.8 run-control create)
        .route("/api/runs", get(runs::list_runs).post(runs::create_run))
        // Static segments before the `{id}` route so they aren't swallowed as an id.
        .route("/api/runs/pause", post(runs::pause_run))
        .route("/api/runs/plan", post(runs::register_plan))
        .route("/api/runs/{id}", get(runs::get_run))
        // Automated-run coordinator contract (AR-3): flip a task's state, mark the
        // run terminal. More specific than `/api/runs/{id}`, so ordering is safe.
        .route("/api/runs/{id}/tasks/{task_id}", post(runs::update_task_status))
        .route("/api/runs/{id}/outcome", post(runs::report_run_outcome))
        .route("/api/runs/{id}/nudges", get(runs::get_pending_nudges))
        // Front-door intake: axes -> playbook recommendation
        .route("/api/playbook/guide", get(playbook::get_intake_guide))
        .route("/api/playbook/recommend", post(playbook::recommend_playbook))
        // §9 learning loop: accept path + model-stats read
        .route("/api/playbook/rule-proposals", get(playbook::list_rule_proposals))
        .route("/api/playbook/rule/{id}/accept", post(playbook::accept_rule))
        .route("/api/playbook/model-stats", get(playbook::model_stats))
        // Patterns
        .route("/api/patterns/{project}/detect", post(codebase::detect_patterns))
        .route("/api/patterns/{project}", get(codebase::list_patterns))
        .route("/api/patterns/{project}/match", get(codebase::match_pattern_handler))
        .route("/api/patterns/{project}/for/{symbol}", get(codebase::pattern_for_symbol))
        .route("/api/patterns/{project}/duplicates", get(codebase::find_duplicates_handler))
        .route("/api/patterns/{project}/conventions", get(codebase::project_conventions_handler))
        // Hook event ingestion (from sensei-hook.ts)
        .route("/hook/event", post(sessions::ingest_hook_event))
        // Hook gate (relay-engine feature B): a PreToolUse hook asks whether a
        // tool may proceed; the daemon raises a phone gate and blocks for the
        // answer. Fail-open (allow) unless a human explicitly denies. Gating is
        // OFF unless SENSEI_RELAY_GATE_TOOLS names the tool.
        .route("/hook/gate", post(sessions::hook_gate))
        // Intake nudge hook (front-door intake plan, T5): non-blocking, informs
        // whether a session has no confirmed playbook run yet. Not wired to any
        // registered plugin hook by default — see marketplace/plugins/sensei/hooks.
        .route("/hook/nudge", post(sessions::hook_nudge))
        // Structured logging: POST ingests (CLI, MCP, app); GET reads for the
        // Observatory · Logs screen.
        .route("/api/logs", post(logs::ingest_log).get(logs::get_logs))
        // Metrics
        // Phase 7 registry catalog — static `registry` wins over the `{project}`
        // param below (matchit precedence, same as /api/runs/plan vs /{id}).
        .route("/api/metrics/registry", get(metrics::get_metrics_registry))
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
        .route("/api/knowledge/memories/{id}/archive",   post(knowledge::archive_memory))
        .route("/api/knowledge/memories/{id}/reinforce", post(knowledge::reinforce_memory))
        .route("/api/knowledge/memories/{id}/challenge", post(knowledge::challenge_memory))
        .route("/api/knowledge/memories/{id}/dismiss",   post(knowledge::dismiss_memory))
        .route("/api/knowledge/memories/{id}/merge",     post(knowledge::merge_memory))
        .route("/api/knowledge/promotion-candidates",    get(knowledge::promotion_candidates))
        .route("/api/knowledge/proposals",               post(knowledge::propose_memory))
        .route("/api/knowledge/proposals/{id}/accept",   post(knowledge::accept_proposal))
        .route("/api/knowledge/proposals/{id}/reject",   post(knowledge::reject_proposal))
        .route("/api/knowledge/outcomes",                post(knowledge::record_outcomes))
        .route("/api/knowledge/context",                 get(knowledge::get_context))
        .route("/api/planner/generate",                  post(planner::generate_plan))
        .route("/api/checkers/run",                      post(checker::run_checkers))
        .route("/api/knowledge/rules",                   get(knowledge::get_rules))
        .route("/api/knowledge/constitution",            get(knowledge::get_constitution))
        .route("/api/knowledge/rules/materialize",       post(knowledge::materialize_rules))
        .route("/api/knowledge/rules/consolidate",       post(knowledge::consolidate_rules))
        .route("/api/knowledge/rules/consolidated",      get(knowledge::get_consolidated))
        .route("/api/knowledge/rules/consolidate/{id}/approve", post(knowledge::approve_consolidated))
        // Federation sources (Dōjō rules tenant path /v1/t/{origin}/{org}/rules)
        .route("/api/knowledge/sources",                 get(knowledge::list_sources).post(knowledge::create_source))
        .route("/api/knowledge/sources/{id}",            delete(knowledge::delete_source))
        .route("/api/knowledge/sources/{id}/sync",       post(knowledge::sync_source))
        .route("/api/knowledge/sources/{id}/status",     get(knowledge::source_status))
        // Collective sharing preferences (Preferences → Sharing, C9)
        .route("/api/preferences/collective",            get(preferences::get_collective).put(preferences::put_collective))
        // Dōjō connections (memberships)
        .route("/api/dojo/memberships",                  get(dojo::list_memberships).post(dojo::create_membership))
        .route("/api/dojo/memberships/{id}/orgs",        put(dojo::set_membership_orgs))
        // R3 infer-at-detect auto-bind: suggestion (read-only) + confirm-bind
        .route("/api/projects/{id}/dojo-suggestion",     get(dojo::project_binding_suggestion))
        .route("/api/projects/{id}/dojo-binding",        post(dojo::bind_project_to_membership))
        // Dōjō upstream share review (C6)
        .route("/api/share-review/next-batch",           get(share_review::next_batch))
        .route("/api/share-review/{batch}/publish",      post(share_review::publish_batch))
        // Dōjō downstream inbox / upgrades (C7)
        .route("/api/upgrades",                          get(upgrades::list))
        .route("/api/upgrades/{id}/apply",               post(upgrades::apply))
        .route("/api/upgrades/{id}/mute",                post(upgrades::mute))
        .route("/api/upgrades/{id}/pin",                 post(upgrades::pin))
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
// Test gates are blocking `std::sync::Mutex` held across awaits ON PURPOSE —
// see `crate::tasks::test_support::TestGate` for why an async mutex loses
// wakeups across per-test runtimes. One allow per test module, not per site.
#[allow(clippy::await_holding_lock)]
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
            provisioning: None,
        });
        let router = create_router(state.clone());
        (router, state)
    }

    /// Send one request through the real router; returns `(status, json_body)`.
    async fn req(app: Router, method: &str, uri: &str, body: Option<serde_json::Value>) -> (StatusCode, serde_json::Value) {
        let builder = Request::builder().method(method).uri(uri);
        let request = match body {
            Some(v) => builder.header("content-type", "application/json").body(Body::from(v.to_string())).unwrap(),
            None => builder.body(Body::empty()).unwrap(),
        };
        let resp = app.oneshot(request).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    /// `/hook/event` must answer with a decodable JSON body. The MCP proxy
    /// (`crates/mcp` `daemon_result`) decodes every 2xx as JSON, so a bare
    /// `StatusCode::OK` (empty body) makes `log_event` report "unreadable
    /// success response" and silently drop the capture event.
    #[tokio::test]
    async fn hook_event_returns_decodable_json_ack() {
        let (app, _state) = test_app().await;
        let (status, body) = req(
            app,
            "POST",
            "/hook/event",
            Some(serde_json::json!({
                "hook_event_name": "PostToolUse",
                "session_id": "t-hook-ack",
                "tool_name": "Bash",
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body["ok"], true,
            "empty 2xx body breaks the MCP JSON decoder that log_event relies on",
        );
    }

    /// Seed an ACTIVE `sensei.metrics` registry row for a metrics-endpoint test and
    /// return its id. `type`/`direction` are the enum text values; the facets are
    /// fixed known strings so assertions have a stable target. `effective_from`
    /// defaults to `current_date`, so the row is active today. Cleaned up by caller.
    async fn seed_metric(
        pg: &crate::db::pg_store::PgStore, key: &str, mtype: &str, direction: &str,
    ) -> uuid::Uuid {
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.metrics
                (key, name, description, family, type, direction, purpose, how_to_read, formula, task_name)
             VALUES ($1, $1, 'test metric', 'quality'::sensei.metric_family, $2::sensei.metric_type,
                     $3::sensei.metric_direction, 'test purpose', 'test how', 'test formula', 'session_outcomes')
             RETURNING id",
        )
        .bind(key).bind(mtype).bind(direction)
        .fetch_one(pg.pool()).await.unwrap();
        row.0
    }

    /// 7.1 `GET /api/metrics/registry` returns the active catalog: 200 + a non-empty
    /// array in which EVERY row carries the self-describing facets (`purpose` AND
    /// `direction`) — the endpoint serves the daemon-owned registry, not a stub.
    #[tokio::test]
    async fn get_metrics_registry_endpoint() {
        let (app, state) = test_app().await;
        let key = format!("_test:reg:{}", uuid::Uuid::new_v4());
        let mid = seed_metric(&state.pg, &key, "pct", "higher_better").await;

        let (st, body) = req(app, "GET", "/api/metrics/registry", None).await;
        assert_eq!(st, StatusCode::OK);
        let metrics = body["metrics"].as_array().expect("metrics is an array");
        assert!(!metrics.is_empty(), "the active registry is non-empty");
        for m in metrics {
            assert!(m["purpose"].as_str().is_some_and(|s| !s.is_empty()),
                "every metric carries a purpose: {m}");
            assert!(m["direction"].as_str().is_some_and(|s| !s.is_empty()),
                "every metric carries a direction: {m}");
        }
        let ours = metrics.iter().find(|m| m["key"].as_str() == Some(key.as_str()))
            .expect("seeded metric is in the active registry");
        assert_eq!(ours["direction"], "higher_better");
        assert_eq!(ours["purpose"], "test purpose");

        sqlx_core::query::query("DELETE FROM sensei.metrics WHERE id = $1")
            .bind(mid).execute(state.pg.pool()).await.unwrap();
    }

    /// 7.2 `GET /api/projects/{id}/metrics`: latest-per-metric + facets + trend +
    /// retired-metric exclusion (a retired metric's durable rows must NOT render).
    /// Also pins the never-fabricate cases — a project with no rows is honest-empty
    /// (200 []), an unknown project is a 404.
    #[tokio::test]
    async fn get_project_metrics_endpoint() {
        let (app, state) = test_app().await;
        let uniq = uuid::Uuid::new_v4();
        let pid = state.pg.create_project(&format!("_test:pme:{uniq}"), None, None).await.unwrap();
        // Repo-grain (_v2): `project_health` is a SHARED registry metric — a
        // NULL-repository write is unique only per (metric,day,grain), so concurrent
        // test projects collide on the fixed `w2` day. Pin its durable row to a
        // per-test repository so the retirement-exclusion assertion can't be poisoned
        // by another run's write.
        let (rid,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, $1) RETURNING id")
            .bind(format!("test/{uniq}")).fetch_one(state.pg.pool()).await.unwrap();

        // Base metric A (ratio) across TWO ISO weeks → weekly trend has prior/delta.
        let key_a = format!("_test:pme:{uniq}:cov");
        let mid_a = seed_metric(&state.pg, &key_a, "ratio", "higher_better").await;
        let w1 = chrono::NaiveDate::from_ymd_opt(2020, 1, 6).unwrap();  // Monday
        let w2 = chrono::NaiveDate::from_ymd_opt(2020, 1, 13).unwrap(); // next Monday
        state.pg.upsert_project_metric(&mid_a, &pid, None, None, w1, "daily", 0.5,
            &serde_json::json!({"numerator": 1, "denominator": 2}), "measured").await.unwrap();
        state.pg.upsert_project_metric(&mid_a, &pid, None, None, w2, "daily", 0.75,
            &serde_json::json!({"numerator": 3, "denominator": 4}), "measured").await.unwrap();

        // Base metric B (ratio) — a SINGLE week → no prior period (trend is null).
        let key_b = format!("_test:pme:{uniq}:dup");
        let mid_b = seed_metric(&state.pg, &key_b, "ratio", "lower_better").await;
        state.pg.upsert_project_metric(&mid_b, &pid, None, None, w1, "daily", 0.25,
            &serde_json::json!({"numerator": 1, "denominator": 4}), "measured").await.unwrap();

        // A RETIRED metric with a durable daily row: project_health carries a past
        // effective_until in the registry (retirement is in-place, never hand-delete a
        // row), so its stored value must NOT render as an active card — asserted below.
        let (health_mid,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.metrics WHERE key = 'project_health'")
            .fetch_one(state.pg.pool()).await.expect("project_health seeded in registry");
        state.pg.upsert_project_metric_repo(&health_mid, &pid, Some(&rid), "user", None, None,
            None, None, w2, "daily", 82.0,
            &serde_json::json!({"components": 2}), "measured").await.unwrap();

        let (st, body) = req(app.clone(), "GET", &format!("/api/projects/{pid}/metrics"), None).await;
        assert_eq!(st, StatusCode::OK, "{body}");
        let metrics = body["metrics"].as_array().expect("metrics array");

        let a = metrics.iter().find(|m| m["metric"].as_str() == Some(key_a.as_str()))
            .expect("metric A present");
        assert_eq!(a["value"], 0.75, "latest daily value for A");
        assert_eq!(a["purpose"], "test purpose", "facet: purpose attached");
        assert_eq!(a["direction"], "higher_better", "facet: direction attached");
        assert_eq!(a["how_to_read"], "test how", "facet: how_to_read attached");
        // Trend attached where available: A has two weekly periods → prior/delta.
        assert_eq!(a["prior"].as_f64(), Some(0.5), "A weekly prior (Σnum/Σden of week 1)");
        assert_eq!(a["delta"].as_f64(), Some(0.25), "A weekly delta = value - prior");

        let b = metrics.iter().find(|m| m["metric"].as_str() == Some(key_b.as_str()))
            .expect("metric B present");
        assert_eq!(b["value"], 0.25);
        // Honest-null trend: B has a single week → prior/delta null, never a fake 0.
        assert!(b["prior"].is_null(), "B has no prior week — null, not a fabricated 0");
        assert!(b["delta"].is_null());

        // Retired-metric exclusion: project_health is past its effective_until, so
        // even with a durable daily row the endpoint filters it out of the active
        // dashboard (get_project_metrics' ACTIVE_METRIC_PREDICATE) — never a stale card.
        assert!(metrics.iter().all(|m| m["metric"].as_str() != Some("project_health")),
            "retired project_health is excluded from the active metrics, not rendered as a stale card");

        // Honest-empty: a project with NO metric rows → 200 with an empty list.
        let empty_pid = state.pg.create_project(&format!("_test:pme-empty:{uniq}"), None, None).await.unwrap();
        let (st, body) = req(app.clone(), "GET", &format!("/api/projects/{empty_pid}/metrics"), None).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(body["metrics"].as_array().map(|a| a.len()), Some(0),
            "no rows → honest-empty list, not a fabricated row");

        // Unknown project id → 404 (matches GET /api/projects/{id}/ftr), never a 200-empty.
        let (st, _) = req(app, "GET", &format!("/api/projects/{}/metrics", uuid::Uuid::new_v4()), None).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "unknown project → 404, never a fabricated empty");

        // cleanup — project_metrics cascade from the metric + project; the shared
        // project_health registry row is left intact (its test row cascades w/ pid).
        sqlx_core::query::query("DELETE FROM sensei.metrics WHERE id = ANY($1)")
            .bind(vec![mid_a, mid_b]).execute(state.pg.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = ANY($1)")
            .bind(vec![pid, empty_pid]).execute(state.pg.pool()).await.unwrap();
        // Repo-grain: drop the per-test repository (its uniq-derived repo_key must not
        // leak — the health_mid row already cascaded with `pid`).
        sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = $1")
            .bind(rid).execute(state.pg.pool()).await.unwrap();
    }

    /// 8.2: the legacy FTR surfaces — the HTTP route `GET /api/metrics/{project}`
    /// and the MCP `get_metrics` tool — are store-backed and NEVER fabricate a `0`
    /// FTR on a miss. A project WITH `ftr` rows reports the store number (the same
    /// Σnum/Σden `/projects/{id}/ftr` serves); a project WITHOUT any is honest
    /// `null`, not `0.0`. Both surfaces are pinned.
    #[tokio::test]
    async fn no_fabricated_zero_ftr_in_metrics_surfaces() {
        let (app, state) = test_app().await;
        let (ftr_mid,): (uuid::Uuid,) =
            sqlx_core::query_as::query_as("SELECT id FROM sensei.metrics WHERE key = 'ftr'")
                .fetch_one(state.pg.pool()).await.expect("ftr metric seeded in registry");
        let today = chrono::Utc::now().date_naive();

        // Project WITH ftr data — folder name is what both surfaces resolve on.
        let uniq_has = uuid::Uuid::new_v4();
        let (pid_has, fid_has) =
            crate::tasks::test_support::seed_metrics_project_folder(&state.pg, &uniq_has).await;
        let name_has = format!("metrics-{uniq_has}");
        // Repo-grain (_v2): `ftr` is a SHARED registry metric — a NULL-repository write
        // is unique only per (metric,day,grain), so concurrent test projects writing
        // `today` collide and steal each other's project_id. Pin it to the fixture's
        // repository (scope=user) so this project's row is its own.
        let rid_has = crate::tasks::test_support::repository_for_folder(&state.pg, &fid_has).await;
        state.pg.upsert_project_metric_repo(&ftr_mid, &pid_has, Some(&rid_has), "user", None, None,
            None, None, today, "daily", 0.75,
            &serde_json::json!({"numerator": 3, "denominator": 4}), "measured").await.unwrap();

        // Project WITHOUT any ftr data.
        let uniq_no = uuid::Uuid::new_v4();
        let (pid_no, fid_no) =
            crate::tasks::test_support::seed_metrics_project_folder(&state.pg, &uniq_no).await;
        let name_no = format!("metrics-{uniq_no}");
        let rid_no = crate::tasks::test_support::repository_for_folder(&state.pg, &fid_no).await;

        // ── HTTP surface: GET /api/metrics/{project} ──────────────────────
        let (st, body) = req(app.clone(), "GET", &format!("/api/metrics/{name_has}"), None).await;
        assert_eq!(st, StatusCode::OK, "{body}");
        assert!((body["ftr"].as_f64().expect("ftr is a number when data exists") - 0.75).abs() < 1e-9,
            "HTTP ftr = store Σnum/Σden (0.75), not a re-computed completion rate");

        let (st, body) = req(app.clone(), "GET", &format!("/api/metrics/{name_no}"), None).await;
        assert_eq!(st, StatusCode::OK, "{body}");
        assert!(body["ftr"].is_null(),
            "HTTP ftr is honest-null with no data — NEVER a fabricated 0.0; got {}", body["ftr"]);

        // ── MCP surface: POST /api/mcp/call get_metrics ───────────────────
        let call = |name: &str| serde_json::json!({"tool": "get_metrics", "params": {"repoId": name}});
        let (st, body) = req(app.clone(), "POST", "/api/mcp/call", Some(call(&name_has))).await;
        assert_eq!(st, StatusCode::OK, "{body}");
        assert!((body["ftr"].as_f64().expect("mcp ftr is a number when data exists") - 0.75).abs() < 1e-9,
            "MCP ftr = store Σnum/Σden (0.75)");

        let (st, body) = req(app.clone(), "POST", "/api/mcp/call", Some(call(&name_no))).await;
        assert_eq!(st, StatusCode::OK, "{body}");
        assert!(body["ftr"].is_null(),
            "MCP ftr is honest-null with no data — NEVER a fabricated 0.0; got {}", body["ftr"]);

        // cleanup — sessions (none) + folders + projects; project_metrics cascade.
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = ANY($1)")
            .bind(vec![fid_has, fid_no]).execute(state.pg.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = ANY($1)")
            .bind(vec![pid_has, pid_no]).execute(state.pg.pool()).await.unwrap();
        // Repo-grain: drop the per-fixture repositories (folders.repository_id is
        // ON DELETE SET NULL, and the ftr row already cascaded with its project).
        sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = ANY($1)")
            .bind(vec![rid_has, rid_no]).execute(state.pg.pool()).await.unwrap();
    }

    /// 7.3 `GET /api/projects/{id}/metrics/{key}?grain=weekly`: the weekly series
    /// matches the view's re-derived Σnum/Σden (never the mean of daily ratios).
    /// Also pins: an invalid grain is a 400; an unknown key is honest-empty (200 []).
    #[tokio::test]
    async fn get_project_metric_series_endpoint() {
        let (app, state) = test_app().await;
        let uniq = uuid::Uuid::new_v4().simple().to_string();
        let pid = state.pg.create_project(&format!("_test_pms_{uniq}"), None, None).await.unwrap();
        let key = format!("_test_pms_{uniq}_cov"); // URL-safe (no colons) — it goes in the path
        let mid = seed_metric(&state.pg, &key, "ratio", "higher_better").await;

        // Four daily ratio rows across two ISO weeks, with UNEQUAL denominators
        // within each week so Σnum/Σden differs from the mean of daily ratios —
        // proving the weekly view re-derives from sums, not by averaging ratios.
        // (daily value = num/den; the weekly roll-up reads props, not this value.)
        let days = [
            (chrono::NaiveDate::from_ymd_opt(2020, 1, 6).unwrap(),  1, 1), // wk 01-06: ratio 1.000
            (chrono::NaiveDate::from_ymd_opt(2020, 1, 8).unwrap(),  1, 3), //           ratio 0.333
            (chrono::NaiveDate::from_ymd_opt(2020, 1, 13).unwrap(), 3, 4), // wk 01-13: ratio 0.750
            (chrono::NaiveDate::from_ymd_opt(2020, 1, 15).unwrap(), 3, 12),//           ratio 0.250
        ];
        for (d, num, den) in days {
            state.pg.upsert_project_metric(&mid, &pid, None, None, d, "daily",
                num as f64 / den as f64,
                &serde_json::json!({"numerator": num, "denominator": den}), "measured").await.unwrap();
        }
        // Expected weekly values re-derived Σnum/Σden:
        //   wk1 = (1+1)/(1+3)  = 0.5   [mean of daily ratios would be 0.667 — rejected]
        //   wk2 = (3+3)/(4+12) = 0.375 [mean of daily ratios would be 0.5   — rejected]
        let expect = [
            (chrono::NaiveDate::from_ymd_opt(2020, 1, 6).unwrap(),  0.5_f64),
            (chrono::NaiveDate::from_ymd_opt(2020, 1, 13).unwrap(), 0.375_f64),
        ];

        let (st, body) = req(app.clone(), "GET",
            &format!("/api/projects/{pid}/metrics/{key}?grain=weekly"), None).await;
        assert_eq!(st, StatusCode::OK, "{body}");
        assert_eq!(body["grain"], "weekly");
        assert_eq!(body["metric"].as_str(), Some(key.as_str()));
        assert_eq!(body["formula"].as_str(), Some("test formula"),
            "series carries the metric's formula facet (how it's calculated)");
        let series = body["series"].as_array().expect("series array");
        assert_eq!(series.len(), 2, "one point per ISO week");
        for (point, (period, value)) in series.iter().zip(expect) {
            assert_eq!(point["period"].as_str(), Some(period.to_string().as_str()),
                "period is the week start");
            let got = point["value"].as_f64().expect("numeric value");
            assert!((got - value).abs() < 1e-9,
                "weekly value re-derived Σnum/Σden: got {got}, want {value}");
        }

        // Invalid grain → 400 (never a silent default that would mismeasure).
        let (st, _) = req(app.clone(), "GET",
            &format!("/api/projects/{pid}/metrics/{key}?grain=yearly"), None).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "invalid grain → 400");

        // Unknown metric key (no rows) → 200 with an empty series (honest-empty)
        // and an honest-null formula (the key names no registered metric).
        let (st, body) = req(app.clone(), "GET",
            &format!("/api/projects/{pid}/metrics/{key}_nope?grain=weekly"), None).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(body["series"].as_array().map(|a| a.len()), Some(0),
            "unknown key → honest-empty series, not a failure");
        assert!(body["formula"].is_null(), "unknown key → honest-null formula, not a fabricated string");

        sqlx_core::query::query("DELETE FROM sensei.metrics WHERE id = $1")
            .bind(mid).execute(state.pg.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(pid).execute(state.pg.pool()).await.unwrap();
    }

    /// `GET /api/projects/{id}/metrics/{key}/sessions?day=YYYY-MM-DD`: the
    /// datapoint→sessions drill-down returns that day's measurable sessions
    /// (folder-join scope) with the structural one-liner fields; an absent or
    /// malformed `day` is a 400; a day with none is honest-empty (200 []).
    #[tokio::test]
    async fn get_project_metric_day_sessions_endpoint() {
        let (app, state) = test_app().await;
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = crate::tasks::test_support::seed_metrics_project_folder(&state.pg, &uniq).await;
        let key = format!("_test_pmds_{}", uniq.simple()); // URL-safe (no colons) — it goes in the path

        let utc = |ts: &str| chrono::DateTime::parse_from_rfc3339(ts).unwrap().with_timezone(&chrono::Utc);
        // `client_session_id` is globally unique — scope the ids to this run's uniq.
        let sid = |n: &str| format!("cs-{n}-{}", uniq.simple());
        // Two measurable sessions on the target day + one in-flight (excluded).
        for (cs, task, outcome, ftr, turns, corr, at) in [
            (sid("1"), "task-a", Some("completed"), Some(true),  3, 0, "2020-06-15T12:00:00Z"),
            (sid("2"), "task-b", Some("corrected"), Some(false), 5, 2, "2020-06-15T20:00:00Z"),
            (sid("x"), "task-x", None,              None,        0, 0, "2020-06-15T13:00:00Z"),
        ] {
            sqlx_core::query::query(
                "INSERT INTO activity.sessions
                    (folder_id, project_id, client_session_id, task, outcome, ftr, turns, corrections, started_at)
                 VALUES ($1, $2, $3, $4, $5::sensei.session_outcome, $6, $7, $8, $9)")
                .bind(fid).bind(pid).bind(cs).bind(task).bind(outcome).bind(ftr)
                .bind(turns).bind(corr).bind(utc(at))
                .execute(state.pg.pool()).await.unwrap();
        }

        // Happy path: 200 + the two measurable sessions (newest-first), each with the
        // structural fields the client renders its one-liner from.
        let (st, body) = req(app.clone(), "GET",
            &format!("/api/projects/{pid}/metrics/{key}/sessions?day=2020-06-15"), None).await;
        assert_eq!(st, StatusCode::OK, "{body}");
        assert_eq!(body["day"], "2020-06-15");
        assert_eq!(body["metric"].as_str(), Some(key.as_str()));
        assert_eq!(body["count"].as_u64(), Some(2), "two measurable sessions that day (in-flight excluded)");
        let sessions = body["sessions"].as_array().expect("sessions array");
        assert_eq!(sessions.len(), 2);
        let s0 = &sessions[0];
        assert_eq!(s0["client_session_id"].as_str(), Some(sid("2").as_str()), "newest-first");
        for field in ["outcome", "ftr", "turns", "corrections", "task", "summary", "started_at"] {
            assert!(s0.get(field).is_some(), "structural field {field} present");
        }
        assert_eq!(s0["outcome"], "corrected");
        assert_eq!(s0["ftr"], serde_json::json!(false));
        assert_eq!(s0["turns"], 5);
        assert_eq!(s0["corrections"], 2);

        // Absent day → 400 (no datapoint to scope to).
        let (st, _) = req(app.clone(), "GET",
            &format!("/api/projects/{pid}/metrics/{key}/sessions"), None).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "absent day → 400");
        // Malformed day → 400 (never a silent default).
        let (st, _) = req(app.clone(), "GET",
            &format!("/api/projects/{pid}/metrics/{key}/sessions?day=nope"), None).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "malformed day → 400");
        // A day with no measurable session → honest-empty (200 []).
        let (st, body) = req(app.clone(), "GET",
            &format!("/api/projects/{pid}/metrics/{key}/sessions?day=2019-01-01"), None).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(body["count"].as_u64(), Some(0), "no sessions that day → honest-empty");

        sqlx_core::query::query("DELETE FROM activity.sessions WHERE folder_id = $1")
            .bind(fid).execute(state.pg.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(pid).execute(state.pg.pool()).await.unwrap();
    }

    /// The drill-down attaches a per-session, per-metric `observation` (the "why
    /// this session moved this metric" line). On a CACHE HIT it serves the stored
    /// model copy; on a MISS it serves the deterministic, row-derived fallback
    /// (title = metric label, detail = a structural line) — never blank, never
    /// fabricated. Seeds one `insight_copy` row for one session's exact
    /// `facts_hash` and leaves a second session uncached to pin both paths.
    #[tokio::test]
    async fn get_project_metric_day_sessions_attaches_observation() {
        use crate::analysis::insight_copy::{facts_hash, InsightKind};
        use crate::analysis::session_metric_note::SessionMetricFacts;

        let (app, state) = test_app().await;
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = crate::tasks::test_support::seed_metrics_project_folder(&state.pg, &uniq).await;
        // URL-safe key (goes in the path). seed_metric sets name = key and
        // how_to_read = 'test how', so label = key and meaning = "test how".
        let key = format!("_test_smo_{}", uniq.simple());
        let mid = seed_metric(&state.pg, &key, "ratio", "higher_better").await;

        let utc = |ts: &str| chrono::DateTime::parse_from_rfc3339(ts).unwrap().with_timezone(&chrono::Utc);
        let sid = |n: &str| format!("cs-{n}-{}", uniq.simple());
        // Two measurable sessions on the target day. The MISS one is newer → first.
        for (cs, task, summary, outcome, ftr, turns, corr, at) in [
            (sid("hit"),  "task-hit",  "did the hit thing",  "completed", true,  3, 0, "2020-06-15T12:00:00Z"),
            (sid("miss"), "task-miss", "did the miss thing", "corrected", false, 5, 2, "2020-06-15T20:00:00Z"),
        ] {
            sqlx_core::query::query(
                "INSERT INTO activity.sessions
                    (folder_id, project_id, client_session_id, task, summary, outcome, ftr, turns, corrections, started_at)
                 VALUES ($1, $2, $3, $4, $5, $6::sensei.session_outcome, $7, $8, $9, $10)")
                .bind(fid).bind(pid).bind(cs).bind(task).bind(summary).bind(outcome).bind(ftr)
                .bind(turns).bind(corr).bind(utc(at))
                .execute(state.pg.pool()).await.unwrap();
        }

        // Seed the cache for exactly the HIT session's facts (label is display-only
        // and NOT hashed, so the meaning + session signals are what the hash keys on).
        let hit_row = serde_json::json!({
            "outcome": "completed", "ftr": true, "turns": 3, "corrections": 0,
            "task": "task-hit", "summary": "did the hit thing",
        });
        let hit_facts = SessionMetricFacts::from_session_row(&hit_row, &key, &key, "test how");
        let hit_hash = facts_hash(InsightKind::SessionMetricObservation, &hit_facts.to_facts_json());
        state.pg.upsert_insight_copy(
            "session_metric_observation", &hit_hash,
            "first-try win", "this session resolved on the first attempt, lifting the rate",
            None, None,
        ).await;

        let (st, body) = req(app.clone(), "GET",
            &format!("/api/projects/{pid}/metrics/{key}/sessions?day=2020-06-15"), None).await;
        assert_eq!(st, StatusCode::OK, "{body}");
        let sessions = body["sessions"].as_array().expect("sessions array");
        assert_eq!(sessions.len(), 2);

        let by_id = |cs: &str| sessions.iter()
            .find(|s| s["client_session_id"].as_str() == Some(cs))
            .unwrap_or_else(|| panic!("session {cs} present"));

        // HIT: the stored model copy is served verbatim.
        let hit = by_id(&sid("hit"));
        assert_eq!(hit["observation"]["title"], "first-try win", "cache hit → stored model title");
        assert_eq!(hit["observation"]["detail"],
            "this session resolved on the first attempt, lifting the rate",
            "cache hit → stored model detail");

        // MISS: the deterministic, row-derived fallback — title = metric label
        // (the seeded name == key), detail = the structural line from the row. This is
        // a NON-EFFORT metric (is_effort gates the correction count to ftr/rework_ratio),
        // so the fallback is the neutral "outcome · turns" line — the row's corrections
        // are deliberately NOT shown (they'd imply an effort relevance this metric lacks).
        let miss = by_id(&sid("miss"));
        assert_eq!(miss["observation"]["title"], key, "cache miss → metric label as title");
        assert_eq!(miss["observation"]["detail"], "outcome corrected; 5 turns",
            "cache miss → deterministic neutral line for a non-effort metric, never blank or fabricated");

        sqlx_core::query::query(
            "DELETE FROM sensei.insight_copy WHERE kind = 'session_metric_observation' AND facts_hash = $1")
            .bind(&hit_hash).execute(state.pg.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE folder_id = $1")
            .bind(fid).execute(state.pg.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.metrics WHERE id = $1")
            .bind(mid).execute(state.pg.pool()).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(pid).execute(state.pg.pool()).await.unwrap();
    }

    /// End-to-end automated-run flow through the REAL router against sensei_test:
    /// register a plan graph, flip both tasks, read nudges, mark the run done.
    /// Proves the AR routes dispatch to their handlers — including the static
    /// `/api/runs/plan` (must win over `/{id}`) and the two-param
    /// `/api/runs/{id}/tasks/{task_id}` — and that the stored graph reflects the flips.
    #[tokio::test]
    async fn automated_run_flow_through_the_router() {
        let (app, state) = test_app().await;
        let plan = serde_json::json!({
            "goal": "smoke the automated-run path",
            "plan": { "phases": [{ "title": "Build", "tasks": [
                { "id": "t1", "title": "one", "agent": "general-purpose", "model": "sonnet" },
                { "id": "t2", "title": "two", "model": "opus", "deps": ["t1"] }
            ]}]}
        });

        // register_plan → 201 + a running run (the static /plan route beats /{id}).
        let (st, body) = req(app.clone(), "POST", "/api/runs/plan", Some(plan)).await;
        assert_eq!(st, StatusCode::CREATED, "register_plan routed + created: {body}");
        assert_eq!(body["run"]["status"], "running");
        let run_id = body["run"]["id"].as_str().unwrap().to_string();

        // GET the run resolves (not swallowed by a static route).
        let (st, _) = req(app.clone(), "GET", &format!("/api/runs/{run_id}"), None).await;
        assert_eq!(st, StatusCode::OK, "GET /api/runs/{{id}} routed");

        // Flip both tasks done via the two-param /tasks/{task_id} route.
        let (st, _) = req(app.clone(), "POST", &format!("/api/runs/{run_id}/tasks/t1"), Some(serde_json::json!({ "state": "done" }))).await;
        assert_eq!(st, StatusCode::OK, "update_task_status routed (t1)");
        let (st, ok) = req(app.clone(), "POST", &format!("/api/runs/{run_id}/tasks/t2"), Some(serde_json::json!({ "state": "done" }))).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(ok["state"], "done");

        // nudges → 200 with an empty list (no enrolled dojo in the test DB).
        let (st, nj) = req(app.clone(), "GET", &format!("/api/runs/{run_id}/nudges"), None).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(nj["nudges"], serde_json::json!([]));

        // The stored plan graph reflects both flips (the authored-run source of truth).
        let rid = uuid::Uuid::parse_str(&run_id).unwrap();
        let g = state.pg.run_plan_graph(&rid).await.unwrap().unwrap();
        assert_eq!(g["phases"][0]["tasks"][0]["state"], "done");
        assert_eq!(g["phases"][0]["tasks"][1]["state"], "done");

        // report_run_outcome → terminal done.
        let (st, body) = req(app.clone(), "POST", &format!("/api/runs/{run_id}/outcome"), Some(serde_json::json!({ "outcome": "done", "summary": "smoke ok" }))).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(body["run"]["status"], "done");

        sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
            .bind(rid).execute(state.pg.pool()).await.unwrap();
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

    /// With no provisioning supervisor (default build / non-embedded), the
    /// provision POST must degrade to 501 with a clear JSON error — never a
    /// panic or a silent success. `test_app()` carries `provisioning: None`.
    #[tokio::test]
    async fn provision_model_without_supervisor_is_501() {
        let (app, _) = test_app().await;
        let resp = app.oneshot(
            Request::builder().method("POST")
                .uri("/api/gateway/models/gemma2:2b/provision")
                .body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            json["error"].as_str().unwrap_or_default().contains("not available"),
            "expected an explanatory error, got {json}"
        );
    }

    /// The status GET always answers 200 with a `models` array — empty when no
    /// supervisor is wired (default build), so a UI can poll unconditionally.
    #[tokio::test]
    async fn provision_status_without_supervisor_is_empty_list() {
        let (app, _) = test_app().await;
        let resp = app.oneshot(
            Request::builder()
                .uri("/api/gateway/models/provision/status")
                .body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["models"].as_array().map(|a| a.len()), Some(0));
    }

    #[tokio::test]
    async fn index_doctor_endpoint_returns_readonly_report() {
        let (app, _) = test_app().await;
        let resp = app.oneshot(
            Request::builder().uri("/api/index/doctor").body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // The read-only doctor report carries the per-class drift shape.
        assert_eq!(json["repair"], false, "doctor is read-only");
        assert!(json["orphan_files"].is_number());
        assert!(json["ghost_folders"].is_number());
        assert!(json["nested_standalone"].is_number());
        assert!(json["duplicate_name_projects"].is_number());
        assert!(json["samples"].is_object());
    }

    #[tokio::test]
    async fn folder_remap_endpoint_404_when_new_path_not_indexed() {
        let (app, _) = test_app().await;
        let resp = app.oneshot(
            Request::builder().method("POST").uri("/api/folders/remap")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"old":"/_test/remap-ep-gone","new":"/_test/remap-ep-never-indexed"}"#)).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "the rename destination must be an indexed folder");
    }

    #[tokio::test]
    async fn folder_remap_endpoint_aliases_old_and_reattaches_sessions() {
        let (app, state) = test_app().await;
        // The rename destination must be an indexed folder.
        state.pg.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000001','/_test','_test','watching'::sensei.watch_status) ON CONFLICT DO NOTHING"
        ).await.unwrap();
        state.pg.execute_raw(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path) VALUES('00000000-0000-0000-0000-000000000001','git'::sensei.folder_kind,'remap-ep-new','remap-ep-new','/_test/remap-ep-new') ON CONFLICT(abs_path) DO NOTHING"
        ).await.unwrap();
        // Start clean, then leave an orphaned event captured under the OLD (renamed) path.
        state.pg.execute_raw("DELETE FROM activity.sessions WHERE client_session_id = '_test-remap-ep-session'").await.unwrap();
        state.pg.execute_raw("DELETE FROM activity.assistant_events WHERE session_id = '_test-remap-ep-session'").await.unwrap();
        state.pg.insert_hook_event("_test-remap-ep-session", "claude", "PreToolUse", None, Some("/_test/remap-ep-old"), 1_700_000_500, None, &serde_json::json!({})).await.unwrap();

        let resp = app.oneshot(
            Request::builder().method("POST").uri("/api/folders/remap")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"old":"/_test/remap-ep-old","new":"/_test/remap-ep-new"}"#)).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["remapped"], false, "old had no folder row → alias-only");
        assert_eq!(json["aliased"], true);
        assert!(json["sessions_repaired"].as_u64().unwrap() >= 1, "the orphaned session under old is re-attached");

        // The old path now aliases forward to the new folder.
        let new_id = state.pg.folder_id_by_abs_path("/_test/remap-ep-new").await.unwrap().unwrap();
        let resolved = state.pg.get_folder_ids_by_path("/_test/remap-ep-old").await.unwrap();
        assert_eq!(resolved.map(|(f, _)| f), Some(new_id), "the old path resolves to the new folder via the alias");
    }

    #[tokio::test]
    async fn save_memory_rejects_a_secret_and_does_not_leak_it() {
        let (app, _) = test_app().await;
        // Token built at runtime (split parts) so no secret-format literal is in the source.
        let token = format!("ghp_{}", "b".repeat(38));
        let body = format!(
            r#"{{"scope":"project","type":"convention","title":"note","content":"the token is {token}"}}"#
        );
        let resp = app.oneshot(
            Request::builder().method("POST").uri("/api/knowledge/memories")
                .header("content-type", "application/json")
                .body(Body::from(body)).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "a memory carrying a secret is rejected (fail closed)");
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("secret") && text.contains("github token"), "the error names the kind: {text}");
        assert!(!text.contains(&token), "the secret value is never echoed back: {text}");
    }

    #[tokio::test]
    async fn save_memory_records_and_surfaces_evidence() {
        let (app, _) = test_app().await;
        let body = r#"{"scope":"global","type":"convention","title":"ev-note","content":"reuse the shared helper","evidence":"crates/senseid/src/x.rs:42"}"#;
        let resp = app.clone().oneshot(
            Request::builder().method("POST").uri("/api/knowledge/memories")
                .header("content-type", "application/json")
                .body(Body::from(body)).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let saved: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let id = saved["id"].as_str().expect("id");
        // The read surfaces the save-time source note as evidence.
        let get = app.oneshot(
            Request::builder().uri(format!("/api/knowledge/memories/{id}")).body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(get.status(), StatusCode::OK);
        let gb = axum::body::to_bytes(get.into_body(), usize::MAX).await.unwrap();
        let detail: serde_json::Value = serde_json::from_slice(&gb).unwrap();
        let ev = detail["evidence"].as_array().expect("evidence array");
        assert!(ev.iter().any(|e| e["note"] == "crates/senseid/src/x.rs:42" && e["session_id"].is_null()),
            "the save-time source note is surfaced with a null session_id: {:?}", ev);
    }

    #[tokio::test]
    async fn library_skills_endpoints_serve_capabilities() {
        use crate::libraries::manifest::ProvidedSkill;
        let (app, state) = test_app().await;
        let lib = format!("_libep_{}", uuid::Uuid::new_v4());
        let lid = state.pg.upsert_library(&lib, "npm", Some("1"), None, None, None).await.unwrap();
        state.pg.replace_library_capabilities(&lid, "manifest", Some("1"),
            &[ProvidedSkill { name: "semantic-styles-rokkit".into(), focus: "styling".into(), path: Some("p.md".into()), body: Some("# body".into()) }],
            &[]).await.unwrap();

        let list = app.clone().oneshot(Request::builder().uri(format!("/api/libs/{lib}/skills")).body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(list.status(), StatusCode::OK);
        let lb = axum::body::to_bytes(list.into_body(), usize::MAX).await.unwrap();
        let arr: serde_json::Value = serde_json::from_slice(&lb).unwrap();
        assert_eq!(arr.as_array().unwrap().len(), 1);
        assert_eq!(arr[0]["focus"], "styling");

        let get = app.clone().oneshot(Request::builder().uri(format!("/api/libs/{lib}/skills/styling")).body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(get.status(), StatusCode::OK);
        let gb = axum::body::to_bytes(get.into_body(), usize::MAX).await.unwrap();
        let one: serde_json::Value = serde_json::from_slice(&gb).unwrap();
        assert_eq!(one["name"], "semantic-styles-rokkit");
        assert_eq!(one["body"], "# body");

        let miss = app.oneshot(Request::builder().uri(format!("/api/libs/{lib}/skills/nope")).body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(miss.status(), StatusCode::NOT_FOUND, "unknown focus → 404, not a fabricated empty");
    }

    #[tokio::test]
    async fn risk_class_endpoint_classifies_by_path() {
        let (app, _) = test_app().await;
        let resp = app.oneshot(
            Request::builder().method("POST").uri("/api/review/risk-class")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"paths":["database/ddl/x.ddl","docs/y.md"],"task":"tweak schema"}"#)).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["class"], "approve", "a DDL change requires approve");
        assert!(json["reasons"].as_array().unwrap().iter().any(|r| r.as_str().unwrap_or("").contains(".ddl")),
            "reasons name the DDL driver: {:?}", json["reasons"]);
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
    async fn collective_preferences_put_then_get_roundtrip() {
        let (app, state) = test_app().await;
        // Serialize with the pg_store singleton test (shared one-row table).
        let _guard = crate::collective::preferences::test_lock().enter();

        // PUT a known-valid body → 200; the response echoes the saved shape.
        let put = app.clone().oneshot(
            Request::builder().method("PUT").uri("/api/preferences/collective")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({
                    "destination": "global", "cadence": "weekly", "attribution_default": "anonymous",
                    "categories": { "memory": false }
                }).to_string())).unwrap()
        ).await.unwrap();
        assert_eq!(put.status(), StatusCode::OK);
        let body = axum::body::to_bytes(put.into_body(), usize::MAX).await.unwrap();
        let saved: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(saved["destination"], "global");
        assert_eq!(saved["cadence"], "weekly");
        assert_eq!(saved["attribution_default"], "anonymous");
        assert_eq!(saved["categories"]["memory"], false);
        assert_eq!(saved["categories"]["pattern"], true);
        assert!(saved["updated_at"].is_string(), "PUT response carries updated_at");

        // GET reflects the write, with the full category shape.
        let get = app.oneshot(
            Request::builder().uri("/api/preferences/collective").body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(get.status(), StatusCode::OK);
        let body = axum::body::to_bytes(get.into_body(), usize::MAX).await.unwrap();
        let got: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(got["destination"], "global");
        assert_eq!(got["categories"]["memory"], false);
        for cat in ["memory", "pattern", "rule", "prompt", "guard", "skill", "agent"] {
            assert!(got["categories"][cat].is_boolean(), "category {cat} present in GET");
        }

        // Cleanup the shared singleton row.
        sqlx_core::query::query("DELETE FROM sensei.collective_preferences")
            .execute(state.pg.pool()).await.ok();
    }

    #[tokio::test]
    async fn collective_preferences_put_rejects_invalid_destination() {
        let (app, _) = test_app().await;
        let resp = app.oneshot(
            Request::builder().method("PUT").uri("/api/preferences/collective")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"destination":"everyone"}"#)).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["error"].as_str().unwrap_or_default().contains("destination"));
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

    // ── Memory lifecycle action routes (archive/reinforce/challenge/dismiss/merge) ──

    /// Seed one active memory (`status='active'`, `strength=1.0`) and return its id.
    async fn seed_memory(state: &AppState, title: &str, content: &str) -> uuid::Uuid {
        state.pg.create_memory(None, "global", None, "decision", title, content, None, None, None, None)
            .await.unwrap()
    }

    #[tokio::test]
    async fn memory_archive_action_sets_archived() {
        let (app, state) = test_app().await;
        let mid = seed_memory(&state, "_test:route_archive", "rule a").await;
        let resp = app.oneshot(
            Request::builder().method("POST")
                .uri(format!("/api/knowledge/memories/{mid}/archive"))
                .body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let m = state.pg.get_memory(&mid).await.unwrap().unwrap();
        assert_eq!(m["status"], "archived");
    }

    #[tokio::test]
    async fn memory_reinforce_action_bumps_strength() {
        let (app, state) = test_app().await;
        let mid = seed_memory(&state, "_test:route_reinforce", "rule r").await;
        assert_eq!(state.pg.get_memory(&mid).await.unwrap().unwrap()["strength"], 1.0);
        let resp = app.oneshot(
            Request::builder().method("POST")
                .uri(format!("/api/knowledge/memories/{mid}/reinforce"))
                .body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        // Strength moves; status promotion is the analyzer's job, not this action.
        assert_eq!(state.pg.get_memory(&mid).await.unwrap().unwrap()["strength"], 2.0);
    }

    #[tokio::test]
    async fn memory_challenge_action_sets_challenged() {
        let (app, state) = test_app().await;
        let mid = seed_memory(&state, "_test:route_challenge", "rule c").await;
        let resp = app.oneshot(
            Request::builder().method("POST")
                .uri(format!("/api/knowledge/memories/{mid}/challenge"))
                .body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "challenged");
        assert_eq!(state.pg.get_memory(&mid).await.unwrap().unwrap()["status"], "challenged");
    }

    #[tokio::test]
    async fn memory_dismiss_action_sets_rejected() {
        let (app, state) = test_app().await;
        let mid = seed_memory(&state, "_test:route_dismiss", "rule d").await;
        let resp = app.oneshot(
            Request::builder().method("POST")
                .uri(format!("/api/knowledge/memories/{mid}/dismiss"))
                .body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(state.pg.get_memory(&mid).await.unwrap().unwrap()["status"], "rejected");
    }

    #[tokio::test]
    async fn memory_merge_action_links_and_archives_member() {
        let (app, state) = test_app().await;
        let member = seed_memory(&state, "_test:route_merge_member", "member text").await;
        let rep = seed_memory(&state, "_test:route_merge_rep", "surviving text").await;
        let resp = app.oneshot(
            Request::builder().method("POST")
                .uri(format!("/api/knowledge/memories/{member}/merge"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"into": rep.to_string()}).to_string())).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        // Member leaves the active set (archived) and is linked under the survivor.
        assert_eq!(state.pg.get_memory(&member).await.unwrap().unwrap()["status"], "archived");
        assert_eq!(state.pg.get_memory_parent(&member).await.unwrap(), Some(rep));
    }

    #[tokio::test]
    async fn memory_action_bad_uuid_is_400() {
        let (app, _) = test_app().await;
        let resp = app.oneshot(
            Request::builder().method("POST")
                .uri("/api/knowledge/memories/not-a-uuid/archive")
                .body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn memory_merge_missing_into_is_400() {
        let (app, state) = test_app().await;
        let mid = seed_memory(&state, "_test:route_merge_nointo", "x").await;
        let resp = app.oneshot(
            Request::builder().method("POST")
                .uri(format!("/api/knowledge/memories/{mid}/merge"))
                .header("content-type", "application/json")
                .body(Body::from("{}")).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn memory_merge_invalid_into_is_400() {
        let (app, state) = test_app().await;
        let mid = seed_memory(&state, "_test:route_merge_badinto", "x").await;
        let resp = app.oneshot(
            Request::builder().method("POST")
                .uri(format!("/api/knowledge/memories/{mid}/merge"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"into":"not-a-uuid"}"#)).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // ── Spine anchoring on the save/propose HTTP path ────────────────────

    #[tokio::test]
    async fn save_memory_scope_validates_spine_slot() {
        let (app, state) = test_app().await;

        // brief is a feature-scope slot; without a feature it must be rejected.
        let resp = app.clone().oneshot(
            Request::builder().method("POST")
                .uri("/api/knowledge/memories")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({
                    "scope": "global", "type": "decision",
                    "title": "_test:slot_brief_nofeature", "content": "c",
                    "spine_slot": "brief",
                }).to_string())).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "feature-scope slot needs a feature");

        // Unknown slot is a 400 too.
        let resp = app.clone().oneshot(
            Request::builder().method("POST")
                .uri("/api/knowledge/memories")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({
                    "scope": "global", "type": "decision",
                    "title": "_test:slot_nonsense", "content": "c",
                    "spine_slot": "nonsense",
                }).to_string())).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "unknown spine_slot rejected");

        // brief + feature is valid and persists both columns.
        let resp = app.oneshot(
            Request::builder().method("POST")
                .uri("/api/knowledge/memories")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({
                    "scope": "global", "type": "decision",
                    "title": "_test:slot_brief_auth", "content": "c",
                    "spine_slot": "brief", "feature": "auth",
                }).to_string())).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = uuid::Uuid::parse_str(json["id"].as_str().unwrap()).unwrap();
        let row: (Option<String>, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT spine_slot::text, feature FROM sensei.memories WHERE id = $1"
        ).bind(id).fetch_one(state.pg.pool()).await.unwrap();
        assert_eq!(row, (Some("brief".into()), Some("auth".into())));

        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
            .bind(id).execute(state.pg.pool()).await.unwrap();
    }

    #[tokio::test]
    async fn get_context_slot_hint_leads_the_bundle() {
        // The MCP `get_layered_context` slot hint (Task 5) reaches this handler as
        // ?slot=&feature=, which get_context forwards into assemble_context's
        // `slot` param. A slot-anchored memory must lead the bundle when the hint
        // is present, and the endpoint's prior behavior (no slot) must stay
        // unchanged when the hint is absent.
        let (app, state) = test_app().await;
        let pid = state.pg.ensure_test_project("route-slot-context").await.unwrap();

        let m_unanchored = state.pg.create_memory(
            Some(&pid), "project", None, "decision", "_test:route_slot_unanchored", "c",
            None, None, None, None,
        ).await.unwrap();
        let m_design = state.pg.create_memory(
            Some(&pid), "project", None, "decision", "_test:route_slot_design", "c",
            None, None, Some("design"), None,
        ).await.unwrap();

        // With ?slot=design: the slot-anchored memory leads.
        let (status, blob) = get_json(
            &app, &format!("/api/knowledge/context?project_id={pid}&slot=design"),
        ).await;
        assert_eq!(status, StatusCode::OK);
        let ids: Vec<String> = blob["memories"].as_array().unwrap().iter()
            .map(|m| m["id"].as_str().unwrap().to_string()).collect();
        assert_eq!(ids.first().map(String::as_str), Some(m_design.to_string().as_str()),
            "slot-anchored memory must lead when ?slot= is present");
        assert!(ids.contains(&m_unanchored.to_string()), "general blend still present");

        // Without ?slot=: unchanged general blend (order not slot-led).
        let (status_plain, blob_plain) = get_json(
            &app, &format!("/api/knowledge/context?project_id={pid}"),
        ).await;
        assert_eq!(status_plain, StatusCode::OK);
        let plain_ids: Vec<String> = blob_plain["memories"].as_array().unwrap().iter()
            .map(|m| m["id"].as_str().unwrap().to_string()).collect();
        assert!(plain_ids.contains(&m_design.to_string()) && plain_ids.contains(&m_unanchored.to_string()),
            "both memories still present with no slot hint");

        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)")
            .bind(&[m_unanchored, m_design][..]).execute(state.pg.pool()).await.unwrap();
    }

    // ── MCP-proxy ↔ daemon seam ──────────────────────────────────────────
    //
    // The sensei MCP proxy (crates/mcp) resolves a caller's project to its
    // *name* (see `resolve_project`) and drives these two knowledge endpoints
    // with it. This test crosses that seam by issuing the EXACT query shapes
    // the (fixed) proxy sends — `?project=<name>` — and asserting a genuine,
    // correctly-resolved, non-empty result.
    //
    // It goes RED on the original bug: the proxy sent a name but the context
    // handler required `project_id=<uuid>` and the rules handler required
    // `folder=<abs_path>`, so `?project=<name>` → HTTP 400 and every sensei
    // MCP knowledge lookup came back empty. Unit tests on each side stayed
    // green in isolation because nothing exercised the crossing.
    async fn get_json(app: &Router, uri: &str) -> (StatusCode, serde_json::Value) {
        let resp = app.clone().oneshot(
            Request::builder().uri(uri).body(Body::empty()).unwrap()
        ).await.unwrap();
        let status = resp.status();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice(&body).unwrap_or(serde_json::json!(null));
        (status, json)
    }

    #[tokio::test]
    async fn mcp_proxy_knowledge_context_and_rules_resolve_by_project_name() {
        let (app, state) = test_app().await;

        // Seed: a project + a git repo folder linked to it (so name→abs_path
        // resolves) + a project-scoped memory + a general-scoped rule.
        let pid = state.pg.ensure_test_project("mcp-seam").await.unwrap();
        let name = state.pg.get_project(&pid).await.unwrap().unwrap()["name"]
            .as_str().unwrap().to_string();               // "_test:mcp-seam"
        let abs_path = "/_test/mcp-seam/repo".to_string();
        let root_id = state.pg.add_watch_root(&abs_path, "mcp-seam", &serde_json::json!([]))
            .await.unwrap();
        state.pg.upsert_folder(&root_id, "git", "repo", "repo", &abs_path, None, Some(&pid))
            .await.unwrap();

        let mem_title = format!("_test:mcp-seam-memory-{}", std::process::id());
        state.pg.create_memory(Some(&pid), "project", None, "convention", &mem_title, "seam memory", None, None, None, None)
            .await.unwrap();
        // A general-scoped rule (namespace_id NULL, active) so the resolved
        // folder's ruleset is non-empty regardless of DB baseline.
        state.pg.create_memory(None, "global", None, "convention", "_test:mcp-seam-rule", "seam rule", None, None, None, None)
            .await.unwrap();

        // ── get_layered_context: proxy sends ?project=<name> ──
        let (st_name, ctx_by_name) =
            get_json(&app, &format!("/api/knowledge/context?project={name}")).await;
        assert_eq!(st_name, StatusCode::OK,
            "context by project NAME must resolve — this was HTTP 400 on the original bug");
        let has_title = |ctx: &serde_json::Value| ctx["memories"].as_array().unwrap().iter()
            .any(|m| m["title"].as_str() == Some(mem_title.as_str()));
        assert!(has_title(&ctx_by_name),
            "context resolved by name must contain the project-scoped memory (proves name→correct project uuid)");

        // Old contract still works AND resolves to the SAME project's memories.
        let (st_uuid, ctx_by_uuid) =
            get_json(&app, &format!("/api/knowledge/context?project_id={pid}")).await;
        assert_eq!(st_uuid, StatusCode::OK, "explicit project_id=<uuid> must keep working");
        assert!(has_title(&ctx_by_uuid),
            "name and uuid must resolve to the same project → same project memory present");

        // ── get_rules: proxy sends ?project=<name> ──
        let (st_rules, rules_by_name) =
            get_json(&app, &format!("/api/knowledge/rules?project={name}")).await;
        assert_eq!(st_rules, StatusCode::OK,
            "rules by project NAME must resolve the folder — this was HTTP 400 on the original bug");
        assert_eq!(rules_by_name["folder"], abs_path,
            "rules-by-name must resolve to the project's repo abs_path");
        assert!(rules_by_name["total"].as_i64().unwrap_or(0) >= 1,
            "the general-scoped rule must resolve for the project");

        // Old contract still works: ?folder=<abs_path>.
        let (st_folder, rules_by_folder) =
            get_json(&app, &format!("/api/knowledge/rules?folder={abs_path}")).await;
        assert_eq!(st_folder, StatusCode::OK, "explicit folder=<abs_path> must keep working");
        assert_eq!(rules_by_folder["folder"], abs_path);
    }

    // ── MCP↔daemon CONTRACT (anti-drift, table-driven) ───────────────────
    //
    // Generalizes the single seam test above into a table over the knowledge /
    // project MCP tools. For EACH tool we:
    //   1. shape the request with the REAL proxy code — `sensei_mcp::
    //      daemon_request_for(tool, args, cwd, Some(<seeded project name>))` —
    //      exactly what the sensei MCP binary sends;
    //   2. issue that shaped request against the in-process daemon router;
    //   3. assert HTTP 200 (never 400/404) AND that the SEEDED project's data
    //      genuinely comes back (name → the right project uuid / folders).
    //
    // This fails the suite on ANY future drift: a renamed daemon path, a changed
    // query key, a dropped tool, or a proxy that stops matching the daemon. The
    // per-crate unit suites stay green in isolation — only this crossing catches
    // the disagreement (the original context/rules bug class).

    /// Percent-encode a query value (space, `&`, `:`, … → %XX). Path segments are
    /// left raw: a `:` in a non-leading path segment is a valid pchar and axum's
    /// matchit captures the segment verbatim.
    fn qenc(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(b as char),
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    }

    /// Issue a lib-shaped [`sensei_mcp::DaemonRequest`] against the in-process
    /// router — the same translation the binary's `send_daemon_request` does,
    /// minus the network. Returns `(status, raw daemon JSON)`.
    async fn send_shaped(app: &Router, req: &sensei_mcp::DaemonRequest) -> (StatusCode, serde_json::Value) {
        let mut uri = req.path.clone();
        if !req.query.is_empty() {
            let qs: Vec<String> = req.query.iter()
                .map(|(k, v)| format!("{}={}", qenc(k), qenc(v)))
                .collect();
            uri.push('?');
            uri.push_str(&qs.join("&"));
        }
        let method = match req.method {
            sensei_mcp::HttpMethod::Get => "GET",
            sensei_mcp::HttpMethod::Post => "POST",
            sensei_mcp::HttpMethod::Put => "PUT",
            sensei_mcp::HttpMethod::Delete => "DELETE",
        };
        let mut builder = Request::builder().method(method).uri(&uri);
        let body = match &req.body {
            Some(b) => {
                builder = builder.header("content-type", "application/json");
                Body::from(b.to_string())
            }
            None => Body::empty(),
        };
        let request = builder.body(body).unwrap();
        let resp = app.clone().oneshot(request).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::json!(null));
        (status, json)
    }

    #[tokio::test]
    async fn mcp_proxy_knowledge_and_project_tools_contract() {
        use sensei_mcp::daemon_request_for;
        let (app, state) = test_app().await;

        // Unique suffix so parallel test runs on the shared DB never collide.
        let uniq = uuid::Uuid::new_v4().simple().to_string();
        let short = uniq[..8].to_string();

        // Seed: a project + ONE git-repo folder whose NAME equals the project
        // name (so both the project-scoped resolvers AND the folder-name
        // resolvers — get_file_tags — land on it) + a project memory + a global
        // rule + code nodes + a tagged file + a discoverable command.
        let pid = state.pg.ensure_test_project(&format!("contract-{short}")).await.unwrap();
        let name = state.pg.get_project(&pid).await.unwrap().unwrap()["name"]
            .as_str().unwrap().to_string();                    // "_test:contract-XXXX"
        let abs_path = format!("/_test/contract-{short}/repo");
        let root_id = state.pg.add_watch_root(&abs_path, &format!("contract-{short}"), &serde_json::json!([]))
            .await.unwrap();
        // Folder name == project name: get_file_tags looks up folders.name.
        let folder_id = state.pg
            .upsert_folder(&root_id, "git", &name, "repo", &abs_path, None, Some(&pid))
            .await.unwrap();

        let mem_title = format!("_test:contract-mem-{short}");
        state.pg.create_memory(Some(&pid), "project", None, "convention", &mem_title, "seam memory", None, None, None, None)
            .await.unwrap();
        let rule_title = format!("_test:contract-rule-{short}");
        state.pg.create_memory(None, "global", None, "convention", &rule_title, "seam rule", None, None, None, None)
            .await.unwrap();

        let fn_name = format!("contract_fn_{short}");
        state.pg.upsert_node(&folder_id, "function", &fn_name, "src/lib.rs",
            None, Some(&format!("fn {fn_name}()")), Some(1), Some(5)).await.unwrap();
        let struct_name = format!("ContractType{short}");
        state.pg.upsert_node(&folder_id, "struct", &struct_name, "src/lib.rs",
            None, Some(&format!("struct {struct_name}")), Some(7), Some(9)).await.unwrap();

        // A tagged `file` node so get_patterns (→ get_file_tags) resolves a hit.
        let file_path = "src/widget.rs".to_string();
        let file_id = state.pg.upsert_node(&folder_id, "file", "widget.rs", &file_path,
            None, None, None, None).await.unwrap();
        let tag = "route";
        sqlx_core::query::query("UPDATE sensei.nodes SET tags = $2 WHERE id = $1")
            .bind(file_id).bind(vec![tag.to_string()])
            .execute(state.pg.pool()).await.unwrap();

        // A discoverable command so get_commands resolves a hit.
        state.pg.replace_folder_commands(&folder_id, "npm", Some("package.json"),
            &[("test".to_string(), "cargo test".to_string(), Some("test"))]).await.unwrap();

        // cwd is only consulted by the rules FALLBACK (no project) + governance;
        // here a project always resolves, so its value is inert.
        let cwd = "/tmp/contract-cwd";

        // ── Direct Chunk-A guards: the request SHAPE the proxy produces ──────
        // These go RED on the exact original bug (name sent as project_id /
        // cwd sent as folder) regardless of daemon tolerance.
        let ctx_req = daemon_request_for("get_layered_context", &serde_json::json!({}), cwd, Some(&name)).unwrap();
        assert!(ctx_req.query.iter().any(|(k, v)| k == "project" && v == &name),
            "context proxy must send ?project=<name>");
        assert!(!ctx_req.query.iter().any(|(k, _)| k == "project_id"),
            "context proxy must NOT send the name as project_id (the Chunk-A bug shape)");
        let rules_req = daemon_request_for("get_rules", &serde_json::json!({}), cwd, Some(&name)).unwrap();
        assert!(rules_req.query.iter().any(|(k, v)| k == "project" && v == &name),
            "rules proxy must send ?project=<name>, not folder=<cwd> (the Chunk-A bug shape)");

        // ── The contract table ──────────────────────────────────────────────
        // Each entry: (tool, caller args, discriminating check on the daemon
        // body proving the SEEDED project resolved). The check returns Err with
        // a reason so a failure names the offending tool.
        type Check = Box<dyn Fn(&serde_json::Value) -> Result<(), String>>;
        let arr_has = |v: &serde_json::Value, key: &str, field: &str, want: &str| -> bool {
            v[key].as_array().map(|a| a.iter().any(|e| e[field].as_str() == Some(want))).unwrap_or(false)
        };
        let cases: Vec<(&str, serde_json::Value, Check)> = vec![
            ("get_layered_context", serde_json::json!({}), Box::new({
                let t = mem_title.clone();
                move |b| if arr_has(b, "memories", "title", &t) { Ok(()) }
                         else { Err(format!("seeded project memory '{t}' absent from context")) }
            })),
            ("get_rules", serde_json::json!({}), Box::new({
                let ap = abs_path.clone();
                move |b| if b["folder"].as_str() == Some(&ap) && b["total"].as_i64().unwrap_or(0) >= 1 { Ok(()) }
                         else { Err(format!("rules folder={:?} total={:?} (want folder={ap}, total>=1)", b["folder"], b["total"])) }
            })),
            ("search", serde_json::json!({ "query": fn_name.clone() }), Box::new({
                let f = fn_name.clone();
                move |b| if arr_has(b, "functions", "name", &f) { Ok(()) }
                         else { Err(format!("search missing seeded function '{f}'")) }
            })),
            ("get_project_summary", serde_json::json!({}), Box::new({
                let n = name.clone();
                move |b| if b["project"]["name"].as_str() == Some(&n) && b["functions"].as_i64().unwrap_or(0) >= 1 { Ok(()) }
                         else { Err(format!("summary project={:?} functions={:?}", b["project"]["name"], b["functions"])) }
            })),
            ("get_project_conventions", serde_json::json!({}), Box::new(move |b| {
                if arr_has(b, "naming", "kind", "function") { Ok(()) }
                else { Err(format!("conventions missing function naming; got {}", b["naming"])) }
            })),
            ("get_patterns", serde_json::json!({ "pattern": tag }), Box::new({
                let fp = file_path.clone();
                move |b| if arr_has(b, "files", "file_path", &fp) { Ok(()) }
                         else { Err(format!("get_patterns missing tagged file '{fp}'; got {b}")) }
            })),
            ("get_duplicates", serde_json::json!({}), Box::new(move |b| {
                // No code embeddings are seeded in-process, so the cosine
                // self-join yields 0 rows — but the seam MUST resolve PROJECT
                // scope (name → folders), which is what drift would break.
                // NOTE (gap): actual duplicate detection needs embeddings and is
                // exercised by the daemon's own embedding tests, not here.
                if b["scope"].as_str() == Some("project") && b["folder_count"].as_i64().unwrap_or(0) >= 1 { Ok(()) }
                else { Err(format!("duplicates scope={:?} folder_count={:?} (want project scope over >=1 folder)", b["scope"], b["folder_count"])) }
            })),
            ("get_commands", serde_json::json!({}), Box::new(move |b| {
                if arr_has(b, "commands", "raw_name", "test") { Ok(()) }
                else { Err(format!("get_commands missing seeded 'test' command; got {}", b["commands"])) }
            })),
        ];

        for (tool, args, check) in &cases {
            let req = daemon_request_for(tool, args, cwd, Some(&name))
                .unwrap_or_else(|| panic!("daemon_request_for returned None for '{tool}' (project resolved) — the proxy stopped shaping a daemon request"));
            let (status, body) = send_shaped(&app, &req).await;
            assert_eq!(status, StatusCode::OK,
                "{tool}: expected HTTP 200, got {status} — proxy request shape ({} {}) drifted from the daemon.\nbody={body}",
                match req.method { sensei_mcp::HttpMethod::Get => "GET", sensei_mcp::HttpMethod::Post => "POST", sensei_mcp::HttpMethod::Put => "PUT", sensei_mcp::HttpMethod::Delete => "DELETE" },
                req.path);
            // The /api/mcp/call proxy answers 200 even for an unrecognized tool,
            // carrying {"error":"Unknown tool: …"} — treat that as drift too.
            if req.path == "/api/mcp/call" {
                assert!(body.get("error").and_then(|e| e.as_str()).is_none(),
                    "{tool}: daemon mcp proxy returned an error (tool-name drift?): {}", body["error"]);
            }
            check(&body).unwrap_or_else(|e| panic!("{tool}: request crossed the seam but did NOT genuinely resolve the seeded project: {e}"));
        }

        // ── Chunk-A bug class, at the DATA level: project=name ≡ project_id=uuid ──
        // Resolve context by the explicit UUID and assert it lands on the SAME
        // project → the same seeded memory. Proves name and uuid are interchangeable.
        let by_uuid = daemon_request_for(
            "get_layered_context",
            &serde_json::json!({ "project_id": pid.to_string() }),
            cwd, Some(&name),
        ).unwrap();
        assert!(by_uuid.query.iter().any(|(k, v)| k == "project_id" && v == &pid.to_string()),
            "explicit uuid must ride on ?project_id=");
        let (st_uuid, body_uuid) = send_shaped(&app, &by_uuid).await;
        assert_eq!(st_uuid, StatusCode::OK, "context by explicit uuid must keep working");
        assert!(body_uuid["memories"].as_array().unwrap().iter()
                .any(|m| m["title"].as_str() == Some(mem_title.as_str())),
            "project=name and project_id=uuid must resolve to the SAME project → same seeded memory");
    }

    // ── find_projects: folder-scoped project discovery crosses the seam ──────
    //
    // The MCP `find_projects` tool shapes `GET /api/projects?under=<path>`. This
    // proves the shape reaches the daemon (never 404/400) AND that the daemon's
    // path-boundary filter genuinely scopes: a project with a folder under the
    // path is present; the same project is absent for a sibling path that only
    // shares the textual prefix. Goes RED on any drift in the route/query key.
    #[tokio::test]
    async fn mcp_proxy_find_projects_scopes_by_under() {
        use sensei_mcp::daemon_request_for;
        let (app, state) = test_app().await;

        let short = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
        let base = format!("/_test/fp-{short}");
        let under = format!("{base}/repo");

        // Seed: a project with ONE folder under `base`.
        let pid = state.pg.ensure_test_project(&format!("fp-{short}")).await.unwrap();
        let name = state.pg.get_project(&pid).await.unwrap().unwrap()["name"]
            .as_str().unwrap().to_string();
        let root = state.pg.add_watch_root(&base, &format!("fp-{short}"), &serde_json::json!([]))
            .await.unwrap();
        state.pg.upsert_folder(&root, "git", &name, "repo", &under, None, Some(&pid))
            .await.unwrap();

        // Shape guard: no `under` arg → default to the call cwd.
        let dflt = daemon_request_for("find_projects", &serde_json::json!({}), &base, None).unwrap();
        assert_eq!(dflt.path, "/api/projects");
        assert!(dflt.query.iter().any(|(k, v)| k == "under" && v == &base),
            "find_projects with no `under` arg must default to the call cwd");

        let names = |b: &serde_json::Value| -> Vec<String> {
            b.as_array()
                .map(|a| a.iter().filter_map(|p| p["name"].as_str().map(str::to_string)).collect())
                .unwrap_or_default()
        };

        // Under `base` → the seeded project appears.
        let req = daemon_request_for(
            "find_projects", &serde_json::json!({ "under": base.clone() }), "/irrelevant/cwd", None,
        ).unwrap();
        let (status, body) = send_shaped(&app, &req).await;
        assert_eq!(status, StatusCode::OK,
            "find_projects must reach GET /api/projects?under=<path>; body={body}");
        assert!(names(&body).contains(&name),
            "a project with a folder under `{base}` must be in the scoped list");

        // Under a sibling that only shares the textual prefix → project is gone.
        let sib = daemon_request_for(
            "find_projects", &serde_json::json!({ "under": format!("{base}-other") }), "/x", None,
        ).unwrap();
        let (st_sib, body_sib) = send_shaped(&app, &sib).await;
        assert_eq!(st_sib, StatusCode::OK);
        assert!(!names(&body_sib).contains(&name),
            "sibling `{base}-other` must NOT include the project (path boundary)");

        state.pg.delete_project(&pid).await.unwrap();
    }

    // ── find_projects returns COMPACT folders (no `kind:'folder'` bloat) ─────
    //
    // The `?under=` call is the MCP find_projects discovery path. Its response
    // carried each project's FULL nested folder tree (hundreds of
    // `kind:'folder'` descendants) → ~72K chars for sensei, over the MCP client
    // token cap. The scoped call must now return ONLY repo-root folders
    // (git/standalone) while keeping the scalar summary fields; the un-`under`
    // app path is unchanged (full tree). Goes RED if the compact trim regresses.
    #[tokio::test]
    async fn find_projects_under_returns_compact_folders() {
        use sensei_mcp::daemon_request_for;
        let (app, state) = test_app().await;

        let short = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
        let base = format!("/_test/fpc-{short}");
        let name = format!("fpc-{short}");
        let pid = state.pg.ensure_test_project(&name).await.unwrap();
        let pname = state.pg.get_project(&pid).await.unwrap().unwrap()["name"]
            .as_str().unwrap().to_string();
        let root = state.pg.add_watch_root(&base, &name, &serde_json::json!([]))
            .await.unwrap();

        // One git repo root + many nested `kind:'folder'` descendants.
        state.pg.upsert_folder(&root, "git", &pname, "repo", &format!("{base}/repo"), None, Some(&pid))
            .await.unwrap();
        for i in 0..40 {
            state.pg.upsert_folder(
                &root, "folder", &format!("d{i}"), &format!("repo/src/d{i}"),
                &format!("{base}/repo/src/d{i}"), None, Some(&pid),
            ).await.unwrap();
        }

        // Scoped (find_projects) → compact folders.
        let req = daemon_request_for(
            "find_projects", &serde_json::json!({ "under": base.clone() }), "/x", None,
        ).unwrap();
        let (status, body) = send_shaped(&app, &req).await;
        assert_eq!(status, StatusCode::OK);
        let proj = body.as_array().unwrap().iter()
            .find(|p| p["name"].as_str() == Some(&pname))
            .expect("seeded project must be in the scoped list");

        let folders = proj["folders"].as_array().unwrap();
        assert_eq!(folders.len(), 1, "scoped response must carry ONLY the repo root, not the 40 descendants");
        assert!(
            folders.iter().all(|f| matches!(f["kind"].as_str(), Some("git") | Some("standalone"))),
            "no `kind:'folder'` rows may leak into the scoped response",
        );
        // Scalar summary fields still present.
        for k in ["id", "name", "maturity", "repos_count", "libs_count", "sessions7d"] {
            assert!(!proj[k].is_null(), "compact row must still carry `{k}`");
        }
        // Rough size guard: even a many-folder project's compact row stays small.
        let row_bytes = serde_json::to_vec(proj).unwrap().len();
        assert!(row_bytes < 4096, "compact project row should be small; was {row_bytes} bytes");

        // Un-`under` (the app path: GET /api/projects, no query) → full tree
        // unchanged. Exercise the real handler branch and assert the seeded
        // project still carries all 41 folders including `kind:'folder'` rows.
        let resp = app.clone().oneshot(
            Request::builder().uri("/api/projects").body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let all: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let app_proj = all.as_array().unwrap().iter()
            .find(|p| p["name"].as_str() == Some(&pname))
            .expect("un-scoped list must include the seeded project");
        let app_folders = app_proj["folders"].as_array().unwrap();
        assert_eq!(app_folders.len(), 41, "un-scoped (app) folder set keeps roots + descendants");
        assert!(
            app_folders.iter().any(|f| f["kind"].as_str() == Some("folder")),
            "app path must still expose the nested `kind:'folder'` descendants",
        );

        state.pg.delete_project(&pid).await.unwrap();
    }

    /// PRE-DEPLOY GATE (P0–P8) — ONE end-to-end pass of the WHOLE metrics pipeline
    /// on a single seeded project in `sensei_test`, exercised together in sequence:
    ///
    ///   scheduler enqueue contract → real `TaskQueue` → real `spawn_workers`
    ///   executor dispatch (`ComputeGroupMetrics`→compute, `ComputeHealth`→compute_health)
    ///   → the `blocked_by` health barrier → all six base computers → the retired
    ///   health barrier no-ops → the three read endpoints → FTR parity.
    ///
    /// DRIVE PATH: the strongest feasible one — the FULL worker queue. We replicate
    /// the metrics scheduler's `enqueue_metrics_pass` (that fn is private) using the
    /// public `Task`/`TaskKind`/`TaskQueue` API and the REAL active registry
    /// (`active_task_names()` minus the `health` barrier name), enqueue one
    /// `ComputeGroupMetrics` per active base group + one `blocked_by` `ComputeHealth`, then
    /// let real `spawn_workers` drain the graph through the actual executor dispatch.
    /// This proves the health barrier only releases after its base metrics land.
    ///
    /// RETIRED composite: `project_health` carries a past `effective_until` in the
    /// registry (seeded from the catalog via `import_metrics`), so it is inactive. The
    /// `ComputeHealth` barrier is still enqueued + blocked + released, but
    /// `compute_health` resolves no active composite id and writes NO `project_health`
    /// row — the barrier lands as a harmless no-op (honest-empty, never fabricated).
    /// So exactly the 11 base rows are stored, and the registry/values endpoints do NOT
    /// carry a composite. The `quality` group (`duplication_ratio`/`module_quality`) is
    /// GIT-worktree + `qlty`-sourced: the seed repo has no committed `.qlty` config, so
    /// the scan misses → honest-empty (no quality rows) — the group is exercised end-to-
    /// end but writes nothing here (its computer is unit-tested with an injected scan).
    /// Seed values (still exercised by the base computers):
    ///   ftr 0.75 · rework_ratio 0.5 · throughput 4 · churn_rate 2 · churn_concentration
    ///   4/6 · rework_density 0.25 · interruption_rate 0.4 ·
    ///   run_completion 0.8 · memory_promotion 0.5 · unused_tools 0 · time_to_useful_result.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn metrics_pipeline_end_to_end() {
        // Seeds corrections and asserts on `memory_promotion`, whose denominator
        // counts them. A sibling test prunes that table by keep-set (table-wide
        // by design), which silently deleted these rows mid-run and turned
        // 2/4 into 2/2. Share the lock rather than pretend it can be scoped.
        let _serialised = crate::tasks::test_support::CORRECTIONS_TABLE_LOCK.enter();
        use crate::tasks::executor::spawn_workers;
        use crate::tasks::handlers::metrics::HEALTH_TASK_NAME;
        use crate::tasks::test_support::{
            cleanup_metrics_fixture, git_commit_on_day, make_ctx, purge_assistant_events,
            purge_corrections, purge_runs, seed_assistant_event, seed_correction,
            seed_detected_pattern, seed_file_node, seed_git_project_folder, seed_memory,
            seed_metrics_client_session, seed_metrics_session, seed_metrics_turn,
            seed_pattern_with_instances, seed_run,
        };
        use crate::tasks::{Task, TaskKind};
        use sqlx_core::query_as::query_as;
        use std::collections::HashMap;

        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;

        let (app, state) = test_app().await;
        let pg = &state.pg;
        let uniq = uuid::Uuid::new_v4();
        // The folder's abs_path is a REAL on-disk git repo so the git-sourced churn
        // computer (via project_root_path) has a repo to read. KEEP `repo` bound —
        // dropping the TempDir would delete the repo mid-test.
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        let folder_abs = repo.path().to_string_lossy().to_string();
        let pid_str = pid.to_string();

        // no-FK client-session ids / correction signatures (explicit purge).
        let csid_auto = format!("_test:e2e-auto:{uniq}");
        let sig_a = format!("_test:e2e-corr-a:{uniq}");
        let sig_b = format!("_test:e2e-corr-b:{uniq}");

        // Idempotent pre-clean of the tables that never cascade (guards a prior
        // crashed run that leaked rows under these ids).
        purge_assistant_events(pg, &[&csid_auto]).await;
        purge_corrections(pg, &[&sig_a, &sig_b]).await;
        purge_runs(pg, &[&pid]).await;

        let ts = chrono::Utc::now() - chrono::Duration::hours(2); // in-window, fixed day
        let now = chrono::Utc::now();

        // ── session_outcomes: 3 first-try + 1 corrected + 1 in-flight (excluded) ──
        for _ in 0..3 {
            let sid = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
            seed_metrics_turn(pg, &sid, 2, ts).await; // 3×2 = 6 first-try tool-calls
        }
        let corrected = seed_metrics_session(pg, &fid, &pid, Some("corrected"), Some(false), 2, ts).await;
        seed_metrics_turn(pg, &corrected, 6, ts).await; // 6 corrected tool-calls
        let inflight = seed_metrics_session(pg, &fid, &pid, None, None, 0, ts).await; // outcome NULL
        seed_metrics_turn(pg, &inflight, 5, ts).await; // must be excluded from every metric
        // → ftr 3/4 = 0.75 · rework 6/12 = 0.5 · throughput 4

        // ── churn: a git commit today (the git-sourced churn source) + file nodes
        //    + a rework flag. One commit changing a.rs (+4 lines) and b.rs (+2 lines)
        //    → 2 distinct files (churn_rate), line-churn a=4/b=2 (concentration). ──
        let churn_day = ts.format("%Y-%m-%d").to_string(); // today (in the rolling window)
        git_commit_on_day(repo.path(), &churn_day, &[("a.rs", "1\n2\n3\n4\n"), ("b.rs", "1\n2\n")]);
        for f in ["a.rs", "b.rs", "c.rs", "d.rs"] {
            seed_file_node(pg, &fid, &format!("{folder_abs}/{f}")).await;
        }
        seed_detected_pattern(pg, &pid, Some(&fid), "rework: a.rs", true).await;
        // → churn_rate 2 (a.rs,b.rs) · churn_concentration 4/6 (top-1 a.rs=4 / total 6) · rework_density 1/4 = 0.25

        // ── quality (duplication_ratio / module_quality): GIT-worktree + qlty-sourced.
        //    Config-pinning (P-B) injects the fixed ruler into the scanned worktree, so
        //    the seed repo's committed files ARE measured when the qlty CLI is present
        //    (real rows) and honest-empty when it is absent — asserted (qlty-gated) below. ──

        // ── autonomy: client-session-linked assistant events + runs ──
        seed_metrics_client_session(pg, &fid, &pid, &csid_auto, ts).await;
        for _ in 0..4 {
            seed_assistant_event(pg, &csid_auto, "Stop", ts).await;
        }
        for _ in 0..10 {
            seed_assistant_event(pg, &csid_auto, "UserPromptSubmit", ts).await;
        }
        for _ in 0..4 {
            seed_run(pg, &pid, "done", ts).await;
        }
        seed_run(pg, &pid, "crashed", ts).await;
        // → interruption_rate 4/10 = 0.4 · run_completion 4/5 = 0.8 · false_crash_rate NONE

        // ── knowledge: in-window memories + eligible patterns + eligible corrections ──
        seed_memory(pg, &pid, now - chrono::Duration::hours(2)).await;
        seed_memory(pg, &pid, now - chrono::Duration::hours(3)).await;
        seed_pattern_with_instances(pg, &pid, Some(&fid), &format!("kp1-{uniq}"), true, 3).await;
        seed_pattern_with_instances(pg, &pid, Some(&fid), &format!("kp2-{uniq}"), true, 4).await;
        seed_correction(pg, &sig_a, 3, &[pid]).await;
        seed_correction(pg, &sig_b, 5, &[pid]).await;
        // → memory_promotion 2/(2 patterns + 2 corrections) = 2/4 = 0.5

        // (tool: the `unused_tools` snapshot is RETIRED — tool usage is now the
        //  repo-grain `activity.tool_usage_by_repository` view over the raw event
        //  stream, not a computed metric, so nothing is seeded/asserted for it here.)

        // ── DRIVE: replicate the scheduler's enqueue, run the REAL worker pool ──
        let ctx = make_ctx().await;
        // `active_task_names()` is a GLOBAL registry read, and the health roll-up
        // tests seed ACTIVE synthetic metrics (`_test:health:<uuid>:…`) with their
        // own task_name. Those make the parent fan out extra children, and — worse —
        // the count moves between this read and the parent's own read when a sibling
        // test seeds or purges concurrently. Counting them made the drain assertion
        // race: observed `pending=0 blocked=0 running=0 completed=40` with the graph
        // fully drained and only the arithmetic wrong.
        //
        // Depend on the REAL base groups only. Synthetic ones can add tasks but never
        // remove them, so `completed >= expected` stays a valid floor either way.
        let base_names: Vec<String> = pg
            .active_task_names()
            .await
            .unwrap()
            .into_iter()
            .filter(|t| t != HEALTH_TASK_NAME) // `health` is the ComputeHealth kind, not a base group
            .filter(|t| !t.starts_with("_test:")) // another test's fixture, not a shipped group
            .collect();
        for g in ["session_outcomes", "churn", "quality", "autonomy", "knowledge", "coverage"] {
            assert!(base_names.iter().any(|t| t == g), "base group `{g}` is active in the registry: {base_names:?}");
        }
        // Enqueue the per-project PARENT only: ComputeProjectMetrics freezes one
        // frozen as_of, fans out one ComputeGroupMetrics per active base group, and
        // enqueues the ComputeHealth barrier blocked on them — the whole engine graph.
        ctx.queue.enqueue(Task::new(TaskKind::ComputeProjectMetrics, &pid_str, "")).await;
        // parent (1) + one child per active base group + the health barrier (1).
        let expected_tasks = base_names.len() + 2;

        // Only the parent is runnable up front; the group children + the health
        // barrier are enqueued when the parent runs.
        let pre = ctx.queue.status().await;
        assert_eq!(pre.pending, 1, "just the ComputeProjectMetrics parent is pending up front");

        spawn_workers(ctx.clone(), 2);

        // Poll until the whole graph drains through the real executor (bounded — a
        // stuck barrier or a failing handler fails loud instead of hanging forever).
        let mut drained = false;
        for _ in 0..800 {
            let s = ctx.queue.status().await;
            if s.pending == 0 && s.blocked == 0 && s.running == 0 && s.completed >= expected_tasks {
                drained = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        let fs = ctx.queue.status().await;
        assert!(
            drained,
            "metrics graph must drain through the real executor (barrier released after its ComputeGroupMetrics); \
             pending={} blocked={} running={} completed={}",
            fs.pending, fs.blocked, fs.running, fs.completed,
        );

        // ── STORE: every seeded group produced ≥1 project-scope daily row ──
        let rows = pg.get_project_metrics(&pid).await.unwrap();
        let mut val: HashMap<String, f64> = HashMap::new();
        let mut props: HashMap<String, serde_json::Value> = HashMap::new();
        let mut mtype: HashMap<String, String> = HashMap::new();
        for r in &rows {
            val.insert(r.metric.clone(), r.value);
            props.insert(r.metric.clone(), r.props.clone());
            mtype.insert(r.metric.clone(), r.metric_type.clone());
        }
        let get = |k: &str| {
            *val.get(k)
                .unwrap_or_else(|| panic!("BLOCKER: group produced NO project daily row for metric `{k}`"))
        };

        assert!(close(get("ftr"), 0.75), "ftr = 3/4 = 0.75 (got {})", get("ftr"));
        assert!(close(get("rework_ratio"), 0.5), "rework_ratio = 6/12 = 0.5 (got {})", get("rework_ratio"));
        assert!(close(get("throughput"), 4.0), "throughput = 4 measurable sessions (got {})", get("throughput"));
        assert!(close(get("churn_rate"), 2.0), "churn_rate = 2 distinct files changed by the day's commit (got {})", get("churn_rate"));
        assert!(close(get("churn_concentration"), 4.0 / 6.0), "churn_concentration = top-file line-churn 4 / total 6 (got {})", get("churn_concentration"));
        assert!(close(get("rework_density"), 0.25), "rework_density = 1/4 = 0.25 (got {})", get("rework_density"));
        // duplication_ratio / module_quality (quality group) are GIT-worktree + qlty-
        // sourced; config-pinning measures them when the qlty CLI is present — asserted
        // (qlty-gated) below, alongside the row count.
        assert!(close(get("interruption_rate"), 0.4), "interruption_rate = 4/10 = 0.4 (got {})", get("interruption_rate"));
        assert!(close(get("run_completion"), 0.8), "run_completion = 4/5 = 0.8 (got {})", get("run_completion"));
        assert!(close(get("memory_promotion"), 0.5), "memory_promotion = 2/4 = 0.5 (got {})", get("memory_promotion"));
        // (unused_tools RETIRED — replaced by the tool_usage_by_repository view; not computed.)
        // context_pressure_rate: 0 of the 4 measurable sessions carried a context-
        // pressure trouble signal → 0/4 = 0.0 (an honest zero — a real row, not fabricated).
        assert!(close(get("context_pressure_rate"), 0.0), "context_pressure_rate = 0/4 = 0.0 (no pressure signals seeded) (got {})", get("context_pressure_rate"));

        // 11 base metric rows; no composite. The five session_outcomes metrics
        // (ftr, rework_ratio, throughput, time_to_useful_result, context_pressure_rate),
        // churn's three (churn_rate, churn_concentration, rework_density), autonomy's two
        // (interruption_rate, run_completion; false_crash_rate is declared-but-uncomputed),
        // and memory_promotion. unused_tools is RETIRED (→ tool_usage_by_repository view)
        // and coverage is honest-empty here (no lcov in the seed repo). project_health is
        // RETIRED, so the health barrier wrote no composite row.
        //
        // The quality group (duplication_ratio/module_quality) is now CONFIG-PINNED
        // (P-B): the pinned ruler is injected into the scanned worktree, so on a host
        // WITH the qlty CLI it measures the seed repo's committed files (2 more rows);
        // WITHOUT qlty it is honest-empty. Gate on qlty so the E2E passes both locally
        // and on a bare CI — never a fabricated score either way.
        let base_rows = 11;
        let qlty_present = std::process::Command::new("qlty").arg("--version").output()
            .map(|o| o.status.success()).unwrap_or(false);
        if qlty_present {
            assert_eq!(rows.len(), base_rows + 2, "base metrics + config-pinned duplication_ratio + module_quality");
            assert!(val.contains_key("duplication_ratio"), "config-pinning → duplication_ratio measured (qlty present)");
            assert!(val.contains_key("module_quality"), "config-pinning → module_quality measured (qlty present)");
        } else {
            assert_eq!(rows.len(), base_rows, "base metrics only (quality honest-empty without the qlty CLI)");
            assert!(!val.contains_key("duplication_ratio"), "duplication_ratio honest-empty (no qlty CLI — never fabricated)");
            assert!(!val.contains_key("module_quality"), "module_quality honest-empty (no qlty CLI — never fabricated)");
        }

        // ── RETIRED HEALTH: the barrier ran (drained above) but wrote NO composite ──
        // project_health carries a past effective_until in the registry, so it is
        // inactive: absent from the active `health` task, and `compute_health` no-ops.
        let active = pg.active_metrics().await.unwrap();
        assert!(
            !active.iter().any(|m| m.key == "project_health"),
            "project_health is retired → absent from the active registry",
        );
        assert!(!val.contains_key("project_health"),
            "no project_health value row: the retired composite is never computed (honest-empty, not a fabricated score)");
        let (ph_rows,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'project_health'",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(ph_rows, 0, "retired project_health wrote no project_metrics row");

        // ── NEVER-FABRICATE: every ratio/pct row carries numerator + denominator ──
        for r in &rows {
            if r.metric_type == "ratio" || r.metric_type == "pct" {
                assert!(r.props.get("numerator").and_then(|v| v.as_i64()).is_some(),
                    "ratio/pct `{}` carries props.numerator: {}", r.metric, r.props);
                assert!(r.props.get("denominator").and_then(|v| v.as_i64()).is_some(),
                    "ratio/pct `{}` carries props.denominator: {}", r.metric, r.props);
            }
        }
        // Honest-absence spot-check: a metric in a seeded group that is deliberately
        // NOT computed (autonomy's false_crash_rate — NEEDS_CONTEXT) writes NO row.
        let (fcr_rows,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'false_crash_rate'",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(fcr_rows, 0, "honest-absence: false_crash_rate produced NO row (never fabricated)");
        assert!(!val.contains_key("false_crash_rate"), "false_crash_rate is not in the read surface");

        // ── ENDPOINT 1: GET /api/metrics/registry serves every ACTIVE metric + facets ──
        let (st, body) = req(app.clone(), "GET", "/api/metrics/registry", None).await;
        assert_eq!(st, StatusCode::OK, "{body}");
        let reg = body["metrics"].as_array().expect("registry metrics array");
        for k in [
            "ftr", "rework_ratio", "throughput", "time_to_useful_result", "churn_rate",
            "churn_concentration", "rework_density", "duplication_ratio", "interruption_rate",
            "run_completion", "memory_promotion",
        ] {
            let m = reg.iter().find(|m| m["key"].as_str() == Some(k))
                .unwrap_or_else(|| panic!("registry endpoint serves `{k}`"));
            assert!(m["purpose"].as_str().is_some_and(|s| !s.is_empty()), "`{k}` carries a purpose facet");
            assert!(m["direction"].as_str().is_some_and(|s| !s.is_empty()), "`{k}` carries a direction facet");
        }
        // The retired composite + the retired unused_tools are served by NEITHER the
        // registry (active-only) …
        assert!(!reg.iter().any(|m| m["key"].as_str() == Some("project_health")),
            "registry endpoint does NOT serve the retired project_health");
        assert!(!reg.iter().any(|m| m["key"].as_str() == Some("unused_tools")),
            "registry endpoint does NOT serve the retired unused_tools (→ tool_usage_by_repository view)");

        // ── ENDPOINT 2: GET /api/projects/{id}/metrics returns the seeded data ──
        let (st, body) = req(app.clone(), "GET", &format!("/api/projects/{pid}/metrics"), None).await;
        assert_eq!(st, StatusCode::OK, "{body}");
        let ms = body["metrics"].as_array().expect("project metrics array");
        assert_eq!(ms.len(), if qlty_present { base_rows + 2 } else { base_rows },
            "endpoint returns the latest-per-metric rows (base + config-pinned quality when qlty present; no retired composite)");
        // … nor the values endpoint: the retired project_health has no row to serve.
        assert!(!ms.iter().any(|m| m["metric"].as_str() == Some("project_health")),
            "values endpoint does NOT carry the retired project_health");
        let e_ftr = ms.iter().find(|m| m["metric"].as_str() == Some("ftr")).expect("ftr present on the endpoint");
        assert!(close(e_ftr["value"].as_f64().unwrap(), 0.75), "endpoint ftr = 0.75");
        assert_eq!(e_ftr["direction"].as_str(), Some("higher_better"), "facet: ftr direction attached");
        assert!(e_ftr["purpose"].as_str().is_some_and(|s| !s.is_empty()), "facet: ftr purpose attached");
        assert!(e_ftr["props"]["numerator"].as_i64().is_some() && e_ftr["props"]["denominator"].as_i64().is_some(),
            "endpoint ftr carries numerator/denominator facets");
        for m in ms {
            let t = m["metric_type"].as_str().unwrap_or("");
            if t == "ratio" || t == "pct" {
                assert!(m["props"]["numerator"].as_i64().is_some(),
                    "endpoint ratio/pct `{}` carries props.numerator", m["metric"]);
                assert!(m["props"]["denominator"].as_i64().is_some(),
                    "endpoint ratio/pct `{}` carries props.denominator", m["metric"]);
            }
        }

        // ── ENDPOINT 3: GET /api/projects/{id}/metrics/{key}?grain=weekly re-derives Σnum/Σden ──
        let (st, body) = req(app.clone(), "GET",
            &format!("/api/projects/{pid}/metrics/rework_ratio?grain=weekly"), None).await;
        assert_eq!(st, StatusCode::OK, "{body}");
        assert_eq!(body["grain"], "weekly");
        assert_eq!(body["metric"].as_str(), Some("rework_ratio"));
        let series = body["series"].as_array().expect("weekly series array");
        assert!(!series.is_empty(), "the weekly series has a point");
        let wk_val = series.last().unwrap()["value"].as_f64().expect("weekly value");
        let rr_num = props["rework_ratio"]["numerator"].as_f64().unwrap();
        let rr_den = props["rework_ratio"]["denominator"].as_f64().unwrap();
        assert!(close(wk_val, rr_num / rr_den), "weekly rework_ratio re-derives Σnum/Σden = {rr_num}/{rr_den}");
        assert!(close(wk_val, 0.5), "weekly rework_ratio = 0.5");

        // ── FTR PARITY: store daily == get_ftr_daily == direct base arithmetic; headline == Σnum/Σden ──
        let (direct_rate, direct_count): (f64, i64) = query_as(
            "SELECT avg(CASE WHEN s.ftr THEN 1.0 ELSE 0.0 END)::float8, count(*)::int8 \
               FROM activity.sessions s JOIN sensei.folders f ON f.id = s.folder_id \
              WHERE f.project_id = $1 AND s.outcome IS NOT NULL",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert!(close(direct_rate, 0.75) && direct_count == 4, "direct FTR base = 0.75 over 4 measurable sessions");
        assert!(close(get("ftr"), direct_rate), "store daily ftr == avg(case when ftr) over the measurable base");

        let ftr_daily = pg.get_ftr_daily(Some(&pid), 14).await.unwrap();
        assert_eq!(ftr_daily.len(), 1, "one daily ftr row for the project");
        assert!(close(ftr_daily[0]["ftr_rate"].as_f64().unwrap(), 0.75), "get_ftr_daily ftr_rate = 0.75");
        assert_eq!(ftr_daily[0]["session_count"].as_i64(), Some(4), "get_ftr_daily session_count = 4");
        assert!(close(ftr_daily[0]["ftr_rate"].as_f64().unwrap(), get("ftr")),
            "get_ftr_daily (store-backed) == the store daily ftr value");

        let headline = pg.get_project_ftr(&pid).await.unwrap();
        assert!(close(headline["ftr14d"].as_f64().unwrap(), 0.75), "get_project_ftr ftr14d == Σnum/Σden = 3/4 = 0.75");
        assert_eq!(headline["sessions7d"].as_i64(), Some(4), "get_project_ftr sessions7d == Σdenominator = 4");
        let rate = pg.get_project_ftr_rate(&pid).await.unwrap().expect("ftr rate present for a project with ftr rows");
        assert!(close(rate, 0.75), "get_project_ftr_rate == 0.75 headline");

        // ── CLEANUP: purge the no-FK tables, then the project + folder (cascades) ──
        purge_runs(pg, &[&pid]).await;
        purge_assistant_events(pg, &[&csid_auto]).await;
        purge_corrections(pg, &[&sig_a, &sig_b]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }
}
