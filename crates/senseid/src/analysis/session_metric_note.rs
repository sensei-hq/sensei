//! Per-session, per-metric observation — deterministic facts → one mentor-voice
//! line for the metric drill-down.
//!
//! The drill-down (`GET /api/projects/{id}/metrics/{key}/sessions?day=…`) lists
//! the measurable sessions behind one daily datapoint. Each session already
//! carries a "what was achieved" summary (`activity.sessions.summary`, produced
//! by [`session_retro`](crate::tasks::handlers::session_retro)); this module adds
//! the missing half — a one-line "why THIS session moved THIS metric" note,
//! grounded strictly in the session ROW fields the drill-down already has plus
//! the metric's registry meaning. The deterministic facts stay code-owned; only
//! the prose sentence routes through the model.
//!
//! Wire-path producer (mirrors [`metric_narrative`](super::metric_narrative)): the
//! handler routes each session's facts through [`insight_copy::copy_or_warm`] — a
//! cache read on the wire that returns the deterministic row-derived fallback
//! immediately on a miss while warming the model copy off-wire for the next load.
//! This is INTENDED, not a bug: the first drill-down for a (session, metric) shows
//! the deterministic line; the next shows the model copy. Inference never blocks
//! the wire (see [`insight_copy`]).
//!
//! Never fabricates: the fallback is built entirely from the row's own fields, so
//! a cache miss always shows an honest, row-derived line — never a blank and never
//! an invented number.

use serde_json::json;

use super::insight_copy::{self, CopyLimits, FallbackCopy, InsightCopy, InsightKind};
use crate::db::pg_store::PgStore;

/// The deterministic per-(session, metric) facts one observation is built from.
/// Every field comes from the drill-down session row plus the metric's registry
/// meaning (looked up once per request) — nothing here routes through a model, so
/// the same (session, metric) always produces the same facts (and thus the same
/// cached copy under `facts_hash`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionMetricFacts {
    /// The registry key of the drilled-into metric (e.g. `ftr`). Part of the
    /// facts so the SAME session produces a DISTINCT observation per metric.
    pub metric_key: String,
    /// The metric's display label (registry `name`) — the fallback title and the
    /// model's anchor. Falls back to the key when the metric is unregistered.
    pub metric_label: String,
    /// What this metric measures (registry `how_to_read`). Empty when the key
    /// names no registered metric — the model then leans on the metric key alone.
    pub meaning: String,
    /// The session outcome (`completed` / `corrected` / …). Always present on a
    /// measurable session (the drill-down selects `outcome IS NOT NULL`).
    pub outcome: String,
    /// First-try resolution. `None` only when enrichment hasn't stamped it.
    pub ftr: Option<bool>,
    pub corrections: i32,
    pub turns: i32,
    /// The session's task line (may be empty).
    pub task: String,
    /// The session's "what was achieved" summary (may be empty until the
    /// retrospective workstream backfills it).
    pub summary: String,
}

impl SessionMetricFacts {
    /// Build the facts from ONE drill-down session row (the JSON
    /// [`get_project_sessions_for_day`](crate::db::pg_store::PgStore::get_project_sessions_for_day)
    /// emits) plus the metric's registry meaning (looked up once per request).
    /// Reads only fields already present on the row — never fabricates a value.
    /// The JSON accessors default defensively (a structurally-guaranteed column
    /// that decodes oddly reads as empty/`0`, not an invented value); this is
    /// projection of an already-fetched row, not a mask over a failed DB read.
    pub fn from_session_row(
        row: &serde_json::Value,
        metric_key: &str,
        metric_label: &str,
        meaning: &str,
    ) -> Self {
        SessionMetricFacts {
            metric_key: metric_key.to_string(),
            metric_label: metric_label.to_string(),
            meaning: meaning.to_string(),
            outcome: row.get("outcome").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            ftr: row.get("ftr").and_then(|v| v.as_bool()),
            corrections: row.get("corrections").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            turns: row.get("turns").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            task: row.get("task").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            summary: row.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        }
    }

    /// Stable JSON fed to the insight-copy chain (and hashed for the copy cache).
    /// Carries the metric key + its meaning and the session's own signals; the
    /// display-only `metric_label` is deliberately EXCLUDED so a pure rename of a
    /// metric's label never orphans cached copy. Key order is irrelevant —
    /// `facts_hash` canonicalises it.
    pub fn to_facts_json(&self) -> serde_json::Value {
        json!({
            "metric": self.metric_key,
            "meaning": self.meaning,
            "outcome": self.outcome,
            "ftr": self.ftr,
            "corrections": self.corrections,
            "turns": self.turns,
            "task": self.task,
            "summary": self.summary,
        })
    }

    /// The effort descriptor for the fallback line — "first-try" when the session
    /// resolved on the first attempt, otherwise the correction count (or a plain
    /// "not first-try" when corrections weren't recorded). Reads only the row's
    /// own `ftr`/`corrections`, so it is always honest.
    fn effort_phrase(&self) -> String {
        if self.ftr == Some(true) {
            return "first-try".to_string();
        }
        match self.corrections {
            0 if self.ftr == Some(false) => "not first-try".to_string(),
            0 => "no corrections".to_string(),
            1 => "1 correction".to_string(),
            n => format!("{n} corrections"),
        }
    }

    /// Deterministic copy shown immediately on a cache miss (and warmed off-wire
    /// for next load). Title is the metric label; detail is a structural line
    /// read entirely from the row — e.g. `"outcome completed; first-try; 3 turns"`.
    /// Never blank (outcome is always present) and never fabricated.
    pub fn fallback(&self) -> FallbackCopy {
        let turns =
            if self.turns == 1 { "1 turn".to_string() } else { format!("{} turns", self.turns) };
        // FTR / rework are effort metrics → lead with the effort (first-try / N
        // corrections). Other metrics (latency, throughput, …) are NOT about
        // first-try, so a neutral "outcome · turns" line avoids implying an
        // effort relevance the metric doesn't have (e.g. "first-try" on
        // time_to_useful_result read as a non-sequitur).
        let is_effort = matches!(self.metric_key.as_str(), "ftr" | "rework_ratio");
        let detail = if is_effort {
            format!("outcome {}; {}; {turns}", self.outcome, self.effort_phrase())
        } else {
            format!("outcome {}; {turns}", self.outcome)
        };
        FallbackCopy { title: self.metric_label.clone(), detail }
    }
}

/// Produce the drill-down observation for one session against one metric.
/// Wire-path: [`insight_copy::copy_or_warm`] returns the cached model copy on a
/// hit, or the deterministic row-derived [`SessionMetricFacts::fallback`]
/// immediately on a miss (warming the model copy off-wire). Never blocks the
/// wire, never errors, never fabricates.
pub async fn session_metric_observation(
    store: &PgStore,
    gateway: &std::sync::Arc<gateway::Gateway>,
    facts: &SessionMetricFacts,
) -> InsightCopy {
    insight_copy::copy_or_warm(
        store,
        gateway,
        InsightKind::SessionMetricObservation,
        &facts.to_facts_json(),
        CopyLimits::default(),
        facts.fallback(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(outcome: &str, ftr: Option<bool>, corrections: i32, turns: i32) -> serde_json::Value {
        json!({
            "client_session_id": "cs-1",
            "started_at": "2020-06-15T12:00:00Z",
            "outcome": outcome,
            "ftr": ftr,
            "turns": turns,
            "corrections": corrections,
            "task": "wire the drill-down note",
            "summary": "added the observation field",
        })
    }

    #[test]
    fn from_row_reads_only_present_fields() {
        let f = SessionMetricFacts::from_session_row(
            &row("completed", Some(true), 0, 3),
            "ftr",
            "First-try rate",
            "share resolved on the first attempt",
        );
        assert_eq!(f.metric_key, "ftr");
        assert_eq!(f.metric_label, "First-try rate");
        assert_eq!(f.meaning, "share resolved on the first attempt");
        assert_eq!(f.outcome, "completed");
        assert_eq!(f.ftr, Some(true));
        assert_eq!(f.corrections, 0);
        assert_eq!(f.turns, 3);
        assert_eq!(f.task, "wire the drill-down note");
        assert_eq!(f.summary, "added the observation field");
    }

    #[test]
    fn facts_json_carries_metric_and_session_signals_but_not_label() {
        let f = SessionMetricFacts::from_session_row(
            &row("corrected", Some(false), 2, 5),
            "ftr",
            "First-try rate",
            "the meaning",
        );
        let v = f.to_facts_json();
        assert_eq!(v["metric"], json!("ftr"));
        assert_eq!(v["meaning"], json!("the meaning"));
        assert_eq!(v["outcome"], json!("corrected"));
        assert_eq!(v["ftr"], json!(false));
        assert_eq!(v["corrections"], json!(2));
        assert_eq!(v["turns"], json!(5));
        assert_eq!(v["task"], json!("wire the drill-down note"));
        // The display label is NOT hashed — a label rename must not orphan cache.
        assert!(v.get("metric_label").is_none(), "label is display-only, never in the facts");
    }

    #[test]
    fn same_session_hashes_differently_per_metric() {
        // The metric key is part of the facts, so one session drilled from two
        // different metrics produces two distinct cache rows.
        let base = row("completed", Some(true), 0, 3);
        let a = SessionMetricFacts::from_session_row(&base, "ftr", "FTR", "m1").to_facts_json();
        let b =
            SessionMetricFacts::from_session_row(&base, "duplication", "Dup", "m1").to_facts_json();
        assert_ne!(
            insight_copy::facts_hash(InsightKind::SessionMetricObservation, &a),
            insight_copy::facts_hash(InsightKind::SessionMetricObservation, &b),
            "different metric key → different cache key for the same session",
        );
    }

    #[test]
    fn fallback_is_row_derived_and_matches_the_documented_shape() {
        // The exact shape the module doc promises for a clean first-try session.
        let f = SessionMetricFacts::from_session_row(
            &row("completed", Some(true), 0, 3),
            "ftr",
            "First-try rate",
            "meaning",
        );
        let fb = f.fallback();
        assert_eq!(fb.title, "First-try rate", "title is the metric label");
        assert_eq!(fb.detail, "outcome completed; first-try; 3 turns");
    }

    #[test]
    fn fallback_names_corrections_and_singularises_turns() {
        let f = SessionMetricFacts::from_session_row(
            &row("corrected", Some(false), 2, 1),
            "ftr",
            "FTR",
            "meaning",
        );
        assert_eq!(f.fallback().detail, "outcome corrected; 2 corrections; 1 turn");
        let one = SessionMetricFacts::from_session_row(
            &row("corrected", Some(false), 1, 4),
            "ftr",
            "FTR",
            "meaning",
        );
        assert_eq!(one.fallback().detail, "outcome corrected; 1 correction; 4 turns");
    }

    #[test]
    fn fallback_handles_missing_ftr_and_not_first_try() {
        // ftr unknown, no corrections recorded → plain "no corrections".
        let unknown = SessionMetricFacts::from_session_row(
            &row("completed", None, 0, 2),
            "ftr",
            "FTR",
            "meaning",
        );
        assert_eq!(unknown.fallback().detail, "outcome completed; no corrections; 2 turns");
        // ftr explicitly false with no correction count → "not first-try".
        let not_ftr = SessionMetricFacts::from_session_row(
            &row("completed", Some(false), 0, 2),
            "ftr",
            "FTR",
            "meaning",
        );
        assert_eq!(not_ftr.fallback().detail, "outcome completed; not first-try; 2 turns");
    }

    #[test]
    fn fallback_falls_back_to_key_label_when_metric_unregistered() {
        // An unregistered key → label defaults to the key, meaning empty; the
        // fallback still reads honestly (never blank), never fabricated. A
        // non-effort key omits the first-try phrasing (that's FTR/rework only).
        let f = SessionMetricFacts::from_session_row(
            &row("completed", Some(true), 0, 1),
            "_test_key",
            "_test_key",
            "",
        );
        let fb = f.fallback();
        assert_eq!(fb.title, "_test_key");
        assert_eq!(fb.detail, "outcome completed; 1 turn");
    }
}
