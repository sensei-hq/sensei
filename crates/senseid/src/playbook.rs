//! Pure playbook recommender: classified axes + a rule set -> a recommendation.
//! No IO — the rule set is passed in (DB-source-agnostic).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lifecycle { Greenfield, Stable }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent { Explore, Ux, Feature, Enhancement, Bug }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Risk { Low, High }

impl Lifecycle {
    pub fn as_str(self) -> &'static str { match self { Self::Greenfield => "greenfield", Self::Stable => "stable" } }
    pub fn parse(s: &str) -> Option<Self> { match s { "greenfield" => Some(Self::Greenfield), "stable" => Some(Self::Stable), _ => None } }
}
impl Intent {
    pub fn as_str(self) -> &'static str { match self { Self::Explore=>"explore", Self::Ux=>"ux", Self::Feature=>"feature", Self::Enhancement=>"enhancement", Self::Bug=>"bug" } }
    pub fn parse(s: &str) -> Option<Self> { match s { "explore"=>Some(Self::Explore),"ux"=>Some(Self::Ux),"feature"=>Some(Self::Feature),"enhancement"=>Some(Self::Enhancement),"bug"=>Some(Self::Bug),_=>None } }
}
impl Risk {
    pub fn as_str(self) -> &'static str { match self { Self::Low=>"low", Self::High=>"high" } }
    pub fn parse(s: &str) -> Option<Self> { match s { "low"=>Some(Self::Low),"high"=>Some(Self::High),_=>None } }
}

#[derive(Clone, Copy, Debug)]
pub struct Axes { pub lifecycle: Lifecycle, pub intent: Intent, pub risk: Risk }

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
            playbook: r.playbook.clone(), rationale: r.rationale.clone(),
            rule_id: r.id, rule_name: Some(r.name.clone()), defaulted: false,
        },
        None => Recommendation {
            playbook: "gsd".into(),
            rationale: "no rule matched — defaulted to gsd".into(),
            rule_id: None, rule_name: None, defaulted: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed() -> Vec<Rule> {
        vec![
            Rule { id: None, name: "high-blast".into(), match_lifecycle: None, match_intent: None, match_risk: Some(Risk::High), playbook: "spec_driven".into(), rationale: "hi".into(), priority: 100 },
            Rule { id: None, name: "gf-fuzzy".into(), match_lifecycle: Some(Lifecycle::Greenfield), match_intent: Some(Intent::Explore), match_risk: None, playbook: "vibe".into(), rationale: "gf".into(), priority: 60 },
            Rule { id: None, name: "known-low".into(), match_lifecycle: None, match_intent: Some(Intent::Feature), match_risk: Some(Risk::Low), playbook: "gsd".into(), rationale: "gsd".into(), priority: 40 },
        ]
    }

    #[test]
    fn high_risk_wins_by_priority() {
        let axes = Axes { lifecycle: Lifecycle::Greenfield, intent: Intent::Feature, risk: Risk::High };
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
}
