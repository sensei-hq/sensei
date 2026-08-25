//! D-CHECKER endpoint — run a repo's adopted checker-backed rules and return the
//! pass/fail verdicts. See [`crate::checker`].

use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::api::state::AppState;

#[derive(Deserialize)]
pub(crate) struct RunCheckersRequest {
    /// Absolute repo path, or…
    #[serde(default)]
    pub folder: Option<String>,
    /// …a project name / UUID (resolved to its folder).
    #[serde(default)]
    pub project: Option<String>,
}

/// POST /api/checkers/run — `{folder|project}` → `{folder, runs: [CheckRun]}`.
/// Runs each checker-backed rule's command in the repo and records a verdict.
pub(crate) async fn run_checkers(
    State(state): State<AppState>,
    Json(body): Json<RunCheckersRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let (folder_path, folder_id) = crate::api::handlers::knowledge::resolve_folder(
        &state,
        body.folder.as_deref(),
        body.project.as_deref(),
    )
    .await?;
    let runs =
        crate::checker::run_checkers(&state.pg, &folder_id, std::path::Path::new(&folder_path))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e }))))?;
    Ok(Json(json!({ "folder": folder_path, "runs": runs })))
}
