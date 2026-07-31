//! Review endpoints. `POST /api/review/risk-class` — the review-depth gate (E1).

use axum::Json;
use serde::Deserialize;

use crate::review::{resolve_risk_class, RiskAssessment};

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
