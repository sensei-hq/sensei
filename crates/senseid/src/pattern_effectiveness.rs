//! Pattern effectiveness — the `ftrDelta` + `kind` the Observatory's Patterns
//! view shows.
//!
//! Our detected patterns are *behavioral* (re-edit churn, correction-prone
//! folders, stated rule-candidates), not "followed best-practice" patterns, so
//! "FTR when the pattern is applied" doesn't translate directly. Instead we
//! quantify the **locus**: a pattern lives in a folder, and we compare that
//! folder's average First-Try-Right to the project baseline.
//!
//! `ftrDelta = folder_avg_ftr − project_avg_ftr`:
//! - for an **anti-pattern** (churn / corrections) a *negative* delta is the
//!   measured cost — sessions in that area succeed first-try less often;
//! - for a positive/rule pattern a positive delta is the payoff.
//!
//! Both are derived on read from `activity.sessions.ftr`; nothing is stored.

/// `folder_avg_ftr − project_avg_ftr`, rounded to 3 dp. `None` when either side
/// has no FTR-scored sessions (can't compare).
pub fn ftr_delta(folder_ftr: Option<f64>, project_ftr: Option<f64>) -> Option<f64> {
    match (folder_ftr, project_ftr) {
        (Some(f), Some(p)) => Some(((f - p) * 1000.0).round() / 1000.0),
        _ => None,
    }
}

/// Map a detected pattern to the UI's `kind`:
/// - `anti` — an anti-pattern (the locus to fix);
/// - `adopted` — a non-anti pattern already promoted to a rule (`lifecycle='rule'`);
/// - `emerging` — a non-anti candidate not yet promoted.
pub fn pattern_kind(is_anti: bool, lifecycle: &str) -> &'static str {
    if is_anti {
        "anti"
    } else if lifecycle == "rule" {
        "adopted"
    } else {
        "emerging"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ftr_delta_negative_for_underperforming_folder() {
        // folder 0.4 vs project 0.7 → -0.3 (anti-pattern cost)
        assert_eq!(ftr_delta(Some(0.4), Some(0.7)), Some(-0.3));
    }

    #[test]
    fn ftr_delta_positive_when_folder_beats_baseline() {
        assert_eq!(ftr_delta(Some(0.9), Some(0.75)), Some(0.15));
    }

    #[test]
    fn ftr_delta_rounds_to_three_dp() {
        // 0.6666… - 0.3333… computed from fractions
        let d = ftr_delta(Some(2.0 / 3.0), Some(1.0 / 3.0)).unwrap();
        assert_eq!(d, 0.333);
    }

    #[test]
    fn ftr_delta_none_without_both_sides() {
        assert_eq!(ftr_delta(None, Some(0.5)), None);
        assert_eq!(ftr_delta(Some(0.5), None), None);
        assert_eq!(ftr_delta(None, None), None);
    }

    #[test]
    fn kind_maps_anti_adopted_emerging() {
        assert_eq!(pattern_kind(true, "rule"), "anti", "anti wins regardless of lifecycle");
        assert_eq!(pattern_kind(true, "suggested"), "anti");
        assert_eq!(pattern_kind(false, "rule"), "adopted");
        assert_eq!(pattern_kind(false, "suggested"), "emerging");
        assert_eq!(pattern_kind(false, "gap"), "emerging");
    }
}
