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

/// A slope smaller than this (in the metric's own units per period) reads as
/// flat — so float noise on a genuinely level series never becomes a spurious
/// "increasing"/"decreasing" trend.
const SLOPE_EPS: f64 = 1e-9;

/// A whole-series drift smaller than this FRACTION of the metric's own level
/// reads as flat/steady. Relative (not absolute) so it holds across magnitudes
/// (ftr 0–1, churn_rate in thousands, ttr in seconds). Without it, a sub-percent
/// wobble the chart shows as a flat line — e.g. rework_density 0.2571 → 0.2569
/// (~0.08%) — gets narrated as a real "decreasing" trend, so the prose contradicts
/// the sparkline. 2% is comfortably below "visible on the chart".
const REL_DRIFT_EPS: f64 = 0.02;

/// The whole-series trend of a metric — the direction the chart (the daily
/// sparkline) actually shows, so the narrative can't present a one-week dip as
/// the overall trend. `direction` is the raw numeric direction over the window;
/// `assessment` maps it to better/worse via the metric's own `direction` (a
/// rising `lower_better` metric is worsening — reusing [`change_word`] so the
/// sign mapping lives in ONE place). `window` names the span so the copy states
/// its scope. `first`/`last` are the real endpoints for grounding.
#[derive(Debug, Clone, PartialEq)]
struct SeriesTrend {
    direction: &'static str,  // "increasing" | "decreasing" | "flat"
    assessment: &'static str, // "improving" | "worsening" | "steady" | "changed"
    window: String,
    first: f64,
    last: f64,
}

/// Least-squares slope of `values` against their index (0, 1, …, n−1). `None`
/// for fewer than two points (no line to fit — never invent a trend from a
/// single reading). Pure: the trend fact is deterministic and unit-tested
/// without a DB or model.
fn least_squares_slope(values: &[f64]) -> Option<f64> {
    let n = values.len();
    if n < 2 {
        return None;
    }
    let n_f = n as f64;
    let mean_x = (n_f - 1.0) / 2.0;
    let mean_y = values.iter().sum::<f64>() / n_f;
    let (mut num, mut den) = (0.0, 0.0);
    for (i, &y) in values.iter().enumerate() {
        let dx = i as f64 - mean_x;
        num += dx * (y - mean_y);
        den += dx * dx;
    }
    if den == 0.0 {
        return None; // no spread in x — unreachable for n >= 2, guarded anyway
    }
    Some(num / den)
}

/// Classify a metric's movement over its FULL displayed (daily) series: the
/// numeric direction (least-squares slope over the whole window) plus its
/// better/worse reading. `None` when there are too few points to state a
/// direction. A slope within [`SLOPE_EPS`] reads as flat/steady. This is the
/// authoritative "trend" the copy should report — it matches the sparkline,
/// unlike the single most-recent weekly step (`delta`).
fn series_trend(values: &[f64], direction: &str) -> Option<SeriesTrend> {
    let slope = least_squares_slope(values)?;
    let first = *values.first()?;
    let last = *values.last()?;
    let n = values.len();
    let window = format!("the full {n}-point series");
    // Flat when the slope is ~zero OR the fitted end-to-end drift (slope × span)
    // is a negligible fraction of the metric's own level — a sub-threshold wobble
    // the chart renders as a flat line must read "steady", never a fabricated
    // direction. Relative to the level so it holds across metric magnitudes.
    let drift = slope.abs() * (n as f64 - 1.0);
    let level = values.iter().map(|v| v.abs()).sum::<f64>() / n as f64;
    let negligible = slope.abs() <= SLOPE_EPS || (level > SLOPE_EPS && drift / level < REL_DRIFT_EPS);
    if negligible {
        return Some(SeriesTrend { direction: "flat", assessment: "steady", window, first, last });
    }
    let numeric = if slope > 0.0 { "increasing" } else { "decreasing" };
    Some(SeriesTrend { direction: numeric, assessment: change_word(direction, slope), window, first, last })
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
/// framing when present), the OVERALL trend across the full displayed series, and
/// the single most-recent step vs the prior period. Strictly from the row + its
/// series — the model gets no room to invent a number.
///
/// The two movements are kept separate and each carries its own `window`, so the
/// copy can't present a one-week dip as the overall trend: `trend` is the
/// authoritative direction (it matches the sparkline the user sees), `recent` is
/// the latest weekly step. `series` is the metric's full daily values in
/// chronological order (what the chart plots); `None`/too-short omits `trend`.
fn signal_facts(m: &Value, series: Option<&[f64]>) -> Value {
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

    let dir = m.get("direction").and_then(Value::as_str).unwrap_or("neutral");

    // Overall trend across the full displayed (daily) series — the direction the
    // chart shows, mapped to better/worse via the metric's own direction. This is
    // the trend the copy reports; a rising `lower_better` metric reads as
    // worsening even when its most-recent weekly step dipped.
    if let Some(t) = series.and_then(|vals| series_trend(vals, dir)) {
        f.insert(
            "trend".into(),
            json!({
                "direction": t.direction,
                "assessment": t.assessment,
                "window": t.window,
                "first": t.first,
                "last": t.last,
            }),
        );
    }

    // The single most-recent step vs the prior weekly period — a recent detail,
    // never the overall trend. Explicitly windowed so the copy can't pass a
    // one-week move off as the trend.
    if let Some(prior) = m.get("prior").and_then(Value::as_f64) {
        f.insert("prior".into(), json!(prior));
    }
    if let Some(delta) = m.get("delta").and_then(Value::as_f64) {
        f.insert(
            "recent".into(),
            json!({
                "window": "vs the prior week",
                "delta": delta,
                "assessment": change_word(dir, delta),
            }),
        );
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
    series_by_metric: &std::collections::HashMap<String, Vec<f64>>,
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
        let series = series_by_metric.get(key).map(Vec::as_slice);
        let sfacts = signal_facts(m, series);
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
    fn signal_facts_carries_tool_relevance_and_recent_step() {
        let m = json!({
            "metric": "unused_tools", "name": "Unused-tool count", "metric_type": "count",
            "direction": "lower_better", "value": 3.0,
            "props": { "total_tools": 106, "relevant_tools": 12, "used_tools": 9 },
            "prior": 3.0, "delta": 0.0,
        });
        let f = signal_facts(&m, None);
        assert_eq!(f["relevant_tools"].as_i64(), Some(12));
        assert_eq!(f["used_tools"].as_i64(), Some(9));
        assert_eq!(f["total_tools"].as_i64(), Some(106));
        // No series → no overall-trend fact (never fabricate a trend).
        assert!(f.get("trend").is_none(), "no series → no trend fact");
        // The most-recent weekly step is windowed and read via the metric direction.
        assert_eq!(f["recent"]["assessment"].as_str(), Some("steady"));
        assert_eq!(f["recent"]["window"].as_str(), Some("vs the prior week"));
    }

    #[test]
    fn series_trend_rising_lower_better_is_worsening() {
        // interruption_rate is lower_better; a series that rises end-to-end is a
        // regression — the numeric direction is "increasing", the reading "worsening".
        let vals = vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0];
        let t = series_trend(&vals, "lower_better").expect("trend for >= 2 points");
        assert_eq!(t.direction, "increasing");
        assert_eq!(t.assessment, "worsening");
        assert_eq!(t.window, "the full 6-point series", "window is explicit");
    }

    #[test]
    fn series_trend_rising_higher_better_is_improving() {
        let vals = vec![0.1, 0.3, 0.5, 0.9];
        let t = series_trend(&vals, "higher_better").expect("trend");
        assert_eq!(t.direction, "increasing");
        assert_eq!(t.assessment, "improving");
    }

    #[test]
    fn series_trend_flat_is_steady() {
        let vals = vec![1.0, 1.0, 1.0, 1.0];
        let t = series_trend(&vals, "lower_better").expect("trend");
        assert_eq!(t.direction, "flat");
        assert_eq!(t.assessment, "steady");
    }

    #[test]
    fn series_trend_sub_threshold_wobble_reads_steady_not_a_trend() {
        // The live rework_density case: a monotone but microscopic decline
        // (0.2571 → 0.2562, ~0.35% of the level) that the chart renders as a flat
        // line. Its slope is far above the old absolute SLOPE_EPS (1e-9), so the
        // old logic narrated it "decreasing / becoming more efficient" — prose the
        // sparkline contradicts. The relative dead-band reads it "steady".
        let vals = vec![0.2571, 0.2568, 0.2565, 0.2562];
        let t = series_trend(&vals, "lower_better").expect("trend");
        assert_eq!(t.direction, "flat", "a sub-2%-of-level drift is flat");
        assert_eq!(t.assessment, "steady", "never narrated as a real trend the chart doesn't show");
    }

    #[test]
    fn series_trend_short_recent_dip_inside_long_rise_reads_over_full_window() {
        // The exact live bug: a long rise (lower_better, worsening overall) with a
        // late spike then a dip. The most-recent step is DOWN (looks "improving"),
        // but the trend over the full window is still up → worsening. The window
        // is stated so the copy scopes its claim.
        let vals = vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 2.5, 1.0];
        let t = series_trend(&vals, "lower_better").expect("trend");
        assert_eq!(t.direction, "increasing", "the full-window slope is up");
        assert_eq!(t.assessment, "worsening");
        assert_eq!(t.window, "the full 8-point series");
    }

    #[test]
    fn series_trend_needs_two_points() {
        assert!(series_trend(&[], "lower_better").is_none());
        assert!(series_trend(&[0.5], "lower_better").is_none(), "one reading is not a trend");
    }

    #[test]
    fn signal_facts_trend_overrides_the_recent_dip_for_a_rising_lower_better_metric() {
        // Regression guard for the live bug: interruption_rate rises over the
        // whole series (worsening) while its most-recent weekly step dipped from a
        // spike (recent "improving"). The overall `trend` fact must read
        // "worsening" and stay separate from the recent step — so the narrative
        // can no longer call the metric improving.
        let m = json!({
            "metric": "interruption_rate", "name": "Interruption rate", "metric_type": "ratio",
            "direction": "lower_better", "value": 1.0,
            "props": {}, "prior": 2.5, "delta": -1.5,
        });
        let series = vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 2.5, 1.0];
        let f = signal_facts(&m, Some(&series));
        assert_eq!(f["trend"]["direction"].as_str(), Some("increasing"));
        assert_eq!(f["trend"]["assessment"].as_str(), Some("worsening"));
        assert!(f["trend"]["window"].as_str().unwrap().starts_with("the full"));
        // The recent weekly step is present but clearly scoped as recent, and the
        // top-level facts carry no unqualified "change" verdict any more.
        assert_eq!(f["recent"]["assessment"].as_str(), Some("improving"));
        assert_eq!(f["recent"]["window"].as_str(), Some("vs the prior week"));
        assert!(f.get("change").is_none(), "no unwindowed top-level change verdict");
    }
}
