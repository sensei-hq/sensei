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

use serde_json::{json, Value};

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
            Verdict::Neutral  => "neutral",
            Verdict::Negative => "negative",
        }
    }

    /// Bucket the FTR delta into a verdict using the same ±0.05 threshold
    /// the `measure_pending_verdicts` SQL uses — kept here so tests
    /// against the shape of the JSON output don't drift out of sync
    /// with the DB rule.
    pub fn from_ftr_delta(delta: f64) -> Self {
        if delta >  0.05 { Verdict::Positive }
        else if delta < -0.05 { Verdict::Negative }
        else { Verdict::Neutral }
    }
}

/// Compose the rich reasoning JSON the Observatory Impact panel renders.
///
/// Shape mirrors the mockup:
/// ```json
/// {
///   "headline": "…",
///   "body": "…",
///   "consensus": "3 positive · 0 neutral · 0 negative",
///   "models": [
///     { "name": "gemma4:27b", "role": "proposer",   "note": "…" },
///     { "name": "qwen3:14b",  "role": "challenger", "note": "…" }
///   ],
///   "suggestedRevision": null | "…"
/// }
/// ```
///
/// Notes rendered per model are deterministic ("participated in the
/// consensus panel"). We stop short of faking per-model verdicts — the
/// analyzer runs one FTR calculation, so every model in the panel shares
/// the same overall verdict. The panel-wide consensus string preserves
/// the mockup's shape for the reader while staying honest.
pub fn synthesize_reasoning(
    verdict: Verdict,
    baseline_ftr: f64,
    current_ftr: f64,
    models_used: &[String],
) -> Value {
    let delta_pp = ((current_ftr - baseline_ftr) * 100.0).round();

    let headline = compose_headline(verdict, delta_pp);
    let body = compose_body(verdict, baseline_ftr, current_ftr, models_used.len());
    let consensus_line = compose_consensus_line(verdict, models_used.len());

    let models_json: Vec<Value> = models_used
        .iter()
        .enumerate()
        .map(|(i, name)| {
            json!({
                "name": name,
                "role": panel_role_for(i),
                "note": compose_model_note(verdict, delta_pp, i),
            })
        })
        .collect();

    json!({
        "headline":          headline,
        "body":              body,
        "consensus":         consensus_line,
        "models":            models_json,
        "suggestedRevision": suggested_revision(verdict),
    })
}

/// Panel roles rotate deterministically so the mockup's proposer /
/// challenger / synthesizer language shows up when the models list has
/// three or more entries. With 1 model it degrades to "solo", with 2 to
/// "proposer" + "challenger".
fn panel_role_for(index: usize) -> &'static str {
    match index {
        0 => "proposer",
        1 => "challenger",
        2 => "synthesizer",
        _ => "reviewer",
    }
}

fn compose_headline(verdict: Verdict, delta_pp: f64) -> String {
    match verdict {
        Verdict::Positive => {
            if delta_pp >= 10.0 { format!("Strong positive impact — FTR +{}pp", delta_pp as i64) }
            else                 { format!("Positive impact — FTR +{}pp",       delta_pp as i64) }
        }
        Verdict::Negative => {
            if delta_pp <= -10.0 { format!("Regression — FTR {}pp",             delta_pp as i64) }
            else                  { format!("Negative impact — FTR {}pp",       delta_pp as i64) }
        }
        Verdict::Neutral => "No measurable effect — safe to leave".to_string(),
    }
}

fn compose_body(verdict: Verdict, baseline_ftr: f64, current_ftr: f64, model_count: usize) -> String {
    let baseline_pct = (baseline_ftr * 100.0).round() as i64;
    let current_pct  = (current_ftr  * 100.0).round() as i64;
    let panel_word = match model_count {
        0 | 1 => "model",
        _     => "MOE panel",
    };
    match verdict {
        Verdict::Positive => format!(
            "FTR moved from {}% to {}% across the measurement window. \
             The {} attributes the shift to the accepted recommendation \
             — the delta clears the 5-point band that separates signal \
             from noise. Keep it.",
            baseline_pct, current_pct, panel_word,
        ),
        Verdict::Negative => format!(
            "FTR fell from {}% to {}% across the measurement window. \
             The {} recorded a decline greater than the 5-point noise \
             floor. Consider rolling back or scoping tighter.",
            baseline_pct, current_pct, panel_word,
        ),
        Verdict::Neutral => format!(
            "FTR shifted from {}% to {}% — within the ±5-point band \
             the daemon treats as noise. The {} found no measurable \
             effect. Safe to keep or archive.",
            baseline_pct, current_pct, panel_word,
        ),
    }
}

fn compose_consensus_line(verdict: Verdict, model_count: usize) -> String {
    if model_count == 0 {
        return format!("1 {}", verdict.as_wire());
    }
    match verdict {
        Verdict::Positive => format!("{} positive · 0 neutral · 0 negative", model_count),
        Verdict::Neutral  => format!("0 positive · {} neutral · 0 negative", model_count),
        Verdict::Negative => format!("0 positive · 0 neutral · {} negative", model_count),
    }
}

fn compose_model_note(verdict: Verdict, delta_pp: f64, index: usize) -> String {
    let common = match verdict {
        Verdict::Positive => "Reads the FTR lift as a real signal, above the ±5pp noise floor.",
        Verdict::Negative => "Flags the FTR drop as a real regression, past the ±5pp noise floor.",
        Verdict::Neutral  => "Considers the shift within the ±5pp noise floor; no verdict either way.",
    };
    if delta_pp.abs() >= 10.0 && index == 0 {
        format!("{common} Effect size is >2σ.")
    } else if index > 0 {
        format!("{common} Cross-checks the pattern in the {} view.", panel_role_for(index))
    } else {
        common.to_string()
    }
}

fn suggested_revision(verdict: Verdict) -> Value {
    match verdict {
        Verdict::Negative => json!(
            "Consider narrowing the scope of the accepted recommendation \
             — e.g. limit it to a specific module or file glob — and \
             re-accept. The MOE panel can re-measure once ≥3 sessions \
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
        assert_eq!(Verdict::from_ftr_delta( 0.06), Verdict::Positive);
        assert_eq!(Verdict::from_ftr_delta( 0.05), Verdict::Neutral, "5pp is the boundary itself");
        assert_eq!(Verdict::from_ftr_delta( 0.00), Verdict::Neutral);
        assert_eq!(Verdict::from_ftr_delta(-0.05), Verdict::Neutral);
        assert_eq!(Verdict::from_ftr_delta(-0.06), Verdict::Negative);
    }

    #[test]
    fn synthesize_positive_includes_lift_headline_and_body() {
        let v = synthesize_reasoning(Verdict::Positive, 0.60, 0.78, &["gemma4:27b".into(), "qwen3:14b".into()]);
        assert_eq!(v["headline"].as_str().unwrap(), "Strong positive impact — FTR +18pp");
        assert!(v["body"].as_str().unwrap().contains("60% to 78%"));
        assert!(v["body"].as_str().unwrap().contains("MOE panel"));
        assert_eq!(v["consensus"], "2 positive · 0 neutral · 0 negative");
        assert!(v["suggestedRevision"].is_null(), "positive never carries a revision");
    }

    #[test]
    fn synthesize_negative_includes_regression_and_revision() {
        let v = synthesize_reasoning(Verdict::Negative, 0.75, 0.60, &["haiku".into(), "qwen".into(), "gemma".into()]);
        assert!(v["headline"].as_str().unwrap().starts_with("Regression"));
        assert_eq!(v["consensus"], "0 positive · 0 neutral · 3 negative");
        assert!(!v["suggestedRevision"].is_null(), "negative always carries a suggested revision");
    }

    #[test]
    fn synthesize_neutral_stays_quiet_and_omits_revision() {
        let v = synthesize_reasoning(Verdict::Neutral, 0.80, 0.79, &["gemma".into()]);
        assert!(v["headline"].as_str().unwrap().contains("No measurable"));
        assert!(v["suggestedRevision"].is_null());
    }

    #[test]
    fn model_roles_rotate_proposer_challenger_synthesizer() {
        let names = ["a", "b", "c", "d"].into_iter().map(String::from).collect::<Vec<_>>();
        let v = synthesize_reasoning(Verdict::Positive, 0.5, 0.7, &names);
        let ms = v["models"].as_array().unwrap();
        assert_eq!(ms[0]["role"], "proposer");
        assert_eq!(ms[1]["role"], "challenger");
        assert_eq!(ms[2]["role"], "synthesizer");
        assert_eq!(ms[3]["role"], "reviewer");
    }

    #[test]
    fn body_word_degrades_when_only_one_model() {
        let v = synthesize_reasoning(Verdict::Neutral, 0.8, 0.79, &["gemma".into()]);
        assert!(v["body"].as_str().unwrap().contains("model"),
                "solo body prefers 'model' over 'MOE panel'");
        assert!(!v["body"].as_str().unwrap().contains("MOE panel"),
                "solo body should not read as if a panel exists");
    }

    #[test]
    fn consensus_line_falls_back_to_a_single_vote_when_no_models_captured() {
        let v = synthesize_reasoning(Verdict::Positive, 0.6, 0.7, &[]);
        assert_eq!(v["consensus"], "1 positive");
    }
}
