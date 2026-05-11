use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Json, Sse, sse::Event},
};
use serde::Deserialize;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use crate::api::state::AppState;

// ── Repos CRUD ──────────────────────────────────────────────────────────────

pub(crate) async fn list_projects(State(state): State<AppState>) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    state.pg.list_repositories().await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Deserialize)]
pub(crate) struct CreateProjectBody {
    #[serde(rename = "repoId")]
    #[allow(dead_code)]
    repo_id: String,
    name: Option<String>,
    path: String,
}

pub(crate) async fn create_project(
    State(state): State<AppState>,
    Json(body): Json<CreateProjectBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let name = body.name.unwrap_or_else(|| body.path.split('/').next_back().unwrap_or("unknown").to_string());

    // Look up or create a watch root for the parent directory
    let parent_path = std::path::Path::new(&body.path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| body.path.clone());
    let root_id = state.pg.add_watch_root(&parent_path, "auto", &serde_json::json!([])).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let folder_id = state.pg.upsert_repo(&root_id, &name, &body.path).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"ok": true, "folderId": folder_id})))
}

pub(crate) async fn update_project(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Look up folder by name (old string repo_id)
    let folder = state.pg.get_repo_by_name(&repo_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let folder_id = folder["id"].as_str()
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build props from the update body
    let mut props = serde_json::Map::new();
    if let Some(name) = body.get("name").and_then(|v| v.as_str()) {
        props.insert("name".into(), serde_json::json!(name));
    }
    if let Some(status) = body.get("status").and_then(|v| v.as_str()) {
        props.insert("status".into(), serde_json::json!(status));
    }

    state.pg.set_folder_props(&folder_id, &serde_json::Value::Object(props)).await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn delete_project(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state.pg.delete_repo_by_name(&repo_id).await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Exclude / Exclusions ────────────────────────────────────────────────────

pub(crate) async fn exclude_project(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Look up folder path before deleting
    let folder = state.pg.get_repo_by_name(&repo_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let path = folder.as_ref()
        .and_then(|f| f["abs_path"].as_str())
        .unwrap_or_default()
        .to_string();

    // TODO: clear indexed nodes from PG when exclude is called

    // Delete the folder record (exclusions now handled by watcher)
    state.pg.delete_repo_by_name(&repo_id).await
        .map(|_| Json(serde_json::json!({"ok": true, "excluded": path})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn list_exclusions(
    State(_state): State<AppState>,
) -> Json<serde_json::Value> {
    // Exclusions are now handled by the watcher layer; no persistent exclusions table.
    Json(serde_json::json!({ "exclusions": [] }))
}

pub(crate) async fn remove_exclusion(
    State(_state): State<AppState>,
    Path(_path): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Exclusions are now handled by the watcher layer; no persistent exclusions table.
    Ok(Json(serde_json::json!({"ok": true})))
}

// ── Project Tags ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct TagBody {
    pub tag: String,
}

pub(crate) async fn add_project_tag(
    State(state): State<AppState>,
    Path(_repo_id): Path<String>,
    Json(body): Json<TagBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // PgStore tags are a controlled vocabulary (tag, category).
    // Register the tag in the vocabulary; per-entity tagging uses folder props.
    state.pg.add_tag(&body.tag, Some("repo")).await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) async fn remove_project_tag(
    State(state): State<AppState>,
    Path((_repo_id, tag)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // PgStore tags are a controlled vocabulary. Remove from vocabulary.
    state.pg.remove_tag(&tag).await
        .map(|_| Json(serde_json::json!({"ok": true})))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Scan ────────────────────────────────────────────────────────────────────

fn expand_tilde(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        sensei_bootstrap::home_dir()
            .join(stripped)
            .to_string_lossy()
            .to_string()
    } else {
        path.to_string()
    }
}

#[derive(Deserialize)]
pub(crate) struct AddRootBody {
    pub path: String,
}

/// Add a watch root to the DB immediately (synchronous) — does not start scanning.
/// The Scan page is responsible for calling POST /api/scan to trigger the actual scan.
pub(crate) async fn add_watch_root(
    State(state): State<AppState>,
    Json(body): Json<AddRootBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let expanded = expand_tilde(&body.path);
    if expanded.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let name = std::path::Path::new(&expanded)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("root")
        .to_string();
    let id = state.pg
        .add_watch_root(&expanded, &name, &serde_json::json!([]))
        .await
        .map_err(|e| {
            tracing::error!("add_watch_root: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(serde_json::json!({ "ok": true, "id": id, "path": expanded })))
}

/// Delete a watch root by ID.
pub(crate) async fn delete_watch_root(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.remove_watch_root(&uuid).await.map_err(|e| {
        tracing::error!("delete_watch_root: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

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
    if body.root.is_empty() {
        tracing::warn!("scan_folder: empty root path rejected");
        return Err(StatusCode::BAD_REQUEST);
    }
    let root_path = expand_tilde(&body.root);
    let root = std::path::Path::new(&root_path);
    if !root.exists() {
        return Ok(Json(serde_json::json!({"ok": false, "error": "path not found"})));
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
        crate::tasks::TaskKind::ProcessGitFolder, &body.repo_id, &body.repo_path,
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
