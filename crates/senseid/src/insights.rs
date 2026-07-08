//! Pure bucketing rules for the Observatory · Insights (Learnings Triage) screen
//! (Slot 5). Each learning source — recommendation, memory, pattern, correction —
//! is assigned to one triage column (Now / Soon / Settled) by a pure rule, so the
//! `/api/insights` assembler stays testable without a database.
//!
//! The UI trusts the column label the daemon assigns; it does not re-bucket.

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
