//! Model-effectiveness insight (#65 — the analyzer *weighting by model*).
//!
//! The multi-model corpus (Zed + Claude, see [`crate::transcript::zed`]) tags
//! each session with the model that produced it. When one model clearly
//! out-performs the others *within a project* — higher First-Try-Right over a
//! meaningful sample — that's an actionable signal: prefer it here. This module
//! turns per-project model stats into that recommendation.
//!
//! Pure + deterministic; the DB load + rec write live in the handler.

/// FTR over a sample of sessions for one model in a project.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelStat {
    pub provider: String,
    pub model: String,
    pub sessions: i64,
    pub ftr_rate: f64,
}

/// A "prefer this model here" recommendation: the winner, the runner-up it beat,
/// and the FTR gap.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelReco {
    pub provider: String,
    pub model: String,
    pub ftr_rate: f64,
    pub sessions: i64,
    pub runner_up_model: String,
    pub runner_up_ftr: f64,
    pub gain: f64,
}

/// Minimum sessions on a model before its FTR is trustworthy enough to compare.
pub const MIN_SESSIONS: i64 = 3;
/// Minimum FTR lead the top model must hold over the runner-up to recommend it.
pub const MIN_GAIN: f64 = 0.15;

/// Recommend a model when, among those with ≥`min_sessions`, the best beats the
/// runner-up by ≥`min_gain` FTR. `None` when there's no clear winner (fewer than
/// two qualifying models, or the lead is too small — no recommendation rather
/// than a noisy one). Deterministic: ties broken by more sessions, then name.
pub fn recommend_model(stats: &[ModelStat], min_sessions: i64, min_gain: f64) -> Option<ModelReco> {
    let mut qualifying: Vec<&ModelStat> = stats.iter().filter(|s| s.sessions >= min_sessions).collect();
    if qualifying.len() < 2 {
        return None; // need a comparison to say "prefer X over Y"
    }
    qualifying.sort_by(|a, b| {
        b.ftr_rate
            .partial_cmp(&a.ftr_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.sessions.cmp(&a.sessions))
            .then_with(|| a.model.cmp(&b.model))
    });
    let top = qualifying[0];
    let runner = qualifying[1];
    let gain = ((top.ftr_rate - runner.ftr_rate) * 1000.0).round() / 1000.0;
    if gain < min_gain {
        return None;
    }
    Some(ModelReco {
        provider: top.provider.clone(),
        model: top.model.clone(),
        ftr_rate: top.ftr_rate,
        sessions: top.sessions,
        runner_up_model: runner.model.clone(),
        runner_up_ftr: runner.ftr_rate,
        gain,
    })
}

/// Urgency for a model recommendation: a large FTR lead is worth surfacing high.
pub fn reco_urgency(gain: f64) -> &'static str {
    if gain >= 0.30 {
        "high"
    } else {
        "medium"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stat(model: &str, sessions: i64, ftr: f64) -> ModelStat {
        ModelStat { provider: "p".into(), model: model.into(), sessions, ftr_rate: ftr }
    }

    #[test]
    fn recommends_clear_winner() {
        let stats = vec![stat("opus", 10, 0.95), stat("gpt", 8, 0.60), stat("grok", 5, 0.70)];
        let r = recommend_model(&stats, MIN_SESSIONS, MIN_GAIN).unwrap();
        assert_eq!(r.model, "opus");
        assert_eq!(r.runner_up_model, "grok", "runner-up is the 2nd-best qualifying model");
        assert!((r.gain - 0.25).abs() < 1e-9);
    }

    #[test]
    fn no_reco_when_lead_too_small() {
        // 0.80 vs 0.72 → gain 0.08 < 0.15
        let stats = vec![stat("a", 10, 0.80), stat("b", 10, 0.72)];
        assert!(recommend_model(&stats, MIN_SESSIONS, MIN_GAIN).is_none());
    }

    #[test]
    fn no_reco_without_two_qualifying_models() {
        // only one model has enough sessions
        let stats = vec![stat("a", 10, 0.95), stat("b", 2, 0.40)];
        assert!(recommend_model(&stats, MIN_SESSIONS, MIN_GAIN).is_none());
        // single model, plenty of sessions, still nothing to compare against
        assert!(recommend_model(&[stat("a", 20, 0.99)], MIN_SESSIONS, MIN_GAIN).is_none());
    }

    #[test]
    fn ignores_low_sample_models_in_ranking() {
        // 'noise' has the highest FTR but too few sessions → excluded; winner is 'a'
        let stats = vec![stat("noise", 1, 1.0), stat("a", 10, 0.90), stat("b", 10, 0.50)];
        let r = recommend_model(&stats, MIN_SESSIONS, MIN_GAIN).unwrap();
        assert_eq!(r.model, "a");
        assert!((r.gain - 0.40).abs() < 1e-9);
    }

    #[test]
    fn urgency_scales_with_gain() {
        assert_eq!(reco_urgency(0.40), "high");
        assert_eq!(reco_urgency(0.20), "medium");
    }
}
