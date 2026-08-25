//! D-PLANNER — decompose a goal / spec / issue into a structured plan
//! (phases → features → acceptance criteria) shaped to clear the
//! `sensei-plan-depth-reviewer` bar, and render it to the canonical
//! `docs/plan/*.md` format the plan-as-run engine consumes via `plan_ref`.
//!
//! v0 is a **stateless generate**: no plan tables are persisted (the run engine
//! is file-based today — it takes a `plan_ref` path — and the project-window
//! Tasks tab that would consume structured rows isn't built yet, so storing them
//! now would be YAGNI). The daemon returns the structured plan AND the rendered
//! markdown; the caller writes `docs/plan/<slug>.md`, reviews it against the
//! depth bar, then passes the path to `start_run`.
//!
//! The generation prompt encodes the depth bar (observable acceptance criteria,
//! inputs/outputs/deps, no TBDs, explicit scope, goal-alignment) so the output
//! is shaped to pass review; an *automated* self-review loop is a v1 follow-on.

use gateway::types::capability::Capability;
use gateway::types::request::{InferenceRequest, Message, MessageRole, Payload};
use serde::{Deserialize, Serialize};

/// One buildable unit of work within a phase. Mirrors the depth bar: a feature
/// must name what you can OBSERVE when it's done, what it needs/produces, and
/// what it deliberately excludes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Feature {
    pub title: String,
    /// Observable acceptance criteria — a value, screen state, command output,
    /// or row you can point at. Never "tests pass" / "it works".
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    /// What it does AND what it deliberately does NOT (explicit scope).
    #[serde(default)]
    pub scope: String,
    /// What must exist first — other features or named prerequisites.
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// A phase groups features that ship together toward the goal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Phase {
    pub title: String,
    #[serde(default)]
    pub features: Vec<Feature>,
}

/// A structured plan: one singular goal decomposed into ordered phases.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Plan {
    pub goal: String,
    #[serde(default)]
    pub phases: Vec<Phase>,
}

const MAX_TOKENS: u32 = 2048;

/// The depth bar, encoded as the model's operating instructions so the generated
/// plan is shaped to clear `sensei-plan-depth-reviewer`.
const SYSTEM: &str = "\
You are a senior engineer decomposing a goal into a plan an autonomous agent can \
execute unattended. Output ONLY a single JSON object, no prose, no code fences.

Schema:
{
  \"goal\": \"<one singular restated goal>\",
  \"phases\": [
    {
      \"title\": \"<phase title>\",
      \"features\": [
        {
          \"title\": \"<one buildable unit>\",
          \"acceptance_criteria\": [\"<observable result you can point at>\"],
          \"scope\": \"<what it does AND what it deliberately does not>\",
          \"dependencies\": [\"<what must exist first, or empty>\"]
        }
      ]
    }
  ]
}

Rules (a reviewer will reject the plan otherwise):
- Every feature has at least one OBSERVABLE acceptance criterion — a value, screen \
state, command output, or row you can point at. Never \"tests pass\" or \"it works\".
- State each feature's inputs, outputs, and dependencies; never depend on something unnamed.
- No TBDs or placeholders. Resolve every either/or to one choice.
- Each feature states what it does AND what it deliberately does not.
- Order phases so nothing depends on a later one. Keep the goal singular and aligned.";

/// Build the `(system, user)` prompt pair. `context` is optional grounding
/// (a spec, an issue body, project conventions) appended verbatim.
pub fn build_planner_prompt(goal: &str, context: Option<&str>) -> (String, String) {
    let mut user = format!("Goal:\n{}\n", goal.trim());
    if let Some(ctx) = context.map(str::trim).filter(|c| !c.is_empty()) {
        user.push_str(&format!("\nContext (use it, do not restate it):\n{ctx}\n"));
    }
    user.push_str("\nReturn the plan as the JSON object described. JSON only.");
    (SYSTEM.to_string(), user)
}

/// Parse + validate the model's output into a [`Plan`]. Tolerates prose/code
/// fences around the JSON object. Returns `None` unless the plan clears the
/// structural floor of the depth bar: a non-empty goal, at least one phase, each
/// phase with at least one feature, and each feature with at least one
/// acceptance criterion.
pub fn parse_plan(content: &str) -> Option<Plan> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    if end <= start {
        return None;
    }
    let mut plan: Plan = serde_json::from_str(&content[start..=end]).ok()?;

    // Trim + drop empties so the structural checks below are meaningful.
    plan.goal = plan.goal.trim().to_string();
    if plan.goal.is_empty() || plan.phases.is_empty() {
        return None;
    }
    for phase in &mut plan.phases {
        phase.title = phase.title.trim().to_string();
        for feature in &mut phase.features {
            feature.title = feature.title.trim().to_string();
            feature.scope = feature.scope.trim().to_string();
            feature.acceptance_criteria.retain(|c| !c.trim().is_empty());
            feature.dependencies.retain(|d| !d.trim().is_empty());
        }
        phase.features.retain(|f| !f.title.is_empty());
    }
    plan.phases.retain(|p| !p.title.is_empty() && !p.features.is_empty());

    if plan.phases.is_empty() {
        return None;
    }
    // Every surviving feature must carry an observable acceptance criterion.
    let all_features_verifiable =
        plan.phases.iter().flat_map(|p| &p.features).all(|f| !f.acceptance_criteria.is_empty());
    if !all_features_verifiable {
        return None;
    }
    Some(plan)
}

/// Render a [`Plan`] to the canonical `docs/plan/*.md` shape the depth-reviewer
/// reads and the run engine references via `plan_ref`.
pub fn render_plan_md(plan: &Plan) -> String {
    let mut md = format!("# {}\n\n", plan.goal);
    md.push_str(
        "> Generated by the sensei planner. Review against the depth bar \
         (`/sensei:agent plan-depth-reviewer`) before starting an unattended run.\n",
    );
    for (i, phase) in plan.phases.iter().enumerate() {
        md.push_str(&format!("\n## Phase {} — {}\n", i + 1, phase.title));
        for feature in &phase.features {
            md.push_str(&format!("\n### {}\n", feature.title));
            if !feature.acceptance_criteria.is_empty() {
                md.push_str("- **Acceptance:**\n");
                for c in &feature.acceptance_criteria {
                    md.push_str(&format!("  - {c}\n"));
                }
            }
            if !feature.scope.is_empty() {
                md.push_str(&format!("- **Scope:** {}\n", feature.scope));
            }
            if !feature.dependencies.is_empty() {
                md.push_str(&format!("- **Depends on:** {}\n", feature.dependencies.join(", ")));
            }
        }
    }
    md
}

/// Generate a structured plan from a goal via the LLM gateway. Uses the
/// `reasoning` chain (heavier model, for decomposition) and retries once if the
/// first response doesn't parse (LLM JSON is flaky). Returns an error string on
/// gateway failure or if neither attempt yields a valid plan — planning is an
/// explicit user action, so a failure is surfaced, not silently defaulted.
pub async fn generate_plan(
    gateway: &gateway::Gateway,
    goal: &str,
    context: Option<&str>,
) -> Result<Plan, String> {
    if goal.trim().is_empty() {
        return Err("goal is empty".to_string());
    }
    let (system, user) = build_planner_prompt(goal, context);

    for attempt in 0..2 {
        let user_msg = if attempt == 0 {
            user.clone()
        } else {
            format!(
                "{user}\n\nYour previous reply was not valid JSON. Return ONLY the JSON object."
            )
        };
        let request = InferenceRequest {
            capability: Capability::TextChat,
            model: None,
            router: None,
            chain: Some("reasoning".into()),
            payload: Payload::Chat {
                messages: vec![Message::text(MessageRole::User, &user_msg)],
                system: Some(system.clone()),
                max_tokens: Some(MAX_TOKENS),
                temperature: None,
                tools: Vec::new(),
            },
            budget: None,
            auth: None,
            panel: None,
            consensus: None,
            allow_fallback: true,
            credentials: std::collections::HashMap::new(),
        };
        match gateway.execute(&request).await {
            Ok(resp) if resp.success => {
                let content = resp.content.unwrap_or_default();
                if let Some(plan) = parse_plan(&content) {
                    return Ok(plan);
                }
                tracing::warn!(attempt, "planner: model output did not parse");
            }
            Ok(_) | Err(_) => {
                return Err("no model answered the planning request".to_string());
            }
        }
    }
    Err("planner could not produce a valid plan from the model output".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_json() -> &'static str {
        r#"Here is the plan:
        {
          "goal": "Add a dark-mode toggle",
          "phases": [
            {
              "title": "Backend",
              "features": [
                {
                  "title": "Persist the theme preference",
                  "acceptance_criteria": ["GET /api/prefs returns {\"theme\":\"dark\"} after a POST"],
                  "scope": "Store + read the flag; NOT the UI switch.",
                  "dependencies": ["prefs table exists"]
                }
              ]
            }
          ]
        }
        "#
    }

    #[test]
    fn prompt_carries_goal_and_depth_bar() {
        let (system, user) = build_planner_prompt("Ship X", Some("some spec"));
        assert!(user.contains("Ship X"));
        assert!(user.contains("some spec"));
        assert!(system.contains("acceptance criterion") || system.contains("acceptance_criteria"));
        assert!(system.contains("JSON"), "system prompt pins JSON-only output");
    }

    #[test]
    fn parse_extracts_json_from_prose() {
        let plan = parse_plan(sample_json()).expect("valid plan parses out of surrounding prose");
        assert_eq!(plan.goal, "Add a dark-mode toggle");
        assert_eq!(plan.phases.len(), 1);
        assert_eq!(plan.phases[0].features.len(), 1);
        assert_eq!(plan.phases[0].features[0].acceptance_criteria.len(), 1);
    }

    #[test]
    fn parse_rejects_feature_without_acceptance_criteria() {
        let bad = r#"{"goal":"g","phases":[{"title":"P","features":[{"title":"F"}]}]}"#;
        assert!(parse_plan(bad).is_none(), "a feature with no observable criterion fails the bar");
    }

    #[test]
    fn parse_rejects_empty_goal_or_no_phases() {
        assert!(parse_plan(r#"{"goal":"  ","phases":[]}"#).is_none());
        assert!(parse_plan(r#"{"goal":"g","phases":[]}"#).is_none());
        assert!(parse_plan("no json here").is_none());
    }

    #[test]
    fn parse_drops_empty_features_then_rejects_empty_phase() {
        // A phase whose only feature has a blank title collapses to no features → dropped.
        let j = r#"{"goal":"g","phases":[{"title":"P","features":[{"title":"  ","acceptance_criteria":["x"]}]}]}"#;
        assert!(parse_plan(j).is_none());
    }

    #[test]
    fn render_includes_goal_phases_features_and_criteria() {
        let plan = parse_plan(sample_json()).unwrap();
        let md = render_plan_md(&plan);
        assert!(md.contains("# Add a dark-mode toggle"));
        assert!(md.contains("## Phase 1 — Backend"));
        assert!(md.contains("### Persist the theme preference"));
        assert!(md.contains("GET /api/prefs returns"));
        assert!(md.contains("**Scope:**"));
        assert!(md.contains("**Depends on:** prefs table exists"));
    }
}
