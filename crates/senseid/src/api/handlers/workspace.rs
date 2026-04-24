use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Json, Sse, sse::Event},
};
use serde::Deserialize;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use crate::api::state::AppState;
use crate::types::Repo;

// ── Repos CRUD ──────────────────────────────────────────────────────────────

pub(crate) async fn list_projects(State(state): State<AppState>) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    // TODO: Old Store returned Vec<Repo>; PgStore returns folders with kind='git'/'subtree' as JSON.
    // Callers may need updating for the new shape.
    state.pg.list_repositories().await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize)]
pub(crate) struct CreateProjectBody {
    #[serde(rename = "repoId")]
    repo_id: String,
    name: Option<String>,
    path: String,
}

pub(crate) async fn create_project(
    State(state): State<AppState>,
    Json(body): Json<CreateProjectBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: PgStore has no repo CRUD — repos are now folders with kind='git'.
    // Use upsert_folder once a root UUID is available. For now, keep old store as bridge.
    let s = state.store.lock().await;
    let repo = Repo {
        repo_id: body.repo_id,
        name: body.name.unwrap_or_else(|| body.path.split('/').next_back().unwrap_or("unknown").to_string()),
        path: body.path,
        remote_url: None,
        indexed_at: None,
        last_error: None,
        duplicate_of: None,
        stack: vec![],
        libs: vec![],
        tags: vec![],
        status: "active".to_string(),
        project_id: None,
        role: "unknown".to_string(),
        label: None,
    };
    s.upsert_repo(&repo)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn update_project(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: PgStore has no repo CRUD — repos are now folders. Keep old store as bridge.
    let s = state.store.lock().await;
    let mut repo = s.get_repo(&repo_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(name) = body.get("name").and_then(|v| v.as_str()) {
        repo.name = name.to_string();
    }
    if let Some(status) = body.get("status").and_then(|v| v.as_str()) {
        repo.status = status.to_string();
    }

    s.upsert_repo(&repo)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn delete_project(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: PgStore has no repo CRUD — repos are now folders. Keep old store as bridge.
    let s = state.store.lock().await;
    s.delete_repo(&repo_id)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Exclude / Exclusions ────────────────────────────────────────────────────

pub(crate) async fn exclude_project(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: PgStore has no exclude_repo / exclusions table — repos are now folders.
    // Keep old store as bridge until folder-based exclusion is implemented.
    let store = state.store.lock().await;
    // Look up repo path before deleting
    let path = store.get_repo(&repo_id)
        .ok().flatten()
        .map(|p| p.path.clone())
        .unwrap_or_default();

    // Clear indexed data from graph
    {
        let graph = state.graph.lock().await;
        graph.delete_project_graph(&repo_id).ok();
    }

    // Exclude in store (adds to excluded_paths, deletes project)
    store.exclude_repo(&repo_id, &path)
        .map(|_| Json(serde_json::json!({"ok": true, "excluded": path})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn list_exclusions(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    // TODO: PgStore has no exclusions table. Keep old store as bridge.
    let store = state.store.lock().await;
    match store.list_exclusions() {
        Ok(exclusions) => Json(serde_json::json!({
            "exclusions": exclusions.iter().map(|(path, repo_id, at)| serde_json::json!({
                "path": path, "repo_id": repo_id, "excluded_at": at,
            })).collect::<Vec<_>>()
        })),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

pub(crate) async fn remove_exclusion(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: PgStore has no exclusions table. Keep old store as bridge.
    let store = state.store.lock().await;
    // Path comes URL-encoded from the route parameter
    let decoded = path.replace("%2F", "/").replace("%20", " ");
    store.remove_exclusion(&decoded)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Project Tags ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct TagBody {
    pub tag: String,
}

pub(crate) async fn add_project_tag(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
    Json(body): Json<TagBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: PgStore tags are a controlled vocabulary (tag, category), not per-entity.
    // Old Store used add_tag("repo", &repo_id, &tag). Need a join table or tags[] column.
    // Keep old store as bridge.
    let s = state.store.lock().await;
    s.add_tag("repo", &repo_id, &body.tag)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn remove_project_tag(
    State(state): State<AppState>,
    Path((repo_id, tag)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: PgStore tags are a controlled vocabulary, not per-entity. Keep old store as bridge.
    let s = state.store.lock().await;
    s.remove_tag("repo", &repo_id, &tag)
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Scan ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct ScanBody {
    pub root: String,
    #[serde(default = "default_depth")]
    pub _max_depth: u32,
}

fn default_depth() -> u32 { 4 }

pub(crate) async fn scan_folder(
    State(state): State<AppState>,
    Json(body): Json<ScanBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Expand ~ to home directory
    let expanded = if body.root.starts_with("~/") {
        dirs::home_dir()
            .map(|h| h.join(&body.root[2..]).to_string_lossy().to_string())
            .unwrap_or(body.root.clone())
    } else {
        body.root.clone()
    };
    let root_path = expanded.clone();
    let root = std::path::Path::new(&root_path);
    if !root.exists() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Enqueue ScanRoot task — runs asynchronously via task workers
    let task = crate::tasks::Task::new(
        crate::tasks::TaskKind::ScanRoot, "", &root_path,
    );
    let task_id = state.task_queue.enqueue(task).await;

    Ok(Json(serde_json::json!({"ok": true, "scanning": true, "taskId": task_id})))
}

/// Return project grouping suggestions from the last scan.
pub(crate) async fn scan_suggestions(State(state): State<AppState>) -> Json<serde_json::Value> {
    let suggestions = state.pg.get_config("solution_suggestions").await
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .unwrap_or(serde_json::json!([]));
    Json(suggestions)
}

/// List configured scan roots with their scan status.
pub(crate) async fn scan_roots(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut roots = state.pg.list_watch_roots().await.unwrap_or_default();

    // Enrich with repo count per root
    let repos = state.pg.list_repositories().await.unwrap_or_default();
    for root in &mut roots {
        let root_path = root["path"].as_str().unwrap_or("");
        let count = repos.iter().filter(|r| {
            r["abs_path"].as_str().unwrap_or("").starts_with(root_path)
        }).count();
        root["repos_found"] = serde_json::json!(count);
        root["scanned"] = serde_json::json!(count > 0);
    }

    Json(serde_json::json!(roots))
}

// ── Indexing ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct IndexBody {
    #[serde(rename = "repoId")]
    repo_id: String,
    #[serde(rename = "repoPath")]
    repo_path: String,
    #[serde(default)]
    _force: bool,
}

pub(crate) async fn index_project(
    State(state): State<AppState>,
    Json(body): Json<IndexBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Clear errors for this project (PgStore expects UUID)
    if let Ok(folder_id) = uuid::Uuid::parse_str(&body.repo_id) {
        state.pg.clear_index_errors(&folder_id).await.ok();
    }

    let task = crate::tasks::Task::new(
        crate::tasks::TaskKind::ProcessRepo, &body.repo_id, &body.repo_path,
    );
    let task_id = state.task_queue.enqueue(task).await;

    Ok(Json(serde_json::json!({
        "ok": true,
        "queued": true,
        "taskId": task_id,
        "repoId": body.repo_id,
    })))
}

pub(crate) async fn task_status(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let status = state.task_queue.status().await;
    let progress = state.task_queue.progress().await;
    Json(serde_json::json!({ "queue": status, "repos": progress }))
}

pub(crate) async fn index_progress_sse(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.task_queue.sender().subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|result| {
            match result {
                Ok(event) => {
                    let data = serde_json::to_string(&event).unwrap_or_default();
                    Some(Ok(Event::default().data(data)))
                }
                Err(_) => None,
            }
        });
    Sse::new(stream)
}

pub(crate) async fn task_progress_sse(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.task_queue.sender().subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|result| {
            match result {
                Ok(event) => {
                    let data = serde_json::to_string(&event).unwrap_or_default();
                    Some(Ok(Event::default().data(data)))
                }
                Err(_) => None,
            }
        });
    Sse::new(stream)
}

// ── Index Errors ────────────────────────────────────────────────────────────

pub(crate) async fn list_index_errors(State(state): State<AppState>) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    state.pg.get_index_errors(None).await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn list_repo_index_errors(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let folder_id = uuid::Uuid::parse_str(&repo_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.get_index_errors(Some(&folder_id)).await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Stop ────────────────────────────────────────────────────────────────────

pub(crate) async fn stop() -> Json<serde_json::Value> {
    tokio::spawn(async {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        std::process::exit(0);
    });
    Json(serde_json::json!({"ok": true}))
}
