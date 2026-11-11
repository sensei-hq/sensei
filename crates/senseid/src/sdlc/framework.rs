//! Which third-party spec-driven-development (SDD) framework a session worked in
//! (blueprint `docs/blueprints/2026-11-10-ticket-sdd-framework.md`, B1–B12).
//!
//! What the session did decides ([`behavioural`]); when it did nothing that names a
//! framework, the marker directories of its repository on disk are the fallback
//! ([`repo_markers`] → [`from_markers`]). [`decide`] combines the two with what the
//! session already holds. Everything here is pure except [`repo_markers`], whose
//! filesystem access is injected.

use std::path::Path;

use crate::sdlc::classify::stage_commands;
use crate::sdlc::signals::{EDIT_TOOLS, SignalEvent};

/// A third-party SDD framework (`sensei.sdd_framework`). sensei's own plan graph
/// is not one (B3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SddFramework {
    Openspec,
    SpecKit,
    Kiro,
    Bmad,
}

impl SddFramework {
    /// Every framework, in static-fallback precedence order (B11).
    pub const ALL: [SddFramework; 4] =
        [SddFramework::Openspec, SddFramework::SpecKit, SddFramework::Kiro, SddFramework::Bmad];

    pub fn as_str(self) -> &'static str {
        match self {
            SddFramework::Openspec => "openspec",
            SddFramework::SpecKit => "spec_kit",
            SddFramework::Kiro => "kiro",
            SddFramework::Bmad => "bmad",
        }
    }

    pub fn parse(s: &str) -> Option<SddFramework> {
        SddFramework::ALL.into_iter().find(|f| f.as_str() == s)
    }
}

/// How a session's framework was found (`activity.sessions.sdd_framework_source`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionSource {
    /// From what the session did.
    Behavioural,
    /// From the one framework whose markers its repository holds.
    Static,
    /// Its repository holds markers of two or more frameworks; never pinned (B11).
    StaticAmbiguous,
    /// The marker probe failed or timed out: an explicit error state (B10).
    ProbeFailed,
}

impl DetectionSource {
    pub fn as_str(self) -> &'static str {
        match self {
            DetectionSource::Behavioural => "behavioural",
            DetectionSource::Static => "static",
            DetectionSource::StaticAmbiguous => "static_ambiguous",
            DetectionSource::ProbeFailed => "probe_failed",
        }
    }

    pub fn parse(s: &str) -> Option<DetectionSource> {
        use DetectionSource::*;
        [Behavioural, Static, StaticAmbiguous, ProbeFailed].into_iter().find(|d| d.as_str() == s)
    }
}

/// A session's stored detection: both `None`, a framework with its source, or no
/// framework with [`DetectionSource::ProbeFailed`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Detection {
    pub framework: Option<SddFramework>,
    pub source: Option<DetectionSource>,
}

/// How a framework shows itself. Paths are relative to the repository root (B4).
pub struct Signature {
    pub framework: SddFramework,
    /// A typed prompt that starts with one of these, lower-cased (B8).
    pub prompts: &'static [&'static str],
    /// A `Skill` whose name, after any `plugin:` prefix, starts with one of these.
    pub skills: &'static [&'static str],
    /// An edited file whose repo-relative path starts with one of these.
    pub paths: &'static [&'static str],
    /// A top-level directory whose children name features `^[0-9]+[a-z]?-`.
    pub numbered_dir: Option<&'static str>,
    /// A shell command whose command word is one of these.
    pub commands: &'static [&'static str],
    /// Directories whose presence in the repository marks the framework.
    pub markers: &'static [&'static str],
}

/// The one table every framework rule reads.
pub const SIGNATURES: &[Signature] = &[
    Signature {
        framework: SddFramework::Openspec,
        prompts: &["/opsx:", "/openspec:", "/openspec-"],
        skills: &["openspec-"],
        paths: &["openspec/"],
        numbered_dir: None,
        commands: &["openspec"],
        markers: &["openspec"],
    },
    Signature {
        framework: SddFramework::SpecKit,
        prompts: &["/speckit.", "/speckit-"],
        skills: &["speckit-", "speckit."],
        paths: &[".specify/"],
        numbered_dir: Some("specs"),
        commands: &["specify"],
        markers: &[".specify"],
    },
    Signature {
        framework: SddFramework::Kiro,
        prompts: &[],
        skills: &[],
        paths: &[".kiro/specs/"],
        numbered_dir: None,
        commands: &[],
        markers: &[".kiro/specs"],
    },
    Signature {
        framework: SddFramework::Bmad,
        prompts: &["/bmad-", "/bmad:"],
        skills: &["bmad-"],
        paths: &["_bmad/", "_bmad-output/", ".bmad-core/"],
        numbered_dir: None,
        commands: &[],
        markers: &["_bmad", ".bmad-core"],
    },
];

/// The framework the session's earliest evidence names, if any. `repo_root` is the
/// session's repository; without one, file paths are no evidence (B4).
pub fn behavioural(events: &[SignalEvent], repo_root: Option<&str>) -> Option<SddFramework> {
    events
        .iter()
        .filter_map(|e| evidence(e, repo_root).map(|f| ((e.ts, e.id), f)))
        .min_by_key(|(at, _)| *at)
        .map(|(_, f)| f)
}

/// The framework one event names.
fn evidence(e: &SignalEvent, repo_root: Option<&str>) -> Option<SddFramework> {
    let tool = e.tool_name.as_deref().unwrap_or("");
    if e.event_type == "UserPromptSubmit" {
        let command = typed_command(e.prompt.as_deref()?)?.to_ascii_lowercase();
        return find(|s| s.prompts.iter().any(|p| command.starts_with(p)));
    }
    if tool == "Skill" {
        let skill = e.skill.as_deref()?;
        let name = skill.rsplit(':').next().unwrap_or(skill).to_ascii_lowercase();
        return find(|s| s.skills.iter().any(|p| name.starts_with(p)));
    }
    if e.event_type != "PostToolUse" {
        return None;
    }
    if tool == "Bash" {
        let words: Vec<String> = stage_commands(e.command.as_deref()?)
            .iter()
            .filter_map(|c| c.split_whitespace().next())
            .map(|w| w.rsplit('/').next().unwrap_or(w).to_string())
            .collect();
        return find(|s| s.commands.iter().any(|c| words.iter().any(|w| w == c)));
    }
    if !EDIT_TOOLS.contains(&tool) {
        return None;
    }
    let rel = e.path.as_deref()?.strip_prefix(repo_root?)?.strip_prefix('/')?;
    find(|s| {
        s.paths.iter().any(|p| rel.starts_with(p))
            || s.numbered_dir.is_some_and(|dir| numbered_feature(rel, dir))
    })
}

fn find(matches: impl Fn(&Signature) -> bool) -> Option<SddFramework> {
    SIGNATURES.iter().find(|s| matches(s)).map(|s| s.framework)
}

/// The command a typed prompt starts with (B8): leading `<ide_…>…</ide_…>` blocks
/// are what the IDE prepends and are skipped; anything else leading (a relayed
/// `<task-notification>` or `<agent-message>`, prose) means no command.
fn typed_command(prompt: &str) -> Option<&str> {
    let mut rest = prompt.trim_start();
    while let Some(tag) = rest.strip_prefix("<ide_") {
        let name_end = tag.find(|c: char| c == '>' || c.is_whitespace())?;
        let close = format!("</ide_{}>", &tag[..name_end]);
        let end = rest.find(&close)? + close.len();
        rest = rest[end..].trim_start();
    }
    rest.starts_with('/').then(|| rest.split_whitespace().next().unwrap_or(rest))
}

/// Whether `rel` is under `dir/<n>-…/`, a numbered feature directory (`001-auth`,
/// `002a-gaps`, `107917-flags`).
fn numbered_feature(rel: &str, dir: &str) -> bool {
    let Some(rest) = rel.strip_prefix(dir).and_then(|r| r.strip_prefix('/')) else {
        return false;
    };
    let Some((seg, _)) = rest.split_once('/') else { return false };
    let digits = seg.bytes().take_while(u8::is_ascii_digit).count();
    let tail = &seg.as_bytes()[digits..];
    digits > 0
        && match tail {
            [b'-', ..] => true,
            [c, b'-', ..] => c.is_ascii_lowercase(),
            _ => false,
        }
}

/// The framework a repository's present markers name (deduplicated): one →
/// `Static`, two or more → the first in [`SddFramework::ALL`] order, `StaticAmbiguous`.
pub fn from_markers(present: &[SddFramework]) -> Option<(SddFramework, DetectionSource)> {
    let mut distinct = SddFramework::ALL.into_iter().filter(|f| present.contains(f));
    let first = distinct.next()?;
    let source = if distinct.next().is_some() {
        DetectionSource::StaticAmbiguous
    } else {
        DetectionSource::Static
    };
    Some((first, source))
}

/// The frameworks whose markers exist under `repo`. `is_dir` answers whether a path
/// is a directory; `NotFound` reads as no. A missing `repo` holds no markers (B6);
/// any other error is returned. Only the fixed marker paths are checked.
pub fn repo_markers(
    repo: &Path,
    is_dir: impl Fn(&Path) -> std::io::Result<bool>,
) -> Result<Vec<SddFramework>, String> {
    let probe = |p: &Path| match is_dir(p) {
        Ok(found) => Ok(found),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("{}: {e}", p.display())),
    };
    if !probe(repo)? {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    for s in SIGNATURES {
        for marker in s.markers {
            if probe(&repo.join(marker))? {
                found.push(s.framework);
            }
        }
    }
    Ok(found)
}

/// What the marker fallback found for a session's repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticOutcome {
    /// Not probed: a behavioural hit made it unnecessary, or the session has no repository.
    NotProbed,
    Found(Vec<SddFramework>),
    /// The probe errored or timed out (B10).
    Failed,
}

/// The session's detection now, given its behavioural hit, its marker outcome and
/// what it held before. A value is never replaced by none or by a failed probe (B12).
pub fn decide(
    behavioural: Option<SddFramework>,
    markers: &StaticOutcome,
    previous: Detection,
) -> Detection {
    let now = match (behavioural, markers) {
        (Some(f), _) => {
            Detection { framework: Some(f), source: Some(DetectionSource::Behavioural) }
        }
        (None, StaticOutcome::Found(present)) => from_markers(present)
            .map(|(f, s)| Detection { framework: Some(f), source: Some(s) })
            .unwrap_or_default(),
        (None, StaticOutcome::Failed) => {
            Detection { framework: None, source: Some(DetectionSource::ProbeFailed) }
        }
        (None, StaticOutcome::NotProbed) => Detection::default(),
    };
    if now.framework.is_none() && previous.framework.is_some() { previous } else { now }
}

#[cfg(test)]
mod tests {
    use super::*;
    use DetectionSource::*;
    use SddFramework::*;

    const REPO: &str = "/r/a";

    fn ev(id: i64, event_type: &str, tool: Option<&str>) -> SignalEvent {
        SignalEvent {
            id,
            event_type: event_type.into(),
            tool_name: tool.map(Into::into),
            ts: 1_700_000_000_000 + id * 1000,
            ..Default::default()
        }
    }
    fn prompt(id: i64, text: &str) -> SignalEvent {
        SignalEvent { prompt: Some(text.into()), ..ev(id, "UserPromptSubmit", None) }
    }
    fn skill(id: i64, name: &str) -> SignalEvent {
        SignalEvent { skill: Some(name.into()), ..ev(id, "PreToolUse", Some("Skill")) }
    }
    fn edit(id: i64, tool: &str, path: &str) -> SignalEvent {
        SignalEvent { path: Some(path.into()), ..ev(id, "PostToolUse", Some(tool)) }
    }
    fn bash(id: i64, command: &str) -> SignalEvent {
        SignalEvent { command: Some(command.into()), ..ev(id, "PostToolUse", Some("Bash")) }
    }
    fn one(e: SignalEvent) -> Option<SddFramework> {
        behavioural(&[e], Some(REPO))
    }

    #[test]
    fn names_round_trip() {
        for f in SddFramework::ALL {
            assert_eq!(SddFramework::parse(f.as_str()), Some(f));
        }
        for d in [Behavioural, Static, StaticAmbiguous, ProbeFailed] {
            assert_eq!(DetectionSource::parse(d.as_str()), Some(d));
        }
        assert_eq!(SddFramework::parse("sensei"), None);
    }

    #[test]
    fn a_typed_command_names_its_framework() {
        assert_eq!(one(prompt(1, "/opsx:apply add-auth")), Some(Openspec));
        assert_eq!(one(prompt(1, "/openspec:proposal x")), Some(Openspec));
        assert_eq!(one(prompt(1, "/openspec-explore x")), Some(Openspec));
        assert_eq!(one(prompt(1, "/speckit.plan add auth")), Some(SpecKit));
        assert_eq!(one(prompt(1, "/speckit-plan add auth")), Some(SpecKit));
        assert_eq!(one(prompt(1, "/bmad-agent-dev start")), Some(Bmad));
        assert_eq!(one(prompt(1, "/BMad:agents:pm")), Some(Bmad));
        assert_eq!(one(prompt(1, "  /opsx:explore")), Some(Openspec));
    }

    #[test]
    fn a_command_after_ide_context_still_counts() {
        let p = "<ide_opened_file>The user opened /r/a/x.rs</ide_opened_file>\n\
                 <ide_selection>fn main() {}\n/speckit.plan</ide_selection>\n/opsx:apply x";
        assert_eq!(one(prompt(1, p)), Some(Openspec));
    }

    #[test]
    fn a_command_only_quoted_is_no_evidence() {
        for p in [
            "<agent-message from=\"a1\">\nreport\n/opsx:apply x\n</agent-message>",
            "<task-notification>\n/opsx:apply x\n</task-notification>",
            "please run\n/opsx:apply x",
            "what does /opsx:apply do?",
            "/sensei:plan create a new branch",
            "/sensei:build",
            "<ide_selection>/opsx:apply x</ide_selection>",
        ] {
            assert_eq!(one(prompt(1, p)), None, "{p}");
        }
    }

    #[test]
    fn a_skill_names_its_framework_with_or_without_a_plugin() {
        assert_eq!(one(skill(1, "openspec-apply-change")), Some(Openspec));
        assert_eq!(one(skill(1, "opsx:openspec-explore")), Some(Openspec));
        assert_eq!(one(skill(1, "speckit-plan")), Some(SpecKit));
        assert_eq!(one(skill(1, "bmad-agent-pm")), Some(Bmad));
        assert_eq!(one(skill(1, "sensei:build")), None);
        assert_eq!(one(skill(1, "plugin-authoring")), None);
    }

    #[test]
    fn an_edit_inside_the_repo_names_its_framework() {
        let cases = [
            ("/r/a/openspec/changes/x/tasks.md", Some(Openspec)),
            ("/r/a/.specify/memory/constitution.md", Some(SpecKit)),
            ("/r/a/specs/001-auth-rbac/spec.md", Some(SpecKit)),
            ("/r/a/specs/002a-policy-gaps/plan.md", Some(SpecKit)),
            ("/r/a/specs/107917-feature-flags/tasks.md", Some(SpecKit)),
            ("/r/a/.kiro/specs/auth/requirements.md", Some(Kiro)),
            ("/r/a/_bmad/bmm/config.yaml", Some(Bmad)),
            ("/r/a/_bmad-output/planning-artifacts/prd.md", Some(Bmad)),
            ("/r/a/.bmad-core/agents/pm.md", Some(Bmad)),
            ("/r/a/docs/specs/001-z/a.md", None),
            ("/r/a/spec/models/user_spec.rb", None),
            ("/r/a/specs/auth/spec.md", None),
            ("/r/a/docs/openspec/x.md", None),
            ("/r/a/my-openspec/x.md", None),
            ("/r/a/.kiro/settings.json", None),
            ("/r/ab/openspec/x.md", None),
        ];
        for (path, want) in cases {
            for tool in ["Edit", "MultiEdit", "Write", "NotebookEdit"] {
                assert_eq!(one(edit(1, tool, path)), want, "{tool} {path}");
            }
        }
    }

    #[test]
    fn an_edit_outside_the_repo_or_without_one_is_no_evidence() {
        assert_eq!(one(edit(1, "Edit", "/r/b/openspec/changes/x/tasks.md")), None);
        assert_eq!(one(edit(1, "Write", "/tmp/claude/openspec/x.md")), None);
        assert_eq!(behavioural(&[edit(1, "Edit", "/r/a/openspec/x.md")], None), None);
    }

    #[test]
    fn only_a_completed_file_edit_is_evidence() {
        let pre = SignalEvent {
            path: Some("/r/a/openspec/x.md".into()),
            ..ev(1, "PreToolUse", Some("Edit"))
        };
        let failed = SignalEvent {
            path: Some("/r/a/openspec/x.md".into()),
            ..ev(1, "PostToolUseFailure", Some("Write"))
        };
        let read = edit(1, "Read", "/r/a/openspec/x.md");
        for e in [pre, failed, read] {
            assert_eq!(one(e.clone()), None, "{e:?}");
        }
    }

    #[test]
    fn a_shell_command_counts_only_as_the_command_word() {
        assert_eq!(one(bash(1, "cd x && openspec new change y")), Some(Openspec));
        assert_eq!(one(bash(1, "openspec status --json | jq .")), Some(Openspec));
        assert_eq!(one(bash(1, "specify init --here")), Some(SpecKit));
        assert_eq!(one(bash(1, "grep -i openspec f")), None);
        assert_eq!(one(bash(1, "echo 'openspec new change'")), None);
        assert_eq!(one(bash(1, "ls openspec/changes")), None);
    }

    #[test]
    fn the_earliest_evidence_decides() {
        let events =
            [skill(2, "speckit-plan"), prompt(5, "/opsx:apply x"), edit(9, "Edit", "/r/a/_bmad/x")];
        assert_eq!(behavioural(&events, Some(REPO)), Some(SpecKit));
        let unordered = [prompt(5, "/opsx:apply x"), skill(2, "speckit-plan")];
        assert_eq!(behavioural(&unordered, Some(REPO)), Some(SpecKit));
        let sensei = [prompt(1, "/sensei:plan"), skill(2, "sensei:build"), prompt(3, "go")];
        assert_eq!(behavioural(&sensei, Some(REPO)), None);
    }

    #[test]
    fn markers_of_one_framework_are_static_and_of_two_are_ambiguous() {
        assert_eq!(from_markers(&[]), None);
        assert_eq!(from_markers(&[SpecKit]), Some((SpecKit, Static)));
        assert_eq!(from_markers(&[Bmad, Bmad]), Some((Bmad, Static)));
        assert_eq!(from_markers(&[Bmad, Openspec]), Some((Openspec, StaticAmbiguous)));
        assert_eq!(from_markers(&[Kiro, SpecKit, Kiro]), Some((SpecKit, StaticAmbiguous)));
    }

    fn fake_fs(dirs: &'static [&'static str]) -> impl Fn(&Path) -> std::io::Result<bool> {
        move |p: &Path| Ok(dirs.iter().any(|d| Path::new(d) == p))
    }

    #[test]
    fn the_repo_markers_present_on_disk_are_found() {
        let fs = fake_fs(&["/r/a", "/r/a/openspec", "/r/a/_bmad", "/r/a/.bmad-core"]);
        assert_eq!(repo_markers(Path::new("/r/a"), fs), Ok(vec![Openspec, Bmad, Bmad]));
        let kiro = fake_fs(&["/r/a", "/r/a/.kiro"]);
        assert_eq!(repo_markers(Path::new("/r/a"), kiro), Ok(vec![]));
        let kiro_specs = fake_fs(&["/r/a", "/r/a/.kiro/specs"]);
        assert_eq!(repo_markers(Path::new("/r/a"), kiro_specs), Ok(vec![Kiro]));
    }

    #[test]
    fn a_missing_repo_holds_no_markers_and_other_errors_are_returned() {
        let gone = |_: &Path| -> std::io::Result<bool> {
            Err(std::io::Error::from(std::io::ErrorKind::NotFound))
        };
        assert_eq!(repo_markers(Path::new("/r/gone"), gone), Ok(vec![]));
        let not_a_dir = fake_fs(&[]);
        assert_eq!(repo_markers(Path::new("/r/file"), not_a_dir), Ok(vec![]));
        let denied = |p: &Path| -> std::io::Result<bool> {
            if p == Path::new("/r/a") {
                Ok(true)
            } else {
                Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
            }
        };
        let err = repo_markers(Path::new("/r/a"), denied).unwrap_err();
        assert!(err.contains("/r/a/openspec"), "{err}");
    }

    fn d(f: Option<SddFramework>, s: Option<DetectionSource>) -> Detection {
        Detection { framework: f, source: s }
    }

    #[test]
    fn behaviour_wins_then_markers_then_nothing() {
        let none = Detection::default();
        let found = StaticOutcome::Found(vec![Openspec]);
        assert_eq!(decide(Some(SpecKit), &found, none), d(Some(SpecKit), Some(Behavioural)));
        assert_eq!(decide(None, &found, none), d(Some(Openspec), Some(Static)));
        let both = StaticOutcome::Found(vec![Openspec, Bmad]);
        assert_eq!(decide(None, &both, none), d(Some(Openspec), Some(StaticAmbiguous)));
        assert_eq!(decide(None, &StaticOutcome::Found(vec![]), none), none);
        assert_eq!(decide(None, &StaticOutcome::NotProbed, none), none);
        assert_eq!(decide(None, &StaticOutcome::Failed, none), d(None, Some(ProbeFailed)));
    }

    #[test]
    fn a_value_is_never_lost_to_missing_or_failed_markers() {
        let held = d(Some(SpecKit), Some(Static));
        assert_eq!(decide(None, &StaticOutcome::Found(vec![]), held), held);
        assert_eq!(decide(None, &StaticOutcome::NotProbed, held), held);
        assert_eq!(decide(None, &StaticOutcome::Failed, held), held);
        assert_eq!(
            decide(Some(Openspec), &StaticOutcome::NotProbed, held),
            d(Some(Openspec), Some(Behavioural))
        );
        let failed_before = d(None, Some(ProbeFailed));
        assert_eq!(
            decide(None, &StaticOutcome::Found(vec![]), failed_before),
            Detection::default()
        );
    }
}
