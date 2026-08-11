//! Project-metrics narrative — the headline + per-signal "what sensei noticed"
//! prose for the metrics screen. A thin producer over [`insight_copy`]: it turns
//! the already-assembled per-metric JSON (the exact rows the metrics endpoint
//! serves, with `prior`/`delta` merged in) into grounded `facts`, reads the
//! local-model copy from cache, and warms a miss in the background.
//!
//! Contract (DRY across the daemon + the app):
//! - This module NEVER fabricates a sentence. On a cold cache it OMITS the
//!   entry and warms the cache for next time; the app renders its own
//!   deterministic, data-grounded sentence in the gap. So the fallback copy
//!   lives in exactly one place (the app), and the model prose lives here.
//! - Inference stays off the wire: [`insight_copy::warm`] spawns a detached
//!   task; this function only ever does cache reads on the request path.
//! - Facts are built strictly from the real values/props/deltas — the same
//!   never-fabricate rule the metric computers follow.

use serde_json::{json, Map, Value};

use super::insight_copy::{self, CopyLimits, InsightKind};
use crate::db::pg_store::PgStore;

/// The registry key of the composite health metric — it is the hero, not a mover.
const KEY_HEALTH: &str = "project_health";

/// Classify a change as improving / worsening / steady, honouring the metric's
/// own direction (a rising `lower_better` metric is worsening). Mirrors the
/// app's trend-tone logic so the model's framing agrees with the rendered rule.
fn change_word(direction: &str, delta: f64) -> &'static str {
    if delta == 0.0 {
        return "steady";
    }
    let up = delta > 0.0;
    match direction {
        "higher_better" => {
            if up {
                "improving"
            } else {
                "worsening"
            }
        }
        "lower_better" => {
            if up {
                "worsening"
            } else {
                "improving"
            }
        }
        _ => "changed",
    }
}

/// Whole-project facts for the headline: how many signals moved, the split, the
/// movers (name + direction), and the health score. Read from the metric rows.
fn headline_facts(metrics: &[Value]) -> Value {
    let mut movers: Vec<Value> = Vec::new();
    let (mut worsening, mut improving) = (0u32, 0u32);
    let mut health = Value::Null;

    for m in metrics {
        let key = m.get("metric").and_then(Value::as_str).unwrap_or("");
        if key == KEY_HEALTH {
            health = json!({ "value": m.get("value").cloned().unwrap_or(Value::Null),
                             "delta": m.get("delta").cloned().unwrap_or(Value::Null) });
            continue;
        }
        let dir = m.get("direction").and_then(Value::as_str).unwrap_or("neutral");
        let Some(delta) = m.get("delta").and_then(Value::as_f64) else { continue };
        if delta == 0.0 {
            continue;
        }
        let word = change_word(dir, delta);
        match word {
            "worsening" => worsening += 1,
            "improving" => improving += 1,
            _ => {}
        }
        let name = m.get("name").and_then(Value::as_str).unwrap_or(key);
        movers.push(json!({ "name": name, "change": word }));
    }

    json!({
        "moved_count": movers.len(),
        "worsening": worsening,
        "improving": improving,
        "movers": movers,
        "health": health,
    })
}

/// Per-signal facts: the value, its numerator/denominator (and the tool-relevance
/// framing when present), and its change vs the prior period. Strictly from the
/// row — the model gets no room to invent a number.
fn signal_facts(m: &Value) -> Value {
    let mut f = Map::new();
    f.insert("metric".into(), m.get("name").cloned().unwrap_or(Value::Null));
    f.insert("type".into(), m.get("metric_type").cloned().unwrap_or(Value::Null));
    f.insert("direction".into(), m.get("direction").cloned().unwrap_or(Value::Null));
    f.insert("value".into(), m.get("value").cloned().unwrap_or(Value::Null));

    if let Some(props) = m.get("props").and_then(Value::as_object) {
        for k in ["numerator", "denominator", "relevant_tools", "used_tools", "total_tools"] {
            if let Some(v) = props.get(k) {
                f.insert(k.into(), v.clone());
            }
        }
    }
    // The metric's own authored definition — so the model reads what it MEANS
    // (code churn, not customer churn) instead of guessing from the name.
    for (fact_key, row_key) in [("meaning", "how_to_read"), ("purpose", "purpose")] {
        if let Some(v) = m.get(row_key).filter(|v| !v.is_null()) {
            f.insert(fact_key.into(), v.clone());
        }
    }
    if let Some(prior) = m.get("prior").and_then(Value::as_f64) {
        f.insert("prior".into(), json!(prior));
    }
    if let Some(delta) = m.get("delta").and_then(Value::as_f64) {
        let dir = m.get("direction").and_then(Value::as_str).unwrap_or("neutral");
        f.insert("delta".into(), json!(delta));
        f.insert("change".into(), json!(change_word(dir, delta)));
    }
    Value::Object(f)
}

/// Build the `narrative` object for `GET /api/projects/{id}/metrics`:
/// `{ headline?, subhead?, insights: { <metric_key>: sentence } }`. Only
/// model-generated entries are included; a cold-cache entry is omitted (and its
/// generation warmed) so the app renders its own deterministic copy in the gap.
pub async fn build_narrative(
    store: &PgStore,
    gateway: &std::sync::Arc<gateway::Gateway>,
    metrics: &[Value],
) -> Value {
    let limits = CopyLimits::default();
    let mut out = Map::new();
    let mut insights = Map::new();

    // Headline (+ subhead) for the whole snapshot.
    let hfacts = headline_facts(metrics);
    match insight_copy::read_cached_copy(store, InsightKind::MetricNarrativeHeadline, &hfacts).await {
        Some(c) => {
            out.insert("headline".into(), json!(c.title));
            if !c.detail.is_empty() {
                out.insert("subhead".into(), json!(c.detail));
            }
        }
        None => insight_copy::warm(store, gateway, InsightKind::MetricNarrativeHeadline, &hfacts, limits),
    }

    // One "what sensei noticed" sentence per signal.
    for m in metrics {
        let Some(key) = m.get("metric").and_then(Value::as_str) else { continue };
        let sfacts = signal_facts(m);
        match insight_copy::read_cached_copy(store, InsightKind::MetricSignalInsight, &sfacts).await {
            Some(c) => {
                insights.insert(key.to_string(), json!(c.detail));
            }
            None => insight_copy::warm(store, gateway, InsightKind::MetricSignalInsight, &sfacts, limits),
        }
    }

    out.insert("insights".into(), Value::Object(insights));
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metric(key: &str, name: &str, direction: &str, delta: Option<f64>) -> Value {
        json!({
            "metric": key,
            "name": name,
            "metric_type": "pct",
            "direction": direction,
            "value": 0.5,
            "props": {},
            "prior": delta.map(|d| 0.5 - d),
            "delta": delta,
        })
    }

    #[test]
    fn change_word_honours_direction() {
        assert_eq!(change_word("higher_better", 0.2), "improving");
        assert_eq!(change_word("higher_better", -0.2), "worsening");
        assert_eq!(change_word("lower_better", 0.2), "worsening");
        assert_eq!(change_word("lower_better", -0.2), "improving");
        assert_eq!(change_word("neutral", 0.2), "changed");
        assert_eq!(change_word("higher_better", 0.0), "steady");
    }

    #[test]
    fn headline_facts_counts_movers_excluding_health_and_flat() {
        let metrics = vec![
            metric("project_health", "Health", "higher_better", Some(-0.02)), // hero, not a mover
            metric("ftr", "FTR", "higher_better", Some(0.1)),                 // improving
            metric("time_to_useful_result", "TTUR", "lower_better", Some(0.2)), // worsening
            metric("dup", "Duplication", "lower_better", Some(0.0)),          // flat → skipped
            metric("noprior", "New", "higher_better", None),                  // no delta → skipped
        ];
        let f = headline_facts(&metrics);
        assert_eq!(f["moved_count"].as_u64(), Some(2));
        assert_eq!(f["worsening"].as_u64(), Some(1));
        assert_eq!(f["improving"].as_u64(), Some(1));
        assert!(f["health"]["value"].is_number(), "health carried for the hero");
        let names: Vec<&str> = f["movers"].as_array().unwrap().iter().map(|m| m["name"].as_str().unwrap()).collect();
        assert!(!names.contains(&"Health"), "health is never a mover");
    }

    #[test]
    fn signal_facts_carries_tool_relevance_and_change() {
        let m = json!({
            "metric": "unused_tools", "name": "Unused-tool count", "metric_type": "count",
            "direction": "lower_better", "value": 3.0,
            "props": { "total_tools": 106, "relevant_tools": 12, "used_tools": 9 },
            "prior": 3.0, "delta": 0.0,
        });
        let f = signal_facts(&m);
        assert_eq!(f["relevant_tools"].as_i64(), Some(12));
        assert_eq!(f["used_tools"].as_i64(), Some(9));
        assert_eq!(f["total_tools"].as_i64(), Some(106));
        assert_eq!(f["change"].as_str(), Some("steady"));
    }
}
