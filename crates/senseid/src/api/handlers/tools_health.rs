//! `/api/instruments/tools-health` — the Instruments · Health L1 grid + its
//! on-demand capture trigger. The capture itself lives in
//! [`crate::tool_discovery::run_capture`] (also run once at daemon startup);
//! this module is just the HTTP surface. The grid is one card per tool source
//! with `share_invoked = tools_invoked_14d / tools_registered`; sources that
//! can't be probed degrade to `tools_registered: null` (no fabricated bar).

use crate::api::state::AppState;
use axum::{extract::State, http::StatusCode, response::Json};

/// POST /api/instruments/tools/refresh — run the full capture on demand.
pub(crate) async fn refresh(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let c = crate::tool_discovery::run_capture(&state.pg).await.map_err(|e| {
        tracing::error!(error = %e, "tools refresh capture failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(serde_json::json!({
        "discovered_servers": c.discovered,
        "builtins": c.builtins,
        "probed_ok": c.probed_ok,
        "probed_err": c.probed_err,
    })))
}

/// GET /api/instruments/tools-health — the L1 grid.
pub(crate) async fn grid(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sources = state.pg.get_tools_health().await.map_err(|e| {
        tracing::error!(error = %e, "get_tools_health failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(serde_json::json!({ "sources": sources })))
}
