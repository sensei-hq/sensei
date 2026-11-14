//! Lossy transcript export: rebuild an approximate JSONL transcript from kept turns.
//!
//! For sessions whose original transcript FILE is gone (rotated/deleted), the DB
//! holds parsed turns (prose + verbatim attrs + facts) but not the original
//! bytes. This rebuilds Claude-shaped JSONL lines from those rows — readable
//! and re-parseable, but NOT the original. Unrecoverable, by construction:
//!
//! - tool calls/results, attachments, thinking blocks (dropped from `attrs`);
//! - record boundaries (N assistant records merged into one text; per-record
//!   timestamps/usage/ids gone — usage survives only as turn sums);
//! - skipped records (system/summary/snapshot/queue lines, tool-result-only
//!   user records, prose before the first prompt);
//! - the uuid chain (only the turn-starting record's ids are kept) and exact bytes.
//!
//! Every emitted line carries [`REGENERATED_MARKER`], which ingest skips (see
//! `claude::records`): re-ingesting an export must never overwrite rich rows
//! with poor ones.

use super::{TranscriptTurn, TurnFacts};
use chrono::SecondsFormat;
use serde_json::{Map, Value};

/// Top-level key on every regenerated line. Ingest skips lines carrying it.
pub const REGENERATED_MARKER: &str = "sensei_regenerated";

/// Rebuild one session's transcript as JSONL lines, in turn order: two lines
/// per turn (user, assistant), so line pairs map 1:1 onto turn rows.
/// `model` is the turn rows' stored model (turns don't carry it themselves).
/// Only `claude_code` is supported: other families parse different shapes.
pub fn export_turns(
    source: &str,
    session_id: &str,
    turns: &[TranscriptTurn],
    model: Option<&str>,
) -> Result<Vec<String>, String> {
    if source != "claude_code" {
        return Err(format!(
            "transcript export supports claude_code transcripts for now, not '{source}'"
        ));
    }
    let mut lines = Vec::with_capacity(turns.len() * 2);
    for turn in turns {
        lines.push(user_line(session_id, turn));
        lines.push(assistant_line(session_id, turn, model));
    }
    Ok(lines)
}

/// One `user` line: the turn's attrs verbatim (the original record's envelope —
/// uuid, cwd, version, …), with the message rebuilt from the kept prose.
fn user_line(session_id: &str, turn: &TranscriptTurn) -> String {
    let mut obj = turn.attrs.as_object().cloned().unwrap_or_default();
    obj.insert("type".to_string(), Value::String("user".to_string()));
    if let Some(ts) = turn.started_at {
        obj.insert("timestamp".to_string(), Value::String(fmt_ts(ts)));
    }
    obj.insert("sessionId".to_string(), Value::String(session_id.to_string()));
    // Merge into any attrs-carried message subset rather than replacing it.
    let mut msg = obj.get("message").and_then(|m| m.as_object()).cloned().unwrap_or_default();
    msg.insert("role".to_string(), Value::String("user".to_string()));
    msg.insert("content".to_string(), Value::String(turn.user_text.clone().unwrap_or_default()));
    obj.insert("message".to_string(), Value::Object(msg));
    if let Some(pid) = &turn.prompt_id {
        obj.insert("promptId".to_string(), Value::String(pid.clone()));
    }
    obj.insert(REGENERATED_MARKER.to_string(), Value::Bool(true));
    serde_json::to_string(&obj).unwrap_or_default()
}

/// One `assistant` line. What the rows don't keep is omitted, never invented:
/// no uuid (only the turn-starting record's ids are kept), no per-record
/// timestamps, one usage object with the turn's sums.
fn assistant_line(session_id: &str, turn: &TranscriptTurn, model: Option<&str>) -> String {
    let mut obj = Map::new();
    obj.insert("type".to_string(), Value::String("assistant".to_string()));
    if let Some(ts) = turn.started_at {
        // Approximate: the turn's start, not each merged record's own time.
        obj.insert("timestamp".to_string(), Value::String(fmt_ts(ts)));
    }
    obj.insert("sessionId".to_string(), Value::String(session_id.to_string()));
    let mut msg = Map::new();
    msg.insert("role".to_string(), Value::String("assistant".to_string()));
    if let Some(m) = model {
        msg.insert("model".to_string(), Value::String(m.to_string()));
    }
    opt_str(&mut msg, "stop_reason", turn.facts.stop_reason.clone());
    // Ingest joins trimmed blocks with "\n\n"; splitting restores them, except a
    // block that originally contained a blank line (it over-splits — documented loss).
    let blocks: Vec<Value> = turn
        .assistant_text
        .split("\n\n")
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| serde_json::json!({ "type": "text", "text": t }))
        .collect();
    msg.insert("content".to_string(), Value::Array(blocks));
    if let Some(u) = usage_of(&turn.facts) {
        msg.insert("usage".to_string(), u);
    }
    obj.insert("message".to_string(), Value::Object(msg));
    obj.insert(REGENERATED_MARKER.to_string(), Value::Bool(true));
    serde_json::to_string(&obj).unwrap_or_default()
}

/// The turn's usage sums in the record's own spelling. `None` when the turn
/// carries no usage at all, so the key stays absent rather than fabricated.
fn usage_of(facts: &TurnFacts) -> Option<Value> {
    let mut u = Map::new();
    opt_i64(&mut u, "input_tokens", facts.tokens_in);
    opt_i64(&mut u, "output_tokens", facts.tokens_out);
    opt_i64(&mut u, "cache_read_input_tokens", facts.cache_read);
    opt_i64(&mut u, "cache_creation_input_tokens", facts.cache_write);
    opt_str(&mut u, "service_tier", facts.service_tier.clone());
    if u.is_empty() { None } else { Some(Value::Object(u)) }
}

fn opt_str(map: &mut Map<String, Value>, key: &str, v: Option<String>) {
    if let Some(s) = v {
        map.insert(key.to_string(), Value::String(s));
    }
}

fn opt_i64(map: &mut Map<String, Value>, key: &str, v: Option<i64>) {
    if let Some(n) = v {
        map.insert(key.to_string(), Value::Number(n.into()));
    }
}

fn fmt_ts(ts: chrono::DateTime<chrono::Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn turn() -> TranscriptTurn {
        TranscriptTurn {
            turn_index: 1,
            origin: "human".to_string(),
            prompt_id: Some("p1".to_string()),
            user_text: Some("add a login page".to_string()),
            assistant_text: "On it.\n\nDone.".to_string(),
            started_at: Some(
                chrono::DateTime::parse_from_rfc3339("2026-06-20T10:00:00.000Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            ),
            attrs: json!({
                "type": "user",
                "uuid": "u1",
                "parentUuid": "u0",
                "sessionId": "s1",
                "timestamp": "2026-06-20T10:00:00.000Z",
                "cwd": "/r",
                "version": "2.1.0",
                "gitBranch": "develop"
            }),
            facts: TurnFacts {
                tokens_in: Some(100),
                tokens_out: Some(50),
                cache_read: Some(10),
                cache_write: Some(5),
                stop_reason: Some("end_turn".to_string()),
                service_tier: Some("standard".to_string()),
                ..Default::default()
            },
        }
    }

    #[test]
    fn user_line_keeps_attrs_verbatim_builds_message_and_marks() {
        let lines = export_turns("claude_code", "s1", &[turn()], Some("claude-opus-4-8")).unwrap();
        let user: Value = serde_json::from_str(&lines[0]).unwrap();

        assert_eq!(user["type"], "user");
        assert_eq!(user["uuid"], "u1");
        assert_eq!(user["parentUuid"], "u0");
        assert_eq!(user["cwd"], "/r");
        assert_eq!(user["message"]["role"], "user");
        assert_eq!(user["message"]["content"], "add a login page");
        assert_eq!(user["promptId"], "p1");
        assert_eq!(user[REGENERATED_MARKER], true);
    }

    #[test]
    fn assistant_line_splits_blocks_maps_usage_omits_absent() {
        let lines = export_turns("claude_code", "s1", &[turn()], Some("claude-opus-4-8")).unwrap();
        let asst: Value = serde_json::from_str(&lines[1]).unwrap();

        assert_eq!(asst["type"], "assistant");
        assert_eq!(
            asst["message"]["content"],
            json!([
                { "type": "text", "text": "On it." },
                { "type": "text", "text": "Done." }
            ])
        );
        assert_eq!(asst["message"]["model"], "claude-opus-4-8");
        assert_eq!(asst["message"]["stop_reason"], "end_turn");
        assert_eq!(asst["message"]["usage"]["input_tokens"], 100);
        assert_eq!(asst["message"]["usage"]["output_tokens"], 50);
        assert_eq!(asst["message"]["usage"]["cache_read_input_tokens"], 10);
        assert_eq!(asst["message"]["usage"]["cache_creation_input_tokens"], 5);
        assert_eq!(asst["message"]["usage"]["service_tier"], "standard");
        assert!(asst.get("uuid").is_none(), "assistant uuids are not kept");
        assert_eq!(asst[REGENERATED_MARKER], true);
    }

    #[test]
    fn empty_turn_still_emits_its_pair_so_lines_map_to_turns() {
        let t = TranscriptTurn {
            user_text: None,
            assistant_text: String::new(),
            facts: TurnFacts::default(),
            ..turn()
        };
        let lines = export_turns("claude_code", "s1", &[t], None).unwrap();

        assert_eq!(lines.len(), 2);
        let user: Value = serde_json::from_str(&lines[0]).unwrap();
        let asst: Value = serde_json::from_str(&lines[1]).unwrap();
        assert_eq!(user["message"]["content"], "");
        assert_eq!(asst["message"]["content"], json!([]));
        assert!(asst["message"].get("model").is_none());
        assert!(asst["message"].get("usage").is_none());
    }

    #[test]
    fn other_sources_are_refused_not_misshaped() {
        let err = export_turns("zed", "s1", &[turn()], None).unwrap_err();
        assert!(err.contains("zed"), "{err}");
    }

    #[test]
    fn lines_come_out_in_turn_order_two_per_turn() {
        let t2 = TranscriptTurn { turn_index: 2, ..turn() };
        let lines = export_turns("claude_code", "s1", &[turn(), t2], None).unwrap();

        assert_eq!(lines.len(), 4);
        let kinds: Vec<String> = lines
            .iter()
            .map(|l| {
                serde_json::from_str::<Value>(l).unwrap()["type"].as_str().unwrap().to_string()
            })
            .collect();
        assert_eq!(kinds, ["user", "assistant", "user", "assistant"]);
    }
}
