use axum::{
    routing::{get, post, put, delete},
    Router,
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

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health
        .route("/health", get(health::health))
        .route("/api/health/components", get(health::health_components))
        .route("/api/watcher/status", get(health::watcher_status))
        .route("/api/watcher/unregister", axum::routing::post(health::watcher_unregister))
        // Repos (individual git repos)
        .route("/api/repos", get(workspace::list_projects).post(workspace::create_project))
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
        .route("/api/projects/{id}", put(observatory::update_solution).delete(observatory::delete_solution))
        .route("/api/projects/{id}/repos", post(observatory::add_solution_repo))
        .route("/api/projects/{id}/repos/{repo_id}", delete(observatory::remove_solution_repo))
        .route("/api/projects/{id}/tags", post(observatory::add_solution_tag))
        .route("/api/projects/{id}/tags/{tag}", delete(observatory::remove_solution_tag))
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
        .route("/api/projects/{id}/graph", get(observatory::solution_graph))
        .route("/api/projects/{id}/roles", get(observatory::solution_roles))
        // Libraries
        .route("/api/libs", get(libraries::list_libs))
        .route("/api/libs/index", post(libraries::index_lib))
        .route("/api/libs/docs", get(libraries::search_lib_docs))
        .route("/api/libs/{name}/docs", get(libraries::get_lib_docs))
        .route("/api/libs/versions", get(libraries::get_dep_versions))
        // Unified query (desktop/MCP)
        .route("/api/query", post(query::unified_query))
        // MCP tool proxy
        .route("/api/mcp/tools", get(query::mcp_list_tools))
        .route("/api/mcp/call", post(query::mcp_call_tool))
        // Marketplace install (legacy — prefer /api/install endpoints)
        .route("/api/marketplace/install", post(config::marketplace_install))
        // ACP (AI Coding Platform) detection & configuration
        .route("/api/acp/detect", get(config::acp_detect))
        .route("/api/acp/configure", post(config::acp_configure))
        .route("/api/acp/remove", post(config::acp_remove))
        // Installer — hooks, skills, commands, install/remove
        .route("/api/install", post(config::install_all))
        .route("/api/install/hooks", post(config::install_hooks))
        .route("/api/install/item", post(config::install_single_item))
        .route("/api/install/item/remove", post(config::remove_single_item))
        .route("/api/install/catalog", get(config::get_catalog))
        .route("/api/install/installed", get(config::list_installed_items))
        .route("/api/remove", post(config::remove_all))
        // Config (user preferences)
        .route("/api/config", get(config::get_config).put(config::set_config_handler))
        .route("/api/config/{key}", get(config::get_config_key).delete(config::delete_config_key))
        // Sessions
        .route("/api/sessions", get(sessions::get_sessions_stub).post(sessions::create_session))
        .route("/api/sessions/{id}", put(sessions::update_session_handler))
        // Patterns
        .route("/api/patterns/{project}/detect", post(codebase::detect_patterns))
        .route("/api/patterns/{project}", get(codebase::list_patterns))
        .route("/api/patterns/{project}/match", get(codebase::match_pattern_handler))
        .route("/api/patterns/{project}/for/{symbol}", get(codebase::pattern_for_symbol))
        .route("/api/patterns/{project}/duplicates", get(codebase::find_duplicates_handler))
        .route("/api/patterns/{project}/conventions", get(codebase::project_conventions_handler))
        // Events
        .route("/api/events", post(sessions::create_event))
        .route("/api/events/{project}", get(sessions::list_events))
        // Metrics
        .route("/api/metrics/{project}", get(observatory::get_metrics))
        // Workflow state
        .route("/api/state/{project}", get(sessions::get_workflow_state).put(sessions::update_workflow_state))
        // Reset (clears all data)
        .route("/api/reset", post(config::reset_all))
        // Scan
        .route("/api/scan", post(workspace::scan_folder))
        .route("/api/scan/suggestions", get(workspace::scan_suggestions))
        .route("/api/scan/roots", get(workspace::scan_roots))
        // Stop
        .route("/stop", post(workspace::stop))
        .with_state(state)
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
        let state = Arc::new(SharedState {
            task_queue: Arc::new(TaskQueue::new()),
            pg: crate::db::pg_store::PgStore::connect_test().await.unwrap(),
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
        assert_eq!(json["ok"], true);
        assert_eq!(json["name"], "senseid");
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
