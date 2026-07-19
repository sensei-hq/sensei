//! Front-door intake: axes -> playbook recommendation (§8).

use axum::{extract::State, Json};

use crate::api::state::AppState;
use crate::playbook::{recommend, Axes, Intent, Lifecycle, Risk};

/// POST /api/playbook/recommend  { lifecycle, intent, risk, session_id?, feature?, confirm? }
///
/// Pre-classified axes in, a playbook recommendation out. This endpoint takes
/// the axes directly (no LLM involved), so every run it persists is recorded
/// `classified_by = "manual"` — Task 9's `classify_chunk` path is the one that
/// sets `classified_by` to the real gateway model id or "heuristic-fallback".
pub(crate) async fn recommend_playbook(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let (Some(lf), Some(it), Some(rk)) = (
        body["lifecycle"].as_str().and_then(Lifecycle::parse),
        body["intent"].as_str().and_then(Intent::parse),
        body["risk"].as_str().and_then(Risk::parse),
    ) else {
        return Json(serde_json::json!({ "error": "lifecycle/intent/risk required (valid axis values)" }));
    };
    let axes = Axes { lifecycle: lf, intent: it, risk: rk };

    let rules = match state.pg.list_playbook_rules().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("recommend_playbook: list_playbook_rules failed: {e}");
            return Json(serde_json::json!({ "error": e }));
        }
    };
    let rec = recommend(&axes, &rules);

    // Persist the run (recommend-and-confirm defaults confirmed=false until the caller confirms).
    let confirmed = body["confirm"].as_bool().unwrap_or(false);
    let session_id = body["session_id"].as_str().and_then(|s| s.parse().ok());
    if let Err(e) = state
        .pg
        .insert_playbook_run(
            session_id,
            body["feature"].as_str(),
            lf.as_str(),
            it.as_str(),
            rk.as_str(),
            rec.rule_id,
            &rec.playbook,
            &rec.rationale,
            confirmed,
            Some("manual"),
            false,
        )
        .await
    {
        tracing::error!("recommend_playbook: insert_playbook_run failed: {e}");
    }

    Json(serde_json::json!({
        "playbook": rec.playbook,
        "rationale": rec.rationale,
        "rule": rec.rule_name,
        "defaulted": rec.defaulted,
    }))
}
