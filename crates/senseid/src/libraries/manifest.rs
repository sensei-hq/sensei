//! Parse a `sensei.library.json` manifest — a library's declaration of the skills,
//! agents, and MCP tools it provides (workstream D). Pure serde; the ingestion layer
//! resolves `path` entries to `body` and persists via `replace_library_capabilities`.

use serde::Deserialize;

/// The manifest filename a library commits at its repo root.
pub const MANIFEST_FILENAME: &str = "sensei.library.json";

/// A skill a library ships. Exactly one of `path`/`body` should be present; the
/// ingestion resolves `path` → `body` and skips entries with neither (no fabrication).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ProvidedSkill {
    pub name: String,
    pub focus: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
}

/// A review/assistant agent a library ships. Same `path`/`body` rule as a skill.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ProvidedAgent {
    pub name: String,
    pub focus: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
}

/// An MCP tool a library ships — declared for surfacing (D2); sensei does not execute
/// it in v1.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ProvidedTool {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub endpoint: Option<String>,
}

/// A parsed `sensei.library.json`. `library` + `version` are required (serde errors if
/// absent — never fabricated); the capability arrays default to empty.
#[derive(Debug, Clone, Deserialize)]
pub struct LibraryManifest {
    pub library: String,
    #[serde(default)]
    pub ecosystem: Option<String>,
    /// Semver RANGE the declared capabilities apply to (free text in v1).
    pub version: String,
    #[serde(default)]
    pub skills: Vec<ProvidedSkill>,
    #[serde(default)]
    pub agents: Vec<ProvidedAgent>,
    #[serde(default)]
    pub tools: Vec<ProvidedTool>,
}

/// Parse a `sensei.library.json`. `Err` on malformed JSON or a missing required field
/// (`library`, `version`) — never a fabricated default.
pub fn parse_library_manifest(json: &str) -> Result<LibraryManifest, String> {
    serde_json::from_str(json).map_err(|e| format!("invalid {MANIFEST_FILENAME}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"{
      "library": "rokkit",
      "version": ">=1.3",
      "skills": [
        {"name": "semantic-styles-rokkit", "focus": "styling", "path": "packages/cli/skills/semantic-styles-rokkit/SKILL.md"}
      ],
      "agents": [
        {"name": "rokkit-styles-reviewer", "focus": "styling-review", "path": "packages/cli/agents/rokkit-styles-reviewer.md"}
      ]
    }"#;

    #[test]
    fn parses_the_example_manifest() {
        let m = parse_library_manifest(EXAMPLE).unwrap();
        assert_eq!(m.library, "rokkit");
        assert_eq!(m.version, ">=1.3");
        assert_eq!(m.skills.len(), 1);
        assert_eq!(m.skills[0].focus, "styling");
        assert_eq!(m.skills[0].path.as_deref(), Some("packages/cli/skills/semantic-styles-rokkit/SKILL.md"));
        assert_eq!(m.agents[0].name, "rokkit-styles-reviewer");
        assert!(m.tools.is_empty());
    }

    #[test]
    fn minimal_manifest_needs_only_library_and_version() {
        let m = parse_library_manifest(r#"{"library":"x","version":"1"}"#).unwrap();
        assert_eq!(m.library, "x");
        assert!(m.skills.is_empty() && m.agents.is_empty());
    }

    #[test]
    fn missing_required_field_or_bad_json_is_err_not_a_default() {
        assert!(parse_library_manifest(r#"{"version":"1"}"#).is_err(), "missing library → Err");
        assert!(parse_library_manifest(r#"{"library":"x"}"#).is_err(), "missing version → Err");
        assert!(parse_library_manifest("not json").is_err());
    }

    #[test]
    fn an_entry_with_neither_path_nor_body_still_parses() {
        // The parser is lenient; the ingestion is what skips a bodyless entry (+logs).
        let m = parse_library_manifest(r#"{"library":"x","version":"1","skills":[{"name":"s","focus":"f"}]}"#).unwrap();
        assert!(m.skills[0].path.is_none() && m.skills[0].body.is_none());
    }
}
