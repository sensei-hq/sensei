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

/// Replay the journal into the state it describes.
///
/// `kind:0` sets the root, `kind:1` sets a value at a path, and `kind:2` APPENDS
/// to the array at that path — the journal streams a reply in pieces, so a
/// request's `response` grows across many records rather than being rewritten.
/// Treating 2 as a replace keeps only the last fragment.
///
/// Path segments in `k` are strings OR integers (`["requests", 0, "response"]`);
/// on this sample 409 of them are integers, so filtering to strings corrupts
/// nearly every path.
pub(crate) fn replay(text: &str) -> serde_json::Value {
    let mut root = serde_json::json!(null);
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(op) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(v) = op.get("v") else { continue };
        let kind = op["kind"].as_i64().unwrap_or(0);
        if kind == 0 {
            root = v.clone();
            continue;
        }
        let path: Vec<Seg> = op["k"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|p| {
                        p.as_str()
                            .map(|s| Seg::Key(s.to_string()))
                            .or_else(|| p.as_u64().map(|i| Seg::Index(i as usize)))
                    })
                    .collect()
            })
            .unwrap_or_default();
        if path.is_empty() {
            continue;
        }
        apply(&mut root, &path, v.clone(), kind == 2);
    }
    root
}

/// One step of a journal path.
enum Seg {
    Key(String),
    Index(usize),
}

/// Set (or append to) the value at `path`.
fn apply(root: &mut serde_json::Value, path: &[Seg], value: serde_json::Value, append: bool) {
    let mut cur = root;
    for (i, seg) in path.iter().enumerate() {
        let last = i == path.len() - 1;
        match seg {
            Seg::Index(idx) => {
                if !cur.is_array() {
                    *cur = serde_json::json!([]);
                }
                let Some(arr) = cur.as_array_mut() else { return };
                if *idx >= arr.len() {
                    arr.resize(idx + 1, serde_json::Value::Null);
                }
                if last {
                    place(&mut arr[*idx], value, append);
                    return;
                }
                cur = &mut arr[*idx];
            }
            Seg::Key(k) => {
                if !cur.is_object() {
                    *cur = serde_json::json!({});
                }
                let Some(obj) = cur.as_object_mut() else { return };
                if last {
                    let slot = obj.entry(k.clone()).or_insert_with(|| serde_json::json!(null));
                    place(slot, value, append);
                    return;
                }
                cur = obj.entry(k.clone()).or_insert_with(|| serde_json::json!({}));
            }
        }
    }
}

fn place(slot: &mut serde_json::Value, value: serde_json::Value, append: bool) {
    if !append {
        *slot = value;
        return;
    }
    if !slot.is_array() {
        *slot = serde_json::json!([]);
    }
    let Some(arr) = slot.as_array_mut() else { return };
    match value {
        serde_json::Value::Array(items) => arr.extend(items),
        other => arr.push(other),
    }
}

pub fn parse_session(file: &Path) -> Option<(Session, usize)> {
    let text = std::fs::read_to_string(file).ok()?;
    let root = replay(&text);
    let requests = root["requests"].as_array()?;
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

    for req in requests {
        let Some(started) = req["timestamp"].as_i64() else { continue };
        // `responseTimestamp` equals `timestamp` on some records, so a turn can
        // legitimately measure zero; that is the transcript's precision, not a
        // bug to paper over.
        let ended = req["responseTimestamp"].as_i64().unwrap_or(started).max(started);
        first = first.min(started);
        last = last.max(ended);
        activity.push(started);
        activity.push(ended);

        if !req["message"]["text"].as_str().unwrap_or("").trim().is_empty() {
            prompt_ms.push(started);
            prompts += 1;
        }
        // Namespaced as "copilot/claude-opus-4.6".
        let model = req["modelId"].as_str().map(|m| m.rsplit('/').next().unwrap_or(m).to_string());
        if let Some(m) = &model {
            *models.entry(m.clone()).or_default() += 1;
        }
        turns.push(Turn { id: String::new(), started_ms: started, ended_ms: Some(ended), model });

        if let Some(parts) = req["response"].as_array() {
            for part in parts {
                if part["kind"].as_str() != Some("toolInvocationSerialized") {
                    continue;
                }
                // The journal records no arguments, but it renders each call
                // into prose that embeds the file as a link. That message is
                // the only place a journal-only session names the file.
                let msg = part["invocationMessage"]["value"]
                    .as_str()
                    .or_else(|| part["invocationMessage"].as_str())
                    .unwrap_or_default();
                for uri in crate::signals::file_uris(msg) {
                    if let Some(lang) = crate::signals::language_of(&uri) {
                        *languages.entry(lang.to_string()).or_default() += 1;
                    }
                }
                // `invocationMessage` truncates the command for display; the
                // terminal payload keeps it whole, so read git actions there.
                if let Some(cmd) = part["toolSpecificData"]["commandLine"]["original"].as_str() {
                    let (c, u) = crate::signals::git_actions(cmd);
                    git_commits += c;
                    git_pushes += u;
                }
                tools.push(ToolCall {
                    name: part["toolId"].as_str().unwrap_or("<unknown>").to_string(),
                    started_ms: started,
                    ended_ms: Some(ended),
                    // VS Code records no outcome. `None` keeps it out of both the
                    // success and failure counts rather than inventing one.
                    success: None,
                    event_id: req["requestId"].as_str().unwrap_or_default().to_string(),
                });
            }
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
    workspace_folder(&dir.join("chatSessions").join("x.jsonl"))
}

/// The project a chat belongs to, from the `workspace.json` beside it.
///
/// Windows folders are stored percent-encoded (`file:///c%3A/...`); decoding is
/// shared with the journal's file links via [`crate::signals::percent_decode`].
fn workspace_folder(chat_file: &Path) -> Option<String> {
    let ws = chat_file.parent()?.parent()?.join("workspace.json");
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(ws).ok()?).ok()?;
    let uri = v["folder"].as_str()?;
    let path = uri.strip_prefix("file://").unwrap_or(uri);
    let out = crate::signals::percent_decode(path);
    let trimmed = out.strip_prefix('/').unwrap_or(&out).to_string();
    Some(if trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':' { trimmed } else { out })
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
