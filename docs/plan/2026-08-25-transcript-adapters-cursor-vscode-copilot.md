# Transcript Adapters: Cursor, VSCode, Copilot CLI

**Status:** PLANNED
**Date:** 2026-08-25
**Depends on:** #73 (transcript system — shipped)

## Goal

Add three new `TranscriptAdapter` implementations so sensei ingests transcripts from
Cursor, VS Code (Copilot Chat), and the GitHub Copilot CLI — the three most-used
AI coding tools after Claude Code and Zed.

## Scope

| Adapter | `source` | `family` | Enum status |
|---|---|---|---|
| `CursorAdapter` | `cursor` | `cursor` | **already exists** |
| `VscodeAdapter` | `vscode` | `vscode` | **needs adding** |
| `CopilotCliAdapter` | `copilot_cli` | `copilot` | **needs adding** |

All three follow the `TranscriptAdapter` trait (`mod.rs:177`).

---

## Architecture: the contract every adapter follows

Every adapter decomposes `parse()` into the same four sub-responsibilities:

```
fn parse(&self, content: &str) -> ParsedTranscript {
    let session = parse_*_session(content);        // events + cwds
    ParsedTranscript {
        turns:       parse_*_transcript(content),  // prose turns
        cwds:        session.cwds,
        events:      session.events,
        model:       extract_model(content),        // (provider, model)
        tokens:      extract_tokens(content),       // SessionTokens
    }
}
```

Plus two per-turn helpers:

| Helper | Purpose | Pattern (all adapters) |
|---|---|---|
| `merge_facts(facts, record)` | Promote per-record signals into `TurnFacts` (tokens, stop_reason, skill, effort, sidechain, branch) | Sum across the turn's records; LAST stop_reason wins |
| `turn_attrs(record)` | Strip bulky prose/content from the raw record, keep metadata verbatim | `const DROP` array; rebuild `serde_json::Map` minus dropped keys |

Shared constants: `MAX_TURN_CHARS: usize = 50_000` (cap assistant prose per turn).
SQLite adapters: open read-only with `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX`.

---

## Phase 1 — DDL + wiring

### 1a. Extend `assistant_family` enum

```sql
ALTER TYPE sensei.assistant_family ADD VALUE IF NOT EXISTS 'vscode';
ALTER TYPE sensei.assistant_family ADD VALUE IF NOT EXISTS 'copilot';
```

`cursor` is already present.

### 1b. Register adapters in `mod.rs`

Add path helpers + wire into `adapters()` and `adapter_for_source()`:

```rust
fn cursor_transcript_root() -> PathBuf {
    crate::paths::home().join(".cursor/projects")
}

fn vscode_user_root(variant: &str) -> PathBuf {
    // variant = "Code" | "Code - Insiders" | "VSCodium" | "Code - OSS"
    #[cfg(target_os = "macos")]
    { crate::paths::home().join(format!("Library/Application Support/{variant}/User")) }
    #[cfg(target_os = "linux")]
    { crate::paths::home().join(format!(".config/{variant}/User")) }
    #[cfg(target_os = "windows")]
    { dirs::data_dir().unwrap().join(format!("{variant}/User")) }
}

fn copilot_home() -> PathBuf {
    std::env::var("COPILOT_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| crate::paths::home().join(".copilot"))
}
```

---

## Phase 2 — CursorAdapter

### Storage layout

```
~/.cursor/projects/<project-hash>/agent-transcripts/<session-id>/<session-id>.jsonl
```

Flat layout also exists: `agent-transcripts/<session-id>.jsonl` (no subdirectory).

Optional metadata in `~/.cursor/chats/<workspace-hash>/<session-id>/store.db` —
SQLite with `meta` + `blobs` tables. Carries `lastUsedModel`, `createdAt`,
`agentId`, `mode`.

### JSONL format

Each line is a JSON object with `type` discriminator:

| `type` | Meaning | sensei mapping |
|---|---|---|
| `user` | User prompt | `TranscriptTurn::user_text` |
| `assistant` | Assistant text response | `TranscriptTurn::assistant_text` |
| `tool_use` | Tool call (name + input) | `SynthEvent` (limited — no `tool_result`) |
| `system` | System message | Skip (injected) |

**Known limitation:** Cursor JSONL does NOT include `tool_result` content.
`SynthEvent` reconstruction is limited to `tool_use` entries (tool name + input
only). No `PostToolUse` events with output.

### Implementation

```rust
const MAX_TURN_CHARS: usize = 50_000;
const MAX_TRANSCRIPT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_LINE_BYTES: usize = 4 * 1024 * 1024;

const INJECTED_MARKERS: &[&str] = &[
    "<task-notification", "<system-reminder", "<command-name",
    "<command-message", "<local-command", "Caveat:", "## Security Guidance",
];
```

**units()**: glob `~/.cursor/projects/*/agent-transcripts/*/*.jsonl` + flat `*.jsonl`.
Key = full file path, stamp = `mtime_ns()`.

**stamp_for()**: `mtime_ns(Path::new(key))` — identical to Claude.

**session_id_for()**: `cursor-<uuid>` where `<uuid>` is the directory name (NOT the
file stem — the directory name is the session ID in Cursor's layout).

**load_content()**: `std::fs::read_to_string` with `MAX_TRANSCRIPT_BYTES` guard.

**parse()** decomposes into:

| Function | Input | Output |
|---|---|---|
| `parse_cursor_transcript(content)` | JSONL text | `Vec<TranscriptTurn>` |
| `parse_cursor_session(content)` | JSONL text | `Option<SynthSession>` |
| `extract_model(content)` | JSONL text | `Option<(String, String)>` — from `store.db` if available, else `None` |
| `extract_tokens(content)` | JSONL text | `Option<SessionTokens>` — always `None` |

**Turn parsing** (`parse_cursor_transcript`):
- Iterate JSONL lines (skip blank / > `MAX_LINE_BYTES` / malformed)
- `type == "user"` → check `human_prompt_text()` (skip `tool_result`, `isMeta`,
  injected markers, empty) → start new turn
- `type == "assistant"` → extract `text` content blocks only (skip `thinking`,
  `tool_use`) → append to current turn's `assistant_text` with `\n\n` separator
- Cap turns > `MAX_TURN_CHARS`
- Populate `TurnFacts` via `merge_facts()` — extract `gitBranch`, `stop_reason`
  from `message`, per-record token usage if present
- `turn_attrs()` — strip `message` (prose is in columns), keep everything else

**Session reconstruction** (`parse_cursor_session`):
- Collect distinct `cwd` values from JSONL records
- Emit `UserPromptSubmit` per genuine human prompt
- Emit `PostToolUse` per `tool_use` block (name + full `input` as `tool_input`,
  mine `file_path` from `input.file_path` or `input.path`)
- Append synthetic `Stop` at `max_ts`
- Return `None` when no events

**cwds**: Collect from JSONL `cwd` field (present on some Cursor records).
If absent, empty — project resolution falls back to folder hash.

### Session ID

`cursor-<session-id>` where `<session-id>` is the UUID directory name.

---

## Phase 3 — VscodeAdapter

### Storage layers (all three)

**Layer 1 — Chat session journals (preferred):**
```
~/Library/Application Support/Code/User/workspaceStorage/<hash>/chatSessions/<uuid>.jsonl
~/Library/Application Support/Code/User/globalStorage/emptyWindowChatSessions/<uuid>.jsonl
```
Delta journal format: `kind:0` sets root snapshot, `kind:1` sets value at path,
`kind:2` replaces array slice. Reconstruct `requests[]` for turns.

**Layer 2 — Newer transcript format:**
```
~/Library/Application Support/Code/User/workspaceStorage/<hash>/GitHub.copilot-chat/transcripts/<session>.jsonl
```
Event stream: `session.start`, `user.message`, `assistant.message`,
`tool.execution_start`, `tool.execution_complete`. First line must have
`type == "session.start"` and `data.producer == "copilot-agent"`.

**Layer 3 — OTel span store (richest):**
```
~/Library/Application Support/Code/User/globalStorage/github.copilot-chat/agent-traces.db
```
SQLite with real token counts per turn (prompt/output/cache). Schema:
`spans` table with `traceId`, `spanId`, `startTimeUnixNano`, attributes as JSON.

### Variant support

Cover all VS Code variants under their own `User/` root:

| Variant | macOS path component |
|---|---|
| Code | `Code` |
| Code - Insiders | `Code - Insiders` |
| VSCodium | `VSCodium` |
| Code - OSS | `Code - OSS` |

Also cover `~/.vscode-server/data/User` for remote/SSH.

### Priority order

For a given session, sources are read most-authoritative first:
1. `agent-traces.db` (real token counts) → skip journal+transcript for that session
2. `chatSessions/*.jsonl` (token estimates from `result.metadata`)
3. `transcripts/*.jsonl` (chars/4 estimates, no token data)

### Implementation

**units()**: For each variant + vscode-server, glob:
- `workspaceStorage/*/chatSessions/*.jsonl`
- `globalStorage/emptyWindowChatSessions/*.jsonl`
- `workspaceStorage/*/GitHub.copilot-chat/transcripts/*.jsonl`

Key = full file path, stamp = `mtime_ns()`.

For `agent-traces.db`: single unit per variant, key = db path, stamp = mtime_ns().
The adapter opens the DB in `load_content()` and returns a JSON blob with ALL
sessions' turns (one call per variant).

**load_content()**: For journal/transcript units, read the file directly.
For the OTel DB unit, query spans and return a JSON string with structure:

```json
{
  "source": "otel_spans",
  "sessions": {
    "<session-id>": {
      "spans": [...],
      "attributes": {...}
    }
  }
}
```

**parse()** — the key insight: three source layers, same trait contract.

For journal units, `load_content()` returns the raw JSONL (or a reconstructed JSON
blob). For transcript units, returns the raw JSONL. For OTel units, returns the
bundled JSON. The `parse()` method dispatches on the source:

```rust
fn parse(&self, content: &str) -> ParsedTranscript {
    // detect source from first line or content shape
    if content.starts_with('{') && content.contains("\"source\":\"otel_spans\"") {
        parse_otel_content(content)
    } else if is_journal(content) {
        parse_journal_content(content)
    } else {
        parse_transcript_content(content)
    }
}
```

Each path produces `ParsedTranscript` through the same sub-functions:

| Function | Journal | Transcript | OTel |
|---|---|---|---|
| `parse_*_transcript(content)` | Reconstruct requests[], extract turns | Parse event stream | Parse spans → turns |
| `parse_*_session(content)` | Reconstruct requests[], build events | Build events from event stream | Build events from spans |
| `extract_model(content)` | `selectedModel` from root | `data.model` from session.start | Span attributes |
| `extract_tokens(content)` | `result.metadata.promptTokens/outputTokens` | `None` | Real counts from span attributes |

**Turn parsing** — consistent with Claude/Zed/OpenCode pattern:
- User prompt boundaries from `requests[].message.text` (journal) or
  `user.message` events (transcript) or span user turns (OTel)
- Assistant text from response parts, excluding tool calls
- `merge_facts()` populates `TurnFacts` from per-turn metadata
- `turn_attrs()` strips prose, keeps metadata

**Session reconstruction**:
- Collect cwd from `workspace.json` sibling (journal) or
  `session.start.data.context.cwd` (transcript)
- Emit `UserPromptSubmit` + `PostToolUse` + terminal `Stop`

**Workspace resolution**: Each `chatSessions/<uuid>.jsonl` has a sibling
`workspace.json` with the folder URI. Resolve `file://` URIs to local paths;
handle `vscode-remote://wsl+<distro>/` by extracting the Linux path.

**cwds**: From `workspace.json` sibling or `session.start` event. Empty when
neither is available.

### Session ID

`vscode-<session-id>` where `<session-id>` is the UUID filename stem.

---

## Phase 4 — CopilotCliAdapter

### Storage layout

**Source A — events.jsonl (full event stream):**
```
~/.copilot/session-state/<session-id>/events.jsonl
~/.copilot/session-state/<session-id>/workspace.yaml  (cwd)
```
Events: `user.message`, `assistant.message`, `tool_use`, `tool_result`,
`session.model_change`, `session.shutdown`. The `session.shutdown` event
carries the only real token counts (`data.modelMetrics.<model>.usage`).

**Source B — session-store.db (structured turns):**
```
~/.copilot/session-store.db
```
Tables: `sessions` (id, summary, repo, branch, timestamps), `turns`
(user_message, assistant_response), `checkpoints`, `session_files`,
`session_refs`, `search_index` (FTS5).

**Source C — data.db (aggregate tokens):**
```
~/.copilot/data.db
```
Workspace-level app sessions with real token totals.

### Dedup strategy

Both sources are ingested equally. Session ID is the dedup key.
For a given session:
- If only events.jsonl exists → parse from JSONL
- If only session-store.db has the row → parse from SQLite
- If both exist → merge: take richer turn data (events.jsonl has tool calls,
  session-store.db has cleaner text). Token counts from `session.shutdown`
  event override SQLite estimates.

### Implementation

**units()**: Two sources, deduped by session ID.

Source A: glob `~/.copilot/session-state/*/events.jsonl`, stamp = `mtime_ns()`.
Key = directory path (e.g. `~/.copilot/session-state/<uuid>/`).

Source B: query `session-store.db` for `SELECT id, updated_at FROM sessions`,
stamp = `updated_at` as epoch millis.
Key = `copilot-sqlite-<session_id>` (namespaced to avoid collision with JSONL keys).

**session_id_for()**: Both return `copilot-<session_id>` where `<session_id>` is
the UUID. The SQLite key has the `copilot-sqlite-` prefix stripped.

**load_content()**: Two paths.

For JSONL units:
1. Read `events.jsonl` line-by-line into a JSON array
2. Read `workspace.yaml` for `cwd:` line
3. Return JSON: `{"source":"jsonl","cwd":"...","events":[...],...}`

For SQLite units:
1. Open `session-store.db` read-only
2. Query `sessions` + `turns` for the given session
3. Optionally query `data.db` for token aggregates
4. Return JSON: `{"source":"sqlite","session":{...},"turns":[...],"tokens":{...}}`

**parse()** dispatches on source:

```rust
fn parse(&self, content: &str) -> ParsedTranscript {
    let root: serde_json::Value = serde_json::from_str(content).ok()?;
    match root.get("source").and_then(|s| s.as_str()) {
        Some("jsonl") => parse_copilot_jsonl(content),
        Some("sqlite") => parse_copilot_sqlite(content),
        _ => ParsedTranscript::default(),
    }
}
```

Each path produces `ParsedTranscript` through the same sub-functions:

| Function | JSONL path | SQLite path |
|---|---|---|
| `parse_copilot_transcript(content)` | Parse events.jsonl → turns | Parse turns table → turns |
| `parse_copilot_session(content)` | Parse events → events + cwd | Limited — no tool events |
| `extract_model(content)` | `session.model_change` event | `sessions.model` column |
| `extract_tokens(content)` | `session.shutdown` event → `SessionTokens` | `data.db` aggregates or `None` |

**JSONL turn parsing** (`parse_copilot_jsonl`):
- Iterate events line-by-line
- `user.message` → start new turn (extract text from `data.content`)
- `assistant.message` → append text to current turn
- `tool_use` → `SynthEvent` (tool name + input — available here, unlike Cursor!)
- `tool_result` → `SynthEvent` (tool output — also available!)
- `session.shutdown` → extract `SessionTokens` from `data.modelMetrics`
- `merge_facts()` + `turn_attrs()` per turn (strip `data` content, keep metadata)

**SQLite turn parsing** (`parse_copilot_sqlite`):
- `SELECT user_message, assistant_response FROM turns WHERE session_id = ?`
- One turn per row (already bounded)
- `merge_facts()` — limited (no per-turn tokens, no tool events)
- `turn_attrs()` — strip message text, keep row metadata

**Session reconstruction**:
- Collect cwd from `workspace.yaml` (`cwd:` line)
- Emit `UserPromptSubmit` + `PostToolUse` (JSONL only) + terminal `Stop`
- SQLite path: `UserPromptSubmit` + `Stop` only (no tool events)

**cwds**: From `workspace.yaml` (`cwd:` scalar line) in JSONL path.
SQLite path: from `sessions.repo` column if available.

### Session ID

`copilot-<session-id>` where `<session-id>` is the UUID directory name.

---

## Phase 5 — Tests

Each adapter follows the existing pattern (Claude adapter has 35+ tests).
Every test is a pure function test — no DB required for parse tests.

### CursorAdapter tests
- `parse_simple_turn` — user → assistant text
- `parse_multiple_turns` — sequential turns
- `parse_tool_use_events` — tool_use lines become SynthEvents with tool_input
- `parse_injected_messages` — system/injected user messages skipped
- `parse_turn_attrs_strips_message` — message.content not in attrs
- `turn_facts_populated` — gitBranch, stop_reason from records
- `turn_facts_null_when_absent` — honest-empty, never fabricated zeros
- `units_flat_layout` — flat `agent-transcripts/<id>.jsonl`
- `units_nested_layout` — nested `<id>/<id>.jsonl`
- `stamp_matches_mtime` — cursor semantics
- `session_id_from_key` — `cursor-<uuid>`
- `parse_session_reconstructs_events` — UserPromptSubmit + PostToolUse + Stop
- `parse_session_none_when_empty` — empty content → None

### VscodeAdapter tests
- `parse_journal_mutation` — kind:0/1/2 reconstruction → turns
- `parse_journal_turn_attrs` — message stripped, metadata kept
- `parse_transcript_event_stream` — session.start/user.message/assistant.message
- `parse_otel_spans` — span attributes → turns with real token counts
- `workspace_resolution_file_uri` — file:// to local path
- `workspace_resolution_wsl_uri` — vscode-remote:// to Linux path
- `units_cross_variant` — Code + Insiders + VSCodium
- `units_empty_window_sessions` — globalStorage/emptyWindowChatSessions
- `priority_otel_over_journal` — agent-traces.db wins
- `session_id_from_key` — `vscode-<uuid>`
- `parse_session_reconstructs_events` — UserPromptSubmit + PostToolUse + Stop
- `parse_session_none_when_empty` — empty content → None

### CopilotCliAdapter tests
- `parse_events_jsonl_turns` — user.message + assistant.message → turns
- `parse_events_jsonl_tool_use` — tool_use + tool_result → SynthEvents
- `parse_session_shutdown_tokens` — token extraction from shutdown event
- `parse_workspace_yaml_cwd` — cwd extraction
- `parse_sqlite_turns` — session-store.db turns → turns
- `parse_sqlite_limited_events` — no tool events from SQLite
- `dedup_jsonl_and_sqlite` — merge: JSONL tool events + SQLite text
- `units_from_jsonl_source` — glob session-state
- `units_from_sqlite_source` — query session-store.db
- `units_dedup_by_session_id` — JSONL + SQLite don't double-count
- `session_id_from_key` — `copilot-<uuid>`
- `parse_session_reconstructs_events` — UserPromptSubmit + PostToolUse + Stop
- `parse_session_none_when_empty` — empty content → None

### Integration test
- `backfill_all_includes_new_adapters` — `adapters()` returns 6 adapters
- `adapter_for_source_dispatches` — all 6 source strings resolve

---

## Phase 6 — Startup + backfill

No changes needed — the existing `run_backfill()` already iterates all adapters.
New adapters are automatically picked up on daemon restart.

---

## Known limitations

| Source | Missing data | Impact |
|---|---|---|
| Cursor | No `tool_result` content | SynthEvents lack output; PostToolUse events have no output field |
| Cursor | No token counts | `SessionTokens` always `None` |
| Cursor | No reliable cwd on all records | `cwds` may be empty; project resolution falls back to folder hash |
| VSCode journals | Token estimates only (chars/4 when no metadata) | Cost metrics approximate |
| VSCode transcripts | No token data at all | `SessionTokens` `None` for transcript-only sessions |
| Copilot CLI SQLite | No tool events | SynthEvents empty; turns only |
| Copilot CLI JSONL | Token counts only at `session.shutdown` | Per-turn tokens unavailable |

---

## Implementation order

1. DDL enum extension + migration
2. CursorAdapter (simplest — mirrors Claude's file-per-session model)
3. CopilotCliAdapter (dual-source, but familiar JSONL + SQLite)
4. VscodeAdapter (most complex — 3 sources, variant support, workspace resolution)
5. Tests for all three
6. Integration smoke test with real local data (if available)

---

## Files to modify

| File | Change |
|---|---|
| `crates/senseid/src/transcript/mod.rs` | Add path helpers, register in `adapters()` + `adapter_for_source()` |
| `crates/senseid/src/transcript/cursor.rs` | **New** — CursorAdapter |
| `crates/senseid/src/transcript/vscode.rs` | **New** — VscodeAdapter |
| `crates/senseid/src/transcript/copilot_cli.rs` | **New** — CopilotCliAdapter |
| `database/ddl/enum/sensei/assistant_family.ddl` | Add `vscode`, `copilot` |
| `docs/backlog.md` | Mark as planned |

---

## Open questions

None — all decisions resolved by user input during planning.
