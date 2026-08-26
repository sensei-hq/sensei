//! VS Code's chat **delta journal** (`chatSessions/<id>.jsonl`).
//!
//! Not a stream of self-contained records: `kind:0` is a root snapshot, `kind:1`
//! sets a value at a path, and `kind:2` APPENDS to the array at that path. The
//! state has to be replayed before anything can be read out of it.
//!
//! Everything here treats its input as UNTRUSTED. The daemon reads whatever
//! journal is on disk, and the offline report tool reads other people's files by
//! definition, so a malformed record must degrade to "skip this record" and
//! never to an unbounded allocation or a wiped state.

/// Ceiling on a journal array index.
///
/// `k` comes from the transcript. Resizing an array to an index taken straight
/// from it lets a two-line file request an allocation of arbitrary size: index
/// 4e9 asks for ~128 GB and the process is OOM-killed. Real sessions have
/// hundreds of requests, not millions.
pub const MAX_INDEX: usize = 1_000_000;

/// Ceiling on one journal line, so a single pathological record cannot be read
/// into memory whole.
pub const MAX_LINE_BYTES: usize = 4 * 1024 * 1024;

/// One exchange: what the human typed and what came back.
///
/// This is the normalised shape both consumers build their own types from. It
/// exists so the timestamp, the model and the tool calls are read out of the
/// journal in exactly one place — the daemon and the report tool previously
/// disagreed about all three.
#[derive(Debug, Clone, Default)]
pub struct Request {
    pub index: usize,
    /// Epoch millis, from the request itself. `None` only when the journal
    /// genuinely omits it — never synthesised.
    pub timestamp_ms: Option<i64>,
    pub response_timestamp_ms: Option<i64>,
    pub user_text: String,
    pub assistant_text: String,
    /// `modelId` with its `copilot/` prefix stripped.
    pub model: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub thinking_ms: i64,
}

/// One tool invocation, as the journal renders it.
#[derive(Debug, Clone, Default)]
pub struct ToolCall {
    pub tool_id: String,
    /// The rendered prose for the call. The journal records no arguments, so
    /// this is where a file link appears.
    pub invocation_message: String,
    /// The full shell command, when the call was a terminal run. The rendered
    /// message truncates it for display; this does not.
    pub command: Option<String>,
}

/// Replay the delta operations into the state they describe.
pub fn replay(content: &str) -> serde_json::Value {
    let mut root = serde_json::json!(null);
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.len() > MAX_LINE_BYTES {
            continue;
        }
        let Ok(op) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            continue;
        };
        let Some(value) = op.get("v") else { continue };

        // A record with a value but no `kind` is malformed. Defaulting it to 0
        // makes it a ROOT SNAPSHOT — the most destructive branch — so one such
        // record discards everything replayed so far. Skip instead.
        let Some(kind) = op.get("kind").and_then(|k| k.as_i64()) else {
            continue;
        };
        if kind == 0 {
            root = value.clone();
            continue;
        }

        // Segments are strings OR integers (`["requests", 0, "response"]`);
        // most of them are integers, so keeping only the strings corrupts
        // nearly every path.
        let path: Vec<Seg> = op
            .get("k")
            .and_then(|k| k.as_array())
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

        // An empty path means `k` was missing or not an array. Skipping is
        // explicit here, though `set` below is ALSO a no-op on an empty path —
        // it simply never enters its loop. Both matter: the daemon's version had
        // an `if path.is_empty() { *root = value; }` branch, so one malformed
        // record replaced the whole reconstructed state and a session that had
        // replayed 400 good records yielded zero turns.
        if path.is_empty() {
            continue;
        }
        set(&mut root, &path, value.clone(), kind == 2);
    }
    root
}

/// One step of a journal path.
enum Seg {
    Key(String),
    Index(usize),
}

/// Write a value at a path.
///
/// An empty path is a NO-OP by construction — the loop never runs. Do not add a
/// root-replacing branch for it: that was A3, where a single record with no `k`
/// discarded everything replayed before it.
fn set(root: &mut serde_json::Value, path: &[Seg], value: serde_json::Value, append: bool) {
    let mut cur = root;
    for (i, seg) in path.iter().enumerate() {
        let last = i == path.len() - 1;
        match seg {
            Seg::Index(idx) => {
                if !cur.is_array() {
                    *cur = serde_json::json!([]);
                }
                let Some(arr) = cur.as_array_mut() else { return };
                if *idx > MAX_INDEX {
                    return;
                }
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

/// Replay the journal and read out its exchanges.
///
/// Each element of `requests[]` is ONE exchange: `message.text` is what the
/// human typed and `response[]` is the reply. There is no `role` field and no
/// `responseParts`.
pub fn requests(content: &str) -> Vec<Request> {
    let root = replay(content);
    let Some(arr) = root.get("requests").and_then(|r| r.as_array()) else {
        return Vec::new();
    };

    let mut out = Vec::with_capacity(arr.len());
    for (i, req) in arr.iter().enumerate() {
        let mut r = Request {
            index: i,
            timestamp_ms: req["timestamp"].as_i64(),
            response_timestamp_ms: req["responseTimestamp"].as_i64(),
            user_text: req["message"]["text"].as_str().unwrap_or("").trim().to_string(),
            // `modelId` arrives namespaced, e.g. "copilot/claude-opus-4.6".
            model: req["modelId"].as_str().map(|m| m.rsplit('/').next().unwrap_or(m).to_string()),
            ..Default::default()
        };

        // Assistant prose lives in the UNTAGGED parts of `response[]`. Tool
        // invocations and thinking blocks carry a `kind` and are not prose.
        if let Some(parts) = req["response"].as_array() {
            for part in parts {
                match part["kind"].as_str() {
                    Some("toolInvocationSerialized") => {
                        r.tool_calls.push(ToolCall {
                            tool_id: part["toolId"].as_str().unwrap_or("<unknown>").to_string(),
                            invocation_message: part["invocationMessage"]["value"]
                                .as_str()
                                .or_else(|| part["invocationMessage"].as_str())
                                .unwrap_or_default()
                                .to_string(),
                            command: part["toolSpecificData"]["commandLine"]["original"]
                                .as_str()
                                .map(str::to_string),
                        });
                    }
                    Some("thinking") => {
                        r.thinking_ms += part["reasoningDurationMs"].as_i64().unwrap_or(0);
                    }
                    None => {
                        if let Some(t) = part["value"].as_str() {
                            let t = t.trim();
                            if !t.is_empty() {
                                if !r.assistant_text.is_empty() {
                                    r.assistant_text.push_str("\n\n");
                                }
                                r.assistant_text.push_str(t);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        out.push(r);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn journal(ops: &[&str]) -> String {
        ops.join("\n")
    }

    #[test]
    fn a_snapshot_then_a_set_reconstructs_the_state() {
        let j = journal(&[
            r#"{"kind":0,"v":{"requests":[]}}"#,
            r#"{"kind":1,"k":["requests",0,"message"],"v":{"text":"hi"}}"#,
        ]);
        let rs = requests(&j);
        assert_eq!(rs.len(), 1);
        assert_eq!(rs[0].user_text, "hi");
    }

    /// `kind:2` APPENDS. Treating it as a replace keeps only the last fragment,
    /// because a reply is streamed in pieces.
    #[test]
    fn kind_two_appends_rather_than_replacing() {
        let j = journal(&[
            r#"{"kind":0,"v":{"requests":[{"response":[]}]}}"#,
            r#"{"kind":2,"k":["requests",0,"response"],"v":[{"value":"one"}]}"#,
            r#"{"kind":2,"k":["requests",0,"response"],"v":[{"value":"two"}]}"#,
        ]);
        let rs = requests(&j);
        assert_eq!(rs[0].assistant_text, "one\n\ntwo");
    }

    /// A2's sibling: the real timestamp is on the request, so nothing downstream
    /// needs to invent one.
    #[test]
    fn the_real_timestamps_are_read() {
        let j = journal(&[
            r#"{"kind":0,"v":{"requests":[{"timestamp":1786294114560,"responseTimestamp":1786294120000,"message":{"text":"go"}}]}}"#,
        ]);
        let rs = requests(&j);
        assert_eq!(rs[0].timestamp_ms, Some(1786294114560));
        assert_eq!(rs[0].response_timestamp_ms, Some(1786294120000));
    }

    /// A journal that omits a timestamp must report NONE, so a caller cannot
    /// mistake a fabricated value for a real one.
    #[test]
    fn a_missing_timestamp_is_none_not_zero() {
        let j = journal(&[r#"{"kind":0,"v":{"requests":[{"message":{"text":"go"}}]}}"#]);
        assert_eq!(requests(&j)[0].timestamp_ms, None);
    }

    /// A1: `k` is untrusted. An out-of-range index must be skipped, not
    /// allocated. The index is a little over the cap so a regression fails the
    /// test rather than OOM-killing the machine.
    #[test]
    fn a_huge_index_is_refused_not_allocated() {
        let j = journal(&[
            r#"{"kind":0,"v":{"requests":[]}}"#,
            r#"{"kind":1,"k":["requests",2000000,"message"],"v":{"text":"x"}}"#,
        ]);
        assert!(requests(&j).is_empty(), "must not backfill two million nulls");
    }

    /// A3: a `kind:1` with no `k` yields an empty path. The daemon applied such
    /// a value at the ROOT, discarding everything replayed before it.
    ///
    /// This passes today whether or not the explicit skip is present, because
    /// `set` no-ops on an empty path anyway. It is kept as a regression guard:
    /// it fails the moment anyone reintroduces a root-replacing branch.
    #[test]
    fn a_record_with_no_path_cannot_wipe_the_state() {
        let j = journal(&[
            r#"{"kind":0,"v":{"requests":[{"message":{"text":"kept"}}]}}"#,
            r#"{"kind":1,"v":"poison"}"#,
        ]);
        let rs = requests(&j);
        assert_eq!(rs.len(), 1, "the good request must survive the malformed record");
        assert_eq!(rs[0].user_text, "kept");
    }

    /// A3's other door: a record with a value but no `kind` defaulted to 0 —
    /// a full root snapshot — so a missing discriminant took the most
    /// destructive branch.
    #[test]
    fn a_record_with_no_kind_is_skipped_not_treated_as_a_snapshot() {
        let j = journal(&[
            r#"{"kind":0,"v":{"requests":[{"message":{"text":"kept"}}]}}"#,
            r#"{"v":{"requests":[]}}"#,
        ]);
        assert_eq!(requests(&j).len(), 1, "a kind-less record must not snapshot the root");
    }

    #[test]
    fn tool_calls_carry_their_command_and_rendered_message() {
        let j = journal(&[
            r#"{"kind":0,"v":{"requests":[{"response":[{"kind":"toolInvocationSerialized","toolId":"run_in_terminal","invocationMessage":{"value":"Running `git commit`"},"toolSpecificData":{"commandLine":{"original":"git commit -m x"}}}]}]}}"#,
        ]);
        let t = &requests(&j)[0].tool_calls[0];
        assert_eq!(t.tool_id, "run_in_terminal");
        assert_eq!(t.command.as_deref(), Some("git commit -m x"));
        assert!(t.invocation_message.contains("Running"));
    }

    #[test]
    fn the_model_prefix_is_stripped() {
        let j = journal(&[
            r#"{"kind":0,"v":{"requests":[{"modelId":"copilot/claude-opus-4.6","message":{"text":"x"}}]}}"#,
        ]);
        assert_eq!(requests(&j)[0].model.as_deref(), Some("claude-opus-4.6"));
    }

    #[test]
    fn a_non_journal_yields_nothing() {
        assert!(requests(r#"{"type":"user.message","data":{"content":"hi"}}"#).is_empty());
    }
}
