//! Claude Code transcript adapter (#73). Reads
//! `~/.claude/projects/<enc>/<session_id>.jsonl`; the filename stem is the
//! session id (== our `client_session_id`). Each line is a JSON record; a turn
//! spans one genuine human prompt to the next, and the assistant prose is the
//! `text` content blocks (tool_use / thinking excluded).

use super::{
    MAX_LINE_BYTES, MAX_TRANSCRIPT_BYTES, MAX_TURN_CHARS, ParsedTranscript, SessionTokens,
    SynthEvent, SynthSession, TranscriptAdapter, TranscriptMessage, TranscriptTurn, TurnFacts,
    UnitRef, human_prompt_text, merge_facts, parse_timestamp, turn_attrs,
};
use std::path::{Path, PathBuf};

/// File mtime as epoch nanoseconds (the cursor change-stamp). `None` if the file
/// is gone / unreadable.
fn mtime_ns(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_nanos() as i64)
}

pub struct ClaudeAdapter {
    root: PathBuf,
}

impl ClaudeAdapter {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl TranscriptAdapter for ClaudeAdapter {
    fn source(&self) -> &'static str {
        "claude_code"
    }
    fn family(&self) -> &'static str {
        "claude"
    }

    fn units(&self) -> Vec<UnitRef> {
        // layout: <root>/<project-dir>/<session_id>.jsonl — the main session
        // transcripts (direct children). INTENTIONALLY does not recurse into
        // <session_id>/subagents/agent-*.jsonl (subagent sidechains belong to a
        // parent session; ingesting/attributing them is a #73 follow-up).
        let mut units = Vec::new();
        let Ok(projects) = std::fs::read_dir(&self.root) else {
            return units;
        };
        for proj in projects.flatten() {
            let Ok(entries) = std::fs::read_dir(proj.path()) else {
                continue;
            };
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) == Some("jsonl")
                    && let Some(stamp) = mtime_ns(&p)
                {
                    units.push(UnitRef { key: p.to_string_lossy().into_owned(), stamp });
                }
            }
        }
        units
    }

    fn stamp_for(&self, key: &str) -> Option<i64> {
        mtime_ns(Path::new(key))
    }

    fn session_id_for(&self, key: &str) -> Option<String> {
        Path::new(key).file_stem().and_then(|s| s.to_str()).map(str::to_string)
    }

    fn load_content(&self, key: &str) -> Option<String> {
        let path = Path::new(key);
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if size > MAX_TRANSCRIPT_BYTES {
            tracing::warn!(
                file = key,
                size_mb = size / 1_048_576,
                "transcript ingest: skipping oversized transcript"
            );
            return None;
        }
        std::fs::read_to_string(path)
            .map_err(|e| tracing::debug!(error = %e, file = key, "claude: read transcript failed"))
            .ok()
    }

    /// Streamed, with no whole-file cap (#280): the file is read line by line,
    /// ONCE (#336), so a transcript of any size is read in bounded memory — only
    /// an over-long single line is skipped, by the per-line guard. A 162 MB
    /// session transcript was skipped whole as "oversized" before this.
    fn parse_file(&self, key: &str) -> Option<ParsedTranscript> {
        // Lines are split on raw bytes and decoded one at a time, lossily, so a
        // line of invalid UTF-8 costs that line, not the rest of the file. An I/O
        // error mid-read fails the whole read: a partial transcript must not pass
        // for the session.
        let failed = std::cell::Cell::new(false);
        let lines = || -> Option<_> {
            use std::io::BufRead;
            let f = std::fs::File::open(key)
                .map_err(
                    |e| tracing::debug!(error = %e, file = key, "claude: open transcript failed"),
                )
                .ok()?;
            let failed = &failed;
            Some(std::io::BufReader::new(f).split(b'\n').map_while(move |r| match r {
                Ok(bytes) => Some(String::from_utf8_lossy(&bytes).into_owned()),
                Err(e) => {
                    tracing::warn!(error = %e, file = key, "claude: transcript read failed mid-file");
                    failed.set(true);
                    None
                }
            }))
        };
        let parsed = parse_claude_lines(lines()?);
        (!failed.get()).then_some(parsed)
    }

    fn parse(&self, content: &str) -> ParsedTranscript {
        parse_claude_lines(content.lines())
    }
}

/// The records of a Claude transcript: each line's number (counting every raw
/// line, so an id-less message keys the same however it is read) and its JSON.
/// Blank, over-long and malformed lines are skipped here, once, for every reader.
fn records<L: AsRef<str>>(
    lines: impl Iterator<Item = L>,
) -> impl Iterator<Item = (usize, serde_json::Value)> {
    lines.enumerate().filter_map(|(n, raw)| {
        let line = raw.as_ref().trim();
        // An over-long line is a blob or attachment: parsing a multi-MB JSON line
        // would stall the executor.
        if line.is_empty() || line.len() > MAX_LINE_BYTES {
            return None;
        }
        serde_json::from_str(line).ok().map(|v| (n, v))
    })
}

/// One of the things read out of a transcript, built a record at a time so that
/// every reader shares ONE parse of each line (#336).
trait Reader: Default {
    type Out;
    fn push(&mut self, n: usize, v: &serde_json::Value);
    fn finish(self) -> Self::Out;
}

/// Run one reader over a transcript's lines — the tests' view of each reader
/// alone; ingest reads through [`parse_claude_lines`].
#[cfg(test)]
fn read<R: Reader, L: AsRef<str>>(lines: impl Iterator<Item = L>) -> R::Out {
    let mut reader = R::default();
    for (n, v) in records(lines) {
        reader.push(n, &v);
    }
    reader.finish()
}

/// Everything ingest needs from a Claude transcript, in ONE pass over its lines.
///
/// The five readers used to take the lines five times, and each re-parsed every
/// line: 5 s apiece on a 1.28 GB transcript, so a session that kept writing was
/// abandoned by the 180 s watchdog over and over (#336).
pub(crate) fn parse_claude_lines<L: AsRef<str>>(
    lines: impl Iterator<Item = L>,
) -> ParsedTranscript {
    let mut session = SessionReader::default();
    let mut turns = TurnsReader::default();
    let mut model = ModelReader::default();
    let mut tokens = TokensReader::default();
    let mut messages = MessagesReader::default();
    for (n, v) in records(lines) {
        session.push(n, &v);
        turns.push(n, &v);
        model.push(n, &v);
        tokens.push(n, &v);
        messages.push(n, &v);
    }
    let session = session.finish();
    ParsedTranscript {
        turns: turns.finish(),
        cwds: session.as_ref().map(|s| s.cwds.clone()).unwrap_or_default(),
        events: session.map(|s| s.events).unwrap_or_default(),
        model: model.finish().map(|m| ("anthropic".to_string(), m)),
        tokens: tokens.finish(),
        messages: messages.finish(),
    }
}

/// Session-total token usage across a Claude transcript's assistant records, kept
/// SPLIT (fresh input / cache write / cache read / output), each response counted
/// once by `message.id`. `None` when no record carries usage (honest-empty). Pure.
#[cfg(test)]
pub fn claude_tokens(content: &str) -> Option<SessionTokens> {
    read::<TokensReader, _>(content.lines())
}

#[derive(Default)]
struct TokensReader {
    fresh: i64,
    cache_write: i64,
    cache_read: i64,
    output: i64,
    seen: bool,
    counted: std::collections::HashSet<String>,
}

impl Reader for TokensReader {
    type Out = Option<SessionTokens>;
    fn push(&mut self, _n: usize, v: &serde_json::Value) {
        if v.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            return;
        }
        let Some(u) = v.get("message").and_then(|m| m.get("usage")) else {
            return;
        };
        if let Some(id) = v["message"].get("id").and_then(|x| x.as_str())
            && !self.counted.insert(id.to_string())
        {
            return;
        }
        let g = |k: &str| u.get(k).and_then(|x| x.as_i64()).unwrap_or(0);
        let line_in =
            g("input_tokens") + g("cache_creation_input_tokens") + g("cache_read_input_tokens");
        let line_out = g("output_tokens");
        if line_in > 0 || line_out > 0 {
            self.seen = true;
            self.fresh += g("input_tokens");
            self.cache_write += g("cache_creation_input_tokens");
            self.cache_read += g("cache_read_input_tokens");
            self.output += line_out;
        }
    }
    fn finish(self) -> Self::Out {
        self.seen.then_some(SessionTokens {
            input: self.fresh,
            output: self.output,
            cache_read: Some(self.cache_read),
            cache_write: Some(self.cache_write),
            reasoning: None, // Claude does not separate reasoning tokens
            cost: None,      // no metered cost in the transcript
        })
    }
}

/// One row per API response (spec session-api-cost C2): a response written on
/// several records is one row (the last record's usage); a usage record with no
/// `message.id` is its own row; `<synthetic>` and model-less records are none.
/// Pure.
#[cfg(test)]
pub fn claude_messages(content: &str) -> Vec<TranscriptMessage> {
    read::<MessagesReader, _>(content.lines())
}

#[derive(Default)]
struct MessagesReader {
    out: Vec<TranscriptMessage>,
    at_index: std::collections::HashMap<String, usize>,
    turn: Option<i32>,
}

impl Reader for MessagesReader {
    type Out = Vec<TranscriptMessage>;
    fn push(&mut self, n: usize, v: &serde_json::Value) {
        match v.get("type").and_then(|t| t.as_str()) {
            Some("user") if human_prompt_text(v).is_some() => {
                self.turn = Some(self.turn.unwrap_or(0) + 1)
            }
            Some("assistant") => {
                let m = &v["message"];
                let (Some(u), Some(model)) =
                    (m.get("usage"), m.get("model").and_then(|x| x.as_str()))
                else {
                    return;
                };
                if model == "<synthetic>" {
                    return;
                }
                let n_of = |x: &serde_json::Value| x.as_i64().unwrap_or(0);
                let s_of = |x: &serde_json::Value| x.as_str().map(str::to_string);
                let split = u.get("cache_creation").filter(|c| {
                    c.get("ephemeral_5m_input_tokens").is_some()
                        || c.get("ephemeral_1h_input_tokens").is_some()
                });
                let total_write = n_of(&u["cache_creation_input_tokens"]);
                let id = m
                    .get("id")
                    .and_then(|x| x.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("line:{n}"));
                let row = TranscriptMessage {
                    message_id: id.clone(),
                    turn_index: self.turn,
                    at: parse_timestamp(v).map(|t| t.timestamp_millis()),
                    model: model.to_string(),
                    speed: s_of(&u["speed"]),
                    inference_geo: s_of(&u["inference_geo"]),
                    service_tier: s_of(&u["service_tier"]),
                    input: n_of(&u["input_tokens"]),
                    cache_write_5m: split
                        .map_or(total_write, |c| n_of(&c["ephemeral_5m_input_tokens"])),
                    cache_write_1h: split.map_or(0, |c| n_of(&c["ephemeral_1h_input_tokens"])),
                    cache_split_known: split.is_some(),
                    cache_read: n_of(&u["cache_read_input_tokens"]),
                    output: n_of(&u["output_tokens"]),
                    thinking: u["output_tokens_details"]["thinking_tokens"].as_i64(),
                };
                match self.at_index.get(&id) {
                    Some(&i) => self.out[i] = row,
                    None => {
                        self.at_index.insert(id, self.out.len());
                        self.out.push(row);
                    }
                }
            }
            _ => {}
        }
    }
    fn finish(self) -> Self::Out {
        self.out
    }
}

/// The model that ran a Claude transcript: the most frequent `message.model`
/// across assistant records (a session can switch models mid-run; the dominant
/// one wins). `None` if no assistant record carries a model. Pure.
#[cfg(test)]
pub fn claude_model(content: &str) -> Option<String> {
    read::<ModelReader, _>(content.lines())
}

#[derive(Default)]
struct ModelReader {
    counts: std::collections::HashMap<String, u32>,
}

impl Reader for ModelReader {
    type Out = Option<String>;
    fn push(&mut self, _n: usize, v: &serde_json::Value) {
        if v.get("type").and_then(|t| t.as_str()) == Some("assistant")
            && let Some(model) =
                v.get("message").and_then(|m| m.get("model")).and_then(|m| m.as_str())
            && !model.is_empty()
            && model != "<synthetic>"
        {
            *self.counts.entry(model.to_string()).or_default() += 1;
        }
    }
    fn finish(self) -> Self::Out {
        // Most frequent; ties broken by model name for determinism.
        self.counts
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
            .map(|(model, _)| model)
    }
}

/// Reconstruct a session's event stream from a Claude transcript (#75): a
/// `UserPromptSubmit` per genuine human prompt, a `PostToolUse` per `tool_use`
/// block (name + file_path), and a synthetic terminal `Stop` (transcripts carry
/// no end marker). Also collects distinct `cwd`s for workspace resolution. Pure.
#[cfg(test)]
pub fn parse_claude_session(content: &str) -> Option<SynthSession> {
    read::<SessionReader, _>(content.lines())
}

#[derive(Default)]
struct SessionReader {
    cwds: Vec<String>,
    events: Vec<SynthEvent>,
    max_ts: i64,
}

impl Reader for SessionReader {
    type Out = Option<SynthSession>;
    fn push(&mut self, _n: usize, v: &serde_json::Value) {
        if let Some(cwd) = v.get("cwd").and_then(|c| c.as_str())
            && !cwd.is_empty()
            && !self.cwds.iter().any(|c| c == cwd)
        {
            self.cwds.push(cwd.to_string());
        }
        let Some(ts) = parse_timestamp(v).map(|d| d.timestamp_millis()) else {
            return;
        };
        match v.get("type").and_then(|t| t.as_str()).unwrap_or("") {
            "user" => {
                if let Some(prompt) = human_prompt_text(v) {
                    self.events.push(SynthEvent {
                        event_type: "UserPromptSubmit".into(),
                        tool_name: None,
                        file_path: None,
                        prompt: Some(prompt),
                        tool_input: None,
                        ts,
                    });
                    self.max_ts = self.max_ts.max(ts);
                }
            }
            "assistant" => {
                if let Some(serde_json::Value::Array(blocks)) =
                    v.get("message").and_then(|m| m.get("content"))
                {
                    for b in blocks {
                        if b.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                            let tool_name =
                                b.get("name").and_then(|n| n.as_str()).map(str::to_string);
                            // The full tool_use input — the enrich worker derives
                            // call_info/plugin/method from it (bash command, skill/agent
                            // params, …), same as a live-captured event.
                            let tool_input = b.get("input").cloned();
                            let file_path = tool_input
                                .as_ref()
                                .and_then(|i| i.get("file_path").or_else(|| i.get("path")))
                                .and_then(|p| p.as_str())
                                .map(str::to_string);
                            self.events.push(SynthEvent {
                                event_type: "PostToolUse".into(),
                                tool_name,
                                file_path,
                                prompt: None,
                                tool_input,
                                ts,
                            });
                            self.max_ts = self.max_ts.max(ts);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    fn finish(mut self) -> Self::Out {
        if self.events.is_empty() {
            return None;
        }
        // Transcripts have no explicit end marker — synthesize a terminal Stop so
        // the enricher derives outcome=completed.
        self.events.push(SynthEvent {
            event_type: "Stop".into(),
            tool_name: None,
            file_path: None,
            prompt: None,
            tool_input: None,
            ts: self.max_ts,
        });
        Some(SynthSession { cwds: self.cwds, events: self.events })
    }
}

/// The per-turn attributes we PROMOTE to columns, lifted from one transcript
/// record. Everything not named here still survives in `attrs` — see
/// [`turn_attrs`] — so adding a signal later is a query, not a re-ingest.
///
/// Parse a Claude transcript (JSONL) into user-prompt-bounded turns. A new turn
/// starts at each genuine human prompt; assistant `text` blocks until the next
/// prompt form that turn's response. Pure + deterministic.
#[cfg(test)]
pub fn parse_claude_transcript(content: &str) -> Vec<TranscriptTurn> {
    read::<TurnsReader, _>(content.lines())
}

#[derive(Default)]
struct TurnsReader {
    turns: Vec<TranscriptTurn>,
    cur: Option<TranscriptTurn>,
    idx: i32,
}

impl Reader for TurnsReader {
    type Out = Vec<TranscriptTurn>;
    fn push(&mut self, _n: usize, v: &serde_json::Value) {
        match v.get("type").and_then(|t| t.as_str()).unwrap_or("") {
            "user" => {
                if let Some(prompt) = human_prompt_text(v) {
                    if let Some(t) = self.cur.take() {
                        self.turns.push(t);
                    }
                    self.idx += 1;
                    let mut facts = TurnFacts::default();
                    merge_facts(&mut facts, v);
                    self.cur = Some(TranscriptTurn {
                        turn_index: self.idx,
                        user_text: Some(prompt),
                        assistant_text: String::new(),
                        started_at: parse_timestamp(v),
                        attrs: turn_attrs(v, &["message", "attachment", "toolUseResult"]),
                        facts,
                    });
                }
                // tool_result / meta / injected → not a boundary, ignore.
            }
            "assistant" => {
                if let Some(t) = self.cur.as_mut() {
                    // A turn spans several assistant records — tokens sum, and the
                    // LAST stop_reason is the one that ended it.
                    merge_facts(&mut t.facts, v);
                    if let Some(sr) = v
                        .get("message")
                        .and_then(|m| m.get("stop_reason"))
                        .and_then(|x| x.as_str())
                        .filter(|x| !x.is_empty())
                    {
                        t.facts.stop_reason = Some(sr.to_string());
                    }
                    let text = assistant_text_blocks(v);
                    if !text.is_empty() {
                        if !t.assistant_text.is_empty() {
                            t.assistant_text.push_str("\n\n");
                        }
                        t.assistant_text.push_str(&text);
                    }
                }
                // assistant prose before any human prompt → ignore.
            }
            _ => {}
        }
    }
    fn finish(mut self) -> Self::Out {
        if let Some(t) = self.cur.take() {
            self.turns.push(t);
        }
        // Cap pathological turns.
        for t in self.turns.iter_mut() {
            if t.assistant_text.chars().count() > MAX_TURN_CHARS {
                let mut s: String = t.assistant_text.chars().take(MAX_TURN_CHARS).collect();
                s.push('…');
                t.assistant_text = s;
            }
        }
        self.turns
    }
}

/// Concatenated `text` blocks of an `assistant` record (prose only — no
/// thinking, no tool_use).
fn assistant_text_blocks(v: &serde_json::Value) -> String {
    let Some(serde_json::Value::Array(blocks)) = v.get("message").and_then(|m| m.get("content"))
    else {
        return String::new();
    };
    let mut s = String::new();
    for b in blocks {
        if b.get("type").and_then(|t| t.as_str()) == Some("text")
            && let Some(t) = b.get("text").and_then(|t| t.as_str())
        {
            let t = t.trim();
            if !t.is_empty() {
                if !s.is_empty() {
                    s.push_str("\n\n");
                }
                s.push_str(t);
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {

    use super::super::TranscriptAdapter;

    const TRANSCRIPT: &str = concat!(
        r#"{"type":"user","cwd":"/r","timestamp":"2026-06-20T10:00:00.000Z","message":{"role":"user","content":"go"}}"#,
        "\n",
        r#"{"type":"assistant","timestamp":"2026-06-20T10:00:05.000Z","message":{"model":"claude-opus-5-5","content":[{"type":"text","text":"done"}],"usage":{"input_tokens":2,"output_tokens":4,"cache_read_input_tokens":10327,"cache_creation_input_tokens":55095}}}"#,
        "\n",
    );

    /// #280: streaming a file reads exactly what parsing its text reads.
    #[test]
    fn streaming_a_file_reads_what_parsing_its_text_reads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.jsonl");
        std::fs::write(&path, TRANSCRIPT).unwrap();
        let ad = super::ClaudeAdapter::new(dir.path().to_path_buf());
        let streamed = ad.parse_file(&path.to_string_lossy()).expect("readable");
        assert_eq!(streamed, ad.parse(TRANSCRIPT));
        assert_eq!(streamed.turns.len(), 1);
    }

    /// #336: a transcript is read ONCE per ingest. Five readers each re-read and
    /// re-parsed the whole file — 5 × 5 s on a 1.28 GB transcript, so a session
    /// that kept writing was abandoned by the 180 s watchdog 117 times in 14
    /// days. One pass feeds all five, and gives exactly what the five gave.
    #[test]
    fn one_pass_reads_each_line_once_and_gives_what_the_five_readers_gave() {
        let text = concat!(
            r#"{"type":"user","cwd":"/r","timestamp":"2026-06-20T10:00:00.000Z","message":{"role":"user","content":"go"}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-06-20T10:00:05.000Z","message":{"id":"m1","model":"claude-opus-5-5","content":[{"type":"text","text":"first"},{"type":"tool_use","name":"Read","input":{"file_path":"/r/a.rs"}}],"usage":{"input_tokens":2,"output_tokens":4,"cache_read_input_tokens":10,"cache_creation_input_tokens":5}}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-06-20T10:00:06.000Z","message":{"id":"m1","model":"claude-opus-5-5","content":[{"type":"text","text":"again"}],"usage":{"input_tokens":2,"output_tokens":9,"cache_read_input_tokens":10,"cache_creation_input_tokens":5}}}"#,
            "\n",
            "\n",
            "not json\n",
            r#"{"type":"user","cwd":"/r/sub","timestamp":"2026-06-20T10:01:00.000Z","message":{"role":"user","content":"next"}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-06-20T10:01:05.000Z","message":{"model":"claude-sonnet-5-5","content":[{"type":"text","text":"second"}],"usage":{"input_tokens":1,"output_tokens":1}}}"#,
            "\n",
        );
        let read = std::cell::Cell::new(0usize);
        let once = super::parse_claude_lines(text.lines().inspect(|_| read.set(read.get() + 1)));
        assert_eq!(read.get(), text.lines().count(), "every line is read exactly once");

        let session = super::parse_claude_session(text);
        let five = super::ParsedTranscript {
            turns: super::parse_claude_transcript(text),
            cwds: session.as_ref().map(|s| s.cwds.clone()).unwrap_or_default(),
            events: session.map(|s| s.events).unwrap_or_default(),
            model: super::claude_model(text).map(|m| ("anthropic".to_string(), m)),
            tokens: super::claude_tokens(text),
            messages: super::claude_messages(text),
        };
        assert_eq!(once, five);
        assert_eq!((once.turns.len(), once.messages.len(), once.events.len()), (2, 2, 4));
    }

    /// One line of invalid UTF-8 must not end the read: `lines()` stopped at it
    /// and silently dropped every turn after, a partial transcript passed off as
    /// the whole. Lines are decoded one by one, lossily.
    #[test]
    fn an_invalid_utf8_line_does_not_cut_the_transcript_short() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.jsonl");
        let mut bytes = b"{\"type\":\"note\",\"x\":\"\xff\xfe\"}\n".to_vec();
        bytes.extend_from_slice(TRANSCRIPT.as_bytes());
        std::fs::write(&path, bytes).unwrap();
        let ad = super::ClaudeAdapter::new(dir.path().to_path_buf());

        let got = ad.parse_file(&path.to_string_lossy()).expect("readable");
        assert_eq!(got.turns.len(), 1, "the turn after the bad line is read");
    }

    /// #280: a transcript over the whole-file cap is still read. The cap guarded
    /// loading the file as one string; streaming needs only the per-line guard.
    /// This session's own transcript was 162 MB and was skipped as "oversized" on
    /// every pass, so the longest — and costliest — sessions had no turns at all.
    #[test]
    fn a_transcript_over_the_size_cap_is_still_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("long.jsonl");
        std::fs::write(&path, TRANSCRIPT).unwrap();
        // Sparse padding past the cap: one over-long line the per-line guard skips.
        let f = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        f.set_len(super::MAX_TRANSCRIPT_BYTES + 1).unwrap();
        let ad = super::ClaudeAdapter::new(dir.path().to_path_buf());

        let got = ad.parse_file(&path.to_string_lossy()).expect("read despite its size");
        assert_eq!(got.turns.len(), 1);
        assert_eq!(got.tokens.map(|t| t.total_input()), Some(65_424));
    }

    #[test]
    fn turn_keeps_split_tokens_and_promoted_attributes() {
        // The whole point of the split: `tokens_in` here is FRESH input only.
        // Summing it with cache_read (as the session-grain total does) is what makes
        // cost read ~10x high — measured against real transcripts, ~98% of that sum
        // is cache reads, which bill far cheaper.
        let c = concat!(
            r#"{"type":"user","cwd":"/r","gitBranch":"develop","effort":"xhigh","isSidechain":false,"#,
            r#""attributionSkill":"superpowers:brainstorming","attributionPlugin":"superpowers","#,
            r#""timestamp":"2026-06-20T10:00:00.000Z","message":{"role":"user","content":"go"}}"#,
            "\n",
            r#"{"type":"assistant","cwd":"/r","message":{"role":"assistant","model":"claude-opus-4-8","stop_reason":"tool_use","#,
            r#""usage":{"input_tokens":10,"cache_creation_input_tokens":100,"cache_read_input_tokens":900,"output_tokens":20,"service_tier":"standard"}},"#,
            r#""content":[{"type":"text","text":"ok"}]}"#,
            "\n",
            r#"{"type":"assistant","cwd":"/r","message":{"role":"assistant","model":"claude-opus-4-8","stop_reason":"end_turn","#,
            r#""usage":{"input_tokens":5,"cache_read_input_tokens":50,"output_tokens":8}},"content":[{"type":"text","text":"done"}]}"#,
            "\n",
        );
        let turns = parse_claude_transcript(c);
        assert_eq!(turns.len(), 1, "one user prompt = one turn");
        let f = &turns[0].facts;
        assert_eq!(
            f.tokens_in,
            Some(15),
            "fresh input only, summed over the turn's records (10+5)"
        );
        assert_eq!(f.cache_read, Some(950), "cache reads kept SEPARATE (900+50)");
        assert_eq!(f.cache_write, Some(100));
        assert_eq!(f.tokens_out, Some(28));
        // The LAST record's stop_reason is the one that ended the turn.
        assert_eq!(f.stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(f.skill.as_deref(), Some("superpowers:brainstorming"));
        assert_eq!(f.plugin.as_deref(), Some("superpowers"));
        assert_eq!(f.git_branch.as_deref(), Some("develop"));
        assert_eq!(f.effort.as_deref(), Some("xhigh"));
        assert_eq!(f.service_tier.as_deref(), Some("standard"));
        assert_eq!(f.is_sidechain, Some(false));
        // Unpromoted attributes survive verbatim rather than being dropped…
        assert_eq!(turns[0].attrs["cwd"], "/r");
        // …but the bulky prose does not (it is already in assistant_text).
        assert!(
            turns[0].attrs.get("message").and_then(|m| m.get("content")).is_none(),
            "message.content is not duplicated into attrs"
        );
    }

    #[test]
    fn turn_facts_are_null_not_zero_when_the_transcript_lacks_them() {
        // Honest-empty: a transcript with no usage/attribution must not record
        // fabricated zeros that a consumer can't tell from a real reading.
        let c = concat!(
            r#"{"type":"user","message":{"role":"user","content":"hi"}}"#,
            "\n",
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"yo"}]}}"#,
            "\n",
        );
        let turns = parse_claude_transcript(c);
        assert_eq!(turns.len(), 1);
        let f = &turns[0].facts;
        assert_eq!(f.tokens_in, None, "absent usage is null, never 0");
        assert_eq!(f.cache_read, None);
        assert_eq!(f.skill, None);
        assert_eq!(f.is_sidechain, None);
    }

    use super::*;

    // A compact transcript: prompt → (thinking+text+tool_use) → tool_result →
    // text, then a second prompt. Plus injected noise that must NOT split turns.
    const SAMPLE: &str = r#"
{"type":"user","timestamp":"2026-06-22T10:00:00.000Z","message":{"role":"user","content":"add a login page"}}
{"type":"assistant","timestamp":"2026-06-22T10:00:01.000Z","message":{"role":"assistant","content":[{"type":"thinking","thinking":"hmm"}]}}
{"type":"assistant","timestamp":"2026-06-22T10:00:02.000Z","message":{"role":"assistant","content":[{"type":"text","text":"On it — adding the page."}]}}
{"type":"assistant","timestamp":"2026-06-22T10:00:03.000Z","message":{"role":"assistant","content":[{"type":"tool_use","name":"Edit","input":{}}]}}
{"type":"user","timestamp":"2026-06-22T10:00:04.000Z","message":{"role":"user","content":[{"type":"tool_result","content":"ok"}]}}
{"type":"user","timestamp":"2026-06-22T10:00:05.000Z","isMeta":true,"message":{"role":"user","content":[{"type":"text","text":"Base directory for this skill: ..."}]}}
{"type":"user","timestamp":"2026-06-22T10:00:06.000Z","message":{"role":"user","content":"<task-notification>ping</task-notification>"}}
{"type":"assistant","timestamp":"2026-06-22T10:00:07.000Z","message":{"role":"assistant","content":[{"type":"text","text":"Done."}]}}
{"type":"user","timestamp":"2026-06-22T10:01:00.000Z","message":{"role":"user","content":[{"type":"text","text":"now wire it up"}]}}
{"type":"assistant","timestamp":"2026-06-22T10:01:01.000Z","message":{"role":"assistant","content":[{"type":"text","text":"Wired."}]}}
"#;

    #[test]
    fn splits_on_genuine_prompts_only() {
        let turns = parse_claude_transcript(SAMPLE);
        assert_eq!(
            turns.len(),
            2,
            "two human prompts ⇒ two turns (tool_result/meta/notification ignored)"
        );
        assert_eq!(turns[0].turn_index, 1);
        assert_eq!(turns[0].user_text.as_deref(), Some("add a login page"));
        // assistant text accumulates across messages within the turn; tool_use
        // and thinking are excluded.
        assert_eq!(turns[0].assistant_text, "On it — adding the page.\n\nDone.");
        assert_eq!(turns[1].turn_index, 2);
        assert_eq!(turns[1].user_text.as_deref(), Some("now wire it up"));
        assert_eq!(turns[1].assistant_text, "Wired.");
    }

    #[test]
    fn parses_started_at_from_prompt_timestamp() {
        let turns = parse_claude_transcript(SAMPLE);
        assert_eq!(turns[0].started_at.unwrap().to_rfc3339(), "2026-06-22T10:00:00+00:00");
    }

    #[test]
    fn human_prompt_text_rejects_tool_results_meta_and_noise() {
        let tool_result =
            serde_json::json!({"message":{"content":[{"type":"tool_result","content":"x"}]}});
        assert!(super::super::human_prompt_text(&tool_result).is_none());
        let meta =
            serde_json::json!({"isMeta":true,"message":{"content":[{"type":"text","text":"hi"}]}});
        assert!(super::super::human_prompt_text(&meta).is_none());
        let noise =
            serde_json::json!({"message":{"content":"<system-reminder>stuff</system-reminder>"}});
        assert!(super::super::human_prompt_text(&noise).is_none());
        let real = serde_json::json!({"message":{"content":"fix the bug"}});
        assert_eq!(super::super::human_prompt_text(&real).as_deref(), Some("fix the bug"));
    }

    #[test]
    fn ignores_malformed_lines_and_empty() {
        let turns = parse_claude_transcript(
            "not json\n\n{\"type\":\"user\",\"message\":{\"content\":\"hello\"}}\n",
        );
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].user_text.as_deref(), Some("hello"));
    }

    const SESS: &str = r#"
{"type":"user","cwd":"/repo","timestamp":"2026-06-22T10:00:00.000Z","message":{"role":"user","content":"fix the parser"}}
{"type":"assistant","cwd":"/repo","timestamp":"2026-06-22T10:00:02.000Z","message":{"role":"assistant","content":[{"type":"tool_use","name":"Edit","input":{"file_path":"/repo/src/x.rs"}}]}}
{"type":"assistant","cwd":"/repo","timestamp":"2026-06-22T10:00:03.000Z","message":{"role":"assistant","content":[{"type":"tool_use","name":"Bash","input":{"command":"cargo test"}}]}}
"#;

    #[test]
    fn parse_produces_the_common_structure() {
        // The adapter's single `parse` maps raw content → the shared ParsedTranscript
        // (turns + cwds + events + model + tokens) that persistence consumes — verified
        // here without a DB, so a format change is caught at the seam.
        let content = concat!(
            r#"{"type":"user","cwd":"/repo","timestamp":"2026-06-22T10:00:00.000Z","message":{"role":"user","content":"fix the parser"}}"#,
            "\n",
            r#"{"type":"assistant","cwd":"/repo","timestamp":"2026-06-22T10:00:02.000Z","message":{"role":"assistant","model":"claude-opus-4-8","usage":{"input_tokens":10,"cache_read_input_tokens":90,"output_tokens":20},"content":[{"type":"text","text":"On it."},{"type":"tool_use","name":"Edit","input":{"file_path":"/repo/src/x.rs"}}]}}"#,
            "\n",
        );
        let p = ClaudeAdapter::new(std::path::PathBuf::from("/tmp")).parse(content);
        assert_eq!(p.turns.len(), 1, "one user-bounded turn");
        assert_eq!(p.turns[0].user_text.as_deref(), Some("fix the parser"));
        assert_eq!(p.cwds, vec!["/repo".to_string()], "cwd collected for workspace resolution");
        let kinds: Vec<&str> = p.events.iter().map(|e| e.event_type.as_str()).collect();
        assert_eq!(kinds, vec!["UserPromptSubmit", "PostToolUse", "Stop"]);
        assert_eq!(p.model, Some(("anthropic".to_string(), "claude-opus-4-8".to_string())));
        let t = p.tokens.expect("usage parsed");
        assert_eq!(t.input, 10, "FRESH input only — cache is no longer folded in");
        assert_eq!(t.cache_read, Some(90));
        assert_eq!(t.output, 20);
        assert_eq!(t.total_input(), 100, "the old tokens_in meaning is preserved");
    }

    #[test]
    fn parse_session_reconstructs_events() {
        let s = parse_claude_session(SESS).unwrap();
        assert_eq!(
            s.cwds,
            vec!["/repo".to_string()],
            "collects distinct cwd for workspace resolution"
        );
        let kinds: Vec<&str> = s.events.iter().map(|e| e.event_type.as_str()).collect();
        assert_eq!(
            kinds,
            vec!["UserPromptSubmit", "PostToolUse", "PostToolUse", "Stop"],
            "prompts + tool_uses + synthetic Stop"
        );
        assert_eq!(s.events[0].prompt.as_deref(), Some("fix the parser"));
        let edit = s.events.iter().find(|e| e.tool_name.as_deref() == Some("Edit")).unwrap();
        assert_eq!(
            edit.file_path.as_deref(),
            Some("/repo/src/x.rs"),
            "Edit carries file_path (the churn signal)"
        );
        let stop = s.events.last().unwrap();
        assert_eq!(
            stop.ts,
            s.events.iter().map(|e| e.ts).max().unwrap(),
            "Stop at the last timestamp"
        );
    }

    #[test]
    fn parse_session_none_when_no_events() {
        assert!(parse_claude_session("").is_none());
        assert!(parse_claude_session("{\"type\":\"summary\"}\n").is_none());
    }

    #[test]
    fn claude_model_picks_dominant_model() {
        let t = r#"
{"type":"user","message":{"content":"hi"}}
{"type":"assistant","message":{"model":"claude-opus-4-8","content":[{"type":"text","text":"a"}]}}
{"type":"assistant","message":{"model":"claude-opus-4-8","content":[{"type":"text","text":"b"}]}}
{"type":"assistant","message":{"model":"claude-sonnet-4-6","content":[{"type":"text","text":"c"}]}}
"#;
        assert_eq!(claude_model(t).as_deref(), Some("claude-opus-4-8"), "most frequent model wins");
        assert_eq!(
            claude_model("{\"type\":\"user\",\"message\":{\"content\":\"hi\"}}").as_deref(),
            None,
            "no model ⇒ None"
        );
    }

    #[test]
    fn claude_tokens_sums_usage_including_cache() {
        let t = r#"
{"type":"user","message":{"content":"hi"}}
{"type":"assistant","message":{"model":"claude-opus-4-8","usage":{"input_tokens":10,"cache_creation_input_tokens":100,"cache_read_input_tokens":50,"output_tokens":20}}}
{"type":"assistant","message":{"model":"claude-opus-4-8","usage":{"input_tokens":5,"output_tokens":8}}}
"#;
        // Split across the turn's records: fresh 10+5, write 100+0, read 50+0.
        let got = claude_tokens(t).expect("usage parsed");
        assert_eq!(got.input, 15, "fresh input only");
        assert_eq!(got.cache_write, Some(100));
        assert_eq!(got.cache_read, Some(50));
        assert_eq!(got.output, 28);
        // total_input() still reports what tokens_in always meant.
        assert_eq!(got.total_input(), 165);
        // no usage anywhere ⇒ honest-None, never a fabricated (0,0)
        assert_eq!(claude_tokens("{\"type\":\"user\",\"message\":{\"content\":\"hi\"}}"), None);
    }

    // ── Session API cost C1/C2 (docs/spec/daemon/session-api-cost.md) ──

    /// One API response written on three lines (one per content block), each
    /// repeating the response's usage — Claude Code's format — after a prompt.
    const SPLIT_MESSAGE: &str = concat!(
        r#"{"type":"user","timestamp":"2026-10-20T10:00:00.000Z","message":{"role":"user","content":"go"}}"#,
        "\n",
        r#"{"type":"assistant","timestamp":"2026-10-20T10:00:05.000Z","message":{"id":"msg_A","model":"claude-opus-5-5","role":"assistant","#,
        r#""usage":{"input_tokens":2,"cache_creation_input_tokens":725,"cache_read_input_tokens":26784,"output_tokens":313,"#,
        r#""cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":725},"#,
        r#""output_tokens_details":{"thinking_tokens":60},"speed":"standard","inference_geo":"global","service_tier":"standard"},"#,
        r#""content":[{"type":"thinking","thinking":"hmm"}]}}"#,
        "\n",
        r#"{"type":"assistant","timestamp":"2026-10-20T10:00:05.000Z","message":{"id":"msg_A","model":"claude-opus-5-5","role":"assistant","#,
        r#""usage":{"input_tokens":2,"cache_creation_input_tokens":725,"cache_read_input_tokens":26784,"output_tokens":313,"#,
        r#""cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":725},"#,
        r#""output_tokens_details":{"thinking_tokens":60},"speed":"standard","inference_geo":"global","service_tier":"standard"},"#,
        r#""content":[{"type":"text","text":"on it"}]}}"#,
        "\n",
        r#"{"type":"assistant","timestamp":"2026-10-20T10:00:05.000Z","message":{"id":"msg_A","model":"claude-opus-5-5","role":"assistant","#,
        r#""usage":{"input_tokens":2,"cache_creation_input_tokens":725,"cache_read_input_tokens":26784,"output_tokens":313,"#,
        r#""cache_creation":{"ephemeral_5m_input_tokens":0,"ephemeral_1h_input_tokens":725},"#,
        r#""output_tokens_details":{"thinking_tokens":60},"speed":"standard","inference_geo":"global","service_tier":"standard"},"#,
        r#""content":[{"type":"tool_use","name":"Edit","input":{}}]}}"#,
        "\n",
    );

    #[test]
    fn a_response_written_on_several_lines_counts_once() {
        let turns = parse_claude_transcript(SPLIT_MESSAGE);
        let f = &turns[0].facts;
        assert_eq!(
            (f.tokens_in, f.cache_write, f.cache_read, f.tokens_out),
            (Some(2), Some(725), Some(26784), Some(313)),
            "the turn counts msg_A once, not three times"
        );
        let t = claude_tokens(SPLIT_MESSAGE).unwrap();
        assert_eq!(
            (t.input, t.cache_write, t.cache_read, t.output),
            (2, Some(725), Some(26784), 313)
        );
        assert_eq!(claude_messages(SPLIT_MESSAGE).len(), 1);
    }

    #[test]
    fn a_message_row_keeps_what_its_price_needs() {
        let m = &claude_messages(SPLIT_MESSAGE)[0];
        assert_eq!(m.message_id, "msg_A");
        assert_eq!(m.turn_index, Some(1));
        assert_eq!(m.model, "claude-opus-5-5");
        assert_eq!(
            m.at,
            Some(
                chrono::DateTime::parse_from_rfc3339("2026-10-20T10:00:05Z")
                    .unwrap()
                    .timestamp_millis()
            )
        );
        assert_eq!((m.input, m.cache_read, m.output), (2, 26784, 313));
        assert_eq!((m.cache_write_5m, m.cache_write_1h, m.cache_split_known), (0, 725, true));
        assert_eq!(m.thinking, Some(60), "a breakdown of output, never added to it");
        assert_eq!(
            (m.speed.as_deref(), m.inference_geo.as_deref(), m.service_tier.as_deref()),
            (Some("standard"), Some("global"), Some("standard"))
        );
    }

    #[test]
    fn an_unsplit_cache_write_is_marked_unknown_and_lines_without_an_id_count_each() {
        let c = concat!(
            r#"{"type":"assistant","timestamp":"2026-10-20T09:59:00.000Z","message":{"id":"msg_0","model":"claude-haiku-4-5-20251001","#,
            r#""usage":{"input_tokens":1,"cache_creation_input_tokens":40,"output_tokens":3}}}"#,
            "\n",
            r#"{"type":"user","timestamp":"2026-10-20T10:00:00.000Z","message":{"role":"user","content":"go"}}"#,
            "\n",
            r#"{"type":"assistant","message":{"model":"claude-opus-5-5","usage":{"input_tokens":5,"output_tokens":7}}}"#,
            "\n",
            r#"{"type":"assistant","message":{"model":"claude-opus-5-5","usage":{"input_tokens":5,"output_tokens":7}}}"#,
            "\n",
            r#"{"type":"assistant","message":{"id":"msg_s","model":"<synthetic>","usage":{"input_tokens":0,"output_tokens":0}}}"#,
            "\n",
        );
        let ms = claude_messages(c);
        assert_eq!(ms.len(), 3, "msg_0 + two id-less lines; <synthetic> is not a message");
        let first = &ms[0];
        assert_eq!(first.turn_index, None, "before the first prompt");
        assert_eq!(first.model, "claude-haiku-4-5-20251001");
        assert_eq!(
            (first.cache_write_5m, first.cache_write_1h, first.cache_split_known),
            (40, 0, false)
        );
        assert_eq!(first.thinking, None, "no breakdown is unknown, not 0");
        assert_ne!(ms[1].message_id, ms[2].message_id, "each id-less line is its own message");
        assert_eq!(parse_claude_transcript(c)[0].facts.tokens_out, Some(14));
    }
}
