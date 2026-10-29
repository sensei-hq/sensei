//! A session's SDLC timeline, derived from its hook events (spec S2): builds and
//! tests with their outcomes, calls that never reported back, code edits, task
//! ticks, reviews and the fixes that follow them, PRs and pushes, and the base
//! work slices that holds are read from.
//!
//! Pure: the events arrive projected and ordered by `(ts, id)`, and the whole
//! derivation is a function of them, the catalogue, the repository's folder
//! commands and the clock that bounds a sane `ts`.

use crate::sdlc::classify::{
    Catalogue, Category, Exit, FolderCommand, RunSignal, classify, run_categories, stage_commands,
};

/// Base slices are cut where consecutive events are more than this apart. The
/// workspace threshold (1–240 minutes) merges them at read, which gives exactly
/// the slices a cut at that threshold would (spec S2).
pub const BASE_SLICE_GAP_MS: i64 = 60_000;

/// The cap on output and review text read per event: result lines are at the end.
pub const TEXT_TAIL_BYTES: i64 = 65_536;

/// One hook event, projected to what derivation reads (spec S2 "Reading a
/// session's events").
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SignalEvent {
    pub id: i64,
    pub event_type: String,
    pub tool_name: Option<String>,
    /// Unix milliseconds when the event fired.
    pub ts: i64,
    pub tool_use_id: Option<String>,
    pub command: Option<String>,
    /// `file_path`, or `notebook_path` for NotebookEdit.
    pub path: Option<String>,
    pub subagent_type: Option<String>,
    /// `update_task_status`'s `state` and `task_id`.
    pub state: Option<String>,
    pub task_id: Option<String>,
    /// A SubagentHandback's text.
    pub message: Option<String>,
    /// `tool_response.type` (a Write's `create` / `update`).
    pub response_type: Option<String>,
    /// The agent an Agent launch started (`tool_response.agentId`), or the one a
    /// handback / SubagentStop is from (`agent_id`).
    pub agent_ref: Option<String>,
    /// For a path ending `tasks.md` only.
    pub old_string: Option<String>,
    pub new_string: Option<String>,
    pub content: Option<String>,
    pub original_file: Option<String>,
    /// stdout + stderr (or the failure's error, or an MCP tool's response), its last 64 KiB.
    pub output: Option<String>,
    pub notification_type: Option<String>,
    pub is_interrupt: Option<bool>,
    pub last_assistant_message: Option<String>,
}

/// A row of `activity.session_signals`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalKind {
    Slice,
    Prompt,
    CodeEdit,
    BuildOk,
    BuildFail,
    TestOk,
    TestFail,
    RunUnknown,
    UnclassifiedFail,
    UnmatchedCall,
    PrCreated,
    Pushed,
    TaskTick,
    Review,
    ReviewFix,
}

impl SignalKind {
    /// The `sensei.session_signal_kind` label.
    pub fn as_str(self) -> &'static str {
        match self {
            SignalKind::Slice => "slice",
            SignalKind::Prompt => "prompt",
            SignalKind::CodeEdit => "code_edit",
            SignalKind::BuildOk => "build_ok",
            SignalKind::BuildFail => "build_fail",
            SignalKind::TestOk => "test_ok",
            SignalKind::TestFail => "test_fail",
            SignalKind::RunUnknown => "run_unknown",
            SignalKind::UnclassifiedFail => "unclassified_fail",
            SignalKind::UnmatchedCall => "unmatched_call",
            SignalKind::PrCreated => "pr_created",
            SignalKind::Pushed => "pushed",
            SignalKind::TaskTick => "task_tick",
            SignalKind::Review => "review",
            SignalKind::ReviewFix => "review_fix",
        }
    }

    pub fn parse(s: &str) -> Option<SignalKind> {
        use SignalKind::*;
        [
            Slice,
            Prompt,
            CodeEdit,
            BuildOk,
            BuildFail,
            TestOk,
            TestFail,
            RunUnknown,
            UnclassifiedFail,
            UnmatchedCall,
            PrCreated,
            Pushed,
            TaskTick,
            Review,
            ReviewFix,
        ]
        .into_iter()
        .find(|k| k.as_str() == s)
    }
}

/// Why a slice ended: the `sensei.hold_reason` label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldReason {
    BuildFailure,
    TestFailure,
    PermissionWait,
    Interrupted,
    SessionEnded,
    Unknown,
}

impl HoldReason {
    pub fn as_str(self) -> &'static str {
        match self {
            HoldReason::BuildFailure => "build_failure",
            HoldReason::TestFailure => "test_failure",
            HoldReason::PermissionWait => "permission_wait",
            HoldReason::Interrupted => "interrupted",
            HoldReason::SessionEnded => "session_ended",
            HoldReason::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> Option<HoldReason> {
        use HoldReason::*;
        [BuildFailure, TestFailure, PermissionWait, Interrupted, SessionEnded, Unknown]
            .into_iter()
            .find(|r| r.as_str() == s)
    }
}

/// One derived signal. Times are Unix milliseconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signal {
    pub kind: SignalKind,
    pub at: i64,
    pub until: Option<i64>,
    pub category: Option<Category>,
    pub reason: Option<HoldReason>,
    pub reference: Option<String>,
}

/// The session's signals, in order, or why it can't be derived (an event whose
/// `ts` is outside `[2000-01-01, now + 1 day]`).
pub fn derive_signals(
    events: &[SignalEvent],
    catalogue: &Catalogue,
    folder_commands: &[FolderCommand],
    now_ms: i64,
) -> Result<Vec<Signal>, String> {
    const Y2000_MS: i64 = 946_684_800_000;
    if let Some(e) = events.iter().find(|e| e.ts < Y2000_MS || e.ts > now_ms + 86_400_000) {
        return Err(format!("event {} has ts {} outside [2000-01-01, now + 1 day]", e.id, e.ts));
    }

    let answered: std::collections::HashSet<&str> = events
        .iter()
        .filter(|e| is_result(e) && e.tool_name.as_deref() == Some("Bash"))
        .filter_map(|e| e.tool_use_id.as_deref())
        .collect();
    let reviews = review_texts(events);

    let mut out: Vec<Signal> = Vec::new();
    let mut failed_runs: std::collections::HashMap<i64, (bool, bool)> = Default::default();
    // Every review stays open for the session (A4): parallel reviewers hand back
    // minutes apart, and a fix for one must not be lost to the other's arrival.
    let mut open_reviews: Vec<(Vec<String>, std::collections::HashSet<String>)> = Vec::new();
    let push = |out: &mut Vec<Signal>, kind, at, category, reference| {
        out.push(Signal { kind, at, until: None, category, reason: None, reference });
    };

    for e in events {
        let tool = e.tool_name.as_deref().unwrap_or("");
        if let Some((name, text)) = reviews.get(&e.id) {
            push(&mut out, SignalKind::Review, e.ts, None, Some(name.clone()));
            open_reviews.push((path_tokens(text), Default::default()));
        }
        match (e.event_type.as_str(), tool) {
            ("UserPromptSubmit", _) => push(&mut out, SignalKind::Prompt, e.ts, None, None),
            ("PostToolUse" | "PostToolUseFailure", "Bash") => {
                let command = e.command.as_deref().unwrap_or("");
                let exit = if e.event_type == "PostToolUse" { Exit::Ok } else { Exit::Fail };
                let output = e.output.as_deref().unwrap_or("");
                for run in classify(command, output, exit, catalogue, folder_commands) {
                    let (kind, category) = match run {
                        RunSignal::Ok(c) if c.is_build_family() => (SignalKind::BuildOk, Some(c)),
                        RunSignal::Ok(c) => (SignalKind::TestOk, Some(c)),
                        RunSignal::Fail(c) => {
                            let f = failed_runs.entry(e.id).or_default();
                            if c.is_build_family() {
                                f.0 = true
                            } else {
                                f.1 = true
                            }
                            let kind = if c.is_build_family() {
                                SignalKind::BuildFail
                            } else {
                                SignalKind::TestFail
                            };
                            (kind, Some(c))
                        }
                        RunSignal::Unknown(c) => (SignalKind::RunUnknown, Some(c)),
                        RunSignal::UnclassifiedFail => (SignalKind::UnclassifiedFail, None),
                    };
                    push(&mut out, kind, e.ts, category, None);
                }
                if exit == Exit::Ok {
                    if command.contains("gh pr create") || command.contains("glab mr create") {
                        push(&mut out, SignalKind::PrCreated, e.ts, None, None);
                    }
                    if command.contains("git push") {
                        push(&mut out, SignalKind::Pushed, e.ts, None, None);
                    }
                }
            }
            ("PreToolUse", "Bash") => {
                let Some(id) = e.tool_use_id.as_deref() else { continue };
                if answered.contains(id) {
                    continue;
                }
                let cats =
                    run_categories(e.command.as_deref().unwrap_or(""), catalogue, folder_commands);
                if cats.is_empty() {
                    push(&mut out, SignalKind::UnmatchedCall, e.ts, None, None);
                }
                for c in cats {
                    push(&mut out, SignalKind::UnmatchedCall, e.ts, Some(c), None);
                }
            }
            ("PostToolUse", "Edit" | "MultiEdit" | "Write" | "NotebookEdit") => {
                let Some(path) = e.path.as_deref() else { continue };
                for task in ticked(e, path) {
                    push(
                        &mut out,
                        SignalKind::TaskTick,
                        e.ts,
                        None,
                        Some(format!("{path}: {task}")),
                    );
                }
                if !is_code(path) {
                    continue;
                }
                push(&mut out, SignalKind::CodeEdit, e.ts, None, None);
                // One edit is one fix, credited to the earliest open review that
                // named the file and has not been credited for it yet.
                let credited = open_reviews.iter_mut().find_map(|(tokens, fixed)| {
                    let token = tokens.iter().find(|t| names(path, t))?;
                    fixed.insert(path.to_string()).then(|| token.clone())
                });
                if let Some(token) = credited {
                    push(&mut out, SignalKind::ReviewFix, e.ts, None, Some(token));
                }
            }
            ("PostToolUse", "mcp__plugin_sensei_sensei__update_task_status")
                if e.state.as_deref() == Some("done") =>
            {
                let task = e.task_id.as_deref().unwrap_or("");
                push(&mut out, SignalKind::TaskTick, e.ts, None, Some(format!("plan:{task}")));
            }
            _ => {}
        }
    }

    for s in base_slices(events, &failed_runs) {
        out.push(s);
    }
    out.sort_by_key(|s| s.at);
    Ok(out)
}

/// A tool call's result event.
fn is_result(e: &SignalEvent) -> bool {
    matches!(e.event_type.as_str(), "PostToolUse" | "PostToolUseFailure")
}

const REVIEWER_WORDS: &[&str] = &["review", "auditor", "verifier", "hunter"];
const REVIEW_CLIS: &[&str] = &["codex", "gemini", "aider", "cursor-agent", "opencode", "claude"];

/// Each review, keyed by the event at which its text arrived: the reviewer's name
/// and its text (spec S2 `review`).
fn review_texts(events: &[SignalEvent]) -> std::collections::HashMap<i64, (String, String)> {
    let mut out = std::collections::HashMap::new();
    let mut launches: Vec<(&str, String)> = Vec::new();
    for e in events.iter().filter(|e| is_result(e)) {
        let tool = e.tool_name.as_deref().unwrap_or("");
        if tool == "Agent"
            && let (Some(agent), Some(kind)) = (e.agent_ref.as_deref(), e.subagent_type.as_deref())
        {
            let name = kind.rsplit(':').next().unwrap_or(kind);
            if REVIEWER_WORDS.iter().any(|w| name.to_ascii_lowercase().contains(w)) {
                launches.push((agent, name.to_string()));
            }
        } else if tool == "Bash" {
            let command = e.command.as_deref().unwrap_or("");
            let reviewer = stage_commands(command)
                .iter()
                .filter_map(|c| c.split_whitespace().next().map(str::to_string))
                .find(|w| REVIEW_CLIS.contains(&w.as_str()));
            if let Some(cli) = reviewer {
                out.insert(e.id, (cli, e.output.clone().unwrap_or_default()));
            }
        } else if tool.starts_with("mcp__") && tool.ends_with("__consensus") {
            out.insert(e.id, ("consensus".to_string(), e.output.clone().unwrap_or_default()));
        }
    }
    for (agent, name) in launches {
        let from = |kind: &str, tool: Option<&str>| {
            events.iter().find(|e| {
                e.event_type == kind
                    && e.tool_name.as_deref() == tool
                    && e.agent_ref.as_deref() == Some(agent)
            })
        };
        let text = from("PostToolUse", Some("SubagentHandback"))
            .and_then(|e| e.message.clone().map(|m| (e.id, m)))
            .or_else(|| {
                from("SubagentStop", None)
                    .and_then(|e| e.last_assistant_message.clone().map(|m| (e.id, m)))
            });
        if let Some((id, text)) = text {
            out.insert(id, (name, text));
        }
    }
    out
}

/// A code file (spec S2, A4): not a documentation type, and not under a spec or
/// agent folder. A route under `docs/` or an RSpec file under `spec/` is code.
fn is_code(path: &str) -> bool {
    const DOC_TYPES: &[&str] = &["md", "mdx", "rst", "adoc", "txt"];
    const NOT_CODE_DIRS: &[&str] = &["openspec", ".sensei", ".claude"];
    let ext = path
        .rsplit('/')
        .next()
        .and_then(|f| f.rsplit_once('.'))
        .map(|(_, x)| x.to_ascii_lowercase());
    !ext.is_some_and(|x| DOC_TYPES.contains(&x.as_str()))
        && !path.split('/').any(|seg| NOT_CODE_DIRS.contains(&seg))
}

/// The tasks an edit of a `tasks.md` ticked: unchecked before, checked after.
fn ticked(e: &SignalEvent, path: &str) -> Vec<String> {
    if !path.ends_with("tasks.md") {
        return Vec::new();
    }
    let (before, after) = match e.tool_name.as_deref() {
        Some("Edit") => (e.old_string.as_deref(), e.new_string.as_deref()),
        Some("Write") if e.response_type.as_deref() == Some("update") => {
            (e.original_file.as_deref(), e.content.as_deref())
        }
        _ => return Vec::new(),
    };
    let (Some(before), Some(after)) = (before, after) else { return Vec::new() };
    let unchecked: std::collections::HashSet<&str> =
        before.lines().filter_map(|l| task_text(l, false)).collect();
    after
        .lines()
        .filter_map(|l| task_text(l, true))
        .filter(|t| unchecked.contains(t))
        .map(str::to_string)
        .collect()
}

/// A Markdown task line's text, when its box is checked (`checked`) or not.
fn task_text(line: &str, checked: bool) -> Option<&str> {
    let l = line.trim_start();
    let l = l.strip_prefix("- ").or_else(|| l.strip_prefix("* "))?;
    let rest = if checked {
        l.strip_prefix("[x] ").or_else(|| l.strip_prefix("[X] "))?
    } else {
        l.strip_prefix("[ ] ")?
    };
    Some(rest.trim())
}

/// File paths a review's text names (spec S2 `review_fix`): words containing `/`,
/// with quotes, trailing punctuation and `:line` suffixes stripped and brace
/// lists expanded, that end in a file extension.
fn path_tokens(text: &str) -> Vec<String> {
    let line_suffix =
        regex::Regex::new(r":\d+(?:[-:]\d+)?(?:,\d+(?:-\d+)?)*$").expect("static regex");
    let mut out = Vec::new();
    for word in text.split_whitespace().filter(|w| w.contains('/')) {
        let mut w = word.trim_start_matches(['"', '\'', '`', '(', '[', '{']);
        loop {
            let t = w.trim_end_matches(['.', ',', ';', ':', ')', ']', '"', '\'', '`']);
            let t = if t.ends_with('}') && !t.contains('{') { &t[..t.len() - 1] } else { t };
            if t == w {
                break;
            }
            w = t;
        }
        let w = line_suffix.replace(w, "");
        for t in expand_braces(&w) {
            let ext = t.rsplit('/').next().and_then(|f| f.rsplit_once('.')).map(|(_, x)| x);
            if ext.is_some_and(|x| !x.is_empty() && x.chars().all(|c| c.is_ascii_alphanumeric())) {
                out.push(t);
            }
        }
    }
    out
}

/// `a/{b,c}.rs` → `a/b.rs`, `a/c.rs` (one brace group).
fn expand_braces(w: &str) -> Vec<String> {
    match (w.find('{'), w.find('}')) {
        (Some(a), Some(b)) if a < b => w[a + 1..b]
            .split(',')
            .map(|part| format!("{}{}{}", &w[..a], part, &w[b + 1..]))
            .collect(),
        _ => vec![w.to_string()],
    }
}

/// The edited `path` is named by `token`: a token of ≥ 2 segments that is a
/// segment-aligned suffix of it. A bare basename never names a file.
fn names(path: &str, token: &str) -> bool {
    let token = token.trim_start_matches("./");
    token.contains('/') && (path == token || path.ends_with(&format!("/{token}")))
}

/// Base slices, each with the reason it ended (spec S2 "Slice reason").
fn base_slices(
    events: &[SignalEvent],
    failed_runs: &std::collections::HashMap<i64, (bool, bool)>,
) -> Vec<Signal> {
    let mut out = Vec::new();
    let mut start = 0;
    for i in 1..=events.len() {
        if i < events.len() && events[i].ts - events[i - 1].ts <= BASE_SLICE_GAP_MS {
            continue;
        }
        let slice = &events[start..i];
        if let (Some(first), Some(last)) = (slice.first(), slice.last()) {
            out.push(Signal {
                kind: SignalKind::Slice,
                at: first.ts,
                until: Some(last.ts),
                category: None,
                reason: slice_reason(slice, failed_runs),
                reference: None,
            });
        }
        start = i;
    }
    out
}

fn slice_reason(
    slice: &[SignalEvent],
    failed_runs: &std::collections::HashMap<i64, (bool, bool)>,
) -> Option<HoldReason> {
    if slice.last().is_some_and(|e| e.event_type == "SessionEnd") {
        return Some(HoldReason::SessionEnded);
    }
    const TAIL: &[&str] = &["Stop", "SubagentStop", "PreCompact", "UserPromptSubmit"];
    // A slice of only the turn's tail has no reason of its own: a merged slice
    // inherits its last earlier base slice's (A4).
    let e = slice.iter().rev().find(|e| !TAIL.contains(&e.event_type.as_str()))?;
    if e.event_type == "PostToolUseFailure" && e.is_interrupt == Some(true) {
        return Some(HoldReason::Interrupted);
    }
    match failed_runs.get(&e.id) {
        Some((true, _)) => return Some(HoldReason::BuildFailure),
        Some((_, true)) => return Some(HoldReason::TestFailure),
        _ => {}
    }
    if e.event_type == "Notification" && e.notification_type.as_deref() == Some("permission_prompt")
    {
        return Some(HoldReason::PermissionWait);
    }
    Some(HoldReason::Unknown)
}

#[cfg(test)]
mod tests {
    use super::SignalKind::{
        BuildFail, BuildOk, CodeEdit, PrCreated, Pushed, Review, ReviewFix, RunUnknown, Slice,
        TaskTick, TestFail, TestOk, UnclassifiedFail, UnmatchedCall,
    };
    use super::*;

    const MIN: i64 = 60_000;
    const T0: i64 = 1_790_000_000_000; // 2026-09
    const NOW: i64 = T0 + 1_000 * MIN;

    fn catalogue() -> Catalogue {
        crate::sdlc::classify::tests_support::seed()
    }

    fn ev(id: i64, minute: i64, event_type: &str, tool: Option<&str>) -> SignalEvent {
        SignalEvent {
            id,
            event_type: event_type.into(),
            tool_name: tool.map(str::to_string),
            ts: T0 + minute * MIN,
            ..Default::default()
        }
    }
    fn edit(id: i64, minute: i64, path: &str) -> SignalEvent {
        SignalEvent { path: Some(path.into()), ..ev(id, minute, "PostToolUse", Some("Edit")) }
    }
    fn bash(id: i64, minute: i64, ok: bool, cmd: &str, out: &str) -> SignalEvent {
        let kind = if ok { "PostToolUse" } else { "PostToolUseFailure" };
        SignalEvent {
            command: Some(cmd.into()),
            output: Some(out.into()),
            tool_use_id: Some(format!("tu{id}")),
            ..ev(id, minute, kind, Some("Bash"))
        }
    }
    fn kinds(signals: &[Signal]) -> Vec<(SignalKind, i64)> {
        signals.iter().filter(|s| s.kind != Slice).map(|s| (s.kind, (s.at - T0) / MIN)).collect()
    }
    fn slices(signals: &[Signal]) -> Vec<(i64, i64, HoldReason)> {
        signals
            .iter()
            .filter(|s| s.kind == Slice)
            .map(|s| {
                (
                    (s.at - T0) / MIN,
                    (s.until.unwrap() - T0) / MIN,
                    s.reason.unwrap_or(HoldReason::Unknown),
                )
            })
            .collect()
    }
    fn derive(events: &[SignalEvent]) -> Vec<Signal> {
        derive_signals(events, &catalogue(), &[], NOW).expect("derives")
    }

    /// Spec gate 2, fixture A.
    #[test]
    fn fixture_a() {
        let events = vec![
            edit(1, 0, "/r/src/a.rs"),
            bash(2, 1, false, "cargo build", ""),
            ev(3, 2, "Stop", None),
            edit(4, 30, "/r/src/a.rs"),
            bash(5, 31, true, "cargo build", ""),
            bash(6, 32, true, "gh pr create --fill", ""),
            edit(7, 50, "/r/src/a.rs"),
            bash(8, 52, true, "git push", ""),
            ev(9, 53, "SessionEnd", None),
        ];
        let s = derive(&events);
        assert_eq!(
            kinds(&s),
            [
                (CodeEdit, 0),
                (BuildFail, 1),
                (CodeEdit, 30),
                (BuildOk, 31),
                (PrCreated, 32),
                (CodeEdit, 50),
                (Pushed, 52)
            ]
        );
        assert_eq!(
            slices(&s),
            [
                (0, 2, HoldReason::BuildFailure),
                (30, 32, HoldReason::Unknown),
                (50, 50, HoldReason::Unknown),
                (52, 53, HoldReason::SessionEnded)
            ]
        );
        assert!(s.windows(2).all(|w| w[0].at <= w[1].at), "in time order");
    }

    #[test]
    fn a_ts_outside_the_sane_range_fails_naming_it() {
        let err = derive_signals(
            &[SignalEvent { ts: 9_000_000_000_000_000_000, ..ev(1, 0, "Stop", None) }],
            &catalogue(),
            &[],
            NOW,
        )
        .unwrap_err();
        assert!(err.contains("9000000000000000000"), "{err}");
        assert!(
            derive_signals(
                &[SignalEvent { ts: 1_000, ..ev(1, 0, "Stop", None) }],
                &catalogue(),
                &[],
                NOW
            )
            .is_err()
        );
    }

    #[test]
    fn a_call_that_never_reported_is_unmatched() {
        let pre = SignalEvent {
            command: Some("cargo build".into()),
            tool_use_id: Some("lost".into()),
            ..ev(1, 0, "PreToolUse", Some("Bash"))
        };
        let pre_ls = SignalEvent {
            command: Some("ls".into()),
            tool_use_id: Some("lost2".into()),
            ..ev(2, 0, "PreToolUse", Some("Bash"))
        };
        let answered = SignalEvent {
            command: Some("cargo test".into()),
            tool_use_id: Some("tu3".into()),
            ..ev(3, 0, "PreToolUse", Some("Bash"))
        };
        let s = derive(&[pre, pre_ls, answered, bash(3, 1, true, "cargo test", "")]);
        let unmatched: Vec<_> =
            s.iter().filter(|s| s.kind == UnmatchedCall).map(|s| s.category).collect();
        assert_eq!(unmatched, [Some(Category::Build), None]);
    }

    #[test]
    fn only_code_edits_count() {
        // A4: documentation is a file type, not a folder name — a Svelte route under
        // `docs/` and an RSpec file under `spec/` are code.
        let paths = [
            ("/r/src/a.rs", true),
            ("/r/docs/a.rs", true),
            ("/r/website/src/routes/sensei/docs/+page.svelte", true),
            ("/r/spec/models/user_spec.rb", true),
            ("/r/src/README.md", false),
            ("/r/docs/guide.mdx", false),
            ("/r/notes/x.rst", false),
            ("/r/openspec/changes/x/design.ts", false),
            ("/r/.claude/hooks/x.ts", false),
            ("/r/.sensei/rules.md", false),
        ];
        let mut events: Vec<SignalEvent> =
            paths.iter().enumerate().map(|(i, (p, _))| edit(i as i64 + 1, i as i64, p)).collect();
        events.push(SignalEvent {
            path: Some("/r/nb/a.ipynb".into()),
            ..ev(20, 20, "PostToolUse", Some("NotebookEdit"))
        });
        events.push(SignalEvent {
            path: Some("/r/src/m.rs".into()),
            ..ev(21, 21, "PostToolUse", Some("MultiEdit"))
        });
        events.push(SignalEvent {
            path: Some("/r/src/c.rs".into()),
            ..ev(22, 22, "PreToolUse", Some("Edit"))
        });
        let s = derive(&events);
        let got: Vec<i64> =
            s.iter().filter(|s| s.kind == CodeEdit).map(|s| (s.at - T0) / MIN).collect();
        let mut want: Vec<i64> = paths
            .iter()
            .enumerate()
            .filter(|(_, (_, code))| *code)
            .map(|(i, _)| i as i64)
            .collect();
        want.extend([20, 21]);
        assert_eq!(got, want);
    }

    #[test]
    fn prs_and_pushes_count_only_when_they_succeeded() {
        let s = derive(&[
            bash(1, 0, false, "gh pr create", "error"),
            bash(2, 0, false, "git push", "rejected"),
            bash(3, 1, true, "glab mr create -f", ""),
            bash(4, 1, true, "git push origin x", ""),
        ]);
        assert_eq!(
            kinds(&s),
            [(UnclassifiedFail, 0), (UnclassifiedFail, 0), (PrCreated, 1), (Pushed, 1)]
        );
    }

    #[test]
    fn task_ticks_from_edits_writes_and_the_plan_graph() {
        let tick_edit = SignalEvent {
            path: Some("/r/openspec/changes/x/tasks.md".into()),
            old_string: Some("- [ ] 1.1 add table\n- [ ] 1.2 add index\n- [x] 1.0 done".into()),
            new_string: Some("- [x] 1.1 add table\n- [ ] 1.2 add index\n- [x] 1.0 done".into()),
            ..ev(1, 0, "PostToolUse", Some("Edit"))
        };
        let tick_write = SignalEvent {
            path: Some("/r/openspec/changes/x/tasks.md".into()),
            response_type: Some("update".into()),
            original_file: Some("- [ ] a\n- [ ] b\n".into()),
            content: Some("- [x] a\n- [X] b\n".into()),
            ..ev(2, 1, "PostToolUse", Some("Write"))
        };
        let new_file = SignalEvent {
            path: Some("/r/openspec/changes/y/tasks.md".into()),
            response_type: Some("create".into()),
            content: Some("- [x] a\n".into()),
            ..ev(3, 2, "PostToolUse", Some("Write"))
        };
        let plan = SignalEvent {
            state: Some("done".into()),
            task_id: Some("t9".into()),
            ..ev(4, 3, "PostToolUse", Some("mcp__plugin_sensei_sensei__update_task_status"))
        };
        let plan_active = SignalEvent {
            state: Some("active".into()),
            task_id: Some("t9".into()),
            ..ev(5, 3, "PostToolUse", Some("mcp__plugin_sensei_sensei__update_task_status"))
        };
        let s = derive(&[tick_edit, tick_write, new_file, plan, plan_active]);
        let ticks: Vec<_> =
            s.iter().filter(|s| s.kind == TaskTick).map(|s| s.reference.clone().unwrap()).collect();
        assert_eq!(
            ticks,
            [
                "/r/openspec/changes/x/tasks.md: 1.1 add table",
                "/r/openspec/changes/x/tasks.md: a",
                "/r/openspec/changes/x/tasks.md: b",
                "plan:t9"
            ]
        );
    }

    /// Spec gate 2b: an Agent review is the handback's text, at the handback's time.
    #[test]
    fn an_agent_review_starts_when_its_text_arrives() {
        let launch = SignalEvent {
            subagent_type: Some("sensei:sensei-security-reviewer".into()),
            agent_ref: Some("ag1".into()),
            ..ev(1, 0, "PostToolUse", Some("Agent"))
        };
        let handback = SignalEvent {
            message: Some("Finding: `crates/cli/src/lib.rs:40` leaks the token.".into()),
            agent_ref: Some("ag1".into()),
            ..ev(3, 2, "PostToolUse", Some("SubagentHandback"))
        };
        let s = derive(&[
            launch,
            edit(2, 1, "/r/crates/cli/src/lib.rs"),
            handback,
            edit(4, 3, "/r/crates/senseid/src/lib.rs"),
            edit(5, 3, "/r/crates/cli/src/lib.rs"),
            edit(6, 4, "/r/crates/cli/src/lib.rs"),
        ]);
        let got: Vec<_> = s
            .iter()
            .filter(|s| matches!(s.kind, Review | ReviewFix))
            .map(|s| (s.kind, (s.at - T0) / MIN, s.reference.clone().unwrap()))
            .collect();
        assert_eq!(
            got,
            [
                (Review, 2, "sensei-security-reviewer".to_string()),
                (ReviewFix, 3, "crates/cli/src/lib.rs".to_string())
            ]
        );
    }

    #[test]
    fn a_launch_that_never_hands_back_falls_back_to_its_stop_or_is_no_review() {
        let launch = |id, a: &str| SignalEvent {
            subagent_type: Some("code-reviewer".into()),
            agent_ref: Some(a.into()),
            ..ev(id, 0, "PostToolUse", Some("Agent"))
        };
        let stop = SignalEvent {
            agent_ref: Some("ag2".into()),
            last_assistant_message: Some("see src/x/main.rs".into()),
            ..ev(3, 1, "SubagentStop", None)
        };
        let s = derive(&[launch(1, "ag1"), launch(2, "ag2"), stop, edit(4, 2, "/r/src/x/main.rs")]);
        let reviews: Vec<_> =
            s.iter().filter(|s| s.kind == Review).map(|s| (s.at - T0) / MIN).collect();
        assert_eq!(reviews, [1]);
        assert_eq!(s.iter().filter(|s| s.kind == ReviewFix).count(), 1);
        let explorer = SignalEvent { subagent_type: Some("Explore".into()), ..launch(1, "ag9") };
        assert!(derive(&[explorer]).iter().all(|s| s.kind != Review));
    }

    #[test]
    fn cli_and_consensus_reviews_use_their_output() {
        let codex = bash(
            1,
            0,
            true,
            "codex review --uncommitted 2>&1 | tail -40",
            "P1: src/lib/api.ts:12 drops the error",
        );
        let consensus = SignalEvent {
            output: Some("{\"text\":\"flaw in crates/mcp/src/lib.rs\"}".into()),
            ..ev(3, 2, "PostToolUse", Some("mcp__plugin_sensei_sensei__consensus"))
        };
        let s = derive(&[
            codex,
            edit(2, 1, "/r/src/lib/api.ts"),
            consensus,
            edit(4, 3, "/r/crates/mcp/src/lib.rs"),
        ]);
        assert_eq!(s.iter().filter(|s| s.kind == Review).count(), 2);
        assert_eq!(s.iter().filter(|s| s.kind == ReviewFix).count(), 2);
    }

    #[test]
    fn path_tokens_tolerate_punctuation_ranges_and_braces() {
        let launch = SignalEvent {
            subagent_type: Some("reviewer".into()),
            agent_ref: Some("a".into()),
            ..ev(1, 0, "PostToolUse", Some("Agent"))
        };
        let handback = SignalEvent {
            message: Some("See crates/cli/src/main.rs. Also crates/x/src/api/{server.rs,routes.rs}; and `app/src/a.ts:12-40`".into()),
            agent_ref: Some("a".into()),
            ..ev(2, 1, "PostToolUse", Some("SubagentHandback"))
        };
        let s = derive(&[
            launch,
            handback,
            edit(3, 2, "/r/crates/cli/src/main.rs"),
            edit(4, 2, "/r/crates/x/src/api/routes.rs"),
            edit(5, 2, "/r/app/src/a.ts"),
            edit(6, 2, "/r/other/main.rs"),
        ]);
        assert_eq!(s.iter().filter(|s| s.kind == ReviewFix).count(), 3);
    }

    #[test]
    fn slice_reasons() {
        // A permission prompt, an interrupt, and a failed test before the turn's tail.
        let s = derive(&[
            edit(1, 0, "/r/src/a.rs"),
            SignalEvent {
                notification_type: Some("permission_prompt".into()),
                ..ev(2, 0, "Notification", None)
            },
            edit(3, 10, "/r/src/a.rs"),
            SignalEvent {
                is_interrupt: Some(true),
                command: Some("sleep 9".into()),
                ..ev(4, 10, "PostToolUseFailure", Some("Bash"))
            },
            bash(5, 20, true, "cargo test 2>&1 | tail -1", "test result: FAILED. 1 failed"),
            ev(6, 20, "Stop", None),
            ev(7, 20, "UserPromptSubmit", None),
        ]);
        assert_eq!(
            slices(&s),
            [
                (0, 0, HoldReason::PermissionWait),
                (10, 10, HoldReason::Interrupted),
                (20, 20, HoldReason::TestFailure)
            ]
        );
    }

    #[test]
    fn an_empty_session_derives_nothing() {
        assert!(derive(&[]).is_empty());
    }

    // ---- review round (F2 post-implementation swarm, A4) ----

    #[test]
    fn a_tail_only_base_slice_inherits_its_reason() {
        let s = derive(&[
            bash(1, 0, false, "cargo build", ""),
            ev(2, 3, "UserPromptSubmit", None),
            ev(3, 3, "Stop", None),
        ]);
        let reasons: Vec<_> = s.iter().filter(|s| s.kind == Slice).map(|s| s.reason).collect();
        assert_eq!(reasons, [Some(HoldReason::BuildFailure), None]);
    }

    #[test]
    fn every_review_stays_open_for_the_session() {
        let launch = |id, a: &str| SignalEvent {
            subagent_type: Some("sensei:sensei-spec-conformance-auditor".into()),
            agent_ref: Some(a.into()),
            ..ev(id, 0, "PostToolUse", Some("Agent"))
        };
        let back = |id, m, a: &str, text: &str| SignalEvent {
            message: Some(text.into()),
            agent_ref: Some(a.into()),
            ..ev(id, m, "PostToolUse", Some("SubagentHandback"))
        };
        let s = derive(&[
            launch(1, "A"),
            launch(2, "B"),
            back(3, 1, "A", "x/a.rs and z/c.rs are wrong"),
            back(4, 2, "B", "y/b.rs and z/c.rs are wrong"),
            edit(5, 3, "/r/x/a.rs"),
            edit(6, 3, "/r/z/c.rs"),
            edit(7, 4, "/r/x/a.rs"),
        ]);
        let fixes: Vec<_> = s
            .iter()
            .filter(|s| s.kind == ReviewFix)
            .map(|s| s.reference.clone().unwrap())
            .collect();
        assert_eq!(
            fixes,
            ["x/a.rs", "z/c.rs"],
            "a parallel review never closes another's window; one edit counts once"
        );
    }

    #[test]
    fn a_handback_beats_the_stop_and_names_are_kept() {
        let launch = SignalEvent {
            subagent_type: Some("my-hunter".into()),
            agent_ref: Some("h".into()),
            ..ev(1, 0, "PostToolUse", Some("Agent"))
        };
        let stop = SignalEvent {
            agent_ref: Some("h".into()),
            last_assistant_message: Some("stop text names q/stop.rs".into()),
            ..ev(2, 1, "SubagentStop", None)
        };
        let back = SignalEvent {
            message: Some("handback names q/back.rs".into()),
            agent_ref: Some("h".into()),
            ..ev(3, 2, "PostToolUse", Some("SubagentHandback"))
        };
        let codex = bash(4, 3, true, "codex exec review", "");
        let consensus = ev(5, 4, "PostToolUse", Some("mcp__x__consensus"));
        let s = derive(&[
            launch,
            stop,
            back,
            codex,
            consensus,
            edit(6, 5, "/r/q/back.rs"),
            edit(7, 5, "/r/q/stop.rs"),
        ]);
        let reviews: Vec<_> = s
            .iter()
            .filter(|s| s.kind == Review)
            .map(|s| ((s.at - T0) / MIN, s.reference.clone().unwrap()))
            .collect();
        assert_eq!(
            reviews,
            [(2, "my-hunter".to_string()), (3, "codex".to_string()), (4, "consensus".to_string())]
        );
        let fixes: Vec<_> = s
            .iter()
            .filter(|s| s.kind == ReviewFix)
            .map(|s| s.reference.clone().unwrap())
            .collect();
        assert_eq!(fixes, ["q/back.rs"]);
        for word in ["x-auditor", "y-verifier"] {
            let l = SignalEvent {
                subagent_type: Some(word.into()),
                agent_ref: Some("w".into()),
                ..ev(1, 0, "PostToolUse", Some("Agent"))
            };
            let b = SignalEvent {
                message: Some("ok".into()),
                agent_ref: Some("w".into()),
                ..ev(2, 1, "PostToolUse", Some("SubagentHandback"))
            };
            assert_eq!(derive(&[l, b]).iter().filter(|s| s.kind == Review).count(), 1, "{word}");
        }
    }

    #[test]
    fn tokens_names_and_line_lists() {
        assert_eq!(path_tokens("see a/b.rs:1,4-5; and c/d.rs:7:3."), ["a/b.rs", "c/d.rs"]);
        assert!(names("/r/src/x/main.rs", "x/main.rs"));
        assert!(names("/r/src/x/main.rs", "./src/x/main.rs"));
        assert!(!names("/r/src/x/main.rs", "main.rs"), "a bare basename never names a file");
        assert!(!names("/r/src/xx/main.rs", "x/main.rs"), "segment-aligned only");
    }

    #[test]
    fn run_kinds_map_by_family() {
        let s = derive(&[
            bash(1, 0, true, "cargo test 2>&1 | tail -1", "test result: ok. 3 passed"),
            bash(2, 1, true, "npx tsc --noEmit | head", "src/a.ts(1,1): error TS2304: x"),
            bash(3, 2, true, "npx playwright test | tail", "  1 failed\n  2 passed (3s)\n"),
            bash(4, 3, true, "npx vitest run | tail -2", "      Tests  4 passed (4)\n"),
            bash(5, 4, true, "cargo build 2>&1 | tail -1", ""),
        ]);
        let got: Vec<_> =
            s.iter().filter(|s| s.kind != Slice).map(|s| (s.kind, s.category)).collect();
        assert_eq!(
            got,
            [
                (TestOk, Some(Category::Test)),
                (BuildFail, Some(Category::Typecheck)),
                (TestFail, Some(Category::E2e)),
                (TestOk, Some(Category::Test)),
                (RunUnknown, Some(Category::Build))
            ]
        );
    }
}
