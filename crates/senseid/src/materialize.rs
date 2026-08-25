//! File materialization for accepted `write_skill` / `create_agent` recommendations
//! (spec 2026-08-20-insight-acceptance-materialization, P-B).
//!
//! Accepting one of these recs writes a project-scoped Claude Code artifact into the
//! target repo — a skill at `<repo>/.claude/skills/<slug>/SKILL.md` or an agent at
//! `<repo>/.claude/agents/<slug>.md` — which the repo's sessions auto-discover. This
//! CROSSES the dōjō "consent-sensitive install" boundary (writing executable,
//! tool-granting files), so it is only ever invoked from the explicit
//! Accept/materialize request path, never a background task.
//!
//! Rendering + slug + path derivation are pure (unit-tested); the single I/O
//! function ([`write_artifact`]) validates the path stays inside the repo, refuses
//! to clobber an existing (possibly hand-written) file, and creates parent dirs.
//! Files are git-tracked, so a materialization is reviewable + reversible.

use std::path::{Path, PathBuf};

/// The two file-based artifact kinds P-B materializes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    Skill,
    Agent,
}

impl ArtifactKind {
    /// Map a recommendation `action_type` to its file kind, or `None` when the
    /// action isn't a file-materializing one.
    pub fn from_action(action_type: &str) -> Option<Self> {
        match action_type {
            "write_skill" => Some(Self::Skill),
            "create_agent" => Some(Self::Agent),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Skill => "skill",
            Self::Agent => "agent",
        }
    }
}

/// Max slug length — long enough to stay readable, short enough for a tidy dir/file.
const MAX_SLUG: usize = 50;

/// A filesystem-safe kebab-case slug from a free-text title: lowercase, every run of
/// non-`[a-z0-9]` collapsed to a single `-`, trimmed, capped. Empty/degenerate input
/// falls back to `fallback` so a path is never `.../.md` or a traversal token — the
/// slug is `[a-z0-9-]` only, so it can't escape the repo dir.
pub fn slugify(title: &str, fallback: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut prev_dash = false;
    for ch in title.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let s = out.trim_matches('-');
    let s: String = s.chars().take(MAX_SLUG).collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { fallback.to_string() } else { s }
}

/// YAML-frontmatter `SKILL.md`. `description` is a single-line trigger sentence
/// (Claude Code uses it to decide when to load the skill); `body` is the how-to.
pub fn render_skill_md(name: &str, description: &str, body: &str) -> String {
    // Keep the description a single line (frontmatter scalar); collapse newlines.
    let desc = description.split_whitespace().collect::<Vec<_>>().join(" ");
    format!(
        "---\nname: {name}\ndescription: {desc}\n---\n\n# {title}\n\n{body}\n\n---\n_Materialized by sensei from an accepted recommendation._\n",
        name = name,
        desc = desc,
        title = title_case(name),
        body = body.trim(),
    )
}

/// YAML-frontmatter agent `.md`. For a `create_agent` rec the `body` is already an
/// agent system-prompt. Tools default to a conservative read-oriented set + the
/// sensei MCP (the materialized agent reviews/advises; it doesn't get write tools by
/// default — a human can widen them in the file).
pub fn render_agent_md(name: &str, description: &str, body: &str) -> String {
    let desc = description.split_whitespace().collect::<Vec<_>>().join(" ");
    format!(
        "---\nname: {name}\ndescription: {desc}\ntools: Read, Grep, Glob, Bash, mcp__plugin_sensei_sensei__*\nmodel: sonnet\n---\n\n{body}\n\n---\n_Materialized by sensei from an accepted recommendation. Review the tool grant before relying on it._\n",
        name = name,
        desc = desc,
        body = body.trim(),
    )
}

/// Title-case a kebab slug for a heading ("schema-diff-guard" → "Schema Diff Guard").
fn title_case(slug: &str) -> String {
    slug.split('-')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_ascii_uppercase().to_string() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The path a kind+slug writes to, under `repo_root`:
/// - skill → `<repo>/.claude/skills/<slug>/SKILL.md`
/// - agent → `<repo>/.claude/agents/<slug>.md`
pub fn artifact_path(repo_root: &Path, kind: ArtifactKind, slug: &str) -> PathBuf {
    match kind {
        ArtifactKind::Skill => repo_root.join(".claude").join("skills").join(slug).join("SKILL.md"),
        ArtifactKind::Agent => repo_root.join(".claude").join("agents").join(format!("{slug}.md")),
    }
}

/// Write the artifact into the repo. Guards (never fabricate / never clobber):
/// - the resolved path MUST stay under `repo_root` (defense-in-depth on top of the
///   `[a-z0-9-]` slug) — else `Err`;
/// - an EXISTING target is never overwritten (it may be hand-authored) — `Err` so the
///   caller surfaces a conflict and the user renames;
/// - parent dirs are created.
///
/// Returns the repo-relative path written (for display + `materialized_ref`).
pub fn write_artifact(
    repo_root: &Path,
    kind: ArtifactKind,
    slug: &str,
    content: &str,
) -> Result<PathBuf, String> {
    let path = artifact_path(repo_root, kind, slug);
    // Defense-in-depth: the path must live under repo_root.
    let root_norm = repo_root.components().collect::<PathBuf>();
    if !path.starts_with(&root_norm) {
        return Err(format!("refusing to write outside the repo: {}", path.display()));
    }
    if path.exists() {
        return Err(format!(
            "a {} named '{slug}' already exists at {}; rename the recommendation title or edit that file",
            kind.as_str(),
            path.display()
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    std::fs::write(&path, content).map_err(|e| format!("write {}: {e}", path.display()))?;
    // The repo-relative path for provenance/display.
    Ok(path.strip_prefix(&root_norm).unwrap_or(&path).to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_kebabs_and_bounds_and_falls_back() {
        assert_eq!(
            slugify("Establish Core Architectural Guardian!", "agent"),
            "establish-core-architectural-guardian"
        );
        assert_eq!(slugify("  DBD  Schema//Diff  ", "skill"), "dbd-schema-diff");
        assert_eq!(slugify("", "agent"), "agent", "empty → fallback");
        assert_eq!(slugify("!!! @@@", "skill"), "skill", "all-punctuation → fallback");
        assert!(slugify(&"x".repeat(200), "agent").len() <= MAX_SLUG);
        // slug is [a-z0-9-] only — no path separators can appear.
        assert!(!slugify("../../etc/passwd", "agent").contains('/'));
    }

    #[test]
    fn skill_md_has_frontmatter_and_body() {
        let md = render_skill_md(
            "schema-diff-guard",
            "Use when computing a schema diff in dbd-core.",
            "Do X then Y.",
        );
        assert!(md.starts_with("---\nname: schema-diff-guard\ndescription: Use when"));
        assert!(md.contains("# Schema Diff Guard"), "title-cased heading");
        assert!(md.contains("Do X then Y."));
        assert!(md.contains("Materialized by sensei"));
    }

    #[test]
    fn agent_md_has_frontmatter_tools_and_body() {
        let md = render_agent_md(
            "dbd-guardian",
            "Reviews cross-layer changes.",
            "You are an Architectural Review Agent...",
        );
        assert!(md.contains("\nname: dbd-guardian\n"));
        assert!(md.contains("\ntools: Read, Grep, Glob, Bash, mcp__plugin_sensei_sensei__*\n"));
        assert!(md.contains("\nmodel: sonnet\n"));
        assert!(md.contains("You are an Architectural Review Agent..."));
    }

    #[test]
    fn artifact_paths_match_claude_convention() {
        let root = Path::new("/repo");
        assert_eq!(
            artifact_path(root, ArtifactKind::Skill, "s").to_str().unwrap(),
            "/repo/.claude/skills/s/SKILL.md"
        );
        assert_eq!(
            artifact_path(root, ArtifactKind::Agent, "a").to_str().unwrap(),
            "/repo/.claude/agents/a.md"
        );
    }

    #[test]
    fn write_creates_dirs_refuses_overwrite_and_returns_relative() {
        let dir = std::env::temp_dir().join(format!("sensei-mat-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let rel = write_artifact(&dir, ArtifactKind::Agent, "guardian", "content").unwrap();
        assert_eq!(
            rel.to_str().unwrap(),
            ".claude/agents/guardian.md",
            "returns repo-relative path"
        );
        assert!(dir.join(".claude/agents/guardian.md").exists());
        // Second write to the same slug refuses to clobber.
        let err =
            write_artifact(&dir, ArtifactKind::Agent, "guardian", "other").expect_err("no clobber");
        assert!(err.contains("already exists"), "got: {err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn action_kind_mapping() {
        assert_eq!(ArtifactKind::from_action("write_skill"), Some(ArtifactKind::Skill));
        assert_eq!(ArtifactKind::from_action("create_agent"), Some(ArtifactKind::Agent));
        assert_eq!(ArtifactKind::from_action("revise_rule"), None);
    }
}
