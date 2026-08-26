//! Transcript ingestion (#73) — backfill the assistant/user prose the hook
//! stream lacks, from agent transcripts. Resumable per-adapter workers: each
//! adapter knows a source's cache layout + id scheme and parses files into
//! turns; ingestion advances a per-file cursor so re-runs only touch changed
//! files. Ingest is LLM-free — the analyzer's LLM tiers consume the corpus
//! selectively.
//!
//! Chunked execution: `IngestCaptures` is a *dispatcher* that enqueues one
//! `IngestCapture` task per changed transcript, so ingestion interleaves
//! with other work (scans, etc.) and one huge/bad file can't block the rest.

pub mod claude;
pub mod copilot_cli;
pub mod cursor;
pub mod opencode;
pub mod vscode;
pub mod zed;

use crate::tasks::executor::TaskContext;
use crate::tasks::{Task, TaskKind};
use std::path::PathBuf;

// ── Shared constants (used by multiple adapters) ────────────────────────

/// Cap stored assistant prose per turn (safety net for pathological turns).
pub(crate) const MAX_TURN_CHARS: usize = 50_000;

/// Skip transcript files larger than this (logged). A multi-hundred-MB file
/// would spike memory on read and block the executor on parse; rare outlier.
pub(crate) const MAX_TRANSCRIPT_BYTES: u64 = 64 * 1024 * 1024;

/// Skip any single transcript line larger than this — a line this big is a
/// base64 attachment / blob, not prose, and parsing it stalls the executor.
pub(crate) const MAX_LINE_BYTES: usize = 4 * 1024 * 1024;

/// Leading markers that mark an injected (non-human) "user" message — harness
/// notifications, hook context, slash-command echoes. These are not turn
/// boundaries.
pub(crate) const INJECTED_MARKERS: &[&str] = &[
    "<task-notification",
    "<system-reminder",
    "<command-name",
    "<command-message",
    "<local-command",
    "Caveat:",
    "## Security Guidance",
];

/// The human prompt text of a `user` record, or `None` if it's a tool result,
/// a meta/injected message, or empty.
pub(crate) fn human_prompt_text(v: &serde_json::Value) -> Option<String> {
    if v.get("isMeta").and_then(|m| m.as_bool()) == Some(true) {
        return None;
    }
    let content = v.get("message").and_then(|m| m.get("content"))?;
    let text = match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(blocks) => {
            if blocks.iter().any(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_result"))
            {
                return None;
            }
            let mut s = String::new();
            for b in blocks {
                if b.get("type").and_then(|t| t.as_str()) == Some("text")
                    && let Some(t) = b.get("text").and_then(|t| t.as_str())
                {
                    if !s.is_empty() {
                        s.push('\n');
                    }
                    s.push_str(t);
                }
            }
            s
        }
        _ => return None,
    };
    let trimmed = text.trim();
    if trimmed.is_empty() || is_injected_noise(trimmed) {
        return None;
    }
    Some(trimmed.to_string())
}

/// Check whether a prompt string is injected noise (not a genuine human turn).
fn is_injected_noise(text: &str) -> bool {
    INJECTED_MARKERS.iter().any(|m| text.starts_with(m))
}

/// Parse a timestamp from a JSON record's `timestamp` field (RFC 3339).
pub(crate) fn parse_timestamp(v: &serde_json::Value) -> Option<chrono::DateTime<chrono::Utc>> {
    let ts = v.get("timestamp").and_then(|t| t.as_str())?;
    chrono::DateTime::parse_from_rfc3339(ts).ok().map(|d| d.with_timezone(&chrono::Utc))
}

/// Parse a timestamp from a JSON record's `timestamp` field as epoch milliseconds.
pub(crate) fn parse_timestamp_ms(v: &serde_json::Value) -> Option<i64> {
    let ts = v.get("timestamp").and_then(|t| t.as_str())?;
    let dt = chrono::DateTime::parse_from_rfc3339(ts).ok()?;
    Some(dt.timestamp_millis())
}

/// Promote per-record signals into `TurnFacts`. Shared across adapters that use
/// the Claude-style JSON structure (`message.usage`, `gitBranch`, etc.).
/// Sum across the turn's records; LAST `stop_reason` wins.
pub(crate) fn merge_facts(facts: &mut TurnFacts, v: &serde_json::Value) {
    let str_of = |x: Option<&serde_json::Value>| {
        x.and_then(|t| t.as_str()).filter(|t| !t.is_empty()).map(str::to_string)
    };
    if facts.git_branch.is_none() {
        facts.git_branch = str_of(v.get("gitBranch"));
    }
    if facts.effort.is_none() {
        facts.effort = str_of(v.get("effort"));
    }
    if facts.skill.is_none() {
        facts.skill = str_of(v.get("attributionSkill"));
    }
    if facts.plugin.is_none() {
        facts.plugin = str_of(v.get("attributionPlugin"));
    }
    if facts.is_sidechain.is_none() {
        facts.is_sidechain = v.get("isSidechain").and_then(|b| b.as_bool());
    }
    let Some(m) = v.get("message") else { return };
    if facts.stop_reason.is_none() {
        facts.stop_reason = str_of(m.get("stop_reason"));
    }
    let Some(u) = m.get("usage") else { return };
    if facts.service_tier.is_none() {
        facts.service_tier = str_of(u.get("service_tier"));
    }
    let add = |slot: &mut Option<i64>, n: Option<i64>| {
        if let Some(n) = n {
            *slot = Some(slot.unwrap_or(0) + n);
        }
    };
    add(&mut facts.tokens_in, u.get("input_tokens").and_then(|x| x.as_i64()));
    add(&mut facts.tokens_out, u.get("output_tokens").and_then(|x| x.as_i64()));
    add(&mut facts.cache_read, u.get("cache_read_input_tokens").and_then(|x| x.as_i64()));
    add(&mut facts.cache_write, u.get("cache_creation_input_tokens").and_then(|x| x.as_i64()));
}

/// Strip bulky prose/content from a record, keep metadata verbatim.
/// `drop_keys` are top-level keys to exclude (e.g. `["message"]`).
/// Small high-signal sub-keys of `message` (model, stop_reason, usage) are kept.
pub(crate) fn turn_attrs(v: &serde_json::Value, drop_keys: &[&str]) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    if let Some(obj) = v.as_object() {
        for (k, val) in obj {
            if drop_keys.contains(&k.as_str()) {
                continue;
            }
            out.insert(k.clone(), val.clone());
        }
    }
    if let Some(m) = v.get("message").and_then(|m| m.as_object()) {
        let mut mm = serde_json::Map::new();
        for k in ["id", "model", "stop_reason", "stop_sequence", "usage", "container"] {
            if let Some(val) = m.get(k) {
                mm.insert(k.to_string(), val.clone());
            }
        }
        if !mm.is_empty() {
            out.insert("message".into(), serde_json::Value::Object(mm));
        }
    }
    serde_json::Value::Object(out)
}

/// One user-prompt -> assistant-response turn parsed from a transcript.
#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptTurn {
    pub turn_index: i32,
    pub user_text: Option<String>,
    pub assistant_text: String,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Everything else the transcript record carried, verbatim. Adapters see far
    /// more than we model; anything unpromoted used to be discarded at parse time
    /// and was then unrecoverable without the original file. Default `{}` for an
    /// adapter that doesn't (yet) collect it.
    pub attrs: serde_json::Value,
    /// Per-turn signals promoted out of `attrs` because something reads them.
    /// All `Option` — a transcript that doesn't carry one stays honest-null rather
    /// than recording a fabricated 0/false.
    pub facts: TurnFacts,
}

impl Default for TranscriptTurn {
    /// `attrs` defaults to an empty OBJECT, not `serde_json::Value::default()` —
    /// which is JSON `null`. A derived Default wrote `null` into a column
    /// documented as `default '{}'`, so adapters that don't collect attributes yet
    /// (Zed, OpenCode) stored a different empty than the DDL promises, and
    /// `attrs ? 'key'` / `attrs->>'k'` behave differently on the two.
    fn default() -> Self {
        Self {
            turn_index: 0,
            user_text: None,
            assistant_text: String::new(),
            started_at: None,
            attrs: serde_json::Value::Object(serde_json::Map::new()),
            facts: TurnFacts::default(),
        }
    }
}

/// Promoted per-turn attributes. Token counts are kept SPLIT — folding cache reads
/// into a single input total is what makes the session-grain `tokens_in` read ~10x
/// high for cost (measured: ~98% of it is cache reads).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TurnFacts {
    pub tokens_in: Option<i64>,
    pub tokens_out: Option<i64>,
    pub cache_read: Option<i64>,
    pub cache_write: Option<i64>,
    pub stop_reason: Option<String>,
    pub is_sidechain: Option<bool>,
    pub skill: Option<String>,
    pub plugin: Option<String>,
    pub git_branch: Option<String>,
    pub effort: Option<String>,
    pub service_tier: Option<String>,
}

/// A synthesized hook-stream event reconstructed from a transcript (#75) — the
/// transcript is a superset of the live hook stream, so we can rebuild the
/// events the analyzer enriches from.
#[derive(Debug, Clone, PartialEq)]
pub struct SynthEvent {
    pub event_type: String, // UserPromptSubmit | PostToolUse | Stop
    pub tool_name: Option<String>,
    pub file_path: Option<String>,
    pub prompt: Option<String>,
    /// The tool call's FULL `input` object (the bash command, skill name, agent
    /// params, …), carried verbatim from the transcript's `tool_use.input` so the
    /// enrich worker can derive `call_info`/`plugin`/`method` from a backfilled event
    /// exactly as it does from a live-captured one. `None` for non-tool events.
    pub tool_input: Option<serde_json::Value>,
    pub ts: i64, // ms epoch
}

/// A session reconstructed from a transcript: the cwds seen (for project
/// resolution) + the synthesized event stream (#75). Produced by the per-adapter
/// `parse_*_session` helpers and folded into [`ParsedTranscript`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SynthSession {
    pub cwds: Vec<String>,
    pub events: Vec<SynthEvent>,
}

/// The common, adapter-agnostic structured form of one transcript unit — the
/// single contract between the per-source parsers and the persistence layer.
///
/// Every adapter's [`TranscriptAdapter::parse`] produces exactly this, and the
/// coordinator persists ONLY from it (turns → `transcript_turns`, events →
/// `assistant_events`, cwds → project resolution, model + tokens → `sessions`).
/// Nothing downstream of `parse` sees adapter-specific raw content, so a format or
/// parser change in any source never touches persistence, and the whole mapping is
/// independently verifiable — a test asserts the struct from a fixture, no DB.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParsedTranscript {
    /// Prose turns → `activity.transcript_turns`.
    pub turns: Vec<TranscriptTurn>,
    /// Distinct working directories seen → project resolution.
    pub cwds: Vec<String>,
    /// Reconstructed hook-stream events → `activity.assistant_events` (#75). Empty
    /// when the source can't synthesize a session.
    pub events: Vec<SynthEvent>,
    /// Inference `(provider, model)` → `activity.sessions`. `None` when absent.
    pub model: Option<(String, String)>,
    /// Session-total usage → `activity.sessions`. `None` when the source records
    /// none (honest-empty — the coordinator leaves the columns NULL, never a
    /// fabricated 0).
    pub tokens: Option<SessionTokens>,
}

/// One session's total usage, kept SPLIT.
///
/// Session grain on purpose. Claude reports usage per assistant message, but Zed
/// and OpenCode only carry a running total for the whole thread — so spreading
/// those across turns would invent per-turn readings, and summing the repeated
/// total across turns would multiply it. The totals therefore live on
/// `activity.sessions`, where every source can populate them honestly, while
/// `transcript_turns.tokens_*` stays per-turn for the source that genuinely has
/// it.
///
/// `cache_read` matters most: measured across real transcripts it is ~98% of all
/// input, and it bills far cheaper — so folding it into one `tokens_in` makes any
/// cost read from that number roughly 8x high and moves it the wrong way when
/// caching improves.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SessionTokens {
    /// Fresh input only — NOT including cache.
    pub input: i64,
    pub output: i64,
    pub cache_read: Option<i64>,
    pub cache_write: Option<i64>,
    /// Reasoning/thinking tokens, where the source separates them (OpenCode).
    pub reasoning: Option<i64>,
    /// Metered cost in whole currency units, where the source knows it (OpenCode).
    /// `None` on a subscription plan — and 0.0 IS a real reading there, not an
    /// absence, so the two must stay distinguishable.
    pub cost: Option<f64>,
}

impl SessionTokens {
    /// Everything the model processed on the input side: fresh + cache write +
    /// cache read. What the old `(tokens_in, tokens_out)` pair reported as
    /// `tokens_in`, preserved so existing consumers keep their meaning.
    pub fn total_input(&self) -> i64 {
        self.input + self.cache_write.unwrap_or(0) + self.cache_read.unwrap_or(0)
    }
}

/// A logical ingest unit — a transcript file (Claude) or a DB row (Zed) — with a
/// monotonic change-stamp the cursor uses to skip unchanged units. `key`
/// uniquely identifies the unit within its source.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitRef {
    pub key: String,
    pub stamp: i64,
}

/// A per-source transcript reader. Parsing is pure + deterministic (testable);
/// the coordinator owns the cursor + DB writes. Sources expose their content as
/// logical *units* (files for Claude, threads-in-a-SQLite-DB for Zed) rather
/// than assuming a file-per-session layout, so new sources are drop-in.
pub trait TranscriptAdapter: Send + Sync {
    /// Capture origin (distinct from the model `family`), e.g. "claude_code" / "zed".
    fn source(&self) -> &'static str;
    /// Harness/agent family (the `sensei.assistant_family` enum: "claude", "zed", …).
    /// The precise per-unit model, when known, is carried on [`ParsedTranscript::model`].
    fn family(&self) -> &'static str;
    /// Units this adapter can ingest, each with a change-stamp for cursor skipping.
    fn units(&self) -> Vec<UnitRef>;
    /// Cheap change-stamp for a unit key (no content read) — the cursor
    /// pre-check reads this. `None` ⇒ the unit no longer exists.
    fn stamp_for(&self, key: &str) -> Option<i64>;
    /// Session id for a unit key (Claude: filename stem; Zed: `zed-<thread_id>`).
    fn session_id_for(&self, key: &str) -> Option<String>;
    /// Load a unit's raw content (the expensive read/decompress). `None` ⇒ gone
    /// or oversized (logged, never panics on bad data).
    fn load_content(&self, key: &str) -> Option<String>;
    /// Convert a unit's raw content into the common [`ParsedTranscript`] — the ONE
    /// method carrying adapter-specific format knowledge. Everything the persistence
    /// layer needs (turns, cwds, events, model, tokens) comes out of this single
    /// struct, so persistence never sees raw content. Pure + deterministic (unit-
    /// testable without a DB); a source that can't reconstruct events returns them
    /// empty and/or `model`/`tokens` as `None`.
    fn parse(&self, content: &str) -> ParsedTranscript;
}

#[cfg(test)]
#[derive(Debug, Default, PartialEq)]
pub struct BackfillReport {
    pub files_seen: u32,
    pub files_ingested: u32,
    pub files_skipped: u32,
    pub turns_upserted: u32,
}

/// Outcome of ingesting one transcript file.
struct IngestOutcome {
    #[cfg_attr(not(test), allow(dead_code))] // read by backfill_all (test helper)
    skipped: bool,
    turns: u32,
    /// Project to (re)analyze because a historical session was synthesized (#75).
    analyze_project: Option<uuid::Uuid>,
}

impl IngestOutcome {
    fn skipped() -> Self {
        Self { skipped: true, turns: 0, analyze_project: None }
    }
}

fn claude_root() -> PathBuf {
    crate::paths::home().join(".claude/projects")
}

fn zed_db_path() -> PathBuf {
    crate::paths::home().join("Library/Application Support/Zed/threads/threads.db")
}

fn opencode_db_path() -> PathBuf {
    crate::paths::home().join(".local/share/opencode/opencode.db")
}

fn cursor_transcript_root() -> PathBuf {
    crate::paths::home().join(".cursor/projects")
}

fn vscode_user_root(variant: &str) -> Option<PathBuf> {
    // variant = "Code" | "Code - Insiders" | "VSCodium" | "Code - OSS"
    #[cfg(target_os = "macos")]
    {
        Some(crate::paths::home().join(format!("Library/Application Support/{variant}/User")))
    }
    #[cfg(target_os = "linux")]
    {
        Some(crate::paths::home().join(format!(".config/{variant}/User")))
    }
    #[cfg(target_os = "windows")]
    {
        dirs::data_dir().map(|d| d.join(format!("{variant}/User")))
    }
}

fn copilot_home() -> PathBuf {
    std::env::var("COPILOT_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| crate::paths::home().join(".copilot"))
}

/// All configured transcript adapters.
fn adapters() -> Vec<Box<dyn TranscriptAdapter>> {
    vec![
        Box::new(claude::ClaudeAdapter::new(claude_root())),
        Box::new(zed::ZedAdapter::new(zed_db_path())),
        Box::new(opencode::OpenCodeAdapter::new(opencode_db_path())),
        Box::new(cursor::CursorAdapter::new(cursor_transcript_root())),
        Box::new(vscode::VscodeAdapter::new(vscode_user_root("Code").unwrap_or_else(claude_root))),
        Box::new(copilot_cli::CopilotCliAdapter::new(copilot_home())),
    ]
}

/// Resolve an adapter for a capture source (used by the per-unit handler — the
/// root/db-path only matters for `units`, which the per-unit path doesn't use).
fn adapter_for_source(source: &str) -> Option<Box<dyn TranscriptAdapter>> {
    match source {
        "claude_code" => Some(Box::new(claude::ClaudeAdapter::new(claude_root()))),
        "zed" => Some(Box::new(zed::ZedAdapter::new(zed_db_path()))),
        "opencode" => Some(Box::new(opencode::OpenCodeAdapter::new(opencode_db_path()))),
        "cursor" => Some(Box::new(cursor::CursorAdapter::new(cursor_transcript_root()))),
        "vscode" => Some(Box::new(vscode::VscodeAdapter::new(
            vscode_user_root("Code").unwrap_or_else(claude_root),
        ))),
        "copilot_cli" => Some(Box::new(copilot_cli::CopilotCliAdapter::new(copilot_home()))),
        _ => None,
    }
}

/// Ingest one transcript unit (a file for Claude, a thread row for Zed): skip if
/// unchanged since last ingest (cursor) or gone/oversized, else parse + upsert
/// turns + advance the cursor. Idempotent.
async fn ingest_one(
    pg: &crate::db::pg_store::PgStore,
    adapter: &dyn TranscriptAdapter,
    key: &str,
) -> Result<IngestOutcome, String> {
    let Some(session_id) = adapter.session_id_for(key) else {
        return Ok(IngestOutcome::skipped());
    };
    let Some(stamp) = adapter.stamp_for(key) else {
        return Ok(IngestOutcome::skipped()); // unit gone
    };
    // The cursor gates the expensive PROSE re-ingest; synthesis (#75) gates on
    // whether the session has events yet (so an already-prose-ingested but
    // never-synthesized historical session still gets imported). Load the
    // content only if there's something to do.
    let prose_fresh = matches!(
        pg.get_capture_watermark(adapter.source(), key).await,
        Ok(Some(prev)) if prev >= stamp
    );
    let needs_synth = !pg.session_has_events(&session_id).await.unwrap_or(true);
    // A session captured before token-capture existed still needs its model + token
    // usage backfilled from the transcript — force a metadata refresh for it even
    // when prose + events are already fresh. `false` for new sessions (synthesis
    // sets their metadata) and for sessions already attempted (skip the re-read,
    // even a token-less source), keyed on the `meta_synced_at` marker.
    let needs_meta = pg.session_needs_meta_backfill(&session_id).await.unwrap_or(false);
    if prose_fresh && !needs_synth && !needs_meta {
        return Ok(IngestOutcome::skipped());
    }
    // Adapter owns the read + its own oversize/format guards (returns None to skip).
    let Some(content) = adapter.load_content(key) else {
        return Ok(IngestOutcome::skipped());
    };
    // Parse ONCE into the common structure; every persistence step below reads from
    // `parsed`, never the raw content — the adapter→persistence seam.
    let parsed = adapter.parse(&content);
    // 1. prose turns (#73) — only when stale.
    let mut turns = 0u32;
    if !prose_fresh {
        let (provider, model_name) = match &parsed.model {
            Some((p, m)) => (Some(p.as_str()), Some(m.as_str())),
            None => (None, None),
        };
        turns = pg
            .upsert_transcript_turns(
                adapter.source(),
                &session_id,
                adapter.family(),
                provider,
                model_name,
                &parsed.turns,
            )
            .await?;
        pg.set_capture_watermark(
            adapter.source(),
            key,
            Some(&session_id),
            stamp,
            parsed.turns.len() as i32,
        )
        .await?;
    }
    // 2. historical-bootstrap: synthesize the session + events if not already
    // captured (#75), so the existing enricher can derive its metrics.
    let analyze_project = if needs_synth {
        synthesize_session(pg, &parsed, &session_id, adapter.family()).await
    } else {
        None
    };
    // 3. session metadata (inference model + token usage): refresh from the
    //    transcript whether the session was just synthesized or already existed, so
    //    a re-run backfills columns added after the session was first captured.
    //    Idempotent; a miss leaves the column untouched (never a fabricated value).
    set_session_metadata(pg, &session_id, &parsed).await;
    Ok(IngestOutcome { skipped: false, turns, analyze_project })
}

/// Persist a session's inference model + token usage from the already-parsed
/// [`ParsedTranscript`]. The coordinator owns this write centrally (adapters stay
/// pure parsers). Only supplies a value the source actually carries — an absent
/// model/usage leaves the column as-is (never a fabricated default) — and always
/// stamps `meta_synced_at` so the attempt is made at most once per session.
async fn set_session_metadata(
    pg: &crate::db::pg_store::PgStore,
    session_id: &str,
    parsed: &ParsedTranscript,
) {
    let (provider, model_name) = match &parsed.model {
        Some((p, m)) => (Some(p.as_str()), Some(m.as_str())),
        None => (None, None),
    };
    // Token totals fit `integer` (verified << i32::MAX for real sources); a value
    // that somehow overflows is dropped rather than truncated to a wrong number.
    // `tokens_in` keeps its established meaning — ALL input the model processed —
    // so existing consumers are unaffected; the split rides alongside in its own
    // columns rather than changing what the old one means.
    let t = parsed.tokens;
    let (tokens_in, tokens_out) = match &t {
        Some(t) => (
            i32::try_from(t.total_input()).ok(),
            i32::try_from(t.output + t.reasoning.unwrap_or(0)).ok(),
        ),
        None => (None, None),
    };
    if let Err(e) = pg
        .set_session_metadata(session_id, provider, model_name, tokens_in, tokens_out, t.as_ref())
        .await
    {
        tracing::warn!(error = %e, session = %session_id, "set_session_metadata: write failed");
    }
}

/// Historical-bootstrap (#75): if this session has no events yet (not
/// live-captured / not already imported), reconstruct it from the transcript —
/// resolve the project from a cwd, create the session, and synthesize its event
/// stream so `analyze_project` can enrich it. Returns the project to analyze.
async fn synthesize_session(
    pg: &crate::db::pg_store::PgStore,
    parsed: &ParsedTranscript,
    session_id: &str,
    family: &str,
) -> Option<uuid::Uuid> {
    // A source that can't reconstruct events (empty) has no session to synthesize.
    if parsed.events.is_empty() {
        return None;
    }
    // dedup: never double-count a live-captured / already-imported session.
    match pg.session_has_events(session_id).await {
        Ok(true) => return None,
        Ok(false) => {}
        Err(e) => {
            tracing::warn!(error = %e, "synthesize_session: events check failed");
            return None;
        }
    }
    // project mapping: first cwd that is a tracked folder (exact) wins. Both the
    // exact lookup and the ancestor fallback are alias-aware, so a cwd recorded
    // under a since-renamed path resolves to the current folder. When no cwd is an
    // exact folder, fall back to the nearest tracked ANCESTOR — so a subdirectory
    // cwd (or a subdir of a renamed repo covered by a single root alias) still
    // attributes to the right project instead of being dropped.
    let mut resolved = None;
    for cwd in &parsed.cwds {
        if let Ok(Some((folder_id, project_id))) = pg.get_folder_ids_by_path(cwd).await {
            resolved = Some((cwd.clone(), folder_id, project_id));
            break;
        }
    }
    if resolved.is_none() {
        for cwd in &parsed.cwds {
            if let Ok(Some((folder_id, project_id))) = pg.find_folder_for_path(cwd).await {
                resolved = Some((cwd.clone(), folder_id, project_id));
                break;
            }
        }
    }
    let Some((cwd, folder_id, project_id)) = resolved else {
        tracing::debug!(session = %session_id, "synthesize_session: no tracked folder for cwds — skipping");
        return None;
    };
    if let Err(e) =
        pg.record_session_event(session_id, &folder_id, project_id.as_ref(), family, true).await
    {
        tracing::warn!(error = %e, "synthesize_session: record_session_event failed");
        return None;
    }
    let started = parsed.events.iter().map(|e| e.ts).min().unwrap_or(0);
    let completed = parsed.events.iter().map(|e| e.ts).max().unwrap_or(0);
    // Session start/completed timestamps power the "Duration" column
    // on the observatory. If this write fails silently the row still
    // shows up but with a blank duration — surface the failure so the
    // observability isn't itself invisible.
    if let Err(e) = pg.set_session_history(session_id, started, completed).await {
        tracing::warn!(error = %e, session = %session_id, "synthesize_session: set_session_history failed");
    }
    // Model + token usage are refreshed by the caller (`set_session_metadata`) so a
    // re-run backfills sessions synthesized before those columns existed.
    for ev in &parsed.events {
        let payload = match ev.event_type.as_str() {
            "UserPromptSubmit" => serde_json::json!({ "prompt": ev.prompt }),
            // Carry the FULL tool_input (bash command / skill / agent params) so the
            // enrich worker derives call_info/plugin/method just like a live event;
            // fall back to the file_path-only shape when the transcript lacked an input.
            "PostToolUse" => serde_json::json!({
                "tool_input": ev.tool_input.clone()
                    .unwrap_or_else(|| serde_json::json!({ "file_path": ev.file_path })),
            }),
            _ => serde_json::json!({}),
        };
        if let Err(e) = pg
            .insert_hook_event(
                session_id,
                family,
                &ev.event_type,
                ev.tool_name.as_deref(),
                Some(&cwd),
                ev.ts,
                None,
                &payload,
            )
            .await
        {
            tracing::warn!(error = %e, session = %session_id, "synthesize_session: insert_hook_event failed");
        }
    }
    tracing::info!(session = %session_id, events = parsed.events.len(), "synthesize_session: imported historical session");
    project_id
}

/// Ingest every file across all adapters in-process (no queue). Test helper
/// exercising the ingest+skip path end-to-end; the production path is the
/// chunked dispatcher (`run_backfill` -> per-file `run_ingest_capture`).
#[cfg(test)]
pub async fn backfill_all(
    pg: &crate::db::pg_store::PgStore,
    adapters: &[Box<dyn TranscriptAdapter>],
) -> BackfillReport {
    let mut report = BackfillReport::default();
    for ad in adapters {
        for unit in ad.units() {
            report.files_seen += 1;
            match ingest_one(pg, ad.as_ref(), &unit.key).await {
                Ok(o) if o.skipped => report.files_skipped += 1,
                Ok(o) => {
                    report.files_ingested += 1;
                    report.turns_upserted += o.turns;
                }
                Err(e) => {
                    tracing::warn!(error = %e, "transcript backfill: ingest failed");
                    report.files_skipped += 1;
                }
            }
        }
    }
    report
}

/// Dispatcher for `TaskKind::IngestCaptures`: enqueue one
/// `IngestCapture` task per changed/new transcript so ingestion
/// interleaves with other work and a single huge/bad file can't block the rest.
/// Skips files unchanged since last ingest (the per-file task re-checks to stay
/// race-safe). Returns the number of files enqueued.
pub async fn run_backfill(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    // Thin wrapper: the task kind exists to schedule the work, not to define it.
    // The task's own id is passed as the parent so every per-file child links back
    // to it in `activity.task_executions` — that link is what lets a caller follow
    // this dispatcher and see the ingestion progress rather than a task that
    // "completed" in milliseconds having done none of the work.
    let out = backfill(&ctx.queue, ctx.pg(), Some(task.id), task.as_of_stamp_ns()).await;
    Ok(out.enqueued)
}

/// What a transcript backfill IS — enqueue every unit, then repair sessions.
///
/// One definition with ONE caller: the `IngestCaptures` task. It was briefly
/// shared by the task and the `/api/transcripts/backfill` endpoint (before that,
/// two drifted copies of the same sequence — the endpoint ran only the
/// events-based repair, so a fix added to the task path silently did not reach
/// the button).
///
/// Sharing the definition fixed the drift but left the endpoint executing a
/// ~2,700-file filesystem sweep on the request thread. The endpoint now ENQUEUES
/// this task instead, so the work has one home: the queue supplies dedup, retry,
/// restart survival, and progress on `/api/tasks/progress` — none of which an
/// HTTP handler running the sweep inline can offer.
///
/// Keep it that way. A call surface that needs this work enqueues the task; it
/// does not call this directly.
/// `parent` is the dispatching task's id, stamped on every child so the
/// execution log forms a tree a follower can aggregate.
pub async fn backfill(
    queue: &crate::tasks::queue::TaskQueue,
    pg: &crate::db::pg_store::PgStore,
    parent: Option<u64>,
    since: Option<i64>,
) -> BackfillOutcome {
    let (files_seen, enqueued) = dispatch(queue, parent, since).await;
    let sessions_repaired = repair_sessions(pg).await;
    BackfillOutcome { files_seen, enqueued, sessions_repaired }
}

/// What one backfill did — the shape both callers report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BackfillOutcome {
    pub files_seen: u32,
    pub enqueued: u32,
    pub sessions_repaired: u32,
}

/// Both session repairs, run together — the ONE place either is invoked.
///
/// There are two entry points to a transcript backfill (the scheduled task and
/// the HTTP trigger), and they had drifted: the trigger called only the
/// events-based repair, so a fix added to the task path silently did not apply to
/// the button. Sharing one function is what makes "part of the backfill" true of
/// both rather than of whichever path happened to be edited.
///
/// Order matters. The events-based repair runs first because an event carries a
/// cwd per record — a stronger attribution signal than the transcript's
/// thread-level one — so anything it can resolve is resolved better there.
///
/// Both are idempotent and cheap: each only looks at sessions that do not exist,
/// so re-running converges rather than duplicating. Neither is fatal — a repair
/// failure logs and lets the ingest stand.
pub async fn repair_sessions(pg: &crate::db::pg_store::PgStore) -> u32 {
    let mut repaired = 0u32;
    match pg.repair_orphaned_sessions().await {
        Ok(n) => {
            if n > 0 {
                tracing::info!(repaired = n, "repair_sessions: re-attached orphaned sessions");
            }
            repaired += n;
        }
        Err(e) => tracing::warn!(error = %e, "repair_sessions: repair_orphaned_sessions failed"),
    }
    // Transcripts whose prose was ingested but whose session was never
    // synthesized (no reconstructable events, or no cwd resolved at the time). A
    // folder tracked LATER makes an earlier cwd resolvable, so this converges on
    // every backfill instead of needing a one-off sweep.
    match pg.repair_sessions_from_transcripts().await {
        Ok(n) => {
            if n > 0 {
                tracing::info!(repaired = n, "repair_sessions: created sessions from transcripts");
            }
            repaired += n;
        }
        Err(e) => {
            tracing::warn!(error = %e, "repair_sessions: repair_sessions_from_transcripts failed")
        }
    }
    repaired
}

/// Scan all adapters and enqueue one `IngestCapture` task per
/// transcript. Returns `(files_seen, enqueued)`. Each per-file task does the
/// smart skip (cursor for prose + session-has-events for synthesis), so the
/// dispatcher stays trivial and correct across upgrades. Callable from the
/// dispatcher task or directly from the trigger endpoint (immediate feedback).
pub async fn dispatch(
    queue: &crate::tasks::queue::TaskQueue,
    parent: Option<u64>,
    since: Option<i64>,
) -> (u32, u32) {
    let mut count = 0u32;
    let mut skipped = 0u32;
    for ad in adapters() {
        for unit in ad.units() {
            // `since` is what makes a backfill a PARAMETER rather than a separate
            // kind: the same coordinator ingests everything (None) or only units
            // changed after a point (Some). One code path, two requests.
            //
            // Filtering on the unit's cheap change-stamp — no content read — so a
            // catch-up run enqueues only what moved instead of a task per unit
            // across every source (~2,700 here) that then no-ops against its own
            // cursor.
            //
            // Deliberately NOT derived from "the last successful run": a unit
            // modified while a run was in flight would be skipped forever. An
            // explicit caller-supplied bound cannot silently lose data that way,
            // and the default (None) keeps the cursor-guarded full walk.
            if let Some(cutoff) = since
                && unit.stamp < cutoff
            {
                skipped += 1;
                continue;
            }
            // folder_path = capture source, path = unit key (file path or thread id).
            let mut task = Task::for_capture(TaskKind::IngestCapture, ad.source(), &unit.key);
            if let Some(p) = parent {
                task = task.with_parent(p);
            }
            queue.enqueue(task).await;
            count += 1;
        }
    }
    tracing::info!(count, skipped, "capture ingest: dispatched per-unit tasks");
    (count, count)
}

/// Handler for `TaskKind::IngestCapture`: ingest one transcript.
/// `task.folder_path` = capture source, `task.path` = transcript file path.
pub async fn run_ingest_capture(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let Some(adapter) = adapter_for_source(task.capture_source()) else {
        return Err(format!("unknown transcript source '{}'", task.folder_path));
    };
    let outcome = ingest_one(ctx.pg(), adapter.as_ref(), &task.path).await?;
    // A freshly-synthesized historical session needs enrichment to light up its
    // FTR/churn/correction signals (#75). AnalyzeProject is idempotent + incremental.
    if let Some(project_id) = outcome.analyze_project {
        ctx.queue.enqueue(Task::new(TaskKind::AnalyzeProject, "", &project_id.to_string())).await;
    }
    Ok(outcome.turns)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
{"type":"user","timestamp":"2026-06-22T10:00:00.000Z","message":{"role":"user","content":"add a login page"}}
{"type":"assistant","timestamp":"2026-06-22T10:00:02.000Z","message":{"role":"assistant","content":[{"type":"text","text":"On it."}]}}
{"type":"user","timestamp":"2026-06-22T10:00:04.000Z","message":{"role":"user","content":[{"type":"tool_result","content":"ok"}]}}
{"type":"user","timestamp":"2026-06-22T10:01:00.000Z","message":{"role":"user","content":"now wire it up"}}
{"type":"assistant","timestamp":"2026-06-22T10:01:01.000Z","message":{"role":"assistant","content":[{"type":"text","text":"Wired."}]}}
"#;

    #[tokio::test]
    async fn backfill_ingests_then_skips_unchanged() {
        let pg = crate::db::pg_store::PgStore::connect_test().await.unwrap();
        let sid = format!("_test-tx-{}", uuid::Uuid::new_v4());
        // temp transcript: <root>/proj/<sid>.jsonl (mirrors ~/.claude layout)
        let root = std::env::temp_dir().join(format!("sensei-tx-{}", uuid::Uuid::new_v4()));
        let proj = root.join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join(format!("{sid}.jsonl")), SAMPLE).unwrap();

        // pre-existing event ⇒ session already captured, so this test isolates
        // the PROSE cursor-skip path (synthesis is a dedup no-op).
        pg.insert_hook_event(&sid, "claude", "Stop", None, None, 1, None, &serde_json::json!({}))
            .await
            .unwrap();

        let ads: Vec<Box<dyn TranscriptAdapter>> =
            vec![Box::new(claude::ClaudeAdapter::new(root.clone()))];

        // first run ingests the two turns
        let r1 = backfill_all(&pg, &ads).await;
        assert_eq!(r1.files_ingested, 1);
        assert_eq!(r1.turns_upserted, 2);

        let row: (i64, String, String) = sqlx_core::query_as::query_as(
            "SELECT count(*), max(source), max(assistant_text) FILTER (WHERE turn_index=2)
             FROM activity.transcript_turns WHERE session_id=$1",
        )
        .bind(&sid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(row.0, 2, "two turns stored");
        assert_eq!(row.1, "claude_code", "tagged with capture source");
        assert_eq!(row.2, "Wired.", "assistant prose captured per turn");

        // re-run: unchanged file ⇒ skipped via the cursor, no new work
        let r2 = backfill_all(&pg, &ads).await;
        assert_eq!(r2.files_skipped, 1);
        assert_eq!(r2.files_ingested, 0);

        // cleanup
        let pool = pg.pool();
        sqlx_core::query::query("DELETE FROM activity.assistant_events WHERE session_id=$1")
            .bind(&sid)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM activity.transcript_turns WHERE session_id=$1")
            .bind(&sid)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM activity.capture_watermarks WHERE session_id=$1")
            .bind(&sid)
            .execute(pool)
            .await
            .ok();
        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn synthesize_imports_historical_session() {
        let pg = crate::db::pg_store::PgStore::connect_test().await.unwrap();
        let pid = pg
            .create_project(&format!("_test:imp-{}", uuid::Uuid::new_v4()), None, None)
            .await
            .unwrap();
        let root = pg
            .add_watch_root(
                &format!("/_test/imp-root-{}", uuid::Uuid::new_v4()),
                "t",
                &serde_json::json!([]),
            )
            .await
            .unwrap();
        let repo_path = format!("/_test/imp-repo-{}", uuid::Uuid::new_v4());
        let fid = pg.upsert_repo(&root, "imp-repo", &repo_path).await.unwrap();
        // link folder → project (scan/reconcile does this in production; the
        // importer resolves project_id from the folder via cwd).
        sqlx_core::query::query("UPDATE sensei.folders SET project_id=$1 WHERE id=$2")
            .bind(pid)
            .bind(fid)
            .execute(pg.pool())
            .await
            .unwrap();
        let sid = format!("_test-imp-{}", uuid::Uuid::new_v4());

        // a historical transcript whose cwd == the tracked folder's abs_path
        let content = format!(
            "{{\"type\":\"user\",\"cwd\":\"{cwd}\",\"timestamp\":\"2026-06-20T10:00:00.000Z\",\"message\":{{\"role\":\"user\",\"content\":\"add the parser\"}}}}\n\
             {{\"type\":\"assistant\",\"cwd\":\"{cwd}\",\"timestamp\":\"2026-06-20T10:00:05.000Z\",\"message\":{{\"role\":\"assistant\",\"model\":\"claude-opus-4-8\",\"usage\":{{\"input_tokens\":100,\"cache_read_input_tokens\":50,\"output_tokens\":20}},\"content\":[{{\"type\":\"tool_use\",\"name\":\"Edit\",\"input\":{{\"file_path\":\"{cwd}/src/x.rs\"}}}}]}}}}\n\
             {{\"type\":\"assistant\",\"cwd\":\"{cwd}\",\"timestamp\":\"2026-06-20T10:00:06.000Z\",\"message\":{{\"role\":\"assistant\",\"model\":\"claude-opus-4-8\",\"usage\":{{\"input_tokens\":5,\"output_tokens\":8}},\"content\":[{{\"type\":\"tool_use\",\"name\":\"Bash\",\"input\":{{\"command\":\"cargo test\",\"description\":\"run tests\"}}}}]}}}}\n",
            cwd = repo_path
        );
        let root_dir = std::env::temp_dir().join(format!("sensei-imp-{}", uuid::Uuid::new_v4()));
        let proj = root_dir.join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join(format!("{sid}.jsonl")), &content).unwrap();

        let ads: Vec<Box<dyn TranscriptAdapter>> =
            vec![Box::new(claude::ClaudeAdapter::new(root_dir.clone()))];
        backfill_all(&pg, &ads).await;

        // session synthesized, attributed to the project, flagged backfilled,
        // with a historical started_at (not "today").
        // Named alias + a destructuring bind: the row is eight columns, and read
        // positionally (`s.5`, `s.6`) you have to count to know which is which.
        type SessionRow = (
            Option<uuid::Uuid>, // project_id
            bool,               // backfilled
            bool,               // started_at is historical
            Option<String>,     // provider
            Option<String>,     // model
            Option<i32>,        // tokens_in
            Option<i32>,        // tokens_out
            bool,               // meta_synced_at IS NOT NULL
        );
        let (project_id, backfilled, historical, provider, model, tokens_in, tokens_out, meta_synced):
            SessionRow = sqlx_core::query_as::query_as(
            "SELECT project_id, backfilled, (started_at < now() - interval '1 day'), provider, model, tokens_in, tokens_out, meta_synced_at IS NOT NULL FROM activity.sessions WHERE client_session_id=$1"
        ).bind(&sid).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(project_id, Some(pid), "attributed to the project resolved from cwd");
        assert!(backfilled, "flagged backfilled");
        assert!(historical, "started_at set from the transcript timestamp, not now()");
        assert_eq!(provider.as_deref(), Some("anthropic"), "provider captured at synthesis");
        assert_eq!(model.as_deref(), Some("claude-opus-4-8"), "model captured from the transcript");
        // token usage summed across assistant records: in=(100+50)+5=155, out=20+8=28
        assert_eq!(tokens_in, Some(155), "tokens_in = input + cache tokens across the session");
        assert_eq!(tokens_out, Some(28), "tokens_out = output tokens across the session");
        assert!(meta_synced, "meta_synced_at stamped so the metadata backfill runs at most once");

        // events synthesized: prompt + tool-edit + terminal Stop.
        let kinds: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT event_type FROM activity.assistant_events WHERE session_id=$1 ORDER BY ts",
        )
        .bind(&sid)
        .fetch_all(pg.pool())
        .await
        .unwrap();
        let kinds: Vec<&str> = kinds.iter().map(|k| k.0.as_str()).collect();
        assert!(
            kinds.contains(&"UserPromptSubmit")
                && kinds.contains(&"PostToolUse")
                && kinds.contains(&"Stop"),
            "got {kinds:?}"
        );
        let n_before = kinds.len();

        // The Bash tool_use's FULL input survives into the synthesized event's payload
        // (not just file_path), so the enrich worker can derive call_info from a
        // backfilled event exactly as from a live one.
        let bash_cmd: (Option<String>,) = sqlx_core::query_as::query_as(
            "SELECT payload->'tool_input'->>'command' FROM activity.assistant_events \
              WHERE session_id=$1 AND tool_name='Bash'",
        )
        .bind(&sid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(
            bash_cmd.0.as_deref(),
            Some("cargo test"),
            "the full tool_input (bash command) is carried into the backfilled event"
        );

        // re-run: file unchanged ⇒ cursor-skip, no duplicate events.
        backfill_all(&pg, &ads).await;
        let n: (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM activity.assistant_events WHERE session_id=$1",
        )
        .bind(&sid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(n.0, n_before as i64, "re-run does not duplicate events");

        // cleanup
        let pool = pg.pool();
        sqlx_core::query::query("DELETE FROM activity.assistant_events WHERE session_id=$1")
            .bind(&sid)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM activity.transcript_turns WHERE session_id=$1")
            .bind(&sid)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM activity.capture_watermarks WHERE session_id=$1")
            .bind(&sid)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE client_session_id=$1")
            .bind(&sid)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id=$1")
            .bind(fid)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id=$1")
            .bind(root)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id=$1")
            .bind(pid)
            .execute(pool)
            .await
            .ok();
        std::fs::remove_dir_all(&root_dir).ok();
    }

    #[test]
    fn adapter_for_source_dispatches_all_new_sources() {
        assert!(adapter_for_source("cursor").is_some(), "cursor");
        assert!(adapter_for_source("vscode").is_some(), "vscode");
        assert!(adapter_for_source("copilot_cli").is_some(), "copilot_cli");
        assert!(adapter_for_source("claude_code").is_some(), "claude_code");
        assert!(adapter_for_source("zed").is_some(), "zed");
        assert!(adapter_for_source("opencode").is_some(), "opencode");
        assert!(adapter_for_source("unknown_source").is_none(), "unknown returns None");
    }

    #[test]
    fn backfill_all_includes_new_adapters() {
        let ads = adapters();
        let sources: Vec<&str> = ads.iter().map(|a| a.source()).collect();
        assert!(sources.contains(&"cursor"), "cursor in adapters(): {sources:?}");
        assert!(sources.contains(&"vscode"), "vscode in adapters(): {sources:?}");
        assert!(sources.contains(&"copilot_cli"), "copilot_cli in adapters(): {sources:?}");
        assert_eq!(ads.len(), 6, "all 6 adapters registered");
    }
}
