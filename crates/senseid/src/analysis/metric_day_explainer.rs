//! Per-datapoint metric explainer — the one-line companion that explains WHY a
//! single day's metric value is what it is (project × metric × day, DAILY grain).
//!
//! Architecture (this is the load-bearing rule — get it exactly right): the
//! explainer is produced at COMPUTE time, together with the value, by the
//! metrics compute dispatcher (see [`crate::tasks::handlers::metrics::explainer`]).
//! It is NOT lazily generated on the read path.
//!
//! The invariant that makes a compute-time explainer correct is the cache key:
//! the [`MetricDayFacts`] fed to the insight-copy cache INCLUDE THE VALUE (and its
//! prior day + delta). So on a recompute of the same day —
//! - value unchanged → identical `facts_hash` → cache HIT → reuse, NO model call,
//!   the explainer stays byte-identical;
//! - value changed → new `facts_hash` → cache MISS → regenerate.
//!
//! Because the planner only computes MISSING days (data − covered + a trailing
//! window + today), each day is explained once when its value is first written; a
//! forced/trailing recompute re-checks the cache and regenerates only if the value
//! moved.
//!
//! Never fabricates: [`MetricDayFacts::fallback_detail`] is built entirely from the
//! given numbers, so a model miss (or a stubbed/absent gateway) degrades to an
//! honest, number-grounded line — never a blank and never an invented figure.

use serde_json::json;

use super::insight_copy::{self, CopyLimits, InsightKind};
use crate::db::pg_store::PgStore;

/// The deterministic facts one day's explainer is built from — a single
/// (project, metric, day) datapoint. `value`/`prev_value`/`delta` are the
/// load-bearing part of the cache key: the `facts_hash` changes iff the value (or
/// its day-over-day context) changes, so an unchanged value re-hits the cache and
/// never re-calls the model. The display-only metric label is deliberately absent
/// (a label rename must not orphan cached copy — mirrors
/// [`crate::analysis::session_metric_note`]).
#[derive(Debug, Clone, PartialEq)]
pub struct MetricDayFacts {
    /// The registry key of the metric this datapoint measures (e.g. `ftr`).
    pub metric: String,
    /// What this metric measures (registry `how_to_read`). Empty when the key
    /// names no registered metric — the model then leans on the key + numbers.
    pub meaning: String,
    /// This day's stored value (the number the chart plots).
    pub value: f64,
    /// The immediately-prior day's value for this metric, or `None` when this is
    /// the first day (honest-null, never a fabricated 0).
    pub prev_value: Option<f64>,
    /// `value − prev_value` when a prior day exists, else `None`.
    pub delta: Option<f64>,
    /// The day this datapoint is FOR (`YYYY-MM-DD`) — the day context the copy
    /// grounds in, and part of the cache key so each day is its own entry.
    pub day: String,
    /// Measurable sessions that COMPLETED that day (day context, never invented).
    pub sessions_completed: i64,
    /// All measurable (analyzed) sessions that day.
    pub sessions_total: i64,
    /// Sessions resolved first-try that day.
    pub first_try: i64,
}

impl MetricDayFacts {
    /// Stable JSON fed to the insight-copy chain (and hashed for the copy cache).
    /// `value`/`prev_value`/`delta` ARE part of the facts — that is what makes the
    /// hash change iff the value changes (cache miss → regenerate) and stay stable
    /// when the value is unchanged (cache hit → reuse, no model call). Key order is
    /// irrelevant — [`insight_copy::facts_hash`] canonicalises it.
    pub fn to_facts_json(&self) -> serde_json::Value {
        json!({
            "metric": self.metric,
            "meaning": self.meaning,
            "value": self.value,
            "prev_value": self.prev_value,
            "delta": self.delta,
            "day": self.day,
            "sessions_completed": self.sessions_completed,
            "sessions_total": self.sessions_total,
            "first_try": self.first_try,
        })
    }

    /// The deterministic explainer line shown when the model is unavailable or its
    /// copy fails validation. Built ENTIRELY from the given numbers — e.g.
    /// `"value 0.75, up 0.25 from the prior day"` — so it is always honest and never
    /// fabricates a figure. With no prior day it is just `"value <v>"`.
    pub fn fallback_detail(&self) -> String {
        let mut line = format!("value {}", fmt_num(self.value));
        match self.delta {
            Some(d) if d > 0.0 => line.push_str(&format!(", up {} from the prior day", fmt_num(d))),
            Some(d) if d < 0.0 => line.push_str(&format!(", down {} from the prior day", fmt_num(-d))),
            Some(_) => line.push_str(", unchanged from the prior day"),
            None => {}
        }
        line
    }
}

/// Render an `f64` compactly: an integral value drops its fractional part
/// (`20.0` → `"20"`), otherwise trailing zeros are trimmed (`0.7500` → `"0.75"`).
/// Deterministic so two identical values always render identically (the fallback
/// line, like the facts hash, must be stable).
fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return format!("{}", v as i64);
    }
    let s = format!("{v:.4}");
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    trimmed.to_string()
}

/// Produce the explainer line for ONE (project, metric, day) datapoint.
///
/// Cache-guarded and safe to `await` INLINE from a background compute task: on a
/// cache HIT it returns the persisted copy with no model call (so an unchanged
/// value is never re-generated); on a MISS it calls
/// [`insight_copy::generate_and_cache`] — which is time-boxed (8s) and breaker-
/// guarded (60s), so a down/stubbed model degrades to `None` fast without blocking
/// backfill. On any model `None` it returns the deterministic
/// [`MetricDayFacts::fallback_detail`]. Never blocks, never errors, never fabricates.
pub async fn explain(
    store: &PgStore,
    gateway: &gateway::Gateway,
    facts: &MetricDayFacts,
) -> String {
    let facts_json = facts.to_facts_json();
    if let Some(copy) = insight_copy::read_cached_copy(store, InsightKind::MetricDayExplainer, &facts_json).await {
        return copy.detail;
    }
    // Miss → generate inline (off-wire, at compute time). A down/stub model
    // returns None fast; we then fall back to the deterministic line.
    if let Some(copy) =
        insight_copy::generate_and_cache(store, gateway, InsightKind::MetricDayExplainer, &facts_json, CopyLimits::default()).await
    {
        return copy.detail;
    }
    facts.fallback_detail()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(value: f64, prev: Option<f64>) -> MetricDayFacts {
        MetricDayFacts {
            metric: "ftr".into(),
            meaning: "share of sessions resolved first-try".into(),
            value,
            prev_value: prev,
            delta: prev.map(|p| value - p),
            day: "2026-08-11".into(),
            sessions_completed: 3,
            sessions_total: 4,
            first_try: 3,
        }
    }

    #[test]
    fn facts_json_carries_value_prev_and_delta() {
        let f = facts(0.75, Some(0.5));
        let v = f.to_facts_json();
        assert_eq!(v["metric"], json!("ftr"));
        assert_eq!(v["meaning"], json!("share of sessions resolved first-try"));
        assert_eq!(v["value"], json!(0.75));
        assert_eq!(v["prev_value"], json!(0.5));
        assert_eq!(v["delta"], json!(0.25));
        assert_eq!(v["day"], json!("2026-08-11"));
        assert_eq!(v["sessions_completed"], json!(3));
        assert_eq!(v["sessions_total"], json!(4));
        assert_eq!(v["first_try"], json!(3));
        // Display-only label is never hashed (mirrors session_metric_note).
        assert!(v.get("metric_label").is_none());
    }

    #[test]
    fn hash_changes_iff_value_changes_and_is_stable_when_unchanged() {
        let a = facts(0.75, Some(0.5)).to_facts_json();
        // Same facts EXCEPT the value — the hash MUST differ (cache miss → regenerate).
        let mut b = a.clone();
        b["value"] = json!(0.80);
        assert_ne!(
            insight_copy::facts_hash(InsightKind::MetricDayExplainer, &a),
            insight_copy::facts_hash(InsightKind::MetricDayExplainer, &b),
            "the value is part of the hash — a changed value must miss the cache",
        );
        // Two independently-built identical facts hash identically (cache hit →
        // reuse, NO model call for an unchanged value).
        let a2 = facts(0.75, Some(0.5)).to_facts_json();
        assert_eq!(
            insight_copy::facts_hash(InsightKind::MetricDayExplainer, &a),
            insight_copy::facts_hash(InsightKind::MetricDayExplainer, &a2),
            "identical facts (unchanged value) hash identically — no regeneration",
        );
    }

    #[test]
    fn fallback_names_the_value_and_direction() {
        // Up from the prior day.
        assert_eq!(facts(0.75, Some(0.5)).fallback_detail(), "value 0.75, up 0.25 from the prior day");
        // Down from the prior day (the delta sign is honoured, magnitude is positive).
        assert_eq!(facts(0.5, Some(0.75)).fallback_detail(), "value 0.5, down 0.25 from the prior day");
    }

    #[test]
    fn fallback_handles_no_prior_day() {
        // First day for a metric → no prev/delta → just the value, never a fabricated move.
        assert_eq!(facts(0.75, None).fallback_detail(), "value 0.75");
    }

    #[test]
    fn fallback_handles_unchanged_value_and_integers() {
        // Delta of exactly 0 reads as unchanged (never "up 0"/"down 0").
        assert_eq!(facts(0.75, Some(0.75)).fallback_detail(), "value 0.75, unchanged from the prior day");
        // Integral values (counts, whole-second durations) drop the fractional part.
        assert_eq!(facts(4.0, Some(2.0)).fallback_detail(), "value 4, up 2 from the prior day");
        assert_eq!(facts(20.0, None).fallback_detail(), "value 20");
    }

    #[test]
    fn fallback_reads_only_given_numbers_even_with_no_sessions() {
        // A day with zero session context still yields an honest value-only line
        // (the session counts are day context, never required for the fallback).
        let f = MetricDayFacts {
            sessions_completed: 0,
            sessions_total: 0,
            first_try: 0,
            ..facts(0.5, None)
        };
        assert_eq!(f.fallback_detail(), "value 0.5");
    }
}
