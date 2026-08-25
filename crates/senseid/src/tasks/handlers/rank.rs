//! Recommendation ranking pass (analyzer L2 tail, #65 "…and ranking them").
//!
//! Scores every *pending* recommendation for a project via the pure
//! [`crate::ranking`] core and marks the single focal "do first" pick. Runs at
//! the end of [`analyze_project`](super::analyze::analyze_project), after the
//! generator and consolidation have written all recs, so the whole pending set
//! is ranked together. Idempotent: re-running recomputes scores from the
//! current pattern signal and re-derives the focal pick.

use super::super::executor::TaskContext;
use crate::ranking::{self, ScoreFactors};
use std::collections::HashMap;

/// Score + rank a project's pending recommendations; returns how many were
/// scored. Clears any stale focal flag first so exactly one (or zero, when
/// nothing is pending) survives.
pub async fn rank_for_project(ctx: &TaskContext, project_id: &uuid::Uuid) -> Result<u32, String> {
    let pending = ctx.pg().get_pending_recs_for_ranking(project_id).await?;

    // Always clear focal up front: a previously-focal rec that was acted on (no
    // longer pending) or out-scored must not stay flagged.
    ctx.pg().clear_project_focal(project_id).await?;
    if pending.is_empty() {
        return Ok(0);
    }

    // Score each, carrying recurrence for the deterministic tiebreak.
    let scored: Vec<(uuid::Uuid, f64, i32)> = pending
        .iter()
        .map(|(id, action_type, urgency, confidence, max_recurrence)| {
            let s = ranking::score(&ScoreFactors {
                action_type: action_type.clone(),
                urgency: urgency.clone(),
                confidence: *confidence,
                max_recurrence: *max_recurrence,
            });
            (*id, s, *max_recurrence)
        })
        .collect();

    // id → factors, for the explainability breakdown persisted alongside.
    let factors: HashMap<uuid::Uuid, (String, String, Option<f64>, i32)> =
        pending.into_iter().map(|(id, a, u, c, m)| (id, (a, u, c, m))).collect();

    let ranked = ranking::rank_and_focal(scored);
    let mut written = 0u32;
    for r in &ranked {
        let breakdown = match factors.get(&r.id) {
            Some((action_type, urgency, confidence, recurrence)) => serde_json::json!({
                "action_type": action_type,
                "urgency": urgency,
                "confidence": confidence,
                "recurrence": recurrence,
                "action_weight": ranking::action_weight(action_type),
                "urgency_weight": ranking::urgency_weight(urgency),
            }),
            None => serde_json::json!({}),
        };
        match ctx.pg().set_recommendation_rank(&r.id, r.score, r.focal, &breakdown).await {
            Ok(_) => written += 1,
            Err(e) => tracing::warn!(error = %e, rec = %r.id, "rank_for_project: persist failed"),
        }
    }
    tracing::info!("rank_for_project: {project_id} — ranked {written} recs");
    Ok(written)
}
