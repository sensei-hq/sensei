# Transcript Adapter Pattern — Implementation Guide

> This document describes the **`TranscriptAdapter` trait** and the adapter pattern used in `crates/senseid/src/transcript/` for ingesting assistant transcripts. It covers the 6 built-in adapters (Claude Code, Zed, OpenCode, Cursor, VS Code, Copilot CLI), the common data contracts, and provides a skill/template for implementing new adapters.

---

## 1. The Adapter Trait (`TranscriptAdapter`)

Defined in `crates/senseid/src/transcript/mod.rs:330-353`:

```rust
pub trait TranscriptAdapter: Send + Sync {
    /// Capture origin (distinct from the model `family`), e.g. "claude_code" / "zed".
    fn source(&self) -> &'static str;

    /// Harness/agent family (the `sensei.assistant_family` enum: "claude", "zed", …).
    /// The precise per-unit model, when known, is carried on [`ParsedTranscript::model`].
    fn family(&self) -> &'static str;

    /// Units this adapter can ingest, each with a change-stamp for cursor skipping.
    fn units(&self) -> Vec<UnitRef>;

    /// Cheap change-stamp for a unit key (no content read) — the cursor pre-check.
    fn stamp_for(&self, key: &str) -> Option<i64>;

    /// Session id for a unit key (Claude: filename stem; Zed: `zed-<thread_id>`).
    fn session_id_for(&self, key: &str) -> Option<String>;

    /// Load a unit's raw content (the expensive read/decompress).
    /// `None` ⇒ gone or oversized (logged, never panics on bad data).
    fn load_content(&self, key: &str) -> Option<String>;

    /// Convert a unit's raw content into the common [`ParsedTranscript`] — the ONE
    /// method carrying adapter-specific format knowledge. Everything the persistence
    /// layer needs (turns, cwds, events, model, tokens) comes out of this single
    /// struct, so persistence never sees raw content. Pure + deterministic (unit-
    /// testable without a DB); a source that can't reconstruct events returns them
    /// empty and/or `model`/`tokens` as `None`.
    fn parse(&self, content: &str) -> ParsedTranscript;
}
```

---

## 2. Core Data Contracts

All adapters produce the **same** `ParsedTranscript` (mod.rs:263-277):

```rust
pub struct ParsedTranscript {
    /// Prose turns → `activity.transcript_turns`.
    pub turns: Vec<TranscriptTurn>,
    /// Distinct working directories seen → project resolution.
    pub cwds: Vec<String>,
    /// Reconstructed hook-stream events → `activity.assistant_events` (#75).
    pub events: Vec<SynthEvent>,
    /// Inference `(provider, model)` → `activity.sessions`. `None` when absent.
    pub model: Option<(String, String)>,
    /// Session-total usage → `activity.sessions`. `None` when the source records
    /// none (honest-empty — the coordinator leaves the columns NULL, never a
    /// fabricated 0).
    pub tokens: Option<SessionTokens>,
}
```

### Key Types

| Type | Purpose | Key Fields |
|------|---------|------------|
| `TranscriptTurn` | One user→assistant turn | `turn_index`, `user_text`, `assistant_text`, `started_at`, `attrs`, `facts` |
| `TurnFacts` | Promoted per-turn signals | `tokens_in/out`, `cache_read/write`, `stop_reason`, `is_sidechain`, `skill`, `plugin`, `git_branch`, `effort`, `service_tier` |
| `SynthEvent` | Reconstructed hook event | `event_type` (`UserPromptSubmit`/`PostToolUse`/`Stop`), `tool_name`, `file_path`, `prompt`, `tool_input`, `ts` |
| `SynthSession` | Session from transcript | `cwds`, `events` |
| `SessionTokens` | Session-total usage (SPLIT) | `input`, `output`, `cache_read`, `cache_write`, `reasoning`, `cost` |
| `UnitRef` | Logical ingest unit | `key`, `stamp` (monotonic change-stamp for cursor) |

---

## 3. Adapter Lifecycle (Ingestion Flow)

The coordinator (`ingest_one` in mod.rs) handles the **orchestration**; adapters only implement the 6 trait methods:

```
1. units()                    → discover all logical units + change-stamps
2. For each unit (in parallel, chunked):
   a. stamp_for(key)          → cheap cursor pre-check (skip if unchanged)
   b. session_id_for(key)     → namespace the session (e.g. "zed-<thread_id>")
   c. load_content(key)       → expensive read/decompress (may return None)
   d. parse(content)          → pure + deterministic → ParsedTranscript
3. Persistence (coordinator):
   - upsert_transcript_turns
   - synthesize_session (historical bootstrap #75)
   - set_session_metadata (model + tokens, idempotent)
```

**Cursor semantics**: The coordinator stores a watermark `(source, key) → (stamp, turns_written)`. Only when `stamp_for(key) > watermark` does it re-read and re-parse.

---

## 4. Shared Constants & Helpers (mod.rs)

| Constant | Value | Purpose |
|----------|-------|---------|
| `MAX_TURN_CHARS` | 50,000 | Cap assistant prose per turn |
| `MAX_TRANSCRIPT_BYTES` | 64 MB | Skip oversized files |
| `MAX_LINE_BYTES` | 4 MB | Skip single-line blobs (base64, etc.) |
| `INJECTED_MARKERS` | 9 strings | Filter injected "user" messages |

**Shared parsing helpers:**
- `human_prompt_text()` — extract genuine human prompt (filters tool_result, meta, injected)
- `merge_facts()` — promote `gitBranch`, `effort`, `skill`, `plugin`, `isSidechain`, `stop_reason`, usage → `TurnFacts`
- `turn_attrs()` — strip bulky `message` content, keep `id/model/stop_reason/usage` verbatim
- `parse_timestamp()` / `parse_timestamp_ms()` — RFC3339 → `DateTime` / epoch ms

---

## 5. The 6 Built-in Adapters

| Adapter | Source | Family | Storage | Units | Key Features |
|---------|--------|--------|---------|-------|--------------|
| **ClaudeAdapter** | `claude_code` | `claude` | `~/.claude/projects/<enc>/<session>.jsonl` | JSONL files | Per-message usage, `cwd`, model switching, subagent sidechains (skipped) |
| **ZedAdapter** | `zed` | `zed` | `~/Library/Application Support/Zed/threads/threads.db` | SQLite rows (threads) | zstd-compressed JSON, two schema versions, **no per-msg timestamps** (spreads evenly), multi-model |
| **OpenCodeAdapter** | `opencode` | `opencode` | `~/.local/share/opencode/opencode.db` | SQLite rows (sessions) | Per-message usage (cache split), auto-compact handling, multi-model, session totals |
| **CursorAdapter** | `cursor` | `cursor` | `~/.cursor/projects/<hash>/agent-transcripts/` | JSONL files (flat + nested) | No model, no tool_result, gitBranch/stop_reason/usage per message |
| **VscodeAdapter** | `vscode` | `vscode` | `workspaceStorage/**/{chatSessions,GitHub.copilot-chat/transcripts}/*.jsonl` + OTel DB | JSONL files (3 layers) | Delta journals + event streams + OTel spans, explicit root override, Windows path decode |
| **CopilotCliAdapter** | `copilot_cli` | `copilot` | `~/.copilot/session-state/<session>/events.jsonl` + `session-store.db` | JSONL dirs + SQLite | `tool.execution_start`/`complete` events, `session.shutdown` tokens (cumulative, last wins), merged JSONL+SQLite |

---

## 6. Feature Matrix — What Each Adapter Supports

| Feature | Claude Code | Zed | OpenCode | Cursor | VS Code | Copilot CLI |
|---------|-------------|-----|----------|--------|---------|-------------|
| **Prose turns** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Tool calls → `PostToolUse`** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Tool results** | ✅ (separate record) | ❌ (no tool_result) | ✅ (in `tool` part state) | ❌ | ❌ (no tool_result) | ✅ (`tool.execution_complete.success`) |
| **Per-turn token usage** | ✅ (fresh + cache split) | ❌ (thread-cumulative only) | ✅ (per-message, cache split) | ✅ (per-message) | ❌ (no tokens in journals/transcripts) | ✅ (session totals from `shutdown`) |
| **Cache read/write split** | ✅ | ❌ (7/60 threads report) | ✅ | ❌ | ❌ | ✅ (from `shutdown`) |
| **Reasoning tokens** | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ |
| **Metered cost** | ❌ | ❌ | ✅ (`cost` field) | ❌ | ❌ | ✅ (`requests.cost`) |
| **Model per turn** | ✅ (dominant) | ✅ (per-thread `{provider,model}`) | ✅ (per-message `modelID`/`providerID`) | ❌ | ✅ (per-thread `session.start`) | ✅ (per-turn from `assistant.message`) |
| **Git branch** | ✅ (`gitBranch`) | ❌ (column empty) | ❌ | ✅ (`gitBranch`) | ❌ | ❌ |
| **Stop reason** | ✅ (`stop_reason`) | ✅ (limit flags → mapped) | ✅ (`finish`) | ✅ (`stop_reason`) | ❌ | ❌ |
| **Skill / agent** | ✅ (`attributionSkill`) | ❌ | ✅ (`agent` field) | ❌ | ❌ | ❌ |
| **Plugin** | ✅ (`attributionPlugin`) | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Sidechain / subagent** | ✅ (`isSidechain`) | ❌ (parent_id empty) | ✅ (`parentID` on message) | ❌ | ❌ | ❌ |
| **Effort** | ✅ (`effort`) | ✅ (`completion_mode`) | ✅ (`mode`) | ❌ | ❌ | ❌ |
| **Service tier** | ✅ (`service_tier`) | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Per-message timestamps** | ✅ | ❌ (spreads evenly) | ✅ | ✅ | ✅ (event stream + journals) | ✅ |
| **CWD / project resolution** | ✅ (`cwd` per record) | ✅ (`worktree_snapshots`) | ✅ (`directory` per session) | ✅ (`cwd` per record) | ✅ (`workspace.json` + `session.start`) | ✅ (`workspace.yaml`) |
| **Session synthesis (#75)** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Integration test (env var)** | ❌ | `SENSEI_ZED_SAMPLE` | `SENSEI_OPENCODE_SAMPLE` | ❌ | `SENSEI_VSCODE_SAMPLE` | `SENSEI_COPILOT_SAMPLE` |

### Legend
- ✅ = Fully implemented + tested
- ❌ = Not supported by the source format (known limitation)
- Partially = Implemented but with caveats (noted in cell)

---

## 7. Skill: Implementing a New Transcript Adapter

### Prerequisites
- Access to real transcript samples (at least 3 sessions from different users)
- The source stores transcripts in a discoverable location (files, SQLite, etc.)
- You can run the adapter against real data (`cargo test -p senseid -- transcript::<adapter>`)

### Step 1: Create the Adapter File
```bash
touch crates/senseid/src/transcript/<name>.rs
```

### Step 2: Implement the 6 Trait Methods

```rust
//! <Name> transcript adapter — ingests <source> transcripts.
//!
//! Storage: <path> (<format>). Key schema: <brief>.
//! Timestamps: <format>. <Any quirks: auto-compact, compression, etc.>

use super::{
    ParsedTranscript, SessionTokens, SynthEvent, SynthSession, TranscriptAdapter,
    TranscriptTurn, TurnFacts, UnitRef, /* shared helpers as needed */
};
use std::path::PathBuf;

pub struct <Name>Adapter {
    db_path: PathBuf,  // or root: PathBuf for file-based
}

impl <Name>Adapter {
    pub fn new(db_path: PathBuf) -> Self { Self { db_path } }
    fn open(&self) -> Option<rusqlite::Connection> { /* read-only */ }
}

impl TranscriptAdapter for <Name>Adapter {
    fn source(&self) -> &'static str { "<name>" }
    fn family(&self) -> &'static str { "<family>" }

    fn units(&self) -> Vec<UnitRef> {
        // Discover all logical units + change-stamps (mtime, updated_at, etc.)
    }
    fn stamp_for(&self, key: &str) -> Option<i64> { /* cheap check */ }
    fn session_id_for(&self, key: &str) -> Option<String> {
        Some(format!("<name>-<unique>"))  // namespace to avoid collisions
    }
    fn load_content(&self, key: &str) -> Option<String> {
        // Expensive read + decompress; return None for oversize/gone
    }
    fn parse(&self, content: &str) -> ParsedTranscript {
        // Pure + deterministic; NO DB, NO IO
    }
}
```

### Step 3: Implement Pure Parsing Functions (unit-testable)

```rust
// ── Parsing (pure functions, NO IO, NO DB) ────────────────────────

pub fn parse_<name>_messages(content: &str) -> Vec<TranscriptTurn> { /* ... */ }
pub fn parse_<name>_session(content: &str) -> Option<SynthSession> { /* ... */ }
pub fn extract_model(content: &str) -> Option<(String, String)> { /* ... */ }
pub fn extract_tokens(content: &str) -> Option<SessionTokens> { /* ... */ }

// Helper: normalize schema versions if multiple exist
fn normalize_messages(v: &serde_json::Value) -> Vec<NormMsg> { /* ... */ }
```

### Step 4: Register the Adapter

In `mod.rs`:
```rust
pub mod <name>;  // add to module list

fn adapters() -> Vec<Box<dyn TranscriptAdapter>> {
    vec![
        // ... existing ...
        Box::new(<name>::<Name>Adapter::new(<name>_db_path())),
    ]
}
fn adapter_for_source(source: &str) -> Option<Box<dyn TranscriptAdapter>> {
    match source {
        // ... existing ...
        "<name>" => Some(Box::new(<name>::<Name>Adapter::new(<name>_db_path()))),
        _ => None,
    }
}
```

Add path function:
```rust
fn <name>_db_path() -> PathBuf {
    crate::paths::home().join("<platform-specific-path>")
}
```

### Step 5: Write Unit Tests (against fixtures + real data)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_JSON: &str = r#"{...real wire format...}"#;

    #[test]
    fn parse_turns_from_real_wire_format() { /* ... */ }
    #[test]
    fn model_extraction_works() { /* ... */ }
    #[test]
    fn token_totals_sum_correctly() { /* ... */ }

    /// Integration proof against REAL data, when a sample is provided.
    /// Point SENSEI_<NAME>_SAMPLE at a transcript folder to run.
    #[test]
    fn parses_real_<name>_session_when_available() {
        let Ok(dir) = std::env::var("SENSEI_<NAME>_SAMPLE") else { return; };
        let adapter = <Name>Adapter::new(PathBuf::from(&dir));
        // Count on-disk units → require adapter to find ALL
        // Assert tools > 0, prompts > 0, model resolved, tokens > 0
    }
}
```

### Step 6: Add Consent Entry (user opts in)

In `consent.rs` (or wherever consent is managed), ensure `<name>` is in the consent set.

---

## 8. ACP Support Matrix

ACP (Agent Client Protocol) is the protocol layer for agent ↔ client communication. The adapters above are **transcript ingest** (reading historical data). ACP is separate — it's how the **daemon drives an agent at runtime**.

| ACP Capability | Description | Transcript Adapter Coverage |
|----------------|-------------|------------------------------|
| **Session management** | Create, resume, list sessions | All adapters ingest historical sessions |
| **Prompt/response** | Send user prompt, receive assistant response | All adapters reconstruct `UserPromptSubmit` + prose |
| **Tool calls** | Agent invokes tool, client executes, returns result | All adapters reconstruct `PostToolUse` + tool input |
| **Tool results** | Client returns tool output | **Partial** — only Claude, OpenCode, Copilot CLI have tool results |
| **Streaming** | Incremental assistant text | Not captured in transcripts (transcripts are final) |
| **Permissions/gates** | Agent requests approval for sensitive ops | Not in transcripts (runtime only) |
| **Model selection** | Client tells agent which model to use | All adapters capture model per turn/session |
| **Context files** | Agent reads/writes files in workspace | All adapters capture `cwd` + file paths in tools |
| **Agent capabilities** | Declare supported tools, modes | Not in transcripts |

### ACP Runtime vs. Transcript Ingest

| Layer | Responsibility | Code Location |
|-------|----------------|---------------|
| **ACP Runtime** | Drive agent, handle tool calls, gates, streaming | `crates/senseid/src/tasks/relay_drivers/`, `crates/dojo-protocol/` |
| **Transcript Ingest** | Read historical transcripts, backfill turns/events | `crates/senseid/src/transcript/` (this doc) |

**They are separate pipelines.** ACP runtime produces live hook events; transcript ingest backfills historical sessions the hook stream missed.

---

## 9. Claude Mods, Hooks, Plugins, Skills, Agents — Source Comparison

| Capability | Claude Code | OpenCode | Zed | Cursor | VS Code | Copilot CLI |
|------------|-------------|----------|-----|--------|---------|-------------|
| **Mods** (Claude's unified plugin+hooks+skills) | ✅ Native (since v1.0) | ❌ Different architecture | ❌ | ❌ | ❌ | ❌ |
| **Hooks** (pre/post tool, notification) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Plugins / Skills** | ✅ (via Mods) | ✅ (`agent` field = skill analogue) | ❌ | ❌ | ❌ | ❌ |
| **Agents** (subagents, delegation) | ✅ (`isSidechain`) | ✅ (`parentID` on message) | ❌ | ❌ | ❌ | ❌ |
| **Token capture** (per-turn, cache split) | ✅ | ✅ (best — per-message) | ❌ (cumulative) | ✅ (per-message, no cache) | ❌ | ✅ (session totals) |
| **Reasoning tokens** | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Metered cost** | ❌ | ✅ | ❌ | ❌ | ❌ | ✅ |
| **Model switching mid-session** | ✅ | ✅ | ❌ (per-thread) | ❌ | ❌ | ✅ |

### Key Findings
- **Claude Mods** are **Claude-specific** — no other source exposes this unified capability.
- **OpenCode** is the closest multi-model equivalent — it has per-message usage, cache split, agents, and metered cost, but via a different architecture (SQLite + Drizzle ORM).
- **No other source** captures **reasoning tokens** or **metered cost** like OpenCode does.
- **ACP runtime** (driving agents at runtime) is a separate concern — the adapters here only read historical data.

---

## 10. Gap Assessment — ACP Configuration Adapters

The current **relay drivers** (`crates/senseid/src/tasks/relay_drivers/`) implement the **ACP runtime** for driving agents:

| Driver | ACP Support | Status | Gaps |
|--------|-------------|--------|------|
| **Claude** (`claude.rs`) | Hooks-based (legacy) | ✅ Built, tested | Needs migration to ACP native when available |
| **ACP** (`acp.rs`) | Observe-only | 🟡 Partial | `drive_step` = unsupported; only `observe_update` maps plan → segments |
| **Fallback** (`fallback.rs`) | Coarse observe | ✅ Built | Maps activity recency → `running`/`idle`/`done` |

### Identified Gaps

| Gap ID | Description | Priority | Module |
|--------|-------------|----------|--------|
| **ACP-1** | No ACP-native driver for Claude (hooks are legacy) | P1 | M21 (Relay) |
| **ACP-2** | ACP driver `drive_step` unsupported — cannot drive, only observe | P1 | M21 |
| **ACP-3** | No agent capability negotiation (tools, modes, limits) | P2 | M21 |
| **ACP-4** | No streaming response handling in relay | P2 | M21 |
| **ACP-5** | Permission/gate flow not implemented for ACP | P2 | M21 |
| **ACP-6** | Model selection via ACP not wired | P3 | M21 |

These are tracked in the roadmap as **M21 gaps** (see `roadmap/04-team.md` → T-1/T-2 for related).

---

## 11. Gap Resolution Tasks (Roadmap)

Add to `roadmap/06-gaps.md`:

```markdown
| ID    | Module | One line | Evidence | Priority |
|-------|--------|----------|----------|----------|
| ACP-1 | M21    | No ACP-native driver for Claude (hooks are legacy) | relay_drivers/claude.rs uses hooks, not ACP | P1 |
| ACP-2 | M21    | ACP driver drive_step unsupported — observe only | acp.rs: drive_step returns unsupported | P1 |
| ACP-3 | M21    | No agent capability negotiation (tools, modes, limits) | ACP protocol supports; not implemented | P2 |
| ACP-4 | M21    | No streaming response handling in relay | Streaming not parsed in transcript or relay | P2 |
| ACP-5 | M21    | Permission/gate flow not implemented for ACP | Only Claude hooks have gate logic | P2 |
| ACP-6 | M21    | Model selection via ACP not wired | ACP supports; not used | P3 |
```

---

## 12. Verification Checklist for New Adapters

When adding a new adapter, verify all of these before considering it "done":

| Check | How to Verify |
|-------|---------------|
| **Trait compiles** | `cargo check -p senseid` |
| **Units discovery** | `cargo test -p senseid <name>::units` — finds expected units |
| **Cursor works** | Modify a transcript → re-ingest → only changed unit re-parsed |
| **Pure parse** | Unit tests pass without DB; `cargo test -p senseid <name>::parse` |
| **Real data test** | `SENSEI_<NAME>_SAMPLE=/path/to/data cargo test -p senseid parses_real` |
| **Tokens populate** | `cargo test -p senseid <name>::extract_tokens` — real totals |
| **Model resolves** | `cargo test -p senseid <name>::model_extraction` |
| **Events synthesize** | `cargo test -p senseid <name>::parse_session_reconstructs` |
| **Consent gated** | Adapter only runs when user consents (`consented_adapters`) |
| **Oversize guard** | `MAX_TRANSCRIPT_BYTES` triggers skip + warning log |
| **Bad data handled** | Malformed JSON / corrupt zstd / missing DB → logged, skipped, never panics |

---

## 13. Related Files

| File | Purpose |
|------|---------|
| `crates/senseid/src/transcript/mod.rs` | Trait, contracts, coordinator, 6 adapters registered |
| `crates/senseid/src/transcript/claude.rs` | Claude Code adapter |
| `crates/senseid/src/transcript/zed.rs` | Zed adapter (zstd + SQLite) |
| `crates/senseid/src/transcript/opencode.rs` | OpenCode adapter (SQLite, per-message usage) |
| `crates/senseid/src/transcript/cursor.rs` | Cursor adapter (flat + nested JSONL) |
| `crates/senseid/src/transcript/vscode.rs` | VS Code adapter (3 layers + OTel) |
| `crates/senseid/src/transcript/copilot_cli.rs` | Copilot CLI adapter (JSONL + SQLite merged) |
| `crates/senseid/src/transcript/consent.rs` | User consent gating |
| `crates/senseid/src/tasks/transcript_scheduler.rs` | Scheduled ingestion dispatcher |
| `docs/design/transcript-adapters.md` | State + gaps from 2026-08-26 review |
| `docs/roadmap/06-gaps.md` | C-16..C-24 (transcript), ACP-1..ACP-6 (relay) |