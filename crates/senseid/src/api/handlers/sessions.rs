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
    // PgStore uses list_sessions_by_folder(&Uuid, limit) instead of get_sessions(repo_id)
    let sessions = if let Some(folder_str) = q.get("repoId") {
        if let Ok(folder_id) = uuid::Uuid::parse_str(folder_str) {
            state.pg.list_sessions_by_folder(&folder_id, 50).await.unwrap_or_default()
        } else {
            vec![]
        }
    } else {
        // No folder filter — no PgStore equivalent for "all sessions".
        // TODO: Add list_all_sessions to PgStore if needed.
        vec![]
    };
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
    let folder_str = body["repoId"].as_str().unwrap_or("");
    let task = body["task"].as_str().unwrap_or("untitled");
    let acp_id = body["acpId"].as_str();

    let folder_id = match uuid::Uuid::parse_str(folder_str) {
        Ok(id) => id,
        Err(_) => return Json(serde_json::json!({"ok": false, "error": "invalid repoId (expected UUID)"})),
    };

    match state.pg.create_session(&folder_id, task, acp_id).await {
        Ok(id) => Json(serde_json::json!({"ok": true, "id": id})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
    }
}

pub(crate) async fn update_session_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let session_id = match uuid::Uuid::parse_str(&id) {
        Ok(uid) => uid,
        Err(_) => return Json(serde_json::json!({"ok": false, "error": "invalid session id (expected UUID)"})),
    };

    // PgStore complete_session expects: outcome, ftr (bool), turns (i32), corrections (i32)
    let outcome = body["outcome"].as_str().unwrap_or("completed");
    let ftr = body["ftr"].as_bool().unwrap_or(false);
    let turns = body["turns"].as_i64().unwrap_or(0) as i32;
    let corrections = body["corrections"].as_i64().unwrap_or(0) as i32;

    match state.pg.complete_session(&session_id, outcome, ftr, turns, corrections).await {
        Ok(_) => Json(serde_json::json!({"ok": true})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
    }
}

// ── Events ──────────────────────────────────────────────────────────────────

pub(crate) async fn create_event(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let event_type = match body["event_type"].as_str().or(body["type"].as_str()) {
        Some(t) => t,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "event_type is required"}))),
    };

    // PgStore insert_event expects session_uuid, folder_uuid, event_type, turn_number, json data
    let session_id = match body["session_id"].as_str().map(uuid::Uuid::parse_str) {
        Some(Ok(uid)) => uid,
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "session_id (UUID) is required"}))),
    };
    let folder_id = match body["project"].as_str().or(body["folder_id"].as_str()).map(uuid::Uuid::parse_str) {
        Some(Ok(uid)) => uid,
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "project/folder_id (UUID) is required"}))),
    };
    let turn_number = body["turn_number"].as_i64().map(|n| n as i32);
    let data = body.get("data").cloned().unwrap_or(serde_json::json!({}));

    match state.pg.insert_event(&session_id, &folder_id, event_type, turn_number, &data).await {
        Ok(id) => (StatusCode::CREATED, Json(serde_json::json!({"ok": true, "id": id}))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok": false, "error": e}))),
    }
}

#[derive(Deserialize)]
pub(crate) struct EventQuery {
    #[serde(rename = "type")]
    event_type: Option<String>,
    session: Option<String>,
    #[allow(dead_code)]
    limit: Option<u32>,
}

pub(crate) async fn list_events(
    State(state): State<AppState>,
    Path(project): Path<String>,
    Query(q): Query<EventQuery>,
) -> Json<serde_json::Value> {
    // PgStore offers get_events_by_session or get_events_by_type — route based on query params.
    // If session filter is provided, use that; otherwise filter by folder + event_type.
    if let Some(session_str) = &q.session {
        if let Ok(session_id) = uuid::Uuid::parse_str(session_str) {
            return match state.pg.get_events_by_session(&session_id).await {
                Ok(events) => {
                    let count = events.len();
                    Json(serde_json::json!({"events": events, "count": count}))
                }
                Err(e) => Json(serde_json::json!({"error": e})),
            };
        }
    }

    // Fall back to folder + event_type query
    if let (Ok(folder_id), Some(etype)) = (uuid::Uuid::parse_str(&project), &q.event_type) {
        return match state.pg.get_events_by_type(&folder_id, etype).await {
            Ok(events) => {
                let count = events.len();
                Json(serde_json::json!({"events": events, "count": count}))
            }
            Err(e) => Json(serde_json::json!({"error": e})),
        };
    }

    // TODO: PgStore has no list_events(folder, None, None, limit) — need a broader query method.
    Json(serde_json::json!({"events": [], "count": 0}))
}

// ── Workflow State ──────────────────────────────────────────────────────────

pub(crate) async fn get_workflow_state(
    State(state): State<AppState>,
    Path(project): Path<String>,
) -> Json<serde_json::Value> {
    match state.pg.get_workflow_state(&project).await {
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
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

pub(crate) async fn update_workflow_state(
    State(state): State<AppState>,
    Path(project): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let result = state.pg.upsert_workflow_state(
        &project,
        body["active_phase"].as_str(),
        body["active_plan"].as_str(),
        body["active_task"].as_str(),
        body["active_issue"].as_i64(),
        body["last_checkpoint"].as_str(),
        body["rules_hash"].as_str(),
    ).await;

    if let Err(e) = result {
        return Json(serde_json::json!({"ok": false, "error": e}));
    }

    // Sync to .sensei/state.yaml
    // Use explicit project_path from body; old store.get_repo_path() fallback removed.
    // TODO: Add a PgStore lookup for folder abs_path by project name if needed.
    let project_path = body["project_path"].as_str().map(String::from);
    if let Some(project_path) = project_path {
        let sensei_dir = std::path::Path::new(&project_path).join(".sensei");
        std::fs::create_dir_all(&sensei_dir).ok();
        let state_file = sensei_dir.join("state.yaml");

        // Read back the state we just wrote to get all fields
        if let Ok(Some(ws)) = state.pg.get_workflow_state(&project).await {
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
