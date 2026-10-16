//! Review endpoints. `POST /api/review/risk-class` — the review-depth gate (E1);
//! `POST /api/review/subagent-model` — the model that class routes a subagent to (#238).

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::api::state::AppState;
use crate::review::{RISK_MODELS_CONFIG_KEY, RiskAssessment, RiskModels, resolve_risk_class};

#[derive(Deserialize)]
pub(crate) struct RiskClassBody {
    /// Changed file paths (repo-relative or absolute — only the substrings matter).
    #[serde(default)]
    pub paths: Vec<String>,
    /// Optional task description; escalates an otherwise-low change if it's sensitive.
    #[serde(default)]
    pub task: Option<String>,
}

/// Classify a change's required review depth (`auto | review | approve`) from its
/// changed paths + optional task text. Pure — no DB, no auth needed; backs the
/// `resolve_risk_class` MCP tool and `/sensei:review`.
pub(crate) async fn risk_class(Json(body): Json<RiskClassBody>) -> Json<RiskAssessment> {
    Json(resolve_risk_class(&body.paths, body.task.as_deref()))
}

/// A risk assessment plus the model the user's `routing.risk_models` maps its
/// class to — `null` when unmapped or unconfigured, which overrides nothing.
#[derive(Serialize)]
pub(crate) struct SubagentModel {
    #[serde(flatten)]
    assessment: RiskAssessment,
    model: Option<String>,
}

/// Which model a subagent should run on for this change (#238 step 3): the
/// sensei plugin's `agent.spawn` hook asks with the working tree's changed paths
/// and the subagent's task. A config read that fails is a 500, never "unmapped".
pub(crate) async fn subagent_model(
    State(state): State<AppState>,
    Json(body): Json<RiskClassBody>,
) -> Result<Json<SubagentModel>, StatusCode> {
    let raw = state.pg.get_config(RISK_MODELS_CONFIG_KEY).await.map_err(|e| {
        tracing::error!(error = %e, "subagent_model: config read failed");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let assessment = resolve_risk_class(&body.paths, body.task.as_deref());
    let model = RiskModels::parse(raw.as_deref())
        .and_then(|m| m.model_for(assessment.class).map(str::to_string));
    Ok(Json(SubagentModel { assessment, model }))
}
