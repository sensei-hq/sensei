//! D-PLANNER endpoint — decompose a goal into a structured plan via the LLM
//! gateway. Stateless: returns the structured plan + the rendered `docs/plan`
//! markdown; the caller reviews it (depth bar) and writes it where `start_run`
//! can reference it. See [`crate::planner`].

use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::api::state::AppState;

#[derive(Deserialize)]
pub(crate) struct PlanRequest {
    /// The goal / spec / issue to decompose (required, non-empty).
    pub goal: String,
    /// Optional grounding context (a spec, an issue body, conventions).
    #[serde(default)]
    pub context: Option<String>,
}

/// POST /api/planner/generate — `{goal, context?}` → `{plan, markdown}`.
/// A gateway or parse failure is a 502 (planning is an explicit action, not a
/// background best-effort — the caller must know it did not produce a plan).
pub(crate) async fn generate_plan(
    State(state): State<AppState>,
    Json(body): Json<PlanRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let plan = crate::planner::generate_plan(&state.gateway, &body.goal, body.context.as_deref())
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({ "error": e }))))?;
    let markdown = crate::planner::render_plan_md(&plan);
    Ok(Json(json!({ "plan": plan, "markdown": markdown })))
}
