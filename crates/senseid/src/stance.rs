//! Stance resolution — the three behavioural dials (autonomy · sharing · review)
//! a user runs under at a given scope. Complements governance rules (WHAT a run
//! may do) with HOW a run behaves and how its output flows. User-scoped +
//! daemon-local (D-STANCE-SCOPE): a stance follows the user with or without a
//! dōjō and drives the local session.
//!
//! Resolution mirrors rules: the most-specific namespace on the `sensei.scopes`
//! ladder wins, falling back to the user's namespace-less default row, then to
//! the enum defaults when a user has set no stance at all. The precedence lives
//! in the pure [`pick_stance`] so it is unit-testable without a database.
//!
//! The autonomy *decision* — does a dial let a run proceed through a step of a
//! given risk WITHOUT pausing for a human — lives in [`autonomy_permits`]. Its
//! semantics were confirmed with the owner (threshold reading): the label names
//! the LOWEST risk tier that still triggers a pause, so `ask_on_guarded` (the
//! default) proceeds through ordinary + risky and pauses only on guarded. dbd
//! sorts enum variants alphabetically, so this code — not the DDL declaration
//! order — owns the autonomy rank.

/// One stance row in the running for a resolution: its scope level (`None` for
/// the user's namespace-less default) and the three dial values as their enum
/// text. Ordered purely by `level` in [`pick_stance`].
#[derive(Debug, Clone, PartialEq)]
pub struct StanceCandidate {
    /// The scope level of this stance's namespace on the ladder; `None` = the
    /// user's default (namespace-less) row.
    pub level: Option<i32>,
    pub autonomy: String,
    pub sharing: String,
    pub review: String,
}

/// The resolved stance a run operates under.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ResolvedStance {
    pub autonomy: String,
    pub sharing: String,
    pub review: String,
    /// How the value was resolved: `"scoped"` (a namespace stance won), `"default"`
    /// (the user's namespace-less row), or `"fallback"` (no stance set — the enum
    /// defaults). Lets a caller tell "the user chose this" from "nothing was set".
    pub source: String,
}

impl ResolvedStance {
    /// The system defaults when a user has set no stance at all — mirrors the DDL
    /// column defaults (`ask_on_guarded` / `patterns` / `one_maintainer`). Kept in
    /// sync with `database/ddl/table/sensei/stances.ddl`.
    pub fn fallback() -> Self {
        Self {
            autonomy: "ask_on_guarded".into(),
            sharing: "patterns".into(),
            review: "one_maintainer".into(),
            source: "fallback".into(),
        }
    }
}

/// Pick the effective stance from the candidate rows: the most-specific scoped
/// stance (highest scope `level`) wins; with no scoped row, the user's default
/// (level `None`); with no candidates at all, the enum fallback. Pure — the
/// single source of truth for stance precedence.
pub fn pick_stance(candidates: &[StanceCandidate]) -> ResolvedStance {
    // Scoped rows carry a level; the default row is level-less. Highest level
    // (most specific rung) wins among scoped rows.
    if let Some(c) = candidates
        .iter()
        .filter(|c| c.level.is_some())
        .max_by_key(|c| c.level.unwrap())
    {
        return ResolvedStance {
            autonomy: c.autonomy.clone(),
            sharing: c.sharing.clone(),
            review: c.review.clone(),
            source: "scoped".into(),
        };
    }
    if let Some(c) = candidates.iter().find(|c| c.level.is_none()) {
        return ResolvedStance {
            autonomy: c.autonomy.clone(),
            sharing: c.sharing.clone(),
            review: c.review.clone(),
            source: "default".into(),
        };
    }
    ResolvedStance::fallback()
}

/// The risk tier of a step an autonomous run can reach, ascending by how much it
/// warrants a human pause. `Guarded` = money / credentials / destructive or
/// otherwise irreversible actions (the DDL's "guarded step").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepRisk {
    Ordinary,
    Risky,
    Guarded,
}

/// Whether a run at `autonomy` may proceed through a step of `risk` WITHOUT
/// pausing to ask a human. Threshold semantics (owner-confirmed): the dial names
/// the lowest risk tier that still triggers a pause.
///
/// | autonomy         | ordinary | risky | guarded |
/// |------------------|----------|-------|---------|
/// | `ask_always`     | ask      | ask   | ask     |
/// | `ask_on_risky`   | run      | ask   | ask     |
/// | `ask_on_guarded` | run      | run   | ask     | (default)
/// | `run_freely`     | run      | run   | run     |
///
/// Unknown / unrecognised autonomy → safest (always ask). Pure — the single
/// source of truth for the "progress over asking" gate.
pub fn autonomy_permits(autonomy: &str, risk: StepRisk) -> bool {
    match autonomy {
        "run_freely" => true,
        "ask_on_guarded" => risk != StepRisk::Guarded,
        "ask_on_risky" => risk == StepRisk::Ordinary,
        // ask_always + any unknown value → never proceed unattended.
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(level: Option<i32>, autonomy: &str) -> StanceCandidate {
        StanceCandidate {
            level,
            autonomy: autonomy.into(),
            sharing: "patterns".into(),
            review: "one_maintainer".into(),
        }
    }

    #[test]
    fn no_candidates_yields_fallback() {
        let r = pick_stance(&[]);
        assert_eq!(r, ResolvedStance::fallback());
        assert_eq!(r.source, "fallback");
        assert_eq!(r.autonomy, "ask_on_guarded");
    }

    #[test]
    fn default_row_wins_when_no_scoped_stance() {
        let r = pick_stance(&[cand(None, "run_freely")]);
        assert_eq!(r.source, "default");
        assert_eq!(r.autonomy, "run_freely");
    }

    #[test]
    fn most_specific_scoped_stance_beats_default_and_lower_rungs() {
        // org rung (20), project rung (60), and a default row — project wins.
        let r = pick_stance(&[
            cand(None, "ask_always"),
            cand(Some(20), "ask_on_guarded"),
            cand(Some(60), "run_freely"),
        ]);
        assert_eq!(r.source, "scoped");
        assert_eq!(r.autonomy, "run_freely");
    }

    #[test]
    fn autonomy_permits_matches_the_confirmed_threshold_table() {
        use StepRisk::*;
        // ask_always → never proceeds unattended.
        for r in [Ordinary, Risky, Guarded] {
            assert!(!autonomy_permits("ask_always", r));
        }
        // ask_on_risky → ordinary only.
        assert!(autonomy_permits("ask_on_risky", Ordinary));
        assert!(!autonomy_permits("ask_on_risky", Risky));
        assert!(!autonomy_permits("ask_on_risky", Guarded));
        // ask_on_guarded (default) → ordinary + risky, pause on guarded.
        assert!(autonomy_permits("ask_on_guarded", Ordinary));
        assert!(autonomy_permits("ask_on_guarded", Risky));
        assert!(!autonomy_permits("ask_on_guarded", Guarded));
        // run_freely → everything.
        for r in [Ordinary, Risky, Guarded] {
            assert!(autonomy_permits("run_freely", r));
        }
        // unknown / empty → safest (always ask).
        assert!(!autonomy_permits("", Ordinary));
        assert!(!autonomy_permits("bogus", Ordinary));
    }
}
