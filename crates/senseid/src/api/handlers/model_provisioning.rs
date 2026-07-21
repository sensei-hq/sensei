//! On-demand model-provisioning HTTP surface.
//!
//! Two endpoints let a client drive + observe the pull of a local chat model:
//! - `POST /api/gateway/models/{id}/provision` — start (or join) a pull.
//! - `GET  /api/gateway/models/provision/status` — snapshot every tracked model.
//!
//! Both degrade gracefully when the daemon was built without the embedded
//! engine (`state.provisioning` is `None`): the POST returns `501 Not
//! Implemented` with an explanatory JSON body, and the GET returns an empty
//! `models` list. The pull itself only ever begins from the POST handler —
//! nothing here or at startup auto-pulls.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde_json::{json, Value};

use crate::api::state::AppState;

/// The [`kernel::ProvisionPhase`] serde shape (internally tagged `"phase"`,
/// snake_case) as a JSON value, e.g. `{"phase":"ready"}` or
/// `{"phase":"downloading","done":0,"total":null}`.
///
/// `to_value` on this in-memory enum cannot fail (no borrowed data, no custom
/// serializer that errors), but we surface a fallback rather than `unwrap` so a
/// future serde change can never panic the daemon — no silent error either: the
/// fallback still carries the phase name we can read from `Display`-free match.
fn phase_json(phase: &gateway::ProvisionPhase) -> Value {
    serde_json::to_value(phase).unwrap_or_else(|_| json!({"phase": "unknown"}))
}

/// POST /api/gateway/models/{id}/provision — begin (or join) an on-demand pull
/// of model `id`, returning its initial phase. Idempotent: a second call while a
/// pull is in flight joins the existing job (the supervisor dedups by id).
///
/// `501 Not Implemented` when the daemon lacks the embedded engine — the only
/// build that can pull + coldboot a local GGUF.
pub(crate) async fn provision_model(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let Some(sup) = &state.provisioning else {
        return Err((
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({
                "error": "embedded provisioning not available in this build",
            })),
        ));
    };

    // Non-blocking: spawn/join the background job and report the phase right now
    // (`Queued` for a fresh pull, or the live phase of an in-flight/ready one).
    let handle = sup.ensure(&id, local_engine::EnsureOpts { wait: false });
    let phase = handle.phase();
    Ok(Json(json!({
        "model": id,
        "phase": phase_json(&phase),
    })))
}

/// GET /api/gateway/models/provision/status — a snapshot of every model the
/// supervisor is (or has been) provisioning, with its current phase. Empty
/// `models` list when the daemon lacks the embedded engine.
pub(crate) async fn provision_status(State(state): State<AppState>) -> Json<Value> {
    let models: Vec<Value> = match &state.provisioning {
        Some(sup) => sup
            .status_all()
            .await
            .into_iter()
            .map(|(id, phase)| json!({ "id": id, "phase": phase_json(&phase) }))
            .collect(),
        None => Vec::new(),
    };
    Json(json!({ "models": models }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `phase_json` must match `kernel::ProvisionPhase`'s wire shape: an internal
    /// `"phase"` tag, snake_case, with the `Downloading` payload flattened.
    #[test]
    fn phase_json_matches_kernel_serde_shape() {
        assert_eq!(
            phase_json(&gateway::ProvisionPhase::Ready),
            json!({"phase": "ready"})
        );
        assert_eq!(
            phase_json(&gateway::ProvisionPhase::Absent),
            json!({"phase": "absent"})
        );
        assert_eq!(
            phase_json(&gateway::ProvisionPhase::Downloading { done: 5, total: Some(100) }),
            json!({"phase": "downloading", "done": 5, "total": 100})
        );
        assert_eq!(
            phase_json(&gateway::ProvisionPhase::Failed { error: "disk full".into() }),
            json!({"phase": "failed", "error": "disk full"})
        );
    }
}
