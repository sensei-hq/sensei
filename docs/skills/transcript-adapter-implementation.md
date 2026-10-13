# Skill: Implement a Transcript Adapter

> Use this skill when adding a new assistant transcript source to sensei.
> Follows the pattern in `crates/senseid/src/transcript/` — 6 adapters, one trait,
> pure parsing, cursor-based ingestion.

## When to use

- A new assistant (Windsurf, GitHub Copilot Chat native, etc.) stores transcripts
- You have access to real transcript samples (≥3 sessions, different users)
- The source is file-based, SQLite, or similar discoverable layout

---

## Prerequisites

- [ ] Real transcript samples available (not in repo — use env var for integration test)
- [ ] Storage layout documented (paths, schema, timestamps, compression)
- [ ] Token usage, model info, tool calls identified in the format

---

## Implementation Steps

### 1. Create the adapter file

```bash
# e.g. crates/senseid/src/transcript/windsurf.rs
```

### 2. Implement the struct + `TranscriptAdapter` trait

```rust
//! Windsurf transcript adapter — ingests Windsurf's SQLite sessions.
//!
//! Storage: ~/.local/share/windsurf/sessions.db
//! Schema: sessions (id, project_path, model, updated_at), messages (session_id, role, content, tokens), tool_calls
//! Timestamps: unix milliseconds.

use super::{
    ParsedTranscript, SessionTokens, SynthEvent, SynthSession, TranscriptAdapter,
    TranscriptTurn, TurnFacts, UnitRef,
};
use std::path::PathBuf;

pub struct WindsurfAdapter {
    db_path: PathBuf,
}

impl WindsurfAdapter {
    pub fn new(db_path: PathBuf) -> Self { Self { db_path } }
    fn open(&self) -> Option<rusqlite::Connection> { /* read-only */ }
}

impl TranscriptAdapter for WindsurfAdapter {
    fn source(&self) -> &'static str { "windsurf" }
    fn family(&self) -> &'static str { "windsurf" }

    fn units(&self) -> Vec<UnitRef> {
        // Discover all sessions + updated_at as change-stamp
    }
    fn stamp_for(&self, key: &str) -> Option<i64> { /* cheap check */ }
    fn session_id_for(&self, key: &str) -> Option<String> {
        Some(format!("windsurf-{key}"))  // namespace to avoid collisions
    }
    fn load_content(&self, key: &str) -> Option<String> {
        // Load session + messages + tool_calls → JSON blob
    }
    fn parse(&self, content: &str) -> ParsedTranscript {
        // Pure + deterministic; calls pure parsing functions below
    }
}
```

### 3. Write pure parsing functions (unit-testable, no DB/IO)

```rust
// ── Parsing ────────────────────────────────────────────────────

pub fn parse_windsurf_messages(content: &str) -> Vec<TranscriptTurn> { /* ... */ }
pub fn parse_windsurf_session(content: &str) -> Option<SynthSession> { /* ... */ }
pub fn extract_model(content: &str) -> Option<(String, String)> { /* ... */ }
pub fn extract_tokens(content: &str) -> Option<SessionTokens> { /* ... */ }

// Normalize if multiple schema versions exist
fn normalize_messages(v: &serde_json::Value) -> Vec<NormMsg> { /* ... */ }
```

### 4. Register in `mod.rs`

```rust
pub mod windsurf;  // add to module list

fn adapters() -> Vec<Box<dyn TranscriptAdapter>> {
    vec![
        // ... existing ...
        Box::new(windsurf::WindsurfAdapter::new(windsurf_db_path())),
    ]
}
fn adapter_for_source(source: &str) -> Option<Box<dyn TranscriptAdapter>> {
    match source {
        // ... existing ...
        "windsurf" => Some(Box::new(windsurf::WindsurfAdapter::new(windsurf_db_path()))),
        _ => None,
    }
}

fn windsurf_db_path() -> PathBuf {
    crate::paths::home().join(".local/share/windsurf/sessions.db")
}
```

### 5. Add consent entry

In `consent.rs` (or wherever consent is managed), ensure `"windsurf"` is in the consent set.

### 6. Write unit tests (fixture + real data)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{...real wire format...}"#;

    #[test]
    fn parse_turns_from_real_wire_format() { /* ... */ }
    #[test]
    fn model_extraction_works() { /* ... */ }
    #[test]
    fn token_totals_sum_correctly() { /* ... */ }

    /// Integration proof against REAL data — point SENSEI_WINDSURF_SAMPLE at a sessions.db
    #[test]
    fn parses_real_windsurf_session_when_available() {
        let Ok(dir) = std::env::var("SENSEI_WINDSURF_SAMPLE") else { return; };
        let adapter = WindsurfAdapter::new(PathBuf::from(&dir));
        // Count on-disk sessions → require adapter to find ALL
        // Assert tools > 0, prompts > 0, model resolved, tokens > 0
    }
}
```

---

## Verification Checklist

| Check | Command |
|-------|---------|
| Trait compiles | `cargo check -p senseid` |
| Units discovery works | `cargo test -p senseid windsurf::units` |
| Cursor skips unchanged | Modify transcript → re-ingest → only changed re-parsed |
| Pure parse (no DB) | `cargo test -p senseid windsurf::parse` |
| Real data integration | `SENSEI_WINDSURF_SAMPLE=/path cargo test -p senseid parses_real` |
| Tokens populate correctly | `cargo test -p senseid windsurf::extract_tokens` |
| Model resolves | `cargo test -p senseid windsurf::model_extraction` |
| Events synthesize | `cargo test -p senseid windsurf::parse_session_reconstructs` |
| Consent gated | Adapter only runs when user consents |
| Oversize guard | `MAX_TRANSCRIPT_BYTES` triggers skip + warning |
| Bad data handled | Malformed JSON / corrupt DB → logged, skipped, never panics |

---

## Pattern Reference

| Component | Location | Purpose |
|-----------|----------|---------|
| `TranscriptAdapter` trait | `mod.rs:330` | 6 methods every adapter implements |
| `ParsedTranscript` | `mod.rs:262` | Common output contract |
| `TranscriptTurn` | `mod.rs:173` | One user→assistant turn |
| `TurnFacts` | `mod.rs:212` | Promoted per-turn signals |
| `SynthEvent` | `mod.rs:230` | Reconstructed hook event |
| `SessionTokens` | `mod.rs:293` | SPLIT session totals |
| `UnitRef` | `mod.rs:319` | Logical unit + change-stamp |
| Shared constants | `mod.rs:25` | `MAX_TURN_CHARS`, `MAX_TRANSCRIPT_BYTES`, etc. |
| Shared helpers | `mod.rs:51` | `human_prompt_text`, `merge_facts`, `turn_attrs`, timestamps |
| Coordinator | `mod.rs:453` | `ingest_one` — cursor, prose, synthesis, metadata |

---

## Existing Adapters (for reference)

| Adapter | File | Source | Family | Key Feature |
|---------|------|--------|--------|-------------|
| `ClaudeAdapter` | `claude.rs` | `claude_code` | `claude` | Per-message usage, `cwd`, subagents |
| `ZedAdapter` | `zed.rs` | `zed` | `zed` | zstd + SQLite, spreads timestamps |
| `OpenCodeAdapter` | `opencode.rs` | `opencode` | `opencode` | Per-message usage + cache split, agents |
| `CursorAdapter` | `cursor.rs` | `cursor` | `cursor` | Flat + nested JSONL, no model |
| `VscodeAdapter` | `vscode.rs` | `vscode` | `vscode` | 3 layers (journals, transcripts, OTel) |
| `CopilotCliAdapter` | `copilot_cli.rs` | `copilot_cli` | `copilot` | Merged JSONL + SQLite, `shutdown` tokens |

---

## Common Pitfalls

| Pitfall | Fix |
|---------|-----|
| `attrs` defaults to `null` not `{}` | Implement `Default` for `TranscriptTurn` with `attrs: json!({})` |
| Cursor doesn't skip | `stamp_for` must be cheap (no content read); watermark compares `stamp` |
| Token totals are `0` instead of `None` | Return `None` when source has no usage — never fabricate 0 |
| Model is `None` for multi-model sessions | Extract dominant model per session (most frequent) |
| Subagent turns double-count | Filter by `isSidechain` / `parentID` / `isSidechain` |
| Timestamp spread for no-per-msg sources | Use `thread_span_ms` + `spread_ms` (see `zed.rs`) |
| Consent not checked | Register in `consented_adapters()`; coordinator filters by consent |

---

## Testing Real Data (without committing samples)

```bash
# Set env var pointing to the source's data directory
export SENSEI_WINDSURF_SAMPLE=~/Library/Application\ Support/Windsurf

# Run the integration test (skips silently if env var not set)
cargo test -p senseid windsurf::parses_real_windsurf_session_when_available -- --nocapture
```

The test counts on-disk units and asserts the adapter finds **all of them** — not just "not empty" (which would pass while missing a layer).

---

## Related Documentation

- `docs/design/transcript-adapter-pattern.md` — full pattern, feature matrix, ACP matrix
- `docs/design/transcript-adapters.md` — state + gaps from 2026-08-26 review
- `crates/senseid/src/transcript/mod.rs` — trait, contracts, coordinator
- `docs/roadmap/06-gaps.md` — C-16..C-24 (transcript gaps), ACP-1..ACP-6 (relay gaps)