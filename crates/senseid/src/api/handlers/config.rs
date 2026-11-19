use crate::api::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

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
        // Validate BEFORE writing anything, so a bad value in a multi-key PUT
        // can't leave half of it applied.
        for (key, val) in obj {
            let v = match val {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            if !config_value_is_valid(key, &v) {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
        for (key, val) in obj {
            let v = match val {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            state.pg.set_config(key, &v).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        // The glyph choice syncs with the dōjō, the later choice winning (#133):
        // record when it was made.
        use crate::dojo_client::glyph_sync::{GLYPHS_KEY, GLYPHS_SET_AT_KEY};
        if obj.contains_key(GLYPHS_KEY) && !obj.contains_key(GLYPHS_SET_AT_KEY) {
            let now = chrono::Utc::now().to_rfc3339();
            state
                .pg
                .set_config(GLYPHS_SET_AT_KEY, &now)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }
    Ok(Json(serde_json::json!({"ok": true})))
}

/// Whether `value` may be stored under `key`. A structured setting that does not
/// parse would be silently ignored — the screen reading "not configured" while the
/// user believes they configured it — so it is rejected instead. Blank is allowed:
/// that is how a user clears the setting. Keys not listed here are free-form.
fn config_value_is_valid(key: &str, value: &str) -> bool {
    if value.trim().is_empty() {
        return true;
    }
    match key {
        crate::cost::SUBSCRIPTION_CONFIG_KEY => {
            crate::cost::Subscription::parse(Some(value)).is_some()
        }
        crate::review::RISK_MODELS_CONFIG_KEY => {
            crate::review::RiskModels::parse(Some(value)).is_some()
        }
        _ => true,
    }
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
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let target = body["target"].as_str().unwrap_or("");
    let _item_name = body["item"].as_str().unwrap_or("");
    let scope = body["scope"].as_str().unwrap_or("global").to_string();
    let _marketplace_path = body["marketplacePath"].as_str().unwrap_or("");

    if target.is_empty() {
        return Ok(Json(serde_json::json!({"ok": false, "error": "target required"})));
    }

    // Use native Rust installer (replaces shelling out to marketplace/install.ts)
    let acps: Vec<String> = body["acps"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    let result =
        install_then_resync(&state, move || crate::installer::install(&acps, &scope)).await?;
    Ok(Json(serde_json::json!({
        "ok": true,
        "hooks_installed": result.hooks_installed,
        "skills_installed": result.skills_installed,
        "commands_installed": result.commands_installed,
        "acps_configured": result.acps_configured,
        "errors": result.errors,
        "warnings": result.warnings,
    })))
}

// ── Assistants ──────────────────────────────────────────────────────────────

/// The catalogue's names, or a 500: a detection list is never served unnamed (#455).
async fn catalogue_names(
    state: &AppState,
) -> Result<crate::assistants::catalogue::CatalogueNames, StatusCode> {
    state.pg.catalogue_names().await.map_err(|e| {
        tracing::warn!(error = %e, "assistants: catalogue read failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

/// GET /api/assistants/detect — the live probe, each tool named by the catalogue.
pub(crate) async fn assistant_detect(
    State(state): State<AppState>,
) -> Result<Json<Vec<crate::assistants::Named<crate::assistants::AssistantStatus>>>, StatusCode> {
    let names = catalogue_names(&state).await?;
    let statuses = tokio::task::spawn_blocking(crate::assistants::detect).await.map_err(|e| {
        tracing::warn!(error = %e, "assistants: probe panicked");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(crate::assistants::name_statuses(statuses, &names)))
}

/// GET /api/assistants/families — the live probe grouped by family, named by the catalogue.
pub(crate) async fn assistant_detect_families(
    State(state): State<AppState>,
) -> Result<Json<Vec<crate::assistants::Named<crate::assistants::NamedFamily>>>, StatusCode> {
    let names = catalogue_names(&state).await?;
    let families =
        tokio::task::spawn_blocking(crate::assistants::detect_families).await.map_err(|e| {
            tracing::warn!(error = %e, "assistants: probe panicked");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(crate::assistants::name_families(families, &names)))
}

/// POST /api/assistants/detect — probe this machine, store what it found
/// (one row per catalogue tool), and return the stored rows (#455).
pub(crate) async fn assistant_sync_installs(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    crate::assistants::sync_installs(&state.pg, &[]).await.map(Json).map_err(|e| {
        tracing::warn!(error = %e, "assistants: install sync failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

/// GET /api/assistants/installs — the stored rows joined to the catalogue.
/// A failed read is a 500, never `[]`.
pub(crate) async fn assistant_installs(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    state.pg.list_assistant_installs().await.map(Json).map_err(|e| {
        tracing::warn!(error = %e, "assistants: installs read failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

async fn resync_installs(state: &AppState, just_configured: &[String]) -> Option<String> {
    crate::assistants::resync_installs(&state.pg, just_configured).await
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
    let acps = body.acps;
    let tx = state.event_tx.clone();
    let mut result = crate::assistants::configure(&acps, Some(&tx));
    if let Some(w) = resync_installs(&state, &result.configured).await {
        result.warnings.push(w);
    }
    Json(result)
}

#[derive(Deserialize)]
pub(crate) struct AssistantUpgradeBody {
    #[serde(default)]
    acps: Vec<String>,
}

/// POST /api/assistants/upgrade — refresh each assistant's sensei integration
/// after a sensei binary upgrade. Empty `acps` = every detected assistant;
/// otherwise exactly the named ids. Mirrors `assistant_configure`'s fan-out but
/// returns a per-assistant result array (one [`AdapterResolveReport`] each):
/// Claude Code runs `claude plugin update sensei` (and re-verifies the
/// manifest); file-based MCP assistants report a no-op action.
pub(crate) async fn assistant_upgrade(
    State(state): State<AppState>,
    Json(body): Json<AssistantUpgradeBody>,
) -> Json<Vec<crate::assistants::health::AdapterResolveReport>> {
    let (mut reports, refreshed) = crate::assistants::upgrade_refreshing(&body.acps);
    if let Some(w) = resync_installs(&state, &refreshed).await {
        note_resync_failure(&mut reports, &w);
    }
    Json(reports)
}

/// Record a failed post-upgrade re-detect on each report as a warning: the
/// upgrade landed, so `ok` and `errors` stay as the upgrade left them.
fn note_resync_failure(
    reports: &mut [crate::assistants::health::AdapterResolveReport],
    warning: &str,
) {
    for r in reports {
        r.warnings.push(warning.to_string());
    }
}

/// The remove response. A failed re-detect after the removal landed is a
/// warning, as configure reports it — `errors` means the removal itself failed.
fn remove_response(removed: Vec<String>, resync_failure: Option<String>) -> serde_json::Value {
    let warnings: Vec<String> = resync_failure.into_iter().collect();
    serde_json::json!({"assistants_removed": removed, "errors": [], "warnings": warnings})
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
    remove_response(removed, resync_installs(&state, &[]).await).into()
}

/// GET /api/assistants/health — current per-adapter health (config + freshness).
pub(crate) async fn assistants_health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let report = crate::assistants::health_report(&state.pg, now_ms).await;
    let overall = report
        .iter()
        .map(|h| h.status)
        .fold(crate::assistants::CheckStatus::Ok, |acc, s| acc.worse(s));
    Json(serde_json::json!({ "status": overall, "adapters": report }))
}

#[derive(serde::Deserialize)]
pub(crate) struct ResolveBody {
    pub adapter_id: String,
}

/// POST /api/assistants/resolve — reinstall one adapter, clear its breaker.
pub(crate) async fn assistants_resolve(
    State(state): State<AppState>,
    Json(body): Json<ResolveBody>,
) -> Json<crate::assistants::health::AdapterResolveReport> {
    let breaker = state.breaker.clone();
    let id = body.adapter_id.clone();
    Json(
        resolve_then_resync(&state, &body.adapter_id, move || {
            crate::assistants::resolve_adapter(&id, &breaker)
        })
        .await,
    )
}

/// Run a resolve (a re-run of configure for one tool), then re-detect so the
/// install rows follow (#455). A failed re-detect is a warning on the report.
async fn resolve_then_resync(
    state: &AppState,
    adapter_id: &str,
    resolve: impl FnOnce() -> crate::assistants::health::AdapterResolveReport,
) -> crate::assistants::health::AdapterResolveReport {
    let mut report = resolve();
    let configured = if report.ok { vec![adapter_id.to_string()] } else { vec![] };
    if let Some(w) = resync_installs(state, &configured).await {
        report.warnings.push(w);
    }
    report
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
    State(state): State<AppState>,
    Json(body): Json<InstallBody>,
) -> Result<Json<crate::installer::InstallResult>, StatusCode> {
    install_then_resync(&state, move || crate::installer::install(&body.acps, &body.scope))
        .await
        .map(Json)
}

/// Run an install off the async runtime, then re-detect so the install rows
/// follow (#455). The marketplace download is synchronous. A panic in that
/// thread is a 500, NOT a default (empty) result that reads as a clean
/// "0 installed, no errors" no-op and hides the failure.
async fn install_then_resync(
    state: &AppState,
    install: impl FnOnce() -> crate::installer::InstallResult + Send + 'static,
) -> Result<crate::installer::InstallResult, StatusCode> {
    let mut result = tokio::task::spawn_blocking(install).await.map_err(|e| {
        tracing::error!(error = %e, "install: installer thread panicked");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    if let Some(w) = resync_installs(state, &result.acps_configured).await {
        result.warnings.push(w);
    }
    Ok(result)
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
                .map(|i| {
                    serde_json::json!({
                        "name": i.name,
                        "kind": i.kind,
                        "description": i.description,
                        "scope": i.scope,
                        "path": i.path,
                        "recommended_for": i.recommended_for,
                        "stage": i.stage,
                    })
                })
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
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Filesystem work — off the async runtime.
    let name_owned = name.clone();
    let result = tokio::task::spawn_blocking(move || {
        crate::installer::set_item_enabled(&name_owned, &body.kind, body.enabled)
    })
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("spawn_blocking failed: {e}") })),
        )
    })?;

    match result {
        Ok(changed) => Ok(Json(serde_json::json!({ "ok": true, "changed": changed }))),
        Err(e) if e.contains("unknown kind") => {
            Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))))
        }
        Err(e) if e.contains("not found") || e.contains("ambiguous") => {
            Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": e }))))
        }
        Err(e) => {
            tracing::error!(error = %e, item = %name, "set_installed_enabled failed");
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e }))))
        }
    }
}

pub(crate) async fn remove_all(
    State(state): State<AppState>,
    body: Option<Json<crate::installer::RemoveRequest>>,
) -> Result<Json<crate::installer::RemoveResult>, StatusCode> {
    // An absent body → default request (purge:false) is a genuine default.
    let req = body.map(|b| b.0).unwrap_or_default();
    remove_then_resync(&state, move || crate::installer::remove(&req)).await.map(Json)
}

/// Run an uninstall off the async runtime, then re-detect so the install rows
/// follow (#455). A PANIC in the uninstall thread is a 500 — not a default
/// (empty) result that falsely reports "nothing removed, no errors".
async fn remove_then_resync(
    state: &AppState,
    remove: impl FnOnce() -> crate::installer::RemoveResult + Send + 'static,
) -> Result<crate::installer::RemoveResult, StatusCode> {
    let mut result = tokio::task::spawn_blocking(remove).await.map_err(|e| {
        tracing::error!(error = %e, "remove_all: uninstall thread panicked");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    if let Some(w) = resync_installs(state, &[]).await {
        result.warnings.push(w);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_subscription_must_parse_or_be_blank() {
        let key = crate::cost::SUBSCRIPTION_CONFIG_KEY;
        assert!(config_value_is_valid(key, r#"{"amount":200,"period":"monthly"}"#));
        assert!(config_value_is_valid(key, ""), "blank clears the setting");
        assert!(!config_value_is_valid(key, "two hundred"));
    }

    #[test]
    fn a_risk_model_map_must_parse_or_be_blank() {
        let key = crate::review::RISK_MODELS_CONFIG_KEY;
        assert!(config_value_is_valid(key, r#"{"approve":"opus"}"#));
        assert!(config_value_is_valid(key, " "), "blank clears the setting");
        assert!(!config_value_is_valid(key, r#"{"aprove":"opus"}"#));
        assert!(!config_value_is_valid(key, "{}"), "a map that maps nothing is not a setting");
    }

    #[test]
    fn other_keys_are_not_validated_here() {
        assert!(config_value_is_valid("theme", "anything"));
    }

    // A failed re-detect after a remove that landed is a warning, as configure
    // reports it: the wizard reads any `errors` as a failed remove and turns the
    // switch back on while the tool's config no longer holds sensei (#455 review).
    #[test]
    fn a_failed_redetect_after_remove_is_a_warning_not_an_error() {
        let body = remove_response(
            vec!["claude-code".into()],
            Some("re-detect installs: connection refused".into()),
        );
        assert_eq!(body["assistants_removed"], serde_json::json!(["claude-code"]));
        assert_eq!(body["errors"], serde_json::json!([]), "the removal landed: no errors");
        assert_eq!(body["warnings"], serde_json::json!(["re-detect installs: connection refused"]));
    }

    #[test]
    fn a_clean_remove_has_neither_errors_nor_warnings() {
        let body = remove_response(vec!["cursor".into()], None);
        assert_eq!(body["errors"], serde_json::json!([]));
        assert_eq!(body["warnings"], serde_json::json!([]));
    }

    // Same class on upgrade: the report stayed `ok: true` with the re-detect
    // failure in `errors`, a contradiction the CLI resolved by never printing it.
    #[test]
    fn a_failed_redetect_after_upgrade_is_a_warning_on_each_report() {
        let mut reports = vec![crate::assistants::health::AdapterResolveReport {
            adapter_id: "claude-code".into(),
            ok: true,
            actions: vec!["upgraded claude-code plugin".into()],
            errors: vec![],
            warnings: vec![],
        }];
        note_resync_failure(&mut reports, "re-detect installs: boom");
        assert!(reports[0].ok);
        assert!(reports[0].errors.is_empty(), "the upgrade landed: no errors");
        assert_eq!(reports[0].warnings, vec!["re-detect installs: boom".to_string()]);
    }
}

/// The assistant routes' catalogue names and stored installs (#455).
#[cfg(test)]
mod assistant_route_tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use tower::ServiceExt;

    async fn call(app: &axum::Router, method: &str, uri: &str) -> (StatusCode, Value) {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .header("host", "127.0.0.1:7744")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    async fn app() -> (axum::Router, crate::api::state::AppState) {
        let ctx = crate::tasks::test_support::make_ctx().await;
        let state = ctx.app_state.clone();
        (crate::api::routes::create_router(state.clone()), state)
    }

    #[tokio::test]
    async fn detection_names_come_from_the_catalogue() {
        let (app, _) = app().await;
        let (status, body) = call(&app, "GET", "/api/assistants/families").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let claude = body.as_array().unwrap().iter().find(|f| f["family"] == "claude").unwrap();
        // The family reads as the catalogue names of the members its switch
        // acts on (installed ones; all when none is), whatever this machine has.
        let members = claude["members"].as_array().unwrap();
        let installed: Vec<&Value> = members.iter().filter(|m| m["installed"] == true).collect();
        let acted_on: Vec<&Value> =
            if installed.is_empty() { members.iter().collect() } else { installed };
        let expected =
            acted_on.iter().map(|m| m["name"].as_str().unwrap()).collect::<Vec<_>>().join(" and ");
        assert_eq!(claude["name"], expected.as_str(), "{claude}");
        for m in members {
            let want = if m["id"] == "claude-code" { "Claude Code" } else { "Claude Desktop" };
            assert_eq!(m["name"], want, "{m}");
        }
        let (status, body) = call(&app, "GET", "/api/assistants/detect").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let vscode = body.as_array().unwrap().iter().find(|s| s["id"] == "vscode").unwrap();
        assert_eq!(vscode["name"], "VS Code");
        let desktop =
            body.as_array().unwrap().iter().find(|s| s["id"] == "claude-desktop").unwrap();
        assert_eq!(desktop["name"], "Claude Desktop");
    }

    #[tokio::test]
    async fn installs_read_the_stored_rows_joined_to_the_catalogue() {
        let (app, state) = app().await;
        sqlx_core::query::query("delete from sensei.assistant_installs")
            .execute(state.pg.pool())
            .await
            .unwrap();
        sqlx_core::query::query(
            "insert into sensei.assistant_installs (catalogue_id, installed, config_path, configured, detected_at)
             values ('cursor', true, '/home/x/.cursor/mcp.json', true, now())",
        )
        .execute(state.pg.pool())
        .await
        .unwrap();
        let (status, body) = call(&app, "GET", "/api/assistants/installs").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let rows = body.as_array().unwrap();
        assert_eq!(rows.len(), 1, "{body}");
        assert_eq!(rows[0]["id"], "cursor");
        assert_eq!(rows[0]["name"], "Cursor");
        assert_eq!(rows[0]["kind"], "editor");
        assert_eq!(rows[0]["installed"], true);
        assert_eq!(rows[0]["configured"], true);
        assert_eq!(rows[0]["config_path"], "/home/x/.cursor/mcp.json");
        assert!(rows[0]["detected_at"].is_string(), "{body}");
    }

    #[tokio::test]
    async fn detect_stores_what_it_found_and_returns_the_stored_rows() {
        let (app, state) = app().await;
        sqlx_core::query::query("delete from sensei.assistant_installs")
            .execute(state.pg.pool())
            .await
            .unwrap();
        let (status, body) = call(&app, "POST", "/api/assistants/detect").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let stored: i64 =
            sqlx_core::query_scalar::query_scalar("select count(*) from sensei.assistant_installs")
                .fetch_one(state.pg.pool())
                .await
                .unwrap();
        assert_eq!(body.as_array().map(|a| a.len() as i64), Some(stored), "{body}");
        // One row per registry tool, each a catalogue row.
        assert_eq!(stored, 8, "{body}");
    }

    #[tokio::test]
    async fn a_failed_installs_read_is_a_500_not_an_empty_list() {
        let (app, state) = app().await;
        sqlx_core::query::query(
            "alter table sensei.assistant_installs rename to assistant_installs_gone",
        )
        .execute(state.pg.pool())
        .await
        .unwrap();
        let (status, body) = call(&app, "GET", "/api/assistants/installs").await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
        assert!(!body.is_array(), "{body}");
    }

    // Every path that configures or removes sensei re-detects, so the install
    // rows (and the catalogue's `mcp_config`) never outlive the change (#455
    // review): `/api/install`, `/api/marketplace/install`, `/api/remove`,
    // `/api/assistants/resolve` — as configure/remove/upgrade already do.
    async fn seed_stale_cursor(state: &crate::api::state::AppState) {
        sqlx_core::query::query("delete from sensei.assistant_installs")
            .execute(state.pg.pool())
            .await
            .unwrap();
        sqlx_core::query::query(
            "insert into sensei.assistant_installs (catalogue_id, installed, config_path, configured, detected_at)
             values ('cursor', true, '/stale/.cursor/mcp.json', true, '2000-01-01T00:00:00Z')",
        )
        .execute(state.pg.pool())
        .await
        .unwrap();
    }

    /// Whether the stale row was re-detected: a fresh `detected_at` and every
    /// registry tool stored.
    async fn redetected(state: &crate::api::state::AppState) -> bool {
        let (fresh, rows): (bool, i64) = sqlx_core::query_as::query_as(
            "select coalesce(bool_and(detected_at > '2001-01-01'), false), count(*)
               from sensei.assistant_installs",
        )
        .fetch_one(state.pg.pool())
        .await
        .unwrap();
        fresh && rows == 8
    }

    async fn break_installs(state: &crate::api::state::AppState) {
        sqlx_core::query::query(
            "alter table sensei.assistant_installs rename to assistant_installs_gone",
        )
        .execute(state.pg.pool())
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn an_install_redetects_the_installs() {
        let (_, state) = app().await;
        seed_stale_cursor(&state).await;
        let result = super::install_then_resync(&state, crate::installer::InstallResult::default)
            .await
            .unwrap();
        assert!(result.warnings.is_empty(), "{:?}", result.warnings);
        assert!(redetected(&state).await, "the install rows were left stale");
    }

    #[tokio::test]
    async fn a_failed_redetect_after_an_install_is_a_warning() {
        let (_, state) = app().await;
        break_installs(&state).await;
        let result = super::install_then_resync(&state, crate::installer::InstallResult::default)
            .await
            .unwrap();
        assert!(result.errors.is_empty(), "the install landed: {:?}", result.errors);
        assert!(
            result.warnings.iter().any(|w| w.starts_with("re-detect installs:")),
            "{:?}",
            result.warnings
        );
    }

    #[tokio::test]
    async fn a_remove_all_redetects_the_installs() {
        let (_, state) = app().await;
        seed_stale_cursor(&state).await;
        let result = super::remove_then_resync(&state, crate::installer::RemoveResult::default)
            .await
            .unwrap();
        assert!(result.warnings.is_empty(), "{:?}", result.warnings);
        assert!(redetected(&state).await, "the install rows were left stale");
    }

    #[tokio::test]
    async fn a_failed_redetect_after_a_remove_all_is_a_warning() {
        let (_, state) = app().await;
        break_installs(&state).await;
        let result = super::remove_then_resync(&state, crate::installer::RemoveResult::default)
            .await
            .unwrap();
        assert!(result.errors.is_empty(), "the removal landed: {:?}", result.errors);
        assert!(
            result.warnings.iter().any(|w| w.starts_with("re-detect installs:")),
            "{:?}",
            result.warnings
        );
    }

    fn resolved(ok: bool) -> crate::assistants::health::AdapterResolveReport {
        crate::assistants::health::AdapterResolveReport {
            adapter_id: "cursor".into(),
            ok,
            actions: vec!["reinstalled".into()],
            errors: vec![],
            warnings: vec![],
        }
    }

    #[tokio::test]
    async fn a_resolve_redetects_the_installs() {
        let (_, state) = app().await;
        seed_stale_cursor(&state).await;
        let report = super::resolve_then_resync(&state, "cursor", || resolved(true)).await;
        assert!(report.ok && report.warnings.is_empty(), "{:?}", report.warnings);
        assert!(redetected(&state).await, "the install rows were left stale");
    }

    #[tokio::test]
    async fn a_failed_redetect_after_a_resolve_is_a_warning() {
        let (_, state) = app().await;
        break_installs(&state).await;
        let report = super::resolve_then_resync(&state, "cursor", || resolved(true)).await;
        assert!(report.ok, "the resolve landed");
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(
            report.warnings.iter().any(|w| w.starts_with("re-detect installs:")),
            "{:?}",
            report.warnings
        );
    }
}
