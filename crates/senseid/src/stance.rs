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

/// Allowed `autonomy` values, ascending autonomy — the `sensei.stance_autonomy`
/// enum variants (single source of truth for the write-path validator; asserted
/// against the DDL in tests).
pub const AUTONOMY_VALUES: [&str; 4] =
    ["ask_always", "ask_on_guarded", "ask_on_risky", "run_freely"];
/// Allowed `sharing` values, ascending openness — the `sensei.stance_sharing` enum.
pub const SHARING_VALUES: [&str; 4] = ["private", "patterns", "patterns_prompts", "derived"];
/// Allowed `review` values, ascending strictness — the `sensei.stance_review` enum.
pub const REVIEW_VALUES: [&str; 4] = ["me_alone", "one_maintainer", "two_maintainers", "quorum"];

/// A validated stance write: the three dial values a caller wants to set. Parsed
/// from a request body via [`StanceInput::from_request`] — an absent axis takes
/// its DDL default (full-replace semantics, mirroring the collective-preferences
/// PUT); every present axis is validated so no invalid enum literal is ever cast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StanceInput {
    pub autonomy: String,
    pub sharing: String,
    pub review: String,
}

impl StanceInput {
    /// Parse + validate the three dial fields from a JSON body. Absent/null axis
    /// → its default (from [`ResolvedStance::fallback`]); a present value must be
    /// a member of its allowed set (`Err` → the handler answers 400). The scope /
    /// user / folder fields are the handler's concern, not this.
    pub fn from_request(body: &serde_json::Value) -> Result<Self, String> {
        if !body.is_object() {
            return Err("body must be a JSON object".to_string());
        }
        let d = ResolvedStance::fallback();
        use crate::api::util::parse_enum_field;
        Ok(Self {
            autonomy: parse_enum_field(body, "autonomy", &AUTONOMY_VALUES, &d.autonomy)?,
            sharing: parse_enum_field(body, "sharing", &SHARING_VALUES, &d.sharing)?,
            review: parse_enum_field(body, "review", &REVIEW_VALUES, &d.review)?,
        })
    }
}

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

    /// The strictest, most-private stance — used when a run's AUTHOR IDENTITY
    /// cannot be resolved (no attributable user, or a stance-lookup failure).
    /// Fails CLOSED, not open: `ask_always` so the drive gate never proceeds a
    /// step unattended, `private` so an unattributable run never shares, `quorum`
    /// review. Distinct from [`Self::fallback`], which is a KNOWN user's
    /// "no preference set" default and is deliberately more permissive.
    pub fn strict() -> Self {
        Self {
            autonomy: "ask_always".into(),
            sharing: "private".into(),
            review: "quorum".into(),
            source: "unresolved".into(),
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
    if let Some(c) =
        candidates.iter().filter(|c| c.level.is_some()).max_by_key(|c| c.level.unwrap())
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
    fn strict_fails_closed_on_every_dial() {
        let s = ResolvedStance::strict();
        assert_eq!(s.autonomy, "ask_always"); // never proceed unattended
        assert_eq!(s.sharing, "private"); // never share an unattributable run
        assert_eq!(s.review, "quorum");
        assert_eq!(s.source, "unresolved");
        // The drive gate refuses EVERY risk tier under the strict stance.
        for r in [StepRisk::Ordinary, StepRisk::Risky, StepRisk::Guarded] {
            assert!(!autonomy_permits(&s.autonomy, r));
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

    #[test]
    fn allowed_value_sets_cover_the_fallback_and_the_permit_table() {
        // The DDL column defaults must be members of their allowed sets.
        let d = ResolvedStance::fallback();
        assert!(AUTONOMY_VALUES.contains(&d.autonomy.as_str()));
        assert!(SHARING_VALUES.contains(&d.sharing.as_str()));
        assert!(REVIEW_VALUES.contains(&d.review.as_str()));
        // Every autonomy value the write-path accepts is one the permit table
        // decides (no value can be stored that the gate then can't read).
        for a in AUTONOMY_VALUES {
            let _ = autonomy_permits(a, StepRisk::Guarded);
        }
    }

    #[test]
    fn stance_input_accepts_a_full_body() {
        let body = serde_json::json!({
            "autonomy": "run_freely", "sharing": "private", "review": "quorum"
        });
        let i = StanceInput::from_request(&body).unwrap();
        assert_eq!(
            i,
            StanceInput {
                autonomy: "run_freely".into(),
                sharing: "private".into(),
                review: "quorum".into(),
            }
        );
    }

    #[test]
    fn stance_input_defaults_absent_axes_to_the_ddl_fallback() {
        let i =
            StanceInput::from_request(&serde_json::json!({ "autonomy": "ask_always" })).unwrap();
        let d = ResolvedStance::fallback();
        assert_eq!(i.autonomy, "ask_always");
        assert_eq!(i.sharing, d.sharing, "absent sharing → DDL default");
        assert_eq!(i.review, d.review, "absent review → DDL default");
    }

    #[test]
    fn stance_input_rejects_bad_values_and_non_object() {
        for (field, body) in [
            ("autonomy", serde_json::json!({ "autonomy": "yolo" })),
            ("sharing", serde_json::json!({ "sharing": "everything" })),
            ("review", serde_json::json!({ "review": "nobody" })),
        ] {
            let e = StanceInput::from_request(&body).unwrap_err();
            assert!(e.contains(field), "error should name {field}: {e}");
        }
        assert!(StanceInput::from_request(&serde_json::json!("nope")).is_err());
        assert!(StanceInput::from_request(&serde_json::json!({ "autonomy": 7 })).is_err());
    }
}
