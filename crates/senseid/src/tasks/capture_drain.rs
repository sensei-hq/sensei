//! Capture-spool drain.
//!
//! Every assistant hook POSTs its payload to `/hook/event`; the daemon writes
//! it to `activity.assistant_events`. When that POST fails (daemon down, or a
//! POST slower than the hook's 2s curl budget), the hook's `_lib.sh` fallback
//! *dead-letters* the payload to `~/.sensei/events.jsonl` so capture is never
//! lost. Nothing used to drain that file back, so every dead-lettered event was
//! stranded on disk and missing from analysis.
//!
//! This task closes that gap: on boot and on the `capture_drain` schedule
//! it rotates the spool aside and imports each line into
//! `activity.assistant_events` — the same table and the same field mapping the
//! live `ingest_hook_event` handler uses (see [`hook_event_fields`], shared by
//! both paths). Inserts are deduped on the payload so the rare
//! "curl timed out *after* the daemon committed" race can't create a twin row.
//!
//! Durability: the live spool is `rename`d to `events.jsonl.draining` before
//! import (an atomic move within the dir), so new dead-letters append to a
//! fresh file while the rotated copy is consumed. The rotated file is deleted
//! only after every line imported; a mid-drain failure leaves it in place and
//! the next tick retries (dedup makes the retry idempotent). A leftover
//! `.draining` from a crashed drain is swept on the next run.
//!
//! Known limitation: dead-lettered lines carry no client timestamp today, so
//! legacy backlog rows import with `ts` = drain time (approximate). Stamping a
//! client `ts` in the hook fallback is a tracked follow-up; [`event_ts`] already
//! honours a `ts` field when present.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::db::pg_store::PgStore;
use crate::tasks::ticker;

/// The fields `activity.assistant_events` needs, borrowed out of one hook
/// payload. Everything except `ts` (the caller supplies that: the live handler
/// uses receipt time, the drain uses [`event_ts`]) and the raw `payload` itself.
pub struct HookEventFields<'a> {
    pub session_id: &'a str,
    pub family: &'a str,
    pub event_type: &'a str,
    pub tool_name: Option<&'a str>,
    pub cwd: Option<&'a str>,
    pub success: Option<bool>,
}

/// Map a raw hook payload to the columns of `activity.assistant_events`.
///
/// The single source of truth for the mapping, shared by the live
/// `ingest_hook_event` handler and the drain so both paths agree field-for-field.
pub fn hook_event_fields(payload: &serde_json::Value) -> HookEventFields<'_> {
    HookEventFields {
        session_id: payload["session_id"].as_str().unwrap_or(""),
        family: payload["assistant_family"].as_str().unwrap_or("claude"),
        event_type: payload["hook_event_name"].as_str().unwrap_or("unknown"),
        tool_name: payload["tool_name"].as_str(),
        cwd: payload["cwd"].as_str(),
        success: payload.get("exit_code").and_then(|v| v.as_i64()).map(|c| c == 0),
    }
}

/// Recursively strip NUL (U+0000) from every string in a JSON value. Postgres
/// jsonb (and `text`) cannot store a NUL, so an unsanitised hook payload — e.g.
/// captured tool output with a stray NUL byte — is rejected outright and the
/// event is LOST on both the live `/hook/event` path and the drain. Stripping
/// keeps the event; a NUL carries no meaning in captured text. Cleans object
/// keys too (rebuilding the map only when a key actually contains one).
pub fn sanitize_nul(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::String(s) => {
            if s.contains('\u{0}') {
                s.retain(|c| c != '\u{0}');
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(sanitize_nul),
        serde_json::Value::Object(map) => {
            let needs_key_fix = map.keys().any(|k| k.contains('\u{0}'));
            for (_, val) in map.iter_mut() {
                sanitize_nul(val);
            }
            if needs_key_fix {
                *map = std::mem::take(map)
                    .into_iter()
                    .map(|(k, val)| (k.replace('\u{0}', ""), val))
                    .collect();
            }
        }
        _ => {}
    }
}

/// The event timestamp (epoch ms): a positive `ts` field on the payload when
/// present, otherwise `now_ms` (the drain-time fallback for legacy lines).
fn event_ts(payload: &serde_json::Value, now_ms: i64) -> i64 {
    payload.get("ts").and_then(|v| v.as_i64()).filter(|t| *t > 0).unwrap_or(now_ms)
}

/// Parse a JSONL spool into `(events, skipped)`. Blank lines and lines that
/// don't parse as a JSON object are skipped and counted — never fatal, so one
/// corrupt line can't strand the rest of the file.
fn parse_spool_lines(content: &[u8]) -> (Vec<serde_json::Value>, usize) {
    let mut events = Vec::new();
    let mut skipped = 0usize;
    // **BYTES, SPLIT PER LINE, DECODED PER LINE.** Decoding the whole buffer
    // first makes one malformed byte fatal for the file, which is the opposite
    // of what this function promises — see
    // `a_line_that_is_not_utf8_is_skipped_rather_than_stranding_the_file`.
    //
    // A spool file is append-only and written by many processes, so a partial
    // write can leave a multi-byte character cut in half. That is a property of
    // the transport, not of the events either side of it.
    for raw in content.split(|b| *b == b'\n') {
        let Ok(line) = std::str::from_utf8(raw) else {
            // Unreadable — the same bucket an invalid-JSON line lands in.
            skipped += 1;
            continue;
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) if v.is_object() => events.push(v),
            _ => skipped += 1,
        }
    }
    (events, skipped)
}

#[derive(Default, Debug, PartialEq, Eq)]
struct DrainStats {
    imported: usize,
    duplicate: usize,
    /// Lines that weren't valid JSON objects.
    skipped: usize,
    /// Rows the DB rejected as data (e.g. a payload with an embedded NUL escape,
    /// which Postgres jsonb can't store) — quarantined to
    /// `events.jsonl.rejected`, never dropped silently.
    errored: usize,
}

impl DrainStats {
    fn add(&mut self, other: DrainStats) {
        self.imported += other.imported;
        self.duplicate += other.duplicate;
        self.skipped += other.skipped;
        self.errored += other.errored;
    }
    fn total(&self) -> usize {
        self.imported + self.duplicate + self.skipped + self.errored
    }
}

/// Spawn the capture-spool drain for the daemon's lifetime.
pub fn spawn(pg: Arc<PgStore>, sensei_dir: PathBuf) {
    tokio::spawn(run(pg, sensei_dir));
}

async fn run(pg: Arc<PgStore>, sensei_dir: PathBuf) {
    // Cadence lives in `sensei.schedules` (name `capture_drain`). drain_once
    // already returns a Result, so the schedule row records a real outcome
    // rather than an unconditional success.
    let store = pg.clone();
    ticker::run_scheduled(pg, "capture_drain", move || {
        let (pg, dir) = (store.clone(), sensei_dir.clone());
        async move {
            let stats = drain_once(&pg, &dir).await.map_err(|e| e.to_string())?;
            if stats.total() > 0 {
                tracing::info!(
                    imported = stats.imported,
                    duplicate = stats.duplicate,
                    skipped = stats.skipped,
                    "capture_drain: imported {} dead-lettered hook event(s) into activity.assistant_events",
                    stats.imported,
                );
            }
            Ok(())
        }
    })
    .await;
}

/// One drain pass: sweep any leftover `.draining`, then rotate + import the
/// live spool. Returns aggregate stats.
async fn drain_once(pg: &PgStore, sensei_dir: &Path) -> Result<DrainStats, String> {
    let spool = sensei_dir.join("events.jsonl");
    let draining = sensei_dir.join("events.jsonl.draining");
    let mut stats = DrainStats::default();

    // A leftover `.draining` from a crashed prior drain — import it first so the
    // rotate below never clobbers it.
    if draining.exists() {
        stats.add(import_file(pg, &draining).await?);
    }

    // Rotate the live spool aside so new dead-letters append to a fresh file,
    // then import the rotated copy (rename is atomic within one dir).
    match std::fs::metadata(&spool) {
        Ok(m) if m.len() > 0 => {
            std::fs::rename(&spool, &draining).map_err(|e| format!("rotate spool: {e}"))?;
            stats.add(import_file(pg, &draining).await?);
        }
        _ => {} // no spool, or empty — nothing to drain
    }

    Ok(stats)
}

/// Import every line of `path` into `activity.assistant_events`, then delete it.
///
/// A per-row insert error does NOT abort the file — the row is collected and,
/// once the pass finishes, quarantined to `events.jsonl.rejected` so one poison
/// payload (e.g. an embedded NUL that jsonb rejects) can't strand the rest.
/// Neither does an unreadable LINE: the file is read as bytes and decoded one
/// line at a time, because `read_to_string` made a single truncated character
/// fatal for the whole file and so broke that promise before any row was seen.
/// The exception is a *total* failure (nothing imported or deduped, yet rows
/// errored): that signals the DB is unavailable rather than the data being bad,
/// so the file is left untouched for the next tick instead of quarantining real
/// events. Retries are idempotent via the payload dedup.
async fn import_file(pg: &PgStore, path: &Path) -> Result<DrainStats, String> {
    // `read`, not `read_to_string`: a spool file with one truncated character
    // must still yield every other line. See `parse_spool_lines`.
    let content = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let (mut events, skipped) = parse_spool_lines(&content);
    // Strip NULs the DB would reject, so a stray NUL byte in captured output
    // doesn't force an otherwise-good event into quarantine.
    events.iter_mut().for_each(sanitize_nul);
    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut stats = DrainStats { skipped, ..Default::default() };
    let mut rejected: Vec<&serde_json::Value> = Vec::new();
    let mut last_err = String::new();

    for payload in &events {
        let f = hook_event_fields(payload);
        let ts = event_ts(payload, now_ms);
        match pg
            .insert_hook_event_if_absent(
                f.session_id,
                f.family,
                f.event_type,
                f.tool_name,
                f.cwd,
                ts,
                f.success,
                payload,
            )
            .await
        {
            Ok(Some(_)) => stats.imported += 1,
            Ok(None) => stats.duplicate += 1,
            Err(e) => {
                last_err = e;
                rejected.push(payload);
            }
        }
    }

    // Nothing landed but rows errored → the DB is the problem, not the data.
    // Leave the file in place and retry next tick; don't quarantine real events.
    if stats.imported == 0 && stats.duplicate == 0 && !rejected.is_empty() {
        return Err(format!("all {} inserts failed (DB unavailable?): {last_err}", rejected.len()));
    }

    // Some rows landed → the failures are payloads the DB genuinely rejects.
    // Quarantine them (preserved, not dropped) and consume the file.
    if !rejected.is_empty() {
        quarantine(path, &rejected, &last_err);
        stats.errored = rejected.len();
    }

    let _ = std::fs::remove_file(path);
    Ok(stats)
}

/// Append DB-rejected payloads to `events.jsonl.rejected` (next to the spool)
/// so they're preserved for inspection instead of silently dropped. Best-effort.
fn quarantine(path: &Path, payloads: &[&serde_json::Value], last_err: &str) {
    use std::io::Write;
    let dest = path.with_file_name("events.jsonl.rejected");
    let Ok(mut fh) = std::fs::OpenOptions::new().create(true).append(true).open(&dest) else {
        tracing::warn!(count = payloads.len(), "capture_drain: could not open quarantine file");
        return;
    };
    for p in payloads {
        if let Ok(line) = serde_json::to_string(p) {
            let _ = writeln!(fh, "{line}");
        }
    }
    tracing::warn!(
        count = payloads.len(),
        error = %last_err,
        "capture_drain: quarantined hook event(s) the DB rejected → {}",
        dest.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hook_event_fields_maps_every_column_the_insert_consumes() {
        let p = json!({
            "session_id": "sess-1",
            "assistant_family": "cursor",
            "hook_event_name": "PostToolUse",
            "tool_name": "Edit",
            "cwd": "/repo",
            "exit_code": 0
        });
        let f = hook_event_fields(&p);
        assert_eq!(f.session_id, "sess-1");
        assert_eq!(f.family, "cursor");
        assert_eq!(f.event_type, "PostToolUse");
        assert_eq!(f.tool_name, Some("Edit"));
        assert_eq!(f.cwd, Some("/repo"));
        assert_eq!(f.success, Some(true)); // exit_code 0 → success
    }

    #[test]
    fn hook_event_fields_defaults_when_absent() {
        let empty = json!({});
        let f = hook_event_fields(&empty);
        assert_eq!(f.session_id, ""); // empty, not a fabricated id
        assert_eq!(f.family, "claude"); // family defaults to claude
        assert_eq!(f.event_type, "unknown"); // never null (DB column is NOT NULL)
        assert_eq!(f.tool_name, None);
        assert_eq!(f.cwd, None);
        assert_eq!(f.success, None); // no exit_code → unknown, not false
    }

    #[test]
    fn hook_event_fields_success_false_on_nonzero_exit() {
        let nonzero = json!({"exit_code": 1});
        let f = hook_event_fields(&nonzero);
        assert_eq!(f.success, Some(false));
    }

    #[test]
    fn event_ts_prefers_positive_payload_ts_then_now() {
        assert_eq!(event_ts(&json!({"ts": 1_700_000_000_000i64}), 42), 1_700_000_000_000);
        assert_eq!(event_ts(&json!({}), 42), 42); // no ts → now fallback
        assert_eq!(event_ts(&json!({"ts": 0}), 42), 42); // non-positive → now fallback
        assert_eq!(event_ts(&json!({"ts": "nope"}), 42), 42); // wrong type → now fallback
    }

    #[test]
    fn parse_spool_lines_skips_blank_and_invalid_keeps_objects() {
        let content = "\
{\"hook_event_name\":\"Stop\",\"session_id\":\"a\"}

not json
[1,2,3]
{\"hook_event_name\":\"PreToolUse\",\"session_id\":\"b\"}
";
        let (events, skipped) = parse_spool_lines(content.as_bytes());
        assert_eq!(events.len(), 2, "two valid objects survive");
        assert_eq!(skipped, 2, "the non-json line and the json array are skipped");
        assert_eq!(events[0]["session_id"], "a");
        assert_eq!(events[1]["session_id"], "b");
    }

    /// **ONE BAD BYTE MUST NOT STRAND THE FILE.**
    ///
    /// `import_file`'s own doc promises that a poison payload "can't strand the
    /// rest", and the row-level quarantine delivers that — but the promise was
    /// broken on the function's FIRST LINE, by a `read_to_string` that refuses
    /// the whole file when any byte in it is not valid UTF-8.
    ///
    /// LIVE COST, measured before this landed:
    /// the spool's `events.jsonl.draining` was 176 MB and 29,622 lines,
    /// stuck since Sep 16, and `capture_drain` had been failing every 5 minutes
    /// for 8 days with `stream did not contain valid UTF-8`. The bad bytes were
    /// `\xe2\x80` at offset 129,856,805 — the first two bytes of an em-dash,
    /// with the third lost to a partial write. Two bytes held back 29,621 good
    /// events.
    ///
    /// A line that is not UTF-8 is counted as SKIPPED, which is what `skipped`
    /// already means for a line that could not be read — an invalid-JSON line
    /// lands in the same bucket.
    ///
    /// MUTATION: decode the whole buffer up front instead of per line — this
    /// test's spool yields nothing at all and the poison pill is back.
    #[test]
    fn a_line_that_is_not_utf8_is_skipped_rather_than_stranding_the_file() {
        // A truncated em-dash: `\xe2\x80` with its third byte missing, exactly
        // the shape a partial write leaves behind.
        let mut spool: Vec<u8> = Vec::new();
        spool.extend_from_slice(b"{\"hook_event_name\":\"Stop\",\"session_id\":\"before\"}\n");
        spool.extend_from_slice(b"{\"hook_event_name\":\"Stop\",\"session_id\":\"bad");
        spool.extend_from_slice(&[0xE2, 0x80]);
        spool.extend_from_slice(b"\"}\n");
        spool.extend_from_slice(b"{\"hook_event_name\":\"PreToolUse\",\"session_id\":\"after\"}\n");

        let (events, skipped) = parse_spool_lines(&spool);
        assert_eq!(events.len(), 2, "the lines either side of the bad one survive");
        assert_eq!(skipped, 1, "the unreadable line is skipped, not fatal");
        assert_eq!(events[0]["session_id"], "before");
        assert_eq!(events[1]["session_id"], "after", "the file did not stop at the bad byte");
    }

    #[test]
    fn sanitize_nul_strips_from_nested_strings_keys_and_arrays() {
        let mut v = json!({
            "session_id": "a\u{0}b",
            "tool_input": { "content": "line1\u{0}line2", "arr": ["x\u{0}", "y"] },
            "bad\u{0}key": 1
        });
        sanitize_nul(&mut v);
        let s = serde_json::to_string(&v).unwrap();
        assert!(!s.contains('\u{0}'), "no NUL survives anywhere in the value");
        assert_eq!(v["session_id"], "ab");
        assert_eq!(v["tool_input"]["content"], "line1line2");
        assert_eq!(v["tool_input"]["arr"][0], "x");
        assert_eq!(v["badkey"], 1); // key de-NUL'd, value preserved
    }

    #[test]
    fn drain_stats_add_and_total() {
        let mut s = DrainStats::default();
        s.add(DrainStats { imported: 2, duplicate: 1, skipped: 3, errored: 1 });
        s.add(DrainStats { imported: 1, duplicate: 0, skipped: 0, errored: 0 });
        assert_eq!(s, DrainStats { imported: 3, duplicate: 1, skipped: 3, errored: 1 });
        assert_eq!(s.total(), 8);
    }
}
