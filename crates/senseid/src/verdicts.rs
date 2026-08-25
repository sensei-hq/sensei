//! Pure helpers for MOE verdict reasoning synthesis.
//!
//! The verdict measurement pass classifies an accepted recommendation as
//! positive / neutral / negative from a single FTR-delta threshold. That's
//! enough for a decision, but the Observatory Impact panel wants a richer
//! narrative — headline, body, per-model contribution notes, and (when the
//! outcome is negative) a suggested revision.
//!
//! The functions here are deterministic and pure. They compose a
//! `serde_json::Value` shaped to what `get_project_impact` projects
//! straight through to the UI. All inputs come from data the daemon
//! already has, so no additional LLM calls are required at measure time.

use serde_json::{Value, json};

/// The verdict the analyzer's `measure_pending_verdicts` writes.
/// Kept as a small enum so the compiler catches typos at call-sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Positive,
    Neutral,
    Negative,
}

impl Verdict {
    pub fn as_wire(self) -> &'static str {
        match self {
            Verdict::Positive => "positive",
            Verdict::Neutral => "neutral",
            Verdict::Negative => "negative",
        }
    }

    /// Bucket the FTR delta into a verdict using the same ±0.05 threshold
    /// the `measure_pending_verdicts` SQL uses — kept here so tests
    /// against the shape of the JSON output don't drift out of sync
    /// with the DB rule.
    pub fn from_ftr_delta(delta: f64) -> Self {
        if delta > 0.05 {
            Verdict::Positive
        } else if delta < -0.05 {
            Verdict::Negative
        } else {
            Verdict::Neutral
        }
    }
}

/// Compose the honest reasoning JSON the Observatory Impact panel renders.
///
/// Shape:
/// ```json
/// {
///   "headline": "…",
///   "body": "…",
///   "modelsUsed": ["gemma4:27b", "qwen3:14b"],
///   "suggestedRevision": null | "…"
/// }
/// ```
///
/// HONEST SINGLE VERDICT (the #109 fabrication audit): the analyzer runs ONE
/// FTR-delta calculation, so there is exactly one verdict — not an N-model vote.
/// We therefore do NOT synthesize a consensus tally ("3 positive · 0 neutral · …"),
/// per-model roles (proposer/challenger/synthesizer), per-model notes, or a ">2σ"
/// claim — none of those were measured. `modelsUsed` lists the real models that
/// ran in the measured sessions; the single verdict is carried by `headline`/`body`
/// (and by the recommendation row's own `verdict` field).
pub fn synthesize_reasoning(
    verdict: Verdict,
    baseline_ftr: f64,
    current_ftr: f64,
    models_used: &[String],
) -> Value {
    let delta_pp = ((current_ftr - baseline_ftr) * 100.0).round();
    json!({
        "headline":          compose_headline(verdict, delta_pp),
        "body":              compose_body(verdict, baseline_ftr, current_ftr),
        "modelsUsed":        models_used,
        "suggestedRevision": suggested_revision(verdict),
    })
}

fn compose_headline(verdict: Verdict, delta_pp: f64) -> String {
    match verdict {
        Verdict::Positive => {
            if delta_pp >= 10.0 {
                format!("Strong positive impact — FTR +{}pp", delta_pp as i64)
            } else {
                format!("Positive impact — FTR +{}pp", delta_pp as i64)
            }
        }
        Verdict::Negative => {
            if delta_pp <= -10.0 {
                format!("Regression — FTR {}pp", delta_pp as i64)
            } else {
                format!("Negative impact — FTR {}pp", delta_pp as i64)
            }
        }
        Verdict::Neutral => "No measurable effect — safe to leave".to_string(),
    }
}

fn compose_body(verdict: Verdict, baseline_ftr: f64, current_ftr: f64) -> String {
    let baseline_pct = (baseline_ftr * 100.0).round() as i64;
    let current_pct = (current_ftr * 100.0).round() as i64;
    // Attribute to the FTR measurement itself — NOT a multi-model deliberation
    // that never happened (there is one FTR-delta calculation).
    match verdict {
        Verdict::Positive => format!(
            "FTR moved from {}% to {}% across the measurement window — the delta \
             clears the 5-point band that separates signal from noise. Keep it.",
            baseline_pct, current_pct,
        ),
        Verdict::Negative => format!(
            "FTR fell from {}% to {}% across the measurement window — a decline \
             greater than the 5-point noise floor. Consider rolling back or \
             scoping tighter.",
            baseline_pct, current_pct,
        ),
        Verdict::Neutral => format!(
            "FTR shifted from {}% to {}% — within the ±5-point band the daemon \
             treats as noise. No measurable effect. Safe to keep or archive.",
            baseline_pct, current_pct,
        ),
    }
}

fn suggested_revision(verdict: Verdict) -> Value {
    match verdict {
        Verdict::Negative => json!(
            "Consider narrowing the scope of the accepted recommendation \
             — e.g. limit it to a specific module or file glob — and \
             re-accept. The daemon can re-measure once ≥3 sessions \
             have landed under the tighter scope."
        ),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ftr_delta_buckets_verdicts_at_the_5pp_boundary() {
        assert_eq!(Verdict::from_ftr_delta(0.06), Verdict::Positive);
        assert_eq!(Verdict::from_ftr_delta(0.05), Verdict::Neutral, "5pp is the boundary itself");
        assert_eq!(Verdict::from_ftr_delta(0.00), Verdict::Neutral);
        assert_eq!(Verdict::from_ftr_delta(-0.05), Verdict::Neutral);
        assert_eq!(Verdict::from_ftr_delta(-0.06), Verdict::Negative);
    }

    #[test]
    fn synthesize_positive_includes_lift_headline_body_and_real_models() {
        let v = synthesize_reasoning(
            Verdict::Positive,
            0.60,
            0.78,
            &["gemma4:27b".into(), "qwen3:14b".into()],
        );
        assert_eq!(v["headline"].as_str().unwrap(), "Strong positive impact — FTR +18pp");
        assert!(v["body"].as_str().unwrap().contains("60% to 78%"));
        // modelsUsed carries the REAL models that ran — verbatim, no invented order/role.
        assert_eq!(v["modelsUsed"], json!(["gemma4:27b", "qwen3:14b"]));
        assert!(v["suggestedRevision"].is_null(), "positive never carries a revision");
    }

    #[test]
    fn synthesize_negative_includes_regression_and_revision() {
        let v = synthesize_reasoning(
            Verdict::Negative,
            0.75,
            0.60,
            &["haiku".into(), "qwen".into(), "gemma".into()],
        );
        assert!(v["headline"].as_str().unwrap().starts_with("Regression"));
        assert_eq!(v["modelsUsed"], json!(["haiku", "qwen", "gemma"]));
        assert!(!v["suggestedRevision"].is_null(), "negative always carries a suggested revision");
    }

    #[test]
    fn synthesize_neutral_stays_quiet_and_omits_revision() {
        let v = synthesize_reasoning(Verdict::Neutral, 0.80, 0.79, &["gemma".into()]);
        assert!(v["headline"].as_str().unwrap().contains("No measurable"));
        assert!(v["suggestedRevision"].is_null());
    }

    #[test]
    fn no_models_captured_yields_an_empty_models_list_not_a_fabricated_vote() {
        let v = synthesize_reasoning(Verdict::Positive, 0.6, 0.7, &[]);
        assert_eq!(v["modelsUsed"], json!([]));
    }

    /// The #109 audit guard: the synth must NOT fabricate a consensus tally,
    /// per-model panelist entries, or a ">2σ" effect-size claim — none of those
    /// were measured (there is one FTR-delta verdict).
    #[test]
    fn synth_never_fabricates_a_consensus_panel() {
        for models in [
            vec![],
            vec!["a".to_string()],
            vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()],
        ] {
            let v = synthesize_reasoning(Verdict::Positive, 0.5, 0.9, &models);
            assert!(v.get("consensus").is_none(), "no fabricated consensus vote tally");
            assert!(v.get("models").is_none(), "no fabricated per-model panelist entries");
            let body = v["body"].as_str().unwrap();
            assert!(!body.contains("panel"), "body must not imply a multi-model panel");
            assert!(!body.contains("σ") && !body.contains("2σ"), "no invented effect-size claim");
        }
    }
}
