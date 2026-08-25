//! Model-effectiveness recommendation (#65 — analyzer weighting by model).
//!
//! Reads a project's per-model FTR (from the multi-model corpus) and, when one
//! model clearly out-performs the others, writes a `revise_rule` recommendation
//! to prefer it here. Runs in the analyze pass after the heuristic generator,
//! before ranking. Idempotent: skips if a pending rec already proposes that model.

use super::super::executor::TaskContext;
use crate::model_insight;

/// Generate the per-project "prefer model X" recommendation if the data warrants
/// it. Returns 1 if a rec was written, else 0.
pub async fn model_insight_for_project(
    ctx: &TaskContext,
    project_id: &uuid::Uuid,
) -> Result<u32, String> {
    // Stats are already folded to canonical models (label variants merged).
    let stats = ctx.pg().get_project_model_stats(project_id).await?;

    let Some(reco) = model_insight::recommend_model(
        &stats,
        model_insight::MIN_SESSIONS,
        model_insight::MIN_GAIN,
    ) else {
        return Ok(0);
    };

    // Idempotency: one standing rec per recommended model.
    if ctx.pg().model_recommendation_exists(project_id, &reco.model).await.unwrap_or(false) {
        return Ok(0);
    }

    let ftr_pct = (reco.ftr_rate * 100.0).round() as i64;
    let runner_pct = (reco.runner_up_ftr * 100.0).round() as i64;
    let gain_pct = (reco.gain * 100.0).round() as i64;
    let title = format!("Prefer {} for this project", reco.model);
    let why = format!(
        "{} ({}) is first-try-right {ftr_pct}% over {} sessions here, vs {} at {runner_pct}% — a {gain_pct}-point lead. Defaulting to it should reduce rework.",
        reco.model, reco.provider, reco.sessions, reco.runner_up_model
    );
    let impact = format!("Projected FTR uplift toward {ftr_pct}% by defaulting to {}.", reco.model);
    let based_on = serde_json::json!({
        "recommended_model": reco.model,
        "recommended_provider": reco.provider,
        "model_ftr": reco.ftr_rate,
        "sessions": reco.sessions,
        "runner_up_model": reco.runner_up_model,
        "runner_up_ftr": reco.runner_up_ftr,
        "gain": reco.gain,
    });
    let urgency = model_insight::reco_urgency(reco.gain);

    match ctx
        .pg()
        .create_recommendation_full(
            project_id,
            &title,
            &why,
            Some(&impact),
            "revise_rule",
            urgency,
            &based_on,
            None,
            None,
        )
        .await
    {
        Ok(_) => {
            tracing::info!("model_insight: {project_id} — recommend {}", reco.model);
            Ok(1)
        }
        Err(e) => {
            tracing::warn!(error = %e, project = %project_id, "model_insight: create_recommendation failed");
            Ok(0)
        }
    }
}
