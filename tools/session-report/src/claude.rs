//! Read a Claude Code project folder into the same [`Session`] shape.
//!
//! Different transcript, same questions. Where Copilot CLI reports a signal
//! directly, Claude usually implies it:
//!
//! | | Copilot CLI | Claude Code |
//! |---|---|---|
//! | tool success | `success` bool | absence of `is_error` on the result |
//! | turn boundary | `turn_start`/`turn_end` | consecutive assistant messages |
//! | tokens | session totals at shutdown | per message, summed here |
//! | premium requests | reported | not a concept |
//! | lines changed | reported | not reported |
//!
//! So the cross-tool report can compare pace and friction, but not cost — and it
//! says so rather than showing a blank column as a zero.

use crate::model::{Session, ToolCall, Totals, Turn};
use std::collections::HashMap;
use std::path::Path;

/// Text Claude Code injects on the human's behalf. It arrives as a `user`
/// record and is indistinguishable from typing unless you look at the opening
/// marker — the daemon's own adapter filters the same list. Counting these as
/// prompts inflates "prompts written" and deflates every per-prompt ratio.
const INJECTED_MARKERS: &[&str] = &[
    "<task-notification",
    "<system-reminder",
    "<command-name",
    "<command-message",
    "<local-command",
    "Caveat:",
    "## Security Guidance",
];

fn is_injected(text: &str) -> bool {
    let t = text.trim_start();
    INJECTED_MARKERS.iter().any(|m| t.starts_with(m))
}

fn ms(v: Option<&str>) -> Option<i64> {
    v.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()).map(|d| d.timestamp_millis())
}

/// One `.jsonl` under `projects/<slug>/` is one session.
pub fn parse_session(file: &Path) -> Option<(Session, usize)> {
    let text = std::fs::read_to_string(file).ok()?;
    let id = file.file_stem()?.to_string_lossy().to_string();

    let mut skipped = 0usize;
    let mut first_ms: Option<i64> = None;
    let mut last_ms = 0i64;
    let mut activity: Vec<i64> = Vec::new();
    let mut prompts = 0usize;
    let mut turns: Vec<Turn> = Vec::new();
    let mut tools: Vec<ToolCall> = Vec::new();
    let mut open_tools: HashMap<String, usize> = HashMap::new();
    let mut languages: HashMap<String, usize> = HashMap::new();
    let (mut git_commits, mut git_pushes) = (0usize, 0usize);
    let mut prompt_ms: Vec<i64> = Vec::new();
    let mut models: HashMap<String, usize> = HashMap::new();
    let mut cwd: Option<String> = None;
    let mut totals = Totals::default();
    let (mut input, mut output, mut cache_r, mut cache_w, mut thinking) = (0i64, 0, 0, 0, 0);
    let mut event_count = 0usize;
    let mut last_assistant_ms: Option<i64> = None;

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
        let at = ms(v["timestamp"].as_str());
        if let Some(t) = at {
            first_ms.get_or_insert(t);
            last_ms = last_ms.max(t);
            activity.push(t);
        }
        if cwd.is_none()
            && let Some(c) = v["cwd"].as_str()
        {
            cwd = Some(c.to_string());
        }
        let uuid = v["uuid"].as_str().unwrap_or_default().to_string();

        match v["type"].as_str().unwrap_or_default() {
            "user" => {
                // A `user` record is also how tool RESULTS arrive, and `isMeta`
                // marks injected context. Neither is a human typing.
                if v["isMeta"].as_bool() == Some(true) {
                    continue;
                }
                let content = &v["message"]["content"];
                let is_tool_result = content
                    .as_array()
                    .is_some_and(|a| a.iter().any(|b| b["type"] == "tool_result"));
                if is_tool_result {
                    // Close the matching call, and read whether it errored.
                    if let Some(arr) = content.as_array() {
                        for b in arr {
                            if b["type"] != "tool_result" {
                                continue;
                            }
                            if let Some(id) = b["tool_use_id"].as_str()
                                && let Some(idx) = open_tools.remove(id)
                                && let Some(call) = tools.get_mut(idx)
                            {
                                call.ended_ms = at;
                                // Claude reports failure as `is_error`; absence
                                // means it came back fine.
                                call.success = Some(b["is_error"].as_bool() != Some(true));
                            }
                        }
                    }
                    continue;
                }
                let text = match content {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Array(a) => {
                        a.iter().filter_map(|b| b["text"].as_str()).collect::<Vec<_>>().join("\n")
                    }
                    _ => String::new(),
                };
                if text.trim().is_empty() || is_injected(&text) {
                    continue;
                }
                if let Some(t) = at {
                    // A prompt also CLOSES the previous turn: the turn ran from
                    // the last prompt to the assistant's final message before
                    // this one.
                    if let Some(cur) = turns.last_mut()
                        && cur.ended_ms.is_none()
                    {
                        cur.ended_ms = Some(last_assistant_ms.unwrap_or(t));
                    }
                    turns.push(Turn {
                        id: uuid.clone(),
                        started_ms: t,
                        ended_ms: None,
                        model: None,
                    });
                    prompt_ms.push(t);
                    prompts += 1;
                }
            }
            "assistant" => {
                let m = &v["message"];
                if let Some(model) = m["model"].as_str() {
                    *models.entry(model.to_string()).or_default() += 1;
                }
                let u = &m["usage"];
                input += u["input_tokens"].as_i64().unwrap_or(0);
                output += u["output_tokens"].as_i64().unwrap_or(0);
                cache_r += u["cache_read_input_tokens"].as_i64().unwrap_or(0);
                cache_w += u["cache_creation_input_tokens"].as_i64().unwrap_or(0);
                thinking += u["output_tokens_details"]["thinking_tokens"].as_i64().unwrap_or(0);

                // Claude has no turn markers. A turn is the human-visible unit:
                // from a prompt to the assistant's last message before the next
                // prompt. Measuring assistant-message GAPS instead reported a
                // typical turn of one second, which is the pause between two
                // streamed messages, not the time waiting for an answer.
                if let Some(t) = at {
                    last_assistant_ms = Some(t);
                    if let Some(cur) = turns.last_mut()
                        && cur.model.is_none()
                    {
                        cur.model = m["model"].as_str().map(String::from);
                    }
                }

                if let Some(blocks) = m["content"].as_array() {
                    for b in blocks {
                        if b["type"] != "tool_use" {
                            continue;
                        }
                        let Some(t) = at else { continue };
                        // Grep/Glob address a search ROOT via `path`, so only
                        // `file_path` is read — it is the key the tools that
                        // address ONE file use.
                        let input = &b["input"];
                        if let Some(p) =
                            crate::signals::path_argument(input, &["file_path", "notebook_path"])
                            && let Some(lang) = crate::signals::language_of(p)
                        {
                            *languages.entry(lang.to_string()).or_default() += 1;
                        }
                        if let Some(cmd) = input["command"].as_str() {
                            let (c, u) = crate::signals::git_actions(cmd);
                            git_commits += c;
                            git_pushes += u;
                        }
                        let id = b["id"].as_str().unwrap_or_default().to_string();
                        tools.push(ToolCall {
                            name: b["name"].as_str().unwrap_or("<unknown>").to_string(),
                            started_ms: t,
                            ended_ms: None,
                            success: None,
                            event_id: uuid.clone(),
                        });
                        open_tools.insert(id, tools.len() - 1);
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(cur) = turns.last_mut()
        && cur.ended_ms.is_none()
    {
        cur.ended_ms = last_assistant_ms;
    }

    first_ms?;

    totals.input_tokens = Some(input);
    totals.output_tokens = Some(output);
    totals.cache_read_tokens = Some(cache_r);
    totals.cache_write_tokens = Some(cache_w);
    totals.reasoning_tokens = Some(thinking);
    // Deliberately left None: Claude reports no premium requests and no
    // code-change totals. Zero would read as "changed nothing".
    activity.sort_unstable();

    Some((
        Session {
            id,
            cwd,
            first_ms: first_ms.unwrap_or(0),
            last_ms,
            prompts,
            turns,
            tools,
            totals,
            models,
            permission_events: 0,
            event_count,
            activity_ms: activity,
            delegated: 0,
            delegated_models: HashMap::new(),
            unclosed: false,
            source: None,
            languages,
            git_commits,
            git_pushes,
            prompt_ms,
        },
        skipped,
    ))
}

/// Collect every `.jsonl` beneath a directory, at any depth.
fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "jsonl") {
            out.push(p);
        }
    }
}

/// Fold a sub-agent transcript into its parent session.
///
/// A delegated agent runs INSIDE the session that spawned it — same wall clock,
/// same piece of work — but writes a separate file. The parent's transcript
/// records only the hand-off, so without this every tool call, token and minute
/// the sub-agent spent is missing. On this sample that is most of the activity:
/// 626 sub-agent files against 22 sessions for one person.
///
/// Prompts are NOT merged: a sub-agent's instructions come from the assistant,
/// not from the human, and counting them would inflate "prompts written".
fn fold_into(parent: &mut Session, child: Session) {
    parent.turns.extend(child.turns);
    parent.tools.extend(child.tools);
    parent.event_count += child.event_count;
    parent.activity_ms.extend(child.activity_ms);
    parent.delegated += 1;
    // A sub-agent editing a .ts file is real work in TypeScript, and a sub-agent
    // that commits has really committed — both belong to the parent session.
    for (l, c) in &child.languages {
        *parent.languages.entry(l.clone()).or_default() += c;
    }
    parent.git_commits += child.git_commits;
    parent.git_pushes += child.git_pushes;
    // `prompt_ms` is deliberately NOT merged, for the same reason `prompts` is
    // not: a sub-agent's instructions come from the assistant, not the human, so
    // folding them in would invent human reply times that nobody waited through.
    // Record the child's models under `delegated_models` BEFORE merging them
    // into the parent's overall mix, so both questions stay answerable.
    for (m, c) in &child.models {
        *parent.delegated_models.entry(m.clone()).or_default() += c;
    }
    parent.first_ms = parent.first_ms.min(child.first_ms).max(1);
    parent.last_ms = parent.last_ms.max(child.last_ms);
    for (m, c) in child.models {
        *parent.models.entry(m).or_default() += c;
    }
    let (p, c) = (&mut parent.totals, &child.totals);
    for (dst, src) in [
        (&mut p.input_tokens, c.input_tokens),
        (&mut p.output_tokens, c.output_tokens),
        (&mut p.cache_read_tokens, c.cache_read_tokens),
        (&mut p.cache_write_tokens, c.cache_write_tokens),
        (&mut p.reasoning_tokens, c.reasoning_tokens),
    ] {
        if let Some(v) = src {
            *dst = Some(dst.unwrap_or(0) + v);
        }
    }
}

/// Every session under a `projects/` tree, with sub-agent work folded in.
pub fn collect(root: &Path) -> (Vec<Session>, usize) {
    let mut sessions: Vec<Session> = Vec::new();
    let mut skipped = 0usize;
    let projects = if root.join("projects").is_dir() { root.join("projects") } else { root.into() };
    let Ok(slugs) = std::fs::read_dir(&projects) else {
        return (sessions, skipped);
    };
    for slug in slugs.flatten() {
        let Ok(files) = std::fs::read_dir(slug.path()) else { continue };
        for f in files.flatten() {
            let p = f.path();
            if !p.extension().is_some_and(|e| e == "jsonl") {
                continue;
            }
            let Some((mut session, sk)) = parse_session(&p) else { continue };
            skipped += sk;
            // Sub-agents live in `<session-id>/` beside the session file.
            let sub_root = slug.path().join(&session.id);
            if sub_root.is_dir() {
                let mut kids = Vec::new();
                walk(&sub_root, &mut kids);
                for k in kids {
                    if let Some((child, sk)) = parse_session(&k) {
                        skipped += sk;
                        fold_into(&mut session, child);
                    }
                }
                session.activity_ms.sort_unstable();
                session.turns.sort_by_key(|t| t.started_ms);
                session.tools.sort_by_key(|t| t.started_ms);
            }
            sessions.push(session);
        }
    }
    sessions.sort_by_key(|s| s.first_ms);
    (sessions, skipped)
}
