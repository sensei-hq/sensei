//! Read a VS Code `workspaceStorage` tree into the same [`Session`] shape.
//!
//! Chat history is a DELTA JOURNAL: `kind:0` is a root snapshot and `kind:1`/`2`
//! set a value at a path given as an ARRAY (`k`), with the value under `v`. The
//! state has to be replayed before anything can be read out of it.
//!
//! What this transcript does and does not carry, against the others:
//!
//! | | VS Code | Copilot CLI | Claude Code |
//! |---|---|---|---|
//! | model per turn | `modelId` | ✓ | ✓ |
//! | turn latency | `timestamp` → `responseTimestamp` | ✓ | ✓ |
//! | tool calls | `toolInvocationSerialized` parts | ✓ | ✓ |
//! | tool SUCCESS | not recorded | ✓ | ✓ |
//! | tokens | not recorded | ✓ | ✓ |
//!
//! So a VS Code report can speak to pace and model mix but not to friction or
//! cost. Those are left absent rather than shown as zero.

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
fn replay(text: &str) -> serde_json::Value {
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
        },
        0,
    ))
}

/// The project a chat belongs to, from the `workspace.json` beside it.
///
/// Windows folders are stored percent-encoded (`file:///c%3A/...`); left as-is
/// the path matches nothing.
fn workspace_folder(chat_file: &Path) -> Option<String> {
    let ws = chat_file.parent()?.parent()?.join("workspace.json");
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(ws).ok()?).ok()?;
    let uri = v["folder"].as_str()?;
    let path = uri.strip_prefix("file://").unwrap_or(uri);
    let mut out = String::with_capacity(path.len());
    let raw = path.as_bytes();
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'%'
            && i + 2 < raw.len()
            && let (Some(h), Some(l)) =
                ((raw[i + 1] as char).to_digit(16), (raw[i + 2] as char).to_digit(16))
        {
            out.push(((h * 16 + l) as u8) as char);
            i += 3;
            continue;
        }
        out.push(raw[i] as char);
        i += 1;
    }
    let trimmed = out.strip_prefix('/').unwrap_or(&out).to_string();
    Some(if trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':' { trimmed } else { out })
}

/// Every chat journal under a `workspaceStorage` root.
pub fn collect(root: &Path) -> (Vec<Session>, usize) {
    let mut sessions = Vec::new();
    let mut skipped = 0usize;
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
    for dir in dirs {
        let Ok(files) = std::fs::read_dir(dir.join("chatSessions")) else { continue };
        for f in files.flatten() {
            let p = f.path();
            if p.extension().is_some_and(|e| e == "jsonl")
                && let Some((s, sk)) = parse_session(&p)
            {
                skipped += sk;
                sessions.push(s);
            }
        }
    }
    sessions.sort_by_key(|s| s.first_ms);
    (sessions, skipped)
}
