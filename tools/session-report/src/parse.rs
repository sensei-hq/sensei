//! Read a Copilot CLI session directory into [`Session`].
//!
//! Deliberately tolerant: a malformed line is skipped rather than failing the
//! session, because these are other people's transcripts and one bad line should
//! not cost us the other 12,000. Anything skipped is counted and reported, so a
//! quietly-truncated file cannot pass as a complete one.

use crate::model::{Prompt, Session, ToolCall, Totals, Turn};
use std::collections::HashMap;
use std::path::Path;

pub struct ParseOutcome {
    pub session: Session,
    pub skipped_lines: usize,
}

fn ms(v: Option<&str>) -> Option<i64> {
    v.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()).map(|d| d.timestamp_millis())
}

/// Pull `cwd:` out of workspace.yaml without a YAML dependency — the file is a
/// handful of scalars and we want exactly one of them.
fn read_cwd(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("workspace.yaml")).ok()?;
    for line in text.lines() {
        if let Some(rest) = line.trim().strip_prefix("cwd:") {
            let v = rest.trim().trim_matches(['"', '\'']);
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

pub fn parse_session(dir: &Path) -> Option<ParseOutcome> {
    let events = dir.join("events.jsonl");
    let text = std::fs::read_to_string(&events).ok()?;
    let id = dir.file_name()?.to_string_lossy().to_string();

    let mut skipped = 0usize;
    let mut first_ms: Option<i64> = None;
    let mut last_ms: i64 = 0;
    let mut prompts = Vec::new();
    let mut turns: HashMap<String, Turn> = HashMap::new();
    let mut turn_order: Vec<String> = Vec::new();
    let mut open_tools: HashMap<String, ToolCall> = HashMap::new();
    let mut tools: Vec<ToolCall> = Vec::new();
    let mut totals = Totals::default();
    let mut models: HashMap<String, usize> = HashMap::new();
    let mut permission_events = 0usize;
    let mut event_count = 0usize;
    let mut activity_ms: Vec<i64> = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            skipped += 1;
            continue;
        };
        event_count += 1;
        let at = ms(v.get("timestamp").and_then(|t| t.as_str()));
        if let Some(t) = at {
            first_ms.get_or_insert(t);
            last_ms = last_ms.max(t);
            activity_ms.push(t);
        }
        let d = &v["data"];
        let event_id = v["id"].as_str().unwrap_or_default().to_string();

        match v.get("type").and_then(|t| t.as_str()).unwrap_or_default() {
            "user.message" => {
                if let Some(t) = at {
                    prompts.push(Prompt {
                        at_ms: t,
                        text: d["content"].as_str().unwrap_or_default().to_string(),
                        event_id,
                    });
                }
            }
            "assistant.turn_start" => {
                if let (Some(t), Some(tid)) = (at, d["turnId"].as_str()) {
                    let key = format!("{tid}@{t}");
                    turn_order.push(key.clone());
                    turns.insert(
                        key,
                        Turn {
                            id: tid.to_string(),
                            started_ms: t,
                            ended_ms: None,
                            model: d["model"].as_str().map(String::from),
                        },
                    );
                }
            }
            "assistant.turn_end" => {
                // Close the most recent OPEN turn with this id. Turn ids restart
                // per interaction, so matching on id alone would close the wrong
                // one and produce negative durations.
                if let (Some(t), Some(tid)) = (at, d["turnId"].as_str())
                    && let Some(key) = turn_order
                        .iter()
                        .rev()
                        .find(|k| turns.get(*k).is_some_and(|x| x.id == tid && x.ended_ms.is_none()))
                        .cloned()
                    && let Some(turn) = turns.get_mut(&key)
                {
                    turn.ended_ms = Some(t);
                }
            }
            "assistant.message" => {
                if let Some(m) = d["model"].as_str() {
                    *models.entry(m.to_string()).or_default() += 1;
                }
            }
            "tool.execution_start" => {
                if let (Some(t), Some(cid)) = (at, d["toolCallId"].as_str()) {
                    open_tools.insert(
                        cid.to_string(),
                        ToolCall {
                            name: d["toolName"].as_str().unwrap_or("<unknown>").to_string(),
                            started_ms: t,
                            ended_ms: None,
                            success: None,
                            event_id,
                        },
                    );
                }
            }
            "tool.execution_complete" => {
                if let Some(cid) = d["toolCallId"].as_str()
                    && let Some(mut call) = open_tools.remove(cid)
                {
                    call.ended_ms = at;
                    call.success = d["success"].as_bool();
                    tools.push(call);
                }
            }
            "session.permissions_changed" => permission_events += 1,
            "session.shutdown" => {
                totals.premium_requests = d["totalPremiumRequests"].as_i64();
                totals.api_duration_ms = d["totalApiDurationMs"].as_i64();
                let cc = &d["codeChanges"];
                totals.lines_added = cc["linesAdded"].as_i64();
                totals.lines_removed = cc["linesRemoved"].as_i64();
                totals.files_modified = cc["filesModified"].as_array().map(|a| a.len());
                // Token usage is nested per model; sum across whichever were used.
                if let Some(mm) = d["modelMetrics"].as_object() {
                    let mut acc = (0i64, 0i64, 0i64, 0i64);
                    let mut any = false;
                    for m in mm.values() {
                        let u = &m["usage"];
                        if u.is_object() {
                            any = true;
                            acc.0 += u["inputTokens"].as_i64().unwrap_or(0);
                            acc.1 += u["outputTokens"].as_i64().unwrap_or(0);
                            acc.2 += u["cacheReadTokens"].as_i64().unwrap_or(0);
                            acc.3 += u["cacheWriteTokens"].as_i64().unwrap_or(0);
                        }
                    }
                    if any {
                        totals.input_tokens = Some(acc.0);
                        totals.output_tokens = Some(acc.1);
                        totals.cache_read_tokens = Some(acc.2);
                        totals.cache_write_tokens = Some(acc.3);
                    }
                }
            }
            _ => {}
        }
    }

    // Tools still open at EOF never reported back — keep them, flagged, rather
    // than dropping them and understating the work.
    tools.extend(open_tools.into_values());

    let unclosed = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .any(|e| e.file_name().to_string_lossy().starts_with("inuse."))
        })
        .unwrap_or(false);

    let mut turn_list: Vec<Turn> = turn_order.iter().filter_map(|k| turns.remove(k)).collect();
    turn_list.sort_by_key(|t| t.started_ms);
    tools.sort_by_key(|t| t.started_ms);

    Some(ParseOutcome {
        session: Session {
            id,
            cwd: read_cwd(dir),
            first_ms: first_ms.unwrap_or(0),
            last_ms,
            prompts,
            turns: turn_list,
            tools,
            totals,
            models,
            permission_events,
            event_count,
            activity_ms: {
                activity_ms.sort_unstable();
                activity_ms
            },
            unclosed,
        },
        skipped_lines: skipped,
    })
}
