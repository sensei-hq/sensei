use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;

// ── Sessions ────────────────────────────────────────────────────────────────

pub(crate) async fn get_sessions_stub(
    State(state): State<AppState>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let store = state.store.lock().await;
    let repo_id = q.get("repoId").map(|s| s.as_str());
    let sessions = store.get_sessions(repo_id).unwrap_or_default();
    let total = sessions.len();
    let completed = sessions.iter().filter(|s| s["outcome"].as_str() == Some("completed")).count();
    Json(serde_json::json!({
        "stats": { "totalSessions": total, "completed": completed },
        "sessions": sessions,
        "toolUsage": [],
        "benchmarkPairs": []
    }))
}

pub(crate) async fn create_session(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let id = body["id"].as_str().unwrap_or(&uuid::Uuid::new_v4().to_string()).to_string();
    let repo_id = body["repoId"].as_str().unwrap_or("");
    let task = body["task"].as_str().unwrap_or("untitled");
    let store = state.store.lock().await;
    store.create_session(&id, repo_id, task).ok();
    Json(serde_json::json!({"ok": true, "id": id}))
}

pub(crate) async fn update_session_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let store = state.store.lock().await;
    store.update_session(
        &id,
        body["outcome"].as_str(),
        body["summary"].as_str(),
        body["cost"].as_f64(),
        body["tokensIn"].as_i64(),
        body["tokensOut"].as_i64(),
    ).ok();
    Json(serde_json::json!({"ok": true}))
}

// ── Events ──────────────────────────────────────────────────────────────────

pub(crate) async fn create_event(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let id = body["id"].as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let project = body["project"].as_str().unwrap_or("unknown");
    let session_id = body["session_id"].as_str();
    let event_type = match body["event_type"].as_str().or(body["type"].as_str()) {
        Some(t) => t,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "event_type is required"}))),
    };
    let data = body.get("data")
        .map(|d| serde_json::to_string(d).unwrap_or_default())
        .unwrap_or_else(|| "{}".to_string());

    let store = state.store.lock().await;
    match store.insert_event(&id, project, session_id, event_type, &data) {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({"ok": true, "id": id}))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok": false, "error": e.to_string()}))),
    }
}

#[derive(Deserialize)]
pub(crate) struct EventQuery {
    #[serde(rename = "type")]
    event_type: Option<String>,
    session: Option<String>,
    limit: Option<u32>,
}

pub(crate) async fn list_events(
    State(state): State<AppState>,
    Path(project): Path<String>,
    Query(q): Query<EventQuery>,
) -> Json<serde_json::Value> {
    let store = state.store.lock().await;
    let limit = q.limit.unwrap_or(50).min(500);
    match store.list_events(&project, q.event_type.as_deref(), q.session.as_deref(), limit) {
        Ok(events) => Json(serde_json::json!({"events": events, "count": events.len()})),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

// ── Workflow State ──────────────────────────────────────────────────────────

pub(crate) async fn get_workflow_state(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    let store = state.store.lock().await;
    match store.get_workflow_state(&project) {
        Ok(Some(ws)) => Json(ws),
        Ok(None) => Json(serde_json::json!({
            "project": project,
            "active_phase": null,
            "active_plan": null,
            "active_task": null,
            "active_issue": null,
            "last_checkpoint": null,
            "rules_hash": null,
        })),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

pub(crate) async fn update_workflow_state(
    State(state): State<AppState>,
    Path(project): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let store = state.store.lock().await;

    let result = store.upsert_workflow_state(
        &project,
        body["active_phase"].as_str(),
        body["active_plan"].as_str(),
        body["active_task"].as_str(),
        body["active_issue"].as_i64(),
        body["last_checkpoint"].as_str(),
        body["rules_hash"].as_str(),
    );

    if let Err(e) = result {
        return Json(serde_json::json!({"ok": false, "error": e.to_string()}));
    }

    // Sync to .sensei/state.yaml
    // Use explicit project_path from body, or look up from projects table
    let project_path = body["project_path"].as_str().map(String::from)
        .or_else(|| store.get_repo_path(&project).ok().flatten());
    if let Some(project_path) = project_path {
        let sensei_dir = std::path::Path::new(&project_path).join(".sensei");
        std::fs::create_dir_all(&sensei_dir).ok();
        let state_file = sensei_dir.join("state.yaml");

        // Read back the state we just wrote to get all fields
        if let Ok(Some(ws)) = store.get_workflow_state(&project) {
            let yaml = format!(
                "active_phase: {}\nactive_plan: {}\nactive_task: {}\nactive_issue: {}\nlast_checkpoint: {}\nrules_hash: {}\n",
                ws["active_phase"].as_str().unwrap_or("~"),
                ws["active_plan"].as_str().unwrap_or("~"),
                ws["active_task"].as_str().unwrap_or("~"),
                ws["active_issue"].as_i64().map(|n| n.to_string()).unwrap_or("~".to_string()),
                ws["last_checkpoint"].as_str().unwrap_or("~"),
                ws["rules_hash"].as_str().unwrap_or("~"),
            );
            std::fs::write(&state_file, yaml).ok();
        }
    }

    Json(serde_json::json!({"ok": true}))
}
