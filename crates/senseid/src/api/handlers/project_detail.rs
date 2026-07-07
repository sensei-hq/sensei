use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;

#[derive(Deserialize)]
pub(crate) struct RecoQuery {
    status: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct SessionsQuery {
    limit: Option<i64>,
}

#[derive(Deserialize)]
pub(crate) struct CommandsQuery {
    /// Optional canonical verb (`test` / `build` / `lint` / `e2e` / …).
    /// Absent → return every category.
    pub category: Option<String>,
}

/// GET /api/projects/{id}/commands — #83 T1 commands surface. Returns every
/// discoverable command across the project's folders, or a category subset
/// when `?category=<verb>` is set. Populated by
/// `ManifestAdapter::parse_commands` during `extract_deps` — a re-scan
/// refreshes each folder's rows atomically.
///
/// Accepts `{id}` as a project name OR UUID, matching the read-side pattern
/// the MCP tool `get_commands` uses (repo_id from `resolve_project` is a
/// name).
pub(crate) async fn get_project_commands(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<CommandsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Resolve name → uuid, or accept a raw UUID.
    let uuid = if let Some(row) = state.pg.get_project_by_name(&id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        crate::api::util::json_uuid(&row["id"]).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
    } else if let Ok(uid) = uuid::Uuid::parse_str(&id) {
        state.pg.get_project(&uid).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;
        uid
    } else {
        return Err(StatusCode::NOT_FOUND);
    };

    let rows = state.pg.get_project_commands(&uuid, q.category.as_deref()).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"commands": rows, "count": rows.len()})))
}

pub(crate) async fn get_project_ftr(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let data = state.pg.get_project_ftr(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(data))
}

pub(crate) async fn get_project_repos(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let repos = state.pg.get_project_repos(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "repos": repos })))
}

pub(crate) async fn get_project_drift(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let data = state.pg.get_project_drift(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(data))
}

pub(crate) async fn get_project_patterns(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let data = state.pg.get_project_patterns(&uuid).await
        .map_err(|e| { tracing::error!(error = %e, project = %uuid, "get_project_patterns failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(data))
}

pub(crate) async fn get_project_libraries(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let libs = state.pg.get_project_libraries(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "libraries": libs })))
}

pub(crate) async fn get_project_instruments(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let tools = state.pg.get_project_extensions(&uuid, Some(&["skill", "command", "agent"])).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "tools": tools })))
}

/// GET /api/projects/{id}/mcp-tool-stats — per-tool call/error/duration/FTR
/// aggregation scoped to a project. Joins the daemon's tool manifests (the
/// full known catalogue) with per-project usage rows so tools with zero
/// calls still appear in the response (with count=0). T2 Slice F.
pub(crate) async fn get_project_mcp_tool_stats(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let stats = state.pg.get_project_mcp_tool_stats(&uuid).await
        .map_err(|e| {
            tracing::error!(error = %e, project = %uuid, "get_project_mcp_tool_stats failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Merge with the tool manifest so the response covers every tool the
    // daemon dispatches (not just the ones this project called). Manifest
    // fields (kind, summary) let the UI render a full catalogue with usage
    // overlays without a second daemon round-trip.
    let manifests = crate::api::handlers::mcp_manifests::manifests();
    let mut by_name: std::collections::HashMap<String, &serde_json::Value> = std::collections::HashMap::new();
    for stat in &stats {
        if let Some(name) = stat.get("toolName").and_then(|v| v.as_str()) {
            by_name.insert(name.to_string(), stat);
        }
    }
    let tools: Vec<serde_json::Value> = manifests.iter().map(|m| {
        let empty = serde_json::json!({});
        let usage = by_name.get(m.name).copied().unwrap_or(&empty);
        serde_json::json!({
            "id":            m.id,
            "name":          m.name,
            "mcp":           m.mcp,
            "kind":          m.kind,
            "summary":       m.summary,
            "calls":         usage.get("calls").cloned().unwrap_or(serde_json::json!(0)),
            "errors":        usage.get("errors").cloned().unwrap_or(serde_json::json!(0)),
            "avgDurationMs": usage.get("avgDurationMs").cloned().unwrap_or(serde_json::Value::Null),
            "ftr":           usage.get("ftr").cloned().unwrap_or(serde_json::Value::Null),
            "lastUsedAt":    usage.get("lastUsedAt").cloned().unwrap_or(serde_json::Value::Null),
        })
    }).collect();

    Ok(Json(serde_json::json!({ "tools": tools })))
}

pub(crate) async fn get_project_memories(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let data = state.pg.get_project_memories(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(data))
}

pub(crate) async fn get_project_recommendations(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<RecoQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let recs = state.pg.get_project_recommendations(&uuid, q.status.as_deref()).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(recs)))
}

/// GET /api/projects/{id}/impact — acted-on / consolidation recommendations
/// joined to their reasoning trace (before/after FTR + MOE reasoning). Powers
/// the Observatory Impact view (#70 read-path).
pub(crate) async fn get_project_impact(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    let data = state.pg.get_project_impact(&uuid).await
        .map_err(|e| { tracing::error!(error = %e, project = %uuid, "get_project_impact failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(serde_json::json!(data)))
}

/// GET /api/projects/{id}/library-version-conflicts — per-library version drift
/// across the project's folders (excluding local-protocol deps). Powers the
/// Track 3 Libraries screen "version conflicts" signal.
pub(crate) async fn get_project_library_version_conflicts(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let conflicts = state.pg.list_project_library_version_conflicts(&uuid).await
        .map_err(|e| { tracing::error!(error = %e, project = %uuid, "list_project_library_version_conflicts failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(serde_json::json!({ "conflicts": conflicts })))
}

/// GET /api/projects/{id}/project-deps — outgoing project → project edges
/// detected from local-path protocols (npm link:/workspace:/file:,
/// Cargo path=). Powers the Track 3 Libraries screen "depends on other
/// project" section.
pub(crate) async fn get_project_project_deps(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let deps = state.pg.list_project_dependencies(&uuid).await
        .map_err(|e| { tracing::error!(error = %e, project = %uuid, "list_project_dependencies failed"); StatusCode::INTERNAL_SERVER_ERROR })?;
    Ok(Json(serde_json::json!({ "dependencies": deps })))
}

pub(crate) async fn get_project_sessions(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<SessionsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let limit = q.limit.unwrap_or(50);
    let sessions = state.pg.list_sessions_by_project(&uuid, limit).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "sessions": sessions })))
}

// ── Recommendation accept / reject (Gap 1 fix) ──────────────────────────────
//
// The analyzer writes recommendations in `pending` state, and `MeasureVerdicts`
// only computes an FTR delta for recs that reached `accepted`. Without a UI
// affordance to accept, every rec sits at `pending` forever and the verdict
// column stays empty. These two endpoints close the loop.

/// POST /api/projects/{id}/recommendations/{rec_id}/accept
pub(crate) async fn accept_project_recommendation(
    State(state): State<AppState>,
    Path((_project_id, rec_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let rec_uuid = uuid::Uuid::parse_str(&rec_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.accept_recommendation(&rec_uuid).await
        .map_err(|e| {
            if e.contains("not found") || e.contains("already decided") {
                StatusCode::CONFLICT
            } else {
                tracing::error!(error = %e, rec = %rec_uuid, "accept_recommendation failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/projects/{id}/recommendations/{rec_id}/reject
pub(crate) async fn reject_project_recommendation(
    State(state): State<AppState>,
    Path((_project_id, rec_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let rec_uuid = uuid::Uuid::parse_str(&rec_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.reject_recommendation(&rec_uuid).await
        .map_err(|e| {
            if e.contains("not found") || e.contains("already decided") {
                StatusCode::CONFLICT
            } else {
                tracing::error!(error = %e, rec = %rec_uuid, "reject_recommendation failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── Doc drift scan (T3 Slice 2.3) ───────────────────────────────────────────

/// POST /api/projects/{id}/drift/scan — run the doc-drift detector on the
/// project. Returns `{ scannedDocs, newBroken, resolved }` so the UI can
/// flash a "found N drift signals" notice after triggering.
pub(crate) async fn scan_project_doc_drift(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let summary = state.pg.scan_project_doc_drift(&uuid).await
        .map_err(|e| {
            tracing::error!(error = %e, project = %uuid, "scan_project_doc_drift failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(summary))
}

// ── Service scoping (T2 Slice B) ────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct ServiceScopeBody {
    enabled: bool,
}

/// GET /api/projects/{id}/services — installed services with per-project
/// scope resolved (scoped override wins, then global, then default true).
pub(crate) async fn list_project_services(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let services = state.pg.list_services_with_project_scope(&uuid).await
        .map_err(|e| {
            tracing::error!(error = %e, project = %uuid, "list_services_with_project_scope failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(serde_json::json!({ "services": services })))
}

/// PUT /api/projects/{id}/services/{service_id}/scope — toggle a service's
/// enabled state for this project. Body: `{ enabled: bool }`.
pub(crate) async fn set_project_service_scope(
    State(state): State<AppState>,
    Path((id, service_id)): Path<(String, String)>,
    Json(body): Json<ServiceScopeBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let project_uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    let service_uuid = uuid::Uuid::parse_str(&service_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg.get_project(&project_uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    state.pg
        .set_service_project_scope(&service_uuid, Some(&project_uuid), body.enabled)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, project = %project_uuid, service = %service_uuid, "set_service_project_scope failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── Memory share batches ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct BatchListQuery {
    status: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct BatchCreateBody {
    memory_ids: Vec<uuid::Uuid>,
    note: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct BatchDecisionBody {
    status: String,
    note: Option<String>,
}

/// GET /api/projects/{id}/memory-batches?status=proposed
pub(crate) async fn list_memory_share_batches(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<BatchListQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let batches = state.pg.list_memory_share_batches(&uuid, q.status.as_deref()).await
        .map_err(|e| {
            tracing::error!(error = %e, project = %uuid, "list_memory_share_batches failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(serde_json::json!({ "batches": batches })))
}

/// POST /api/projects/{id}/memory-batches
pub(crate) async fn create_memory_share_batch(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<BatchCreateBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if body.memory_ids.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let batch_id = state.pg
        .create_memory_share_batch(&uuid, &body.memory_ids, body.note.as_deref())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, project = %uuid, "create_memory_share_batch failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(serde_json::json!({ "id": batch_id })))
}

// ── Impact verdicts (manual log) ────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct ImpactListQuery {
    verdict: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct ImpactCreateBody {
    title: String,
    note: Option<String>,
    session_id: Option<uuid::Uuid>,
}

#[derive(Deserialize)]
pub(crate) struct ImpactDecisionBody {
    verdict: String,
    note: Option<String>,
}

/// GET /api/projects/{id}/impact-verdicts?verdict=success
pub(crate) async fn list_impact_verdicts(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<ImpactListQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let verdicts = state.pg.list_impact_verdicts(&uuid, q.verdict.as_deref()).await
        .map_err(|e| {
            tracing::error!(error = %e, project = %uuid, "list_impact_verdicts failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(serde_json::json!({ "verdicts": verdicts })))
}

/// POST /api/projects/{id}/impact-verdicts
pub(crate) async fn create_impact_verdict(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ImpactCreateBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    state.pg.get_project(&uuid).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let verdict_id = state.pg
        .create_impact_verdict(&uuid, &body.title, body.note.as_deref(), body.session_id.as_ref())
        .await
        .map_err(|e| {
            if e.contains("title required") {
                StatusCode::BAD_REQUEST
            } else {
                tracing::error!(error = %e, project = %uuid, "create_impact_verdict failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(serde_json::json!({ "id": verdict_id })))
}

/// PUT /api/projects/{id}/impact-verdicts/{verdict_id}
pub(crate) async fn decide_impact_verdict(
    State(state): State<AppState>,
    Path((id, verdict_id)): Path<(String, String)>,
    Json(body): Json<ImpactDecisionBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _project_uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    let verdict_uuid = uuid::Uuid::parse_str(&verdict_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg
        .set_impact_verdict_outcome(&verdict_uuid, &body.verdict, body.note.as_deref())
        .await
        .map_err(|e| {
            if e.contains("not found") || e.contains("already decided") {
                StatusCode::CONFLICT
            } else if e.contains("invalid verdict") {
                StatusCode::BAD_REQUEST
            } else {
                tracing::error!(error = %e, verdict = %verdict_uuid, "set_impact_verdict_outcome failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// PUT /api/projects/{id}/memory-batches/{batch_id}
pub(crate) async fn decide_memory_share_batch(
    State(state): State<AppState>,
    Path((id, batch_id)): Path<(String, String)>,
    Json(body): Json<BatchDecisionBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _project_uuid = crate::api::util::resolve_project_uuid(&state, &id).await.ok_or(StatusCode::NOT_FOUND)?;
    let batch_uuid = uuid::Uuid::parse_str(&batch_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    state.pg
        .set_memory_share_batch_status(&batch_uuid, &body.status, body.note.as_deref())
        .await
        .map_err(|e| {
            if e.contains("not found") || e.contains("already decided") {
                StatusCode::CONFLICT
            } else if e.contains("invalid status") {
                StatusCode::BAD_REQUEST
            } else {
                tracing::error!(error = %e, batch = %batch_uuid, "set_memory_share_batch_status failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
