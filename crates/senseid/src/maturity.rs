//! Project maturity signal (L3, #71).
//!
//! The Observatory shows a project as **early** or **mature** and a
//! "watched N / target M" progress hint on first entry. This is a *derived*
//! signal, not the stored `projects.maturity` lifecycle enum
//! (discovery/active/maintenance/archived) — that stays under the user's
//! control; this is computed on read from observed activity.
//!
//! A project is **mature** once it has watched at least `target` enriched
//! sessions AND produced at least one insight (a recommendation or a learned
//! memory). Until then it's **early** — there isn't enough signal yet to teach
//! anything, and the UI says so honestly ("~N more sessions until the first
//! lesson").

/// Sessions to watch before a project can be considered mature.
pub const MATURITY_TARGET: i64 = 3;

/// The derived early/mature signal for a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaturitySignal {
    /// `"early"` or `"mature"`.
    pub stage: &'static str,
    /// Enriched sessions observed so far.
    pub watched: i64,
    /// Target session count for maturity.
    pub target: i64,
    /// Whether the analyzer has produced any insight yet.
    pub has_insights: bool,
}

/// Derive the early/mature signal. Pure.
///
/// `mature` ⇔ `watched >= target` AND `has_insights`. Both gates matter: a
/// project with many sessions but no derived insight is still early (nothing
/// learned yet), and a project with an insight but only one session hasn't
/// shown a stable enough pattern.
pub fn maturity_signal(watched: i64, has_insights: bool, target: i64) -> MaturitySignal {
    let mature = watched >= target && has_insights;
    MaturitySignal { stage: if mature { "mature" } else { "early" }, watched, target, has_insights }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn early_below_target_even_with_insights() {
        let s = maturity_signal(2, true, 3);
        assert_eq!(s.stage, "early");
        assert_eq!(s.watched, 2);
        assert_eq!(s.target, 3);
    }

    #[test]
    fn early_at_target_without_insights() {
        // Enough sessions, but nothing learned yet → still early.
        assert_eq!(maturity_signal(5, false, 3).stage, "early");
    }

    #[test]
    fn mature_at_target_with_insights() {
        assert_eq!(maturity_signal(3, true, 3).stage, "mature");
        assert_eq!(maturity_signal(10, true, 3).stage, "mature");
    }

    #[test]
    fn fresh_project_is_early() {
        let s = maturity_signal(0, false, MATURITY_TARGET);
        assert_eq!(s.stage, "early");
        assert!(!s.has_insights);
    }
}
