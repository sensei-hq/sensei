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
    let mut qualifying: Vec<&ModelStat> =
        stats.iter().filter(|s| s.sessions >= min_sessions).collect();
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
    if gain >= 0.30 { "high" } else { "medium" }
}

/// Canonicalize a model label so casing/spacing variants aggregate: providers
/// report the same model as "Claude Sonnet 4" / "claude-sonnet-4", "GPT-4.1" /
/// "gpt-4.1", etc. Lowercase; runs of space/`_`/`-` collapse to a single `-`;
/// `.` and `:` (version/tag separators) are preserved. Conservative — it does
/// NOT unify version-format differences (`3.7` vs `3-7`) or tags (`:latest`),
/// which can be semantically distinct.
pub fn normalize_model(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_sep = false;
    for ch in raw.trim().chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_whitespace() || c == '-' || c == '_' {
            if !out.is_empty() && !prev_sep {
                out.push('-');
                prev_sep = true;
            }
        } else {
            out.push(c);
            prev_sep = false;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// Fold raw per-`(provider, raw_model)` session stats into per-`(provider,
/// canonical_model)` [`ModelStat`]s — merging label variants and re-weighting
/// FTR by session count. Input: `(provider, model, sessions, ftr_sessions)`.
/// Deterministic order: more sessions first, then model name.
pub fn fold_model_stats(raw: Vec<(String, String, i64, i64)>) -> Vec<ModelStat> {
    use std::collections::HashMap;
    // key → (sessions, ftr_sessions)
    let mut acc: HashMap<(String, String), (i64, i64)> = HashMap::new();
    for (provider, model, sessions, ftr_sessions) in raw {
        let e = acc.entry((provider, normalize_model(&model))).or_insert((0, 0));
        e.0 += sessions;
        e.1 += ftr_sessions;
    }
    let mut out: Vec<ModelStat> = acc
        .into_iter()
        .map(|((provider, model), (sessions, ftr))| ModelStat {
            provider,
            model,
            sessions,
            ftr_rate: if sessions > 0 { ftr as f64 / sessions as f64 } else { 0.0 },
        })
        .collect();
    out.sort_by(|a, b| b.sessions.cmp(&a.sessions).then_with(|| a.model.cmp(&b.model)));
    out
}

/// Fold raw effectiveness rows into the per-`(provider, canonical_model)` JSON
/// the endpoint serves. Input: `(provider, model, sessions, ftr_sessions,
/// corrections_sum, turns_sum)`. Merges label variants; rounds rates.
pub fn fold_effectiveness(
    raw: Vec<(String, String, i64, i64, i64, i64)>,
) -> Vec<serde_json::Value> {
    use std::collections::HashMap;
    // key → (sessions, ftr_sessions, corrections, turns)
    let mut acc: HashMap<(String, String), (i64, i64, i64, i64)> = HashMap::new();
    for (provider, model, sessions, ftr, corr, turns) in raw {
        let e = acc.entry((provider, normalize_model(&model))).or_insert((0, 0, 0, 0));
        e.0 += sessions;
        e.1 += ftr;
        e.2 += corr;
        e.3 += turns;
    }
    // (sessions, model) carried alongside the JSON only for the sort.
    let mut rows: Vec<(i64, String, serde_json::Value)> = acc
        .into_iter()
        .map(|((provider, model), (sessions, ftr, corr, turns))| {
            let s = sessions.max(1) as f64;
            let json = serde_json::json!({
                "provider": provider,
                "model": model,
                "sessions": sessions,
                "ftrRate": ((ftr as f64 / s) * 1000.0).round() / 1000.0,
                "avgCorrections": ((corr as f64 / s) * 100.0).round() / 100.0,
                "avgTurns": ((turns as f64 / s) * 10.0).round() / 10.0,
            });
            (sessions, model, json)
        })
        .collect();
    rows.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    rows.into_iter().map(|(_, _, json)| json).collect()
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

    #[test]
    fn normalize_model_merges_casing_and_spacing() {
        assert_eq!(normalize_model("Claude Sonnet 4"), "claude-sonnet-4");
        assert_eq!(normalize_model("claude-sonnet-4"), "claude-sonnet-4");
        assert_eq!(normalize_model("GPT-4.1"), "gpt-4.1");
        assert_eq!(normalize_model("gpt-4.1"), "gpt-4.1");
        assert_eq!(normalize_model("GPT-5 mini"), "gpt-5-mini");
        assert_eq!(normalize_model("Grok Code Fast 1"), "grok-code-fast-1");
        assert_eq!(
            normalize_model("  Claude   Opus  4.5 "),
            "claude-opus-4.5",
            "collapses runs + trims"
        );
        // conservative: version-format + tags are NOT unified (kept distinct)
        assert_ne!(normalize_model("claude-3.7-sonnet"), normalize_model("claude-3-7-sonnet"));
        assert_eq!(normalize_model("gemma4:latest"), "gemma4:latest", "tag preserved");
    }

    #[test]
    fn fold_model_stats_merges_label_variants_and_reweights() {
        // "Claude Sonnet 4" (4 sessions, 4 ftr) + "claude-sonnet-4" (6 sessions, 3 ftr)
        // → one canonical model, 10 sessions, ftr 7/10 = 0.7
        let raw = vec![
            ("copilot".into(), "Claude Sonnet 4".into(), 4, 4),
            ("copilot".into(), "claude-sonnet-4".into(), 6, 3),
            ("copilot".into(), "GPT-5".into(), 3, 1),
        ];
        let folded = fold_model_stats(raw);
        let sonnet = folded.iter().find(|m| m.model == "claude-sonnet-4").unwrap();
        assert_eq!(sonnet.sessions, 10, "variants merged");
        assert!((sonnet.ftr_rate - 0.7).abs() < 1e-9, "FTR re-weighted by sessions");
        // different provider stays distinct
        let raw2 = vec![
            ("copilot".into(), "claude-sonnet-4".into(), 5, 5),
            ("anthropic".into(), "claude-sonnet-4".into(), 5, 0),
        ];
        assert_eq!(fold_model_stats(raw2).len(), 2, "same model, different provider → distinct");
    }

    #[test]
    fn fold_effectiveness_merges_and_orders_by_volume() {
        let raw = vec![
            ("copilot".into(), "Claude Sonnet 4".into(), 4, 4, 1, 40),
            ("copilot".into(), "claude-sonnet-4".into(), 6, 2, 5, 60),
            ("openai".into(), "GPT-5".into(), 2, 2, 0, 30),
        ];
        let out = fold_effectiveness(raw);
        assert_eq!(out[0]["model"], "claude-sonnet-4", "highest volume first");
        assert_eq!(out[0]["sessions"], 10);
        assert_eq!(out[0]["ftrRate"], 0.6, "(4+2)/10");
        assert_eq!(out[0]["avgCorrections"], 0.6, "(1+5)/10");
        assert_eq!(out[0]["avgTurns"], 10.0, "(40+60)/10");
    }
}
