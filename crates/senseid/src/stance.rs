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
//! NOTE: the autonomy *decision* (does this dial let a run proceed through a
//! given step without asking?) is intentionally NOT here yet — the autonomy
//! ladder's ordering needs an explicit confirmation before a safety gate keys off
//! it. Resolution + read are safe and stand alone.

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
}
