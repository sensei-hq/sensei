//! Pure playbook recommender: classified axes + a rule set -> a recommendation.
//! No IO — the rule set is passed in (DB-source-agnostic).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lifecycle {
    Greenfield,
    Stable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Explore,
    Ux,
    Feature,
    Enhancement,
    Bug,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Risk {
    Low,
    High,
}

impl Lifecycle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Greenfield => "greenfield",
            Self::Stable => "stable",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "greenfield" => Some(Self::Greenfield),
            "stable" => Some(Self::Stable),
            _ => None,
        }
    }
}
impl Intent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Explore => "explore",
            Self::Ux => "ux",
            Self::Feature => "feature",
            Self::Enhancement => "enhancement",
            Self::Bug => "bug",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "explore" => Some(Self::Explore),
            "ux" => Some(Self::Ux),
            "feature" => Some(Self::Feature),
            "enhancement" => Some(Self::Enhancement),
            "bug" => Some(Self::Bug),
            _ => None,
        }
    }
}
impl Risk {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::High => "high",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "low" => Some(Self::Low),
            "high" => Some(Self::High),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Axes {
    pub lifecycle: Lifecycle,
    pub intent: Intent,
    pub risk: Risk,
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub id: Option<uuid::Uuid>,
    pub name: String,
    pub match_lifecycle: Option<Lifecycle>,
    pub match_intent: Option<Intent>,
    pub match_risk: Option<Risk>,
    pub playbook: String,
    pub rationale: String,
    pub priority: i32,
    pub base_priority: i32,
}

#[derive(Clone, Debug)]
pub struct Recommendation {
    pub playbook: String,
    pub rationale: String,
    pub rule_id: Option<uuid::Uuid>,
    pub rule_name: Option<String>,
    pub defaulted: bool,
}

fn matches(rule: &Rule, a: &Axes) -> bool {
    rule.match_lifecycle.is_none_or(|m| m == a.lifecycle)
        && rule.match_intent.is_none_or(|m| m == a.intent)
        && rule.match_risk.is_none_or(|m| m == a.risk)
}

/// Highest-priority matching rule wins. No match -> `gsd`, flagged (never silent).
pub fn recommend(axes: &Axes, rules: &[Rule]) -> Recommendation {
    let best = rules.iter().filter(|r| matches(r, axes)).max_by_key(|r| r.priority);
    match best {
        Some(r) => Recommendation {
            playbook: r.playbook.clone(),
            rationale: r.rationale.clone(),
            rule_id: r.id,
            rule_name: Some(r.name.clone()),
            defaulted: false,
        },
        None => Recommendation {
            playbook: "gsd".into(),
            rationale: "no rule matched — defaulted to gsd".into(),
            rule_id: None,
            rule_name: None,
            defaulted: true,
        },
    }
}

// ── §9 learning policy (pure) ──────────────────────────────────────────────
const MIN_SAMPLE: i64 = 5;
const FTR_DELTA: f64 = 0.2;
const REWEIGHT_K: f64 = 40.0;
const REWEIGHT_BOUND: i32 = 20;
const REWEIGHT_TARGET_FTR: f64 = 0.5; // neutral FTR midpoint the reweight measures against

#[derive(Clone, Debug)]
pub struct ComboPlaybookStat {
    pub lifecycle: Lifecycle,
    pub intent: Intent,
    pub risk: Risk,
    pub playbook: String,
    pub n: i64,
    pub ftr_rate: f64,
}

#[derive(Clone, Debug)]
pub struct LearnedRule {
    pub lifecycle: Lifecycle,
    pub intent: Intent,
    pub risk: Risk,
    pub playbook: String,
    pub priority: i32,
    pub rationale: String,
}

#[derive(Clone, Debug, Default)]
pub struct LearnPlan {
    pub reweights: Vec<(uuid::Uuid, i32)>, // (rule_id, new_priority)
    pub proposals: Vec<LearnedRule>,
}

fn stat_matches_rule(s: &ComboPlaybookStat, r: &Rule) -> bool {
    r.match_lifecycle.is_none_or(|m| m == s.lifecycle)
        && r.match_intent.is_none_or(|m| m == s.intent)
        && r.match_risk.is_none_or(|m| m == s.risk)
}

/// Pure: current per-(axes×playbook) FTR stats + the live rule set → a plan of
/// bounded priority reweights (existing rules) + proposed new learned rules.
pub fn learn(stats: &[ComboPlaybookStat], rules: &[Rule]) -> LearnPlan {
    let mut plan = LearnPlan::default();

    // Reweight: each rule scored on its playbook's FTR across the combos it matches,
    // measured against a fixed neutral target (REWEIGHT_TARGET_FTR) — robust and
    // degeneracy-free (no dependence on the mix of other data).
    for r in rules {
        let matching: Vec<&ComboPlaybookStat> =
            stats.iter().filter(|s| s.playbook == r.playbook && stat_matches_rule(s, r)).collect();
        let n: i64 = matching.iter().map(|s| s.n).sum();
        if n < MIN_SAMPLE {
            continue;
        }
        let ftr = matching.iter().map(|s| s.ftr_rate * s.n as f64).sum::<f64>() / n as f64;
        let adj = ((REWEIGHT_K * (ftr - REWEIGHT_TARGET_FTR)).round() as i32)
            .clamp(-REWEIGHT_BOUND, REWEIGHT_BOUND);
        let new_priority = r.base_priority + adj;
        if let Some(id) = r.id
            && new_priority != r.priority
        {
            plan.reweights.push((id, new_priority));
        }
    }

    // Propose: for each exact combo, if the best-performing playbook beats the
    // currently-recommended one by >= FTR_DELTA (with enough samples), propose it.
    let mut combos: Vec<(Lifecycle, Intent, Risk)> =
        stats.iter().map(|s| (s.lifecycle, s.intent, s.risk)).collect();
    combos.sort_by_key(|(l, i, r)| (l.as_str(), i.as_str(), r.as_str()));
    combos.dedup();
    for (l, i, rk) in combos {
        let axes = Axes { lifecycle: l, intent: i, risk: rk };
        let here: Vec<&ComboPlaybookStat> = stats
            .iter()
            .filter(|s| s.lifecycle == l && s.intent == i && s.risk == rk && s.n >= MIN_SAMPLE)
            .collect();
        let Some(best) = here.iter().max_by(|a, b| a.ftr_rate.total_cmp(&b.ftr_rate)) else {
            continue;
        };
        let rec = recommend(&axes, rules);
        let rec_ftr = here.iter().find(|s| s.playbook == rec.playbook).map_or(0.0, |s| s.ftr_rate);
        if best.playbook != rec.playbook && best.ftr_rate - rec_ftr >= FTR_DELTA {
            let top = rules
                .iter()
                .filter(|r| stat_matches_rule(here[0], r))
                .map(|r| r.priority)
                .max()
                .unwrap_or(0);
            plan.proposals.push(LearnedRule {
                lifecycle: l,
                intent: i,
                risk: rk,
                playbook: best.playbook.clone(),
                priority: top + 1,
                rationale: format!(
                    "learned: {} out-performed {} here (FTR {:.2} vs {:.2}, n={})",
                    best.playbook, rec.playbook, best.ftr_rate, rec_ftr, best.n
                ),
            });
        }
    }
    plan
}

const TRUST_MIN_SAMPLE: i64 = 10;
const TRUST_FTR: f64 = 0.8;

/// Auto-select gate: a low-risk chunk whose chosen playbook has enough proven FTR history.
/// Stricter than §9's learn thresholds — skipping a human confirm demands more evidence.
pub fn is_trusted(risk: Risk, n: i64, ftr: f64) -> bool {
    risk == Risk::Low && n >= TRUST_MIN_SAMPLE && ftr >= TRUST_FTR
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed() -> Vec<Rule> {
        vec![
            Rule {
                id: None,
                name: "high-blast".into(),
                match_lifecycle: None,
                match_intent: None,
                match_risk: Some(Risk::High),
                playbook: "spec_driven".into(),
                rationale: "hi".into(),
                priority: 100,
                base_priority: 100,
            },
            Rule {
                id: None,
                name: "gf-fuzzy".into(),
                match_lifecycle: Some(Lifecycle::Greenfield),
                match_intent: Some(Intent::Explore),
                match_risk: None,
                playbook: "vibe".into(),
                rationale: "gf".into(),
                priority: 60,
                base_priority: 60,
            },
            Rule {
                id: None,
                name: "known-low".into(),
                match_lifecycle: None,
                match_intent: Some(Intent::Feature),
                match_risk: Some(Risk::Low),
                playbook: "gsd".into(),
                rationale: "gsd".into(),
                priority: 40,
                base_priority: 40,
            },
        ]
    }

    #[test]
    fn high_risk_wins_by_priority() {
        let axes =
            Axes { lifecycle: Lifecycle::Greenfield, intent: Intent::Feature, risk: Risk::High };
        let r = recommend(&axes, &seed());
        assert_eq!(r.playbook, "spec_driven");
        assert_eq!(r.rule_name.as_deref(), Some("high-blast"));
    }

    #[test]
    fn wildcard_and_specific_match() {
        let axes = Axes { lifecycle: Lifecycle::Stable, intent: Intent::Feature, risk: Risk::Low };
        assert_eq!(recommend(&axes, &seed()).playbook, "gsd");
    }

    #[test]
    fn no_match_defaults_to_gsd_flagged() {
        let axes = Axes { lifecycle: Lifecycle::Stable, intent: Intent::Ux, risk: Risk::Low };
        let r = recommend(&axes, &seed());
        assert_eq!(r.playbook, "gsd");
        assert!(r.rule_name.is_none());
        assert!(r.defaulted);
    }

    fn stat(l: Lifecycle, i: Intent, r: Risk, pb: &str, n: i64, ftr: f64) -> ComboPlaybookStat {
        ComboPlaybookStat {
            lifecycle: l,
            intent: i,
            risk: r,
            playbook: pb.into(),
            n,
            ftr_rate: ftr,
        }
    }
    fn rule(
        id: u128,
        l: Option<Lifecycle>,
        i: Option<Intent>,
        r: Option<Risk>,
        pb: &str,
        prio: i32,
    ) -> Rule {
        Rule {
            id: Some(uuid::Uuid::from_u128(id)),
            name: pb.into(),
            match_lifecycle: l,
            match_intent: i,
            match_risk: r,
            playbook: pb.into(),
            rationale: "r".into(),
            priority: prio,
            base_priority: prio,
        }
    }

    #[test]
    fn reweight_bumps_priority_up_for_strong_ftr() {
        let rules =
            vec![rule(1, Some(Lifecycle::Stable), Some(Intent::Bug), None, "debug_flow", 60)];
        let stats = vec![stat(Lifecycle::Stable, Intent::Bug, Risk::Low, "debug_flow", 10, 1.0)];
        let plan = learn(&stats, &rules);
        let (_, np) = plan.reweights.iter().find(|(id, _)| *id == rules[0].id.unwrap()).unwrap();
        assert!(*np > 60, "high FTR should raise priority (got {np})");
        assert!(*np <= 60 + 20, "bounded by REWEIGHT_BOUND");
    }

    #[test]
    fn reweight_ignored_below_min_sample() {
        let rules =
            vec![rule(1, Some(Lifecycle::Stable), Some(Intent::Bug), None, "debug_flow", 60)];
        let stats = vec![stat(Lifecycle::Stable, Intent::Bug, Risk::Low, "debug_flow", 3, 1.0)];
        assert!(learn(&stats, &rules).reweights.is_empty(), "n<5 → no reweight");
    }

    #[test]
    fn reweight_is_idempotent() {
        let mut rules =
            vec![rule(1, Some(Lifecycle::Stable), Some(Intent::Bug), None, "debug_flow", 60)];
        let stats = vec![stat(Lifecycle::Stable, Intent::Bug, Risk::Low, "debug_flow", 10, 1.0)];
        let np = learn(&stats, &rules).reweights[0].1;
        rules[0].priority = np; // apply once (base_priority stays 60)
        assert_eq!(
            learn(&stats, &rules).reweights.iter().find(|(_, p)| *p != np),
            None,
            "same stats + same base → same target priority"
        );
    }

    #[test]
    fn proposes_better_playbook_over_recommended() {
        // recommended for (stable,feature,low) = gsd (seed). But mockup_first scores far higher here.
        let rules = vec![rule(1, None, Some(Intent::Feature), Some(Risk::Low), "gsd", 40)];
        let stats = vec![
            stat(Lifecycle::Stable, Intent::Feature, Risk::Low, "gsd", 8, 0.4),
            stat(Lifecycle::Stable, Intent::Feature, Risk::Low, "mockup_first", 8, 0.9),
        ];
        let plan = learn(&stats, &rules);
        let p = plan
            .proposals
            .iter()
            .find(|p| p.playbook == "mockup_first")
            .expect("propose the winner");
        assert_eq!(
            (p.lifecycle, p.intent, p.risk),
            (Lifecycle::Stable, Intent::Feature, Risk::Low)
        );
        assert!(p.priority > 40, "must out-prioritize the recommended rule");
    }

    #[test]
    fn no_proposal_when_recommended_is_best_or_delta_small() {
        let rules = vec![rule(1, None, Some(Intent::Feature), Some(Risk::Low), "gsd", 40)];
        let stats = vec![
            stat(Lifecycle::Stable, Intent::Feature, Risk::Low, "gsd", 8, 0.8),
            stat(Lifecycle::Stable, Intent::Feature, Risk::Low, "vibe", 8, 0.85), // delta 0.05 < 0.2
        ];
        assert!(learn(&stats, &rules).proposals.is_empty());
    }

    #[test]
    fn trusted_only_for_proven_low_risk() {
        assert!(is_trusted(Risk::Low, 10, 0.8)); // boundary: n==MIN, ftr==TARGET
        assert!(is_trusted(Risk::Low, 40, 0.95));
        assert!(!is_trusted(Risk::High, 40, 0.95)); // high-risk never auto-selects
        assert!(!is_trusted(Risk::Low, 9, 0.95)); // too few samples
        assert!(!is_trusted(Risk::Low, 40, 0.79)); // FTR below target
    }
}
