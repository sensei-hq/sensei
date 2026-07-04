use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;

// ── Config CRUD ─────────────────────────────────────────────────────────────

pub(crate) async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let config = state.pg.get_all_config().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(config)))
}

pub(crate) async fn get_config_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let val = state.pg.get_config(&key).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"key": key, "value": val})))
}

pub(crate) async fn set_config_handler(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Some(obj) = body.as_object() {
        for (key, val) in obj {
            let v = match val { serde_json::Value::String(s) => s.clone(), other => other.to_string() };
            state.pg.set_config(key, &v).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

pub(crate) async fn delete_config_key(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state.pg.delete_config(&key).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

// ── Marketplace Install ─────────────────────────────────────────────────────

pub(crate) async fn marketplace_install(
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let target = body["target"].as_str().unwrap_or("");
    let _item_name = body["item"].as_str().unwrap_or("");
    let scope = body["scope"].as_str().unwrap_or("global");
    let _marketplace_path = body["marketplacePath"].as_str().unwrap_or("");

    if target.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "target required"}));
    }

    // Use native Rust installer (replaces shelling out to marketplace/install.ts)
    let acps: Vec<String> = body["acps"].as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    let result = crate::installer::install(&acps, scope);
    Json(serde_json::json!({
        "ok": true,
        "hooks_installed": result.hooks_installed,
        "skills_installed": result.skills_installed,
        "commands_installed": result.commands_installed,
        "acps_configured": result.acps_configured,
        "errors": result.errors,
        "warnings": result.warnings,
    }))
}

// ── Assistants ──────────────────────────────────────────────────────────────

pub(crate) async fn assistant_detect() -> Json<Vec<crate::assistants::AssistantStatus>> {
    Json(crate::assistants::detect())
}

pub(crate) async fn assistant_detect_families() -> Json<Vec<crate::assistants::AssistantFamily>> {
    Json(crate::assistants::detect_families())
}

#[derive(Deserialize)]
pub(crate) struct AssistantConfigureBody {
    #[serde(default)]
    acps: Vec<String>,
}

pub(crate) async fn assistant_configure(
    State(state): State<AppState>,
    Json(body): Json<AssistantConfigureBody>,
) -> Json<crate::assistants::ConfigureResult> {
    // Hand the state event sender to configure() so it can broadcast
    // per-part configuring/done/error transitions to SSE subscribers
    // (the wizard's AssistantCard) as each variant lands.
    Json(crate::assistants::configure(&body.acps, Some(&state.event_tx)))
}

#[derive(Deserialize)]
pub(crate) struct AssistantRemoveBody {
    #[serde(default)]
    acps: Vec<String>,
}

pub(crate) async fn assistant_remove(
    State(state): State<AppState>,
    Json(body): Json<AssistantRemoveBody>,
) -> Json<serde_json::Value> {
    let removed = crate::assistants::remove_selected(&body.acps, Some(&state.event_tx));
    serde_json::json!({"assistants_removed": removed, "errors": []}).into()
}

/// GET /api/assistants/health — current per-adapter health (config + freshness).
pub(crate) async fn assistants_health(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let report = crate::assistants::health_report(&state.pg, now_ms).await;
    let overall = report.iter().map(|h| h.status)
        .fold(crate::assistants::CheckStatus::Ok, |acc, s| acc.worse(s));
    Json(serde_json::json!({ "status": overall, "adapters": report }))
}

#[derive(serde::Deserialize)]
pub(crate) struct ResolveBody { pub adapter_id: String }

/// POST /api/assistants/resolve — reinstall one adapter, clear its breaker.
pub(crate) async fn assistants_resolve(
    State(state): State<AppState>,
    Json(body): Json<ResolveBody>,
) -> Json<crate::assistants::AdapterResolveReport> {
    let report = crate::assistants::resolve_adapter(&body.adapter_id, &state.breaker);
    Json(report)
}

// ── Installer ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct InstallBody {
    #[serde(default)]
    acps: Vec<String>,
    #[serde(default = "default_scope")]
    scope: String,
}

fn default_scope() -> String {
    "global".into()
}

pub(crate) async fn install_all(
    Json(body): Json<InstallBody>,
) -> Json<crate::installer::InstallResult> {
    // Run in blocking thread — marketplace download is synchronous
    let result = tokio::task::spawn_blocking(move || {
        crate::installer::install(&body.acps, &body.scope)
    })
    .await
    .unwrap_or_default();
    Json(result)
}

pub(crate) async fn install_hooks() -> Json<serde_json::Value> {
    match tokio::task::spawn_blocking(crate::installer::install_hooks_only).await {
        Ok(Ok(n)) => serde_json::json!({"ok": true, "count": n}).into(),
        Ok(Err(e)) => serde_json::json!({"ok": false, "error": e}).into(),
        Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}).into(),
    }
}

#[derive(Deserialize)]
pub(crate) struct InstallItemBody {
    name: String,
    kind: String,
}

pub(crate) async fn install_single_item(
    Json(body): Json<InstallItemBody>,
) -> Json<serde_json::Value> {
    let name = body.name;
    let kind = body.kind;
    match tokio::task::spawn_blocking(move || crate::installer::install_item(&name, &kind)).await {
        Ok(Ok(path)) => serde_json::json!({"ok": true, "path": path}).into(),
        Ok(Err(e)) => serde_json::json!({"ok": false, "error": e}).into(),
        Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}).into(),
    }
}

pub(crate) async fn remove_single_item(
    Json(body): Json<InstallItemBody>,
) -> Json<serde_json::Value> {
    let name = body.name;
    let kind = body.kind;
    match tokio::task::spawn_blocking(move || crate::installer::remove_item(&name, &kind)).await {
        Ok(Ok(())) => serde_json::json!({"ok": true}).into(),
        Ok(Err(e)) => serde_json::json!({"ok": false, "error": e}).into(),
        Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}).into(),
    }
}

pub(crate) async fn get_catalog() -> Json<serde_json::Value> {
    match tokio::task::spawn_blocking(crate::installer::fetch_catalog).await {
        Ok(Ok(catalog)) => {
            let items: Vec<serde_json::Value> = catalog
                .items
                .iter()
                .map(|i| serde_json::json!({
                    "name": i.name,
                    "kind": i.kind,
                    "description": i.description,
                    "scope": i.scope,
                    "path": i.path,
                    "recommended_for": i.recommended_for,
                    "stage": i.stage,
                }))
                .collect();
            serde_json::json!({
                "version": catalog.version,
                "items": items,
            })
            .into()
        }
        Ok(Err(e)) => serde_json::json!({"error": e}).into(),
        Err(e) => serde_json::json!({"error": e.to_string()}).into(),
    }
}

pub(crate) async fn list_installed_items() -> Json<Vec<crate::installer::InstalledItem>> {
    Json(crate::installer::list_installed())
}

#[derive(serde::Deserialize)]
pub(crate) struct SetInstalledEnabledBody {
    pub kind: String,
    pub enabled: bool,
}

/// PUT /api/install/installed/{name}/enabled body `{kind, enabled}` —
/// toggle a skill or command by moving its .md file between
/// `~/.claude/<kind>s/` and its `disabled/` sibling. Returns
/// `{ ok: true, changed: bool }` — `changed=false` when the item was
/// already in the target state (idempotent).
pub(crate) async fn set_installed_enabled(
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<SetInstalledEnabledBody>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Filesystem work — off the async runtime.
    let name_owned = name.clone();
    let result = tokio::task::spawn_blocking(move || {
        crate::installer::set_item_enabled(&name_owned, &body.kind, body.enabled)
    }).await.map_err(|e| (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": format!("spawn_blocking failed: {e}") }))
    ))?;

    match result {
        Ok(changed) => Ok(Json(serde_json::json!({ "ok": true, "changed": changed }))),
        Err(e) if e.contains("unknown kind") => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )),
        Err(e) if e.contains("not found") || e.contains("ambiguous") => Err((
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e })),
        )),
        Err(e) => {
            tracing::error!(error = %e, item = %name, "set_installed_enabled failed");
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e })),
            ))
        }
    }
}

pub(crate) async fn remove_all(
    body: Option<Json<crate::installer::RemoveRequest>>,
) -> Json<crate::installer::RemoveResult> {
    let req = body.map(|b| b.0).unwrap_or_default();
    match tokio::task::spawn_blocking(move || crate::installer::remove(&req)).await {
        Ok(result) => Json(result),
        Err(_) => Json(crate::installer::RemoveResult::default()),
    }
}

