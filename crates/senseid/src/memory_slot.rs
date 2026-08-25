//! Spine-slot anchoring for memories (design 2026-07-18-memory-anchoring).
//! Pure: the slot vocabulary, the analyzer's default-slot heuristic, and the
//! project-vs-feature scope rule. No IO.

/// The doc-slot a memory anchors to. `as_str()` matches the `sensei.spine_slot`
/// enum labels exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpineSlot {
    Vision,
    Personas,
    Journeys,
    Roadmap,
    Design,
    Mockups,
    Decisions,
    Brief,
    Plan,
    Tests,
}

impl SpineSlot {
    pub fn as_str(self) -> &'static str {
        match self {
            SpineSlot::Vision => "vision",
            SpineSlot::Personas => "personas",
            SpineSlot::Journeys => "journeys",
            SpineSlot::Roadmap => "roadmap",
            SpineSlot::Design => "design",
            SpineSlot::Mockups => "mockups",
            SpineSlot::Decisions => "decisions",
            SpineSlot::Brief => "brief",
            SpineSlot::Plan => "plan",
            SpineSlot::Tests => "tests",
        }
    }
    pub fn parse(s: &str) -> Option<SpineSlot> {
        Some(match s {
            "vision" => SpineSlot::Vision,
            "personas" => SpineSlot::Personas,
            "journeys" => SpineSlot::Journeys,
            "roadmap" => SpineSlot::Roadmap,
            "design" => SpineSlot::Design,
            "mockups" => SpineSlot::Mockups,
            "decisions" => SpineSlot::Decisions,
            "brief" => SpineSlot::Brief,
            "plan" => SpineSlot::Plan,
            "tests" => SpineSlot::Tests,
            _ => return None,
        })
    }
    /// Project-only slots never carry a feature.
    fn is_project_only(self) -> bool {
        matches!(
            self,
            SpineSlot::Vision
                | SpineSlot::Personas
                | SpineSlot::Journeys
                | SpineSlot::Roadmap
                | SpineSlot::Mockups
        )
    }
    /// Feature-only slots require a feature.
    fn is_feature_only(self) -> bool {
        matches!(self, SpineSlot::Brief | SpineSlot::Plan | SpineSlot::Tests)
    }
}

/// The analyzer's default slot for a generated memory, from its category/type.
/// Structural knowledge -> design; settled learnings/decisions -> decisions.
/// `category` is `sensei.memory_category`, `mtype` is `sensei.memory_type`.
pub fn default_slot(category: Option<&str>, mtype: &str) -> SpineSlot {
    if category == Some("pattern") || category == Some("convention") {
        return SpineSlot::Design;
    }
    match mtype {
        "pattern" | "convention" => SpineSlot::Design,
        _ => SpineSlot::Decisions, // decision, correctness, preference, continuity, question
    }
}

/// Validate a (slot, feature) pair against the scope rule. Ok(()) or an error msg.
pub fn validate_scope(slot: SpineSlot, feature: Option<&str>) -> Result<(), String> {
    let has_feature = feature.map(|f| !f.is_empty()).unwrap_or(false);
    if slot.is_project_only() && has_feature {
        return Err(format!("slot {:?} is project-scope — drop the feature", slot.as_str()));
    }
    if slot.is_feature_only() && !has_feature {
        return Err(format!("slot {:?} is feature-scope — a feature is required", slot.as_str()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_roundtrips_through_parse() {
        for s in [
            SpineSlot::Vision,
            SpineSlot::Design,
            SpineSlot::Decisions,
            SpineSlot::Brief,
            SpineSlot::Plan,
            SpineSlot::Tests,
            SpineSlot::Mockups,
        ] {
            assert_eq!(SpineSlot::parse(s.as_str()), Some(s));
        }
        assert_eq!(SpineSlot::parse("nope"), None);
    }

    #[test]
    fn default_slot_maps_structural_to_design_and_rest_to_decisions() {
        assert_eq!(default_slot(Some("pattern"), "pattern"), SpineSlot::Design);
        assert_eq!(default_slot(Some("convention"), "convention"), SpineSlot::Design);
        assert_eq!(default_slot(Some("correctness"), "decision"), SpineSlot::Decisions);
        assert_eq!(default_slot(None, "preference"), SpineSlot::Decisions);
        assert_eq!(default_slot(None, "continuity"), SpineSlot::Decisions);
        assert_eq!(default_slot(None, "question"), SpineSlot::Decisions);
    }

    #[test]
    fn validate_scope_enforces_the_rule() {
        assert!(validate_scope(SpineSlot::Vision, None).is_ok());
        assert!(validate_scope(SpineSlot::Vision, Some("auth")).is_err());
        assert!(validate_scope(SpineSlot::Brief, Some("auth")).is_ok());
        assert!(validate_scope(SpineSlot::Brief, None).is_err());
        assert!(validate_scope(SpineSlot::Design, None).is_ok());
        assert!(validate_scope(SpineSlot::Design, Some("auth")).is_ok());
        assert!(validate_scope(SpineSlot::Decisions, Some("auth")).is_ok());
    }
}
