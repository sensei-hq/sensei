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
    let phase = sup.ensure(&id);
    Ok(Json(json!({
        "model": id,
        "phase": phase_json(&phase),
    })))
}

/// Merge the fixed provisioning catalog with the supervisor's live phases into
/// the status wire rows. Every catalog `(id, name)` appears exactly once; its
/// phase is the live phase from `live` when the supervisor is tracking it, else
/// `Absent` (not started). Output order follows the catalog (stable), so a
/// model shows up with phase `absent` before any pull begins.
///
/// Pure over its inputs — unit-testable without a runtime or the embedded
/// engine.
#[cfg(feature = "embedded-llama-cpp")]
fn merge_catalog_phases(
    catalog: Vec<(String, String)>,
    live: Vec<(String, gateway::ProvisionPhase)>,
) -> Vec<Value> {
    use std::collections::HashMap;
    let live: HashMap<String, gateway::ProvisionPhase> = live.into_iter().collect();
    catalog
        .into_iter()
        .map(|(id, name)| {
            let phase = live.get(&id).cloned().unwrap_or(gateway::ProvisionPhase::Absent);
            json!({ "id": id, "name": name, "phase": phase_json(&phase) })
        })
        .collect()
}

/// GET /api/gateway/models/provision/status — the full provisionable catalog,
/// each entry with its current phase. A catalog model appears with phase
/// `absent` before any pull begins; once a pull starts the supervisor's live
/// phase overlays it. Empty `models` list when the daemon lacks the embedded
/// engine (no catalog, nothing to pull).
#[cfg(feature = "embedded-llama-cpp")]
pub(crate) async fn provision_status(State(state): State<AppState>) -> Json<Value> {
    let models: Vec<Value> = match &state.provisioning {
        Some(sup) => merge_catalog_phases(
            crate::api::model_provisioning::provisioning_catalog(),
            sup.status_all().await,
        ),
        None => Vec::new(),
    };
    Json(json!({ "models": models }))
}

/// GET /api/gateway/models/provision/status — non-embedded build. There is no
/// catalog and no supervisor, so the status is always an empty `models` list;
/// a client can still poll unconditionally.
#[cfg(not(feature = "embedded-llama-cpp"))]
pub(crate) async fn provision_status(State(_state): State<AppState>) -> Json<Value> {
    Json(json!({ "models": [] }))
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

    /// The status shape merges the fixed catalog with live phases: a catalog
    /// entry with NO live phase appears with `absent` (so the UI can list a
    /// pullable model before any pull), while an entry the supervisor is
    /// tracking gets its live phase. Order follows the catalog.
    #[cfg(feature = "embedded-llama-cpp")]
    #[test]
    fn merge_catalog_phases_defaults_untracked_to_absent_and_overlays_live() {
        let catalog = vec![
            ("gemma2:2b".to_string(), "Gemma 2 2B Instruct".to_string()),
            ("phantom:1b".to_string(), "Phantom 1B".to_string()),
        ];
        let live = vec![(
            "gemma2:2b".to_string(),
            gateway::ProvisionPhase::Downloading { done: 5, total: Some(100) },
        )];

        let rows = super::merge_catalog_phases(catalog, live);
        assert_eq!(rows.len(), 2, "every catalog entry appears once");

        // Tracked model overlays its live phase, carries id + display name.
        assert_eq!(rows[0]["id"], "gemma2:2b");
        assert_eq!(rows[0]["name"], "Gemma 2 2B Instruct");
        assert_eq!(
            rows[0]["phase"],
            json!({"phase": "downloading", "done": 5, "total": 100}),
        );

        // Untracked catalog model defaults to `absent` — still pullable.
        assert_eq!(rows[1]["id"], "phantom:1b");
        assert_eq!(rows[1]["name"], "Phantom 1B");
        assert_eq!(rows[1]["phase"], json!({"phase": "absent"}));
    }

    /// The real catalog merged against an empty live set (no pull started yet)
    /// yields the whole catalog with every model `absent` — the first-load
    /// state the UI must render before any pull.
    #[cfg(feature = "embedded-llama-cpp")]
    #[test]
    fn merge_catalog_phases_first_load_is_full_catalog_all_absent() {
        let rows = super::merge_catalog_phases(
            crate::api::model_provisioning::provisioning_catalog(),
            Vec::new(),
        );
        assert_eq!(rows.len(), 1, "one on-demand model today");
        assert_eq!(rows[0]["id"], "gemma2:2b");
        assert_eq!(rows[0]["name"], "Gemma 2 2B");
        assert_eq!(rows[0]["phase"], json!({"phase": "absent"}));
    }
}
