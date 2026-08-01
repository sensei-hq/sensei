//! Recommendation ranking (#65 — "…extracting patterns, memory, insights and
//! *ranking* them").
//!
//! Recommendations carry only `urgency` today, and the generator emits almost
//! everything at `medium` — so the janitorial `audit_stale` recs (the bulk of
//! the table) sort identically to the high-leverage `revise_rule` /
//! `write_skill` / `promote_pattern` ones and drown them out. This module
//! computes a transparent 0–100 **score** per recommendation and marks the
//! single highest-scoring pending rec per project as the **focal** "do first"
//! pick (the Observatory's "today's koan").
//!
//! The score is a weighted sum of four signals that already exist in the data:
//! the *kind of action* (most leverage → `revise_rule`; least → `audit_stale`),
//! the rec's `urgency`, and — joined from the source `detected_patterns` via
//! `based_on.patterns` — the average pattern `confidence` and the maximum
//! `instance_count` (recurrence). Weights sum to 1.0 and every factor is
//! normalized to 0–1, so the result lands cleanly in 0–100 and the breakdown is
//! explainable (persisted to `based_on.score_factors`).

/// Recurrence saturates at this many instances — a pattern seen 5× is "very
/// recurring"; more doesn't meaningfully raise the score.
pub const RECURRENCE_CAP: i32 = 5;

// Weighted-sum coefficients (sum to 1.0). Action kind dominates so that
// leverage, not volume, sets the order; recurrence/confidence break ties.
const W_ACTION: f64 = 0.40;
const W_URGENCY: f64 = 0.25;
const W_CONFIDENCE: f64 = 0.20;
const W_RECURRENCE: f64 = 0.15;

/// Confidence to assume when a rec has no joinable pattern confidence — neutral,
/// so such recs are neither rewarded nor punished on that axis.
const NEUTRAL_CONFIDENCE: f64 = 0.5;

/// Leverage of an action kind, 0–1. The canonical `action_type` set (see
/// `recommendations.action_type`): governance changes move the needle most,
/// `audit_stale` is housekeeping. Unknown kinds get a neutral 0.5.
pub fn action_weight(action_type: &str) -> f64 {
    match action_type {
        "revise_rule" => 1.0,
        "write_skill" => 0.9,
        "promote_pattern" => 0.85,
        "create_agent" => 0.8,
        "enrich_memory" => 0.6,
        "cross_project" => 0.6,
        "archive_memory" => 0.4,
        "library_update" => 0.55,
        "audit_stale" => 0.25,
        _ => 0.5,
    }
}

/// Urgency as a 0–1 weight. Unknown → medium.
pub fn urgency_weight(urgency: &str) -> f64 {
    match urgency {
        "high" => 1.0,
        "medium" => 0.6,
        "low" => 0.3,
        _ => 0.6,
    }
}

/// Recurrence normalized to 0–1, saturating at [`RECURRENCE_CAP`].
pub fn recurrence_norm(instance_count: i32) -> f64 {
    let n = instance_count.clamp(0, RECURRENCE_CAP);
    f64::from(n) / f64::from(RECURRENCE_CAP)
}

/// The signals a single recommendation is scored on.
#[derive(Debug, Clone)]
pub struct ScoreFactors {
    pub action_type: String,
    pub urgency: String,
    /// Average `confidence` of the source patterns (`based_on.patterns`).
    /// `None` when the rec references no joinable pattern.
    pub confidence: Option<f64>,
    /// Max `instance_count` across the source patterns (recurrence).
    pub max_recurrence: i32,
}

/// Weighted-sum score in 0–100 (1 dp). All factors are 0–1 and weights sum to
/// 1.0, so the raw sum is 0–1 before scaling.
pub fn score(f: &ScoreFactors) -> f64 {
    let conf = f.confidence.unwrap_or(NEUTRAL_CONFIDENCE).clamp(0.0, 1.0);
    let raw = W_ACTION * action_weight(&f.action_type)
        + W_URGENCY * urgency_weight(&f.urgency)
        + W_CONFIDENCE * conf
        + W_RECURRENCE * recurrence_norm(f.max_recurrence);
    (raw * 1000.0).round() / 10.0
}

/// A ranked recommendation: its score and whether it's the project's focal pick.
#[derive(Debug, Clone, PartialEq)]
pub struct Ranked {
    pub id: uuid::Uuid,
    pub score: f64,
    pub focal: bool,
}

/// Order a project's scored recs and mark the top one focal. Deterministic:
/// score desc → recurrence desc → id asc (recs have no created_at). The first
/// element after sorting is the single focal "do first" pick; an empty input
/// yields no focal.
pub fn rank_and_focal(mut scored: Vec<(uuid::Uuid, f64, i32)>) -> Vec<Ranked> {
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.2.cmp(&a.2))
            .then_with(|| a.0.cmp(&b.0))
    });
    scored
        .into_iter()
        .enumerate()
        .map(|(i, (id, score, _))| Ranked { id, score, focal: i == 0 })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_weight_sinks_audit_stale_below_governance() {
        assert!(action_weight("revise_rule") > action_weight("write_skill"));
        assert!(action_weight("write_skill") > action_weight("promote_pattern"));
        assert!(action_weight("promote_pattern") > action_weight("audit_stale"));
        // The whole point: housekeeping is the lowest-leverage kind.
        assert_eq!(action_weight("audit_stale"), 0.25);
        assert_eq!(action_weight("unknown_kind"), 0.5, "unknown → neutral");
    }

    #[test]
    fn urgency_weight_orders_high_medium_low() {
        assert!(urgency_weight("high") > urgency_weight("medium"));
        assert!(urgency_weight("medium") > urgency_weight("low"));
        assert_eq!(urgency_weight("garbage"), 0.6, "unknown → medium");
    }

    #[test]
    fn recurrence_norm_saturates_at_cap() {
        assert_eq!(recurrence_norm(0), 0.0);
        assert_eq!(recurrence_norm(RECURRENCE_CAP), 1.0);
        assert_eq!(recurrence_norm(RECURRENCE_CAP + 10), 1.0, "saturates");
        assert_eq!(recurrence_norm(-3), 0.0, "clamps negatives");
    }

    #[test]
    fn score_is_bounded_0_to_100() {
        let best = score(&ScoreFactors {
            action_type: "revise_rule".into(),
            urgency: "high".into(),
            confidence: Some(1.0),
            max_recurrence: 100,
        });
        assert_eq!(best, 100.0, "max of every factor → 100");

        let worst = score(&ScoreFactors {
            action_type: "audit_stale".into(),
            urgency: "low".into(),
            confidence: Some(0.0),
            max_recurrence: 0,
        });
        // 0.40*0.25 + 0.25*0.30 + 0 + 0 = 0.175 → 17.5
        assert_eq!(worst, 17.5);
        assert!(worst >= 0.0 && best <= 100.0);
    }

    #[test]
    fn score_demotes_audit_stale_under_revise_rule_at_same_urgency() {
        // The core behavioral fix: a medium audit_stale must rank below a
        // medium revise_rule even with identical confidence/recurrence.
        let common = |action: &str| ScoreFactors {
            action_type: action.into(),
            urgency: "medium".into(),
            confidence: Some(0.75),
            max_recurrence: 2,
        };
        assert!(score(&common("revise_rule")) > score(&common("audit_stale")));
    }

    #[test]
    fn score_missing_confidence_is_neutral_not_zero() {
        let none = score(&ScoreFactors {
            action_type: "promote_pattern".into(),
            urgency: "medium".into(),
            confidence: None,
            max_recurrence: 1,
        });
        let half = score(&ScoreFactors {
            action_type: "promote_pattern".into(),
            urgency: "medium".into(),
            confidence: Some(0.5),
            max_recurrence: 1,
        });
        assert_eq!(none, half, "None confidence treated as 0.5");
    }

    #[test]
    fn rank_and_focal_marks_single_top_pick() {
        let a = uuid::Uuid::from_u128(1);
        let b = uuid::Uuid::from_u128(2);
        let c = uuid::Uuid::from_u128(3);
        let ranked = rank_and_focal(vec![(a, 30.0, 1), (b, 80.0, 1), (c, 50.0, 1)]);
        assert_eq!(ranked[0].id, b, "highest score first");
        assert!(ranked[0].focal, "top is focal");
        assert!(!ranked[1].focal && !ranked[2].focal, "exactly one focal");
        // descending
        assert!(ranked[0].score >= ranked[1].score && ranked[1].score >= ranked[2].score);
    }

    #[test]
    fn rank_and_focal_tiebreaks_recurrence_then_id() {
        let a = uuid::Uuid::from_u128(10);
        let b = uuid::Uuid::from_u128(20);
        let c = uuid::Uuid::from_u128(30);
        // same score: b has higher recurrence → wins; a vs c tie recurrence → id asc.
        let ranked = rank_and_focal(vec![(c, 50.0, 1), (a, 50.0, 1), (b, 50.0, 3)]);
        assert_eq!(ranked[0].id, b, "higher recurrence breaks score tie");
        assert_eq!(ranked[1].id, a, "then lower id");
        assert_eq!(ranked[2].id, c);
    }

    #[test]
    fn rank_and_focal_empty_has_no_focal() {
        assert!(rank_and_focal(vec![]).is_empty());
    }
}
