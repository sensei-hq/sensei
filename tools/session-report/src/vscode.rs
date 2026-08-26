//! Read a VS Code `workspaceStorage` tree into the same [`Session`] shape.
//!
//! Chat history is a DELTA JOURNAL: `kind:0` is a root snapshot and `kind:1`/`2`
//! set a value at a path given as an ARRAY (`k`), with the value under `v`. The
//! state has to be replayed before anything can be read out of it.
//!
//! That journal is only half the story. VS Code ALSO writes
//! `GitHub.copilot-chat/transcripts/*.jsonl` — the same event stream Copilot CLI
//! writes — for a subset of sessions, and it records strictly more:
//!
//! | | journal | event stream | Copilot CLI | Claude Code |
//! |---|---|---|---|---|
//! | model per turn | `modelId` | ✓ | ✓ | ✓ |
//! | turn latency | request == response stamp | real | ✓ | ✓ |
//! | tool calls | `toolInvocationSerialized` | ✓ | ✓ | ✓ |
//! | tool SUCCESS | not recorded | ✓ | ✓ | ✓ |
//! | tokens | not recorded | not recorded | ✓ | ✓ |
//!
//! So [`collect`] prefers the event stream and falls back to the journal, which
//! is what lets a VS Code report speak to friction at all. Tokens are absent
//! from both, so cost stays absent rather than shown as zero.

use crate::model::{Session, ToolCall, Totals, Turn};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn parse_session(file: &Path) -> Option<(Session, usize)> {
    let text = std::fs::read_to_string(file).ok()?;
    let requests = sensei_transcript_formats::journal::requests(&text);
    if requests.is_empty() {
        return None;
    }

    let id = file.file_stem()?.to_string_lossy().to_string();
    let mut prompts = 0usize;
    let mut turns = Vec::new();
    let mut tools = Vec::new();
    let mut models: HashMap<String, usize> = HashMap::new();
    let mut activity: Vec<i64> = Vec::new();
    let mut languages: HashMap<String, usize> = HashMap::new();
    let (mut git_commits, mut git_pushes) = (0usize, 0usize);
    let mut prompt_ms: Vec<i64> = Vec::new();
    let (mut first, mut last) = (i64::MAX, 0i64);

    for req in &requests {
        let Some(started) = req.timestamp_ms else { continue };
        // `responseTimestamp` equals `timestamp` on some records, so a turn can
        // legitimately measure zero; that is the transcript's precision, not a
        // bug to paper over.
        let ended = req.response_timestamp_ms.unwrap_or(started).max(started);
        first = first.min(started);
        last = last.max(ended);
        activity.push(started);
        activity.push(ended);

        if !req.user_text.is_empty() {
            prompt_ms.push(started);
            prompts += 1;
        }
        if let Some(m) = &req.model {
            *models.entry(m.clone()).or_default() += 1;
        }
        turns.push(Turn {
            id: String::new(),
            started_ms: started,
            ended_ms: Some(ended),
            model: req.model.clone(),
        });

        for call in &req.tool_calls {
            // The journal records no arguments, but it renders each call into
            // prose that embeds the file as a link. That message is the only
            // place a journal-only session names the file.
            for uri in sensei_transcript_formats::paths::file_uris(&call.invocation_message) {
                if let Some(lang) = crate::signals::language_of(&uri) {
                    *languages.entry(lang.to_string()).or_default() += 1;
                }
            }
            // The rendered message truncates the command for display; the
            // terminal payload keeps it whole, so read git actions there.
            if let Some(cmd) = &call.command {
                let (c, u) = crate::signals::git_actions(cmd);
                git_commits += c;
                git_pushes += u;
            }
            tools.push(ToolCall {
                name: call.tool_id.clone(),
                started_ms: started,
                ended_ms: Some(ended),
                // The journal records no outcome. `None` keeps it out of both
                // the success and failure counts rather than inventing one.
                success: None,
                event_id: String::new(),
            });
        }
    }

    if first == i64::MAX {
        return None;
    }
    activity.sort_unstable();

    Some((
        Session {
            id,
            cwd: workspace_folder(file),
            first_ms: first,
            last_ms: last,
            prompts,
            turns,
            tools,
            totals: Totals::default(),
            models,
            permission_events: 0,
            event_count: requests.len(),
            activity_ms: activity,
            delegated: 0,
            delegated_models: HashMap::new(),
            unclosed: false,
            source: Some("journal"),
            languages,
            git_commits,
            git_pushes,
            prompt_ms,
            file: None,
        },
        0,
    ))
}

/// The project for a workspace directory.
fn workspace_folder_of(dir: &Path) -> Option<String> {
    sensei_transcript_formats::paths::workspace_folder(&dir.join("chatSessions").join("x.jsonl"))
}

/// The project a chat belongs to, from the `workspace.json` beside it.
fn workspace_folder(chat_file: &Path) -> Option<String> {
    sensei_transcript_formats::paths::workspace_folder(chat_file)
}

/// Every chat session under a `workspaceStorage` root.
///
/// Two formats coexist. `GitHub.copilot-chat/transcripts/*.jsonl` is the same
/// event stream Copilot CLI writes — `tool.execution_start`/`_complete`,
/// `assistant.turn_start`/`_end` — and it carries tool OUTCOMES and real turn
/// timing. The `chatSessions/*.jsonl` delta journal carries neither.
///
/// Where both exist for a session (37 of 84 on the sample, ids matching exactly)
/// the event stream wins, because it can answer questions the journal cannot.
/// The journal covers the rest rather than dropping them.
pub fn collect(root: &Path) -> (Vec<Session>, usize) {
    let mut sessions: Vec<Session> = Vec::new();
    let mut skipped = 0usize;
    let mut from_events: std::collections::HashSet<String> = std::collections::HashSet::new();
    let ws = if root.join("workspaceStorage").is_dir() {
        root.join("workspaceStorage")
    } else {
        root.to_path_buf()
    };
    let Ok(entries) = std::fs::read_dir(&ws) else {
        return (sessions, skipped);
    };
    let mut dirs: Vec<PathBuf> =
        entries.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    // Pass 1: the richer event streams.
    for dir in &dirs {
        let Ok(files) = std::fs::read_dir(dir.join("GitHub.copilot-chat/transcripts")) else {
            continue;
        };
        for f in files.flatten() {
            let p = f.path();
            if !p.extension().is_some_and(|e| e == "jsonl") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&p) else { continue };
            let Some(id) = p.file_stem().map(|s| s.to_string_lossy().to_string()) else {
                continue;
            };
            let cwd = workspace_folder_of(dir);
            if let Some(mut outcome) = crate::parse::parse_events(&text, id.clone(), cwd, false) {
                skipped += outcome.skipped_lines;
                from_events.insert(id);
                outcome.session.source = Some("events");
                outcome.session.file = Some(p.clone());
                sessions.push(outcome.session);
            }
        }
    }

    // Pass 2: journals, for sessions the event stream does not cover.
    for dir in &dirs {
        let Ok(files) = std::fs::read_dir(dir.join("chatSessions")) else { continue };
        for f in files.flatten() {
            let p = f.path();
            if !p.extension().is_some_and(|e| e == "jsonl") {
                continue;
            }
            let id = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if from_events.contains(&id) {
                continue;
            }
            if let Some((mut s, sk)) = parse_session(&p) {
                skipped += sk;
                s.file = Some(p.clone());
                sessions.push(s);
            }
        }
    }
    sessions.sort_by_key(|s| s.first_ms);
    (sessions, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Write a workspaceStorage tree holding BOTH transcripts for one session id.
    fn fixture(dir: &Path, id: &str) {
        let ws = dir.join("workspaceStorage/aaa");
        std::fs::create_dir_all(ws.join("chatSessions")).unwrap();
        std::fs::create_dir_all(ws.join("GitHub.copilot-chat/transcripts")).unwrap();
        std::fs::write(ws.join("workspace.json"), r#"{"folder":"file:///tmp/proj"}"#).unwrap();

        // Journal: request and response stamped identically, no tool outcome.
        let journal = serde_json::json!({
            "kind": 0,
            "v": {"requests": [{
                "timestamp": 1_000_000_i64,
                "responseTimestamp": 1_000_000_i64,
                "message": {"text": "do the thing"},
                "modelId": "copilot/claude-opus-4.6",
                "requestId": "r1",
                "response": [{"kind": "toolInvocationSerialized", "toolId": "read_file"}]
            }]}
        });
        std::fs::write(ws.join("chatSessions").join(format!("{id}.jsonl")), format!("{journal}\n"))
            .unwrap();

        // Event stream: real turn boundaries and a REPORTED tool failure.
        let events = [
            serde_json::json!({"type":"session.start","timestamp":"2026-08-06T07:15:31.000Z",
                "data":{"sessionId":id}}),
            serde_json::json!({"type":"user.message","timestamp":"2026-08-06T07:15:32.000Z",
                "data":{"content":"do the thing"}}),
            serde_json::json!({"type":"assistant.turn_start","id":"t1",
                "timestamp":"2026-08-06T07:15:32.000Z",
                "data":{"turnId":"t1","model":"claude-opus-4.6"}}),
            serde_json::json!({"type":"tool.execution_start","id":"x1",
                "timestamp":"2026-08-06T07:15:33.000Z",
                "data":{"toolCallId":"x1","toolName":"read_file"}}),
            serde_json::json!({"type":"tool.execution_complete","timestamp":"2026-08-06T07:15:34.000Z",
                "data":{"toolCallId":"x1","success":false}}),
            serde_json::json!({"type":"assistant.turn_end","timestamp":"2026-08-06T07:15:40.000Z",
                "data":{"turnId":"t1"}}),
        ]
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join("\n");
        std::fs::write(
            ws.join("GitHub.copilot-chat/transcripts").join(format!("{id}.jsonl")),
            events,
        )
        .unwrap();
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("session-report-{name}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    // The replay itself — index ceiling, empty-path safety, kind:2 append — is
    // owned and tested by `sensei-transcript-formats::journal`. Re-asserting it
    // here would be a second copy of the thing this crate stopped duplicating.

    /// When a session has both transcripts, the event stream must win — it is the
    /// only one that records whether a tool call succeeded.
    #[test]
    fn prefers_event_stream_over_journal() {
        let d = tmp("prefer-events");
        fixture(&d, "sess-1");
        let (sessions, _) = collect(&d);

        assert_eq!(sessions.len(), 1, "the same session must not be counted twice");
        let s = &sessions[0];
        assert_eq!(s.source, Some("events"));
        // The journal cannot produce this: it records no outcome at all.
        assert_eq!(s.tools.len(), 1);
        assert_eq!(s.tools[0].success, Some(false), "reported failure must survive");
        // Nor this: it stamps request and response identically, so every turn
        // measures zero.
        let turn = &s.turns[0];
        assert!(turn.ended_ms.unwrap() > turn.started_ms, "event stream times its turns");
    }

    /// A session with only a journal is still reported — falling back is what
    /// keeps those sessions in the figures rather than silently dropping them.
    #[test]
    fn falls_back_to_journal_when_no_event_stream() {
        let d = tmp("fallback");
        fixture(&d, "sess-1");
        std::fs::remove_file(
            d.join("workspaceStorage/aaa/GitHub.copilot-chat/transcripts/sess-1.jsonl"),
        )
        .unwrap();

        let (sessions, _) = collect(&d);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].source, Some("journal"));
        assert_eq!(sessions[0].tools.len(), 1);
        assert_eq!(sessions[0].tools[0].success, None, "journal records no outcome");
    }
}
