//! Pure bucketing rules for the Observatory · Insights (Learnings Triage) screen
//! (Slot 5). Each learning source — recommendation, memory, pattern, correction —
//! is assigned to one triage column (Now / Soon / Settled) by a pure rule, so the
//! `/api/insights` assembler stays testable without a database.
//!
//! The UI trusts the column label the daemon assigns; it does not re-bucket.

use crate::analysis::narration_cache::{FallbackCopy, InsightCopy, InsightKind};

/// The three triage columns.
pub const NOW: &str = "now";
pub const SOON: &str = "soon";
pub const SETTLED: &str = "settled";

/// A pending recommendation's column, from its `urgency`.
/// high → Now, medium → Soon, low (or anything unknown) → Settled.
pub fn rec_column(urgency: &str) -> &'static str {
    match urgency {
        "high" => NOW,
        "medium" => SOON,
        _ => SETTLED,
    }
}

/// A memory's column, or `None` when it isn't triaged on this screen.
///
/// A violated, non-archived memory is a Now decision regardless of status.
/// Otherwise: proposed → Soon; in-force (active / reinforced / battle_tested) →
/// Settled; everything else (archived / rejected / challenged) is excluded.
pub fn memory_column(status: &str, violated_count: i64) -> Option<&'static str> {
    if violated_count > 0 && status != "archived" {
        return Some(NOW);
    }
    match status {
        "proposed" => Some(SOON),
        "active" | "reinforced" | "battle_tested" => Some(SETTLED),
        _ => None,
    }
}

/// A pattern's column from its `lifecycle`, or `None` when not shown.
/// suggested → Soon, rule → Settled; gap (and anything else) is excluded.
pub fn pattern_column(lifecycle: &str) -> Option<&'static str> {
    match lifecycle {
        "suggested" => Some(SOON),
        "rule" => Some(SETTLED),
        _ => None,
    }
}

/// Corrections are always a Now decision — the top recurring mistakes to act on today.
pub const CORRECTION_COLUMN: &str = NOW;

/// Map one pending recommendation row to the inputs the mentor-voice
/// narration-cache pipeline needs: the [`InsightKind`], the stable `facts` object,
/// and the deterministic [`FallbackCopy`] (the raw DB prose).
///
/// The row is the JSON either `pg_store::get_insights_recommendations` (the
/// Learnings-triage board) or `pg_store::get_project_recommendations` (the
/// per-project endpoint) returns — both project the SAME `inference.recommendations`
/// columns. Building the facts here — once, in one place — is what keeps both
/// screens on ONE `(kind, facts_hash)` cache entry: a single warm serves both.
/// Mirrors `api::handlers::tool_signals::signal_copy_inputs`.
///
/// Pure — no DB, no gateway. The facts read only the prose columns
/// (`title` / `why` / `impact`) that discriminate one rec from another; the
/// row's code-owned display state (`urgency` / `status` / `column` / `score` /
/// …) is deliberately excluded so it can never fragment the cache key.
pub fn rec_copy_inputs(r: &serde_json::Value) -> (InsightKind, serde_json::Value, FallbackCopy) {
    let facts = serde_json::json!({
        "title": r["title"], "why": r["why"], "impact": r["impact"],
    });
    let fallback = FallbackCopy {
        title: r["title"].as_str().unwrap_or_default().to_string(),
        detail: r["why"].as_str().unwrap_or_default().to_string(),
    };
    (InsightKind::InsightRecurringPattern, facts, fallback)
}

/// Write the mentor-voice copy back onto a recommendation row in place. The
/// model owns the *sentence* (title + why); every other field — including
/// `impact` — is code-owned and left untouched, so the wire shape is unchanged
/// (same keys, only the `title` / `why` text differs). Shared by both call
/// sites so the overwrite can never drift between the two screens.
pub fn apply_rec_copy(r: &mut serde_json::Value, copy: InsightCopy) {
    r["title"] = copy.title.into();
    r["why"] = copy.detail.into();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::narration_cache::facts_hash;
    use serde_json::json;

    #[test]
    fn rec_urgency_buckets() {
        assert_eq!(rec_column("high"), "now");
        assert_eq!(rec_column("medium"), "soon");
        assert_eq!(rec_column("low"), "settled");
        assert_eq!(rec_column("anything-else"), "settled");
    }

    #[test]
    fn memory_violation_is_now_regardless_of_status() {
        assert_eq!(memory_column("active", 3), Some("now"));
        assert_eq!(memory_column("battle_tested", 1), Some("now"));
        // archived never surfaces, even when violated.
        assert_eq!(memory_column("archived", 5), None);
    }

    #[test]
    fn memory_status_buckets_when_not_violated() {
        assert_eq!(memory_column("proposed", 0), Some("soon"));
        assert_eq!(memory_column("active", 0), Some("settled"));
        assert_eq!(memory_column("reinforced", 0), Some("settled"));
        assert_eq!(memory_column("battle_tested", 0), Some("settled"));
        assert_eq!(memory_column("rejected", 0), None);
        assert_eq!(memory_column("challenged", 0), None);
    }

    #[test]
    fn pattern_lifecycle_buckets() {
        assert_eq!(pattern_column("suggested"), Some("soon"));
        assert_eq!(pattern_column("rule"), Some("settled"));
        assert_eq!(pattern_column("gap"), None);
    }

    // ── rec narration-cache inputs (pure) ──────────────────────────────────────

    #[test]
    fn rec_copy_inputs_maps_kind_facts_and_fallback() {
        let r = json!({
            "id": "r1", "urgency": "high", "column": "now", "score": 0.9,
            "title": "consolidate duplicate scan helpers",
            "why": "three folders re-implement the same walk",
            "impact": "cuts rescan time",
        });
        let (kind, facts, fb) = rec_copy_inputs(&r);
        assert_eq!(kind, InsightKind::InsightRecurringPattern);
        // Discriminating prose columns are carried into the facts.
        assert_eq!(facts["title"], "consolidate duplicate scan helpers");
        assert_eq!(facts["why"], "three folders re-implement the same walk");
        assert_eq!(facts["impact"], "cuts rescan time");
        // Code-owned display state must NOT poison the cache key.
        assert!(facts.get("urgency").is_none(), "urgency must not appear in facts");
        assert!(facts.get("column").is_none(), "column must not appear in facts");
        assert!(facts.get("score").is_none(), "score must not appear in facts");
        assert!(facts.get("id").is_none(), "id must not appear in facts");
        // Fallback carries the raw title/why verbatim (why → detail).
        assert_eq!(fb.title, "consolidate duplicate scan helpers");
        assert_eq!(fb.detail, "three folders re-implement the same walk");
    }

    #[test]
    fn rec_copy_inputs_is_deterministic_and_order_independent() {
        // Same rec content, keys inserted in a different order.
        let a = json!({ "title": "t", "why": "w", "impact": "i" });
        let b = json!({ "impact": "i", "why": "w", "title": "t" });
        let (ka, fa, _) = rec_copy_inputs(&a);
        let (kb, fb, _) = rec_copy_inputs(&b);
        assert_eq!(ka, kb);
        assert_eq!(fa, fb, "same rec content → identical facts regardless of key order");
        assert_eq!(facts_hash(ka, &fa), facts_hash(kb, &fb));
        // And two calls on the exact same input are stable.
        let (_, fa2, _) = rec_copy_inputs(&a);
        assert_eq!(fa, fa2);
    }

    #[test]
    fn rec_copy_inputs_shared_cache_across_both_endpoints() {
        // A row shaped like `get_insights_recommendations` output …
        let triage = json!({
            "id": "r1", "urgency": "high", "title": "t", "why": "w", "impact": "i",
            "evidence": { "n": 3 }, "project_id": "p1", "name": "sensei", "column": "now",
        });
        // … and a row shaped like `get_project_recommendations` output — SAME rec.
        let project = json!({
            "id": "r1", "title": "t", "urgency": "high", "status": "pending",
            "verdict": null, "why": "w", "impact": "i", "actionType": "consolidate",
            "baseline_ftr": 0.5, "current_ftr": null, "acted_at": null, "measured_at": null,
            "score": 0.9, "focal": true,
        });
        let (k1, f1, fb1) = rec_copy_inputs(&triage);
        let (k2, f2, fb2) = rec_copy_inputs(&project);
        assert_eq!(k1, k2, "same kind");
        assert_eq!(f1, f2, "both endpoints build identical facts for the same rec");
        assert_eq!(fb1, fb2, "both endpoints build the identical fallback");
        // The load-bearing invariant: one warm serves both screens.
        assert_eq!(
            facts_hash(k1, &f1),
            facts_hash(k2, &f2),
            "get_insights and get_project_recommendations must share the narration-cache cache key"
        );
    }

    #[test]
    fn rec_copy_inputs_null_impact_is_stable_and_distinct() {
        // A rec with no impact (Option<String> None → JSON null) is stable and
        // still differs from an empty-string impact (guards a silent collision).
        let none = json!({ "title": "t", "why": "w", "impact": null });
        let (kn, f_none, _) = rec_copy_inputs(&none);
        assert!(f_none["impact"].is_null());
        let empty = json!({ "title": "t", "why": "w", "impact": "" });
        let (_, f_empty, _) = rec_copy_inputs(&empty);
        assert_ne!(facts_hash(kn, &f_none), facts_hash(kn, &f_empty));
    }

    #[test]
    fn apply_rec_copy_overwrites_only_title_and_why_preserving_wire_shape() {
        // A full get_project_recommendations row.
        let mut r = json!({
            "id": "r1", "title": "raw title", "urgency": "high", "status": "pending",
            "verdict": null, "why": "raw why", "impact": "raw impact", "actionType": "consolidate",
            "baseline_ftr": 0.5, "current_ftr": null, "acted_at": null, "measured_at": null,
            "score": 0.9, "focal": true,
        });
        let before_keys: Vec<String> = r.as_object().unwrap().keys().cloned().collect();

        apply_rec_copy(
            &mut r,
            InsightCopy { title: "mentor title".into(), detail: "mentor why".into() },
        );

        // Same key set — no field added or removed (wire shape unchanged).
        let after_keys: Vec<String> = r.as_object().unwrap().keys().cloned().collect();
        assert_eq!(before_keys, after_keys, "routing must not add or drop wire fields");
        // Only title + why changed, and they stayed strings.
        assert_eq!(r["title"], "mentor title");
        assert_eq!(r["why"], "mentor why");
        assert!(r["title"].is_string() && r["why"].is_string());
        // Every other field is byte-identical to the DB row — impact is NOT routed.
        assert_eq!(r["impact"], "raw impact");
        assert_eq!(r["urgency"], "high");
        assert_eq!(r["status"], "pending");
        assert_eq!(r["actionType"], "consolidate");
        assert_eq!(r["score"], 0.9);
        assert_eq!(r["focal"], true);
        assert_eq!(r["baseline_ftr"], 0.5);
    }
}
