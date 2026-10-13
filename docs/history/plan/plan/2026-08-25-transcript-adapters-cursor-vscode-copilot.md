# Transcript Adapters: Cursor, VSCode, Copilot CLI

**Status:** PLANNED
**Date:** 2026-08-25
**Depends on:** #73 (transcript system — shipped)
**Reviewed:** 2026-08-26 (issues A–H resolved)

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

---

## Phase 0 — Hoist shared constants (pure move, green suite)

**Do this first as its own commit.** Fixes issues E and F.

Move these from `claude.rs` / `zed.rs` into `mod.rs` (pub):

| Symbol | Current location | Notes |
|---|---|---|
| `MAX_TURN_CHARS: usize = 50_000` | `claude.rs:36`, `zed.rs:35` | Deduplicate into one |
| `MAX_TRANSCRIPT_BYTES: u64 = 64 * 1024 * 1024` | `claude.rs:37` | Move to mod.rs |
| `MAX_LINE_BYTES: usize = 4 * 1024 * 1024` | `claude.rs:38` | Move to mod.rs |
| `INJECTED_MARKERS: &[&str]` | `claude.rs:39-46` | Move to mod.rs |
| `human_prompt_text(record) -> Option<&str>` | `claude.rs:449` (private) | Make pub, move to mod.rs |

After the move, `claude.rs` and `zed.rs` reference `super::MAX_TURN_CHARS` etc.
Confirm the test suite is still green — pure move, no behavior change.

---

## Phase 1 — DDL + wiring

### 1a. Extend `assistant_family` enum

Edit `database/ddl/enum/sensei/assistant_family.ddl` — append `vscode` and `copilot`.
Then run `dbd reconcile` to generate the migration. dbd emits:

```sql
ALTER TYPE sensei.assistant_family ADD VALUE 'vscode';
ALTER TYPE sensei.assistant_family ADD VALUE 'copilot';
```

No `IF NOT EXISTS` — dbd diffs first. These statements cannot run inside a
transaction block.

`cursor` is already present in the enum.

### 1b. Register adapters in `mod.rs`

Add path helpers + wire into `adapters()` and `adapter_for_source()`:

```rust
fn cursor_transcript_root() -> PathBuf {
    crate::paths::home().join(".cursor/projects")
}

fn vscode_user_root(variant: &str) -> Option<PathBuf> {
    // variant = "Code" | "Code - Insiders" | "VSCodium" | "Code - OSS"
    #[cfg(target_os = "macos")]
    { Some(crate::paths::home().join(format!("Library/Application Support/{variant}/User"))) }
    #[cfg(target_os = "linux")]
    { Some(crate::paths::home().join(format!(".config/{variant}/User"))) }
    #[cfg(target_os = "windows")]
    { dirs::data_dir().map(|d| d.join(format!("{variant}/User"))) }
    // Returns None when dirs::data_dir() fails — units() skips that variant.
}

fn copilot_home() -> PathBuf {
    std::env::var("COPILOT_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| crate::paths::home().join(".copilot"))
}
```

`vscode_user_root` returns `Option<PathBuf>` (fix G). `units()` skips any
variant where this returns `None` — an absent editor is a normal state, not
an error.

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
use super::{
    MAX_TURN_CHARS, MAX_TRANSCRIPT_BYTES, MAX_LINE_BYTES,
    INJECTED_MARKERS, human_prompt_text,
};
```

**units()**: glob `~/.cursor/projects/*/agent-transcripts/*/*.jsonl` + flat `*.jsonl`.
Key = full file path, stamp = `mtime_ns()`.

**stamp_for()**: `mtime_ns(Path::new(key))` — identical to Claude.

**session_id_for()**: Detect layout — use parent directory name when it parses
as a UUID, otherwise use the file stem. This handles both nested (`<uuid>/<uuid>.jsonl`
→ `<uuid>`) and flat (`<id>.jsonl` → `<id>`) layouts without collision (fix H).

```rust
fn session_id_for(&self, key: &str) -> Option<String> {
    let path = Path::new(key);
    let stem = path.file_stem()?.to_str()?;
    let parent = path.parent()?.file_name()?.to_str()?;
    // If parent dir looks like a UUID, it's the session id (nested layout)
    // Otherwise the file stem is the session id (flat layout)
    let id = if looks_like_uuid(parent) { parent } else { stem };
    Some(format!("cursor-{}", id))
}
```

**load_content()**: `std::fs::read_to_string` with `MAX_TRANSCRIPT_BYTES` guard.

**Model extraction:** Accept `None`. The JSONL content passed to `parse()` does not
carry model info. The optional `store.db` lookup could be folded into `load_content()`
but adds complexity for minimal value — model is already captured from the adapter's
runtime context. Keep it `None` for now.

**parse()** decomposes into:

| Function | Input | Output |
|---|---|---|
| `parse_cursor_transcript(content)` | JSONL text | `Vec<TranscriptTurn>` |
| `parse_cursor_session(content)` | JSONL text | `Option<SynthSession>` |
| `extract_model(_content)` | — | `None` (see above) |
| `extract_tokens(_content)` | — | `None` |

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

`cursor-<session-id>` where `<session-id>` is the UUID directory name or file stem.

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

### Priority and dedup

`units()` emits one unit per session. For sessions found in the OTel DB, the
journal and transcript units are **dropped** — the OTel path is most authoritative.
This decision is made in `units()` which sees all sources at once, not in `parse()`
which only sees one unit's content.

```rust
fn units(&self) -> Vec<TranscriptUnit> {
    let mut units = vec![];
    let mut otel_sessions: HashSet<String> = HashSet::new();

    // Pass 1: enumerate OTel sessions (per-session units)
    for variant in &VARIANTS {
        if let Some(db_path) = otel_db_path(variant) {
            let sessions = query_otel_sessions(&db_path);
            for session_id in sessions {
                otel_sessions.insert(session_id.clone());
                units.push(TranscriptUnit {
                    key: format!("{}#{}", db_path.display(), session_id),
                    stamp: mtime_ns(&db_path),
                    source: "vscode",
                });
            }
        }
    }

    // Pass 2: journal + transcript units, but skip sessions OTel covers
    for variant in &VARIANTS {
        if let Some(root) = vscode_user_root(variant) {
            for path in glob_chat_sessions(&root) {
                let sid = extract_session_id(&path);
                if otel_sessions.contains(&sid) { continue; }
                units.push(TranscriptUnit {
                    key: path.to_str()?.to_string(),
                    stamp: mtime_ns(&path),
                    source: "vscode",
                });
            }
        }
    }

    units
}
```

**session_id_for()** splits the key on `#` — for OTel units, extracts the
session id after `#`. For journal/transcript units, extracts from the filename stem.

### Implementation

**units()**: As shown above — enumerate per-session, OTel takes priority.

**load_content()**: Two paths.

For journal/transcript units: read the file directly (raw JSONL).

For OTel units: key is `<db path>#<session-id>`. `load_content()` opens the
SQLite DB, queries spans for that session, returns JSON:

```json
{
  "source": "otel_spans",
  "session_id": "<session-id>",
  "spans": [...],
  "attributes": {...}
}
```

**parse()** dispatches on the source marker in the first line:

```rust
fn parse(&self, content: &str) -> ParsedTranscript {
    if let Some(first_line) = content.lines().next() {
        if first_line.contains("\"source\":\"otel_spans\"") {
            return parse_otel_content(content);
        }
        if is_delta_journal(first_line) {
            return parse_journal_content(content);
        }
    }
    parse_transcript_content(content)
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

### Session identity across layers

The OTel `traceId` corresponds to a VS Code window session. The `chatSessions/<uuid>`
stem is a chat thread within that session. Multiple chat threads may share one OTel
trace. The plan assumes `traceId == chatSession uuid` for priority matching. If this
does not hold, the fallback is: ingest all three separately (no priority skip).
Verification required during implementation — check a real `agent-traces.db` against
the corresponding `workspaceStorage/*/chatSessions/` files.

### Session ID

`vscode-<session-id>` where `<session-id>` is the UUID filename stem or the
OTel session id.

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

### Dedup and merge

**One unit per session.** `units()` dedups by session ID — if both events.jsonl
and session-store.db have the same session, emit ONE unit. The merge happens
inside `load_content()`, which reads both sources and returns a single JSON blob
(fix A).

`stamp_for()` returns `max(jsonl_mtime_ns, sqlite_updated_at_as_ns)` — both
must be in the same monotonic unit (nanoseconds). Convert sqlite epoch millis
to nanos: `updated_at_millis * 1_000_000`.

### Implementation

**units()**: Deduped by session ID.

```rust
fn units(&self) -> Vec<TranscriptUnit> {
    let mut by_session: HashMap<String, TranscriptUnit> = HashMap::new();

    // Source A: events.jsonl
    for dir in glob_session_dirs(&copilot_home().join("session-state")) {
        let sid = dir.file_name()?.to_str()?;
        let events = dir.join("events.jsonl");
        let stamp = mtime_ns(&events);
        by_session.entry(sid.to_string()).or_insert_with(|| TranscriptUnit {
            key: dir.to_str()?.to_string(),
            stamp,
            source: "copilot_cli",
        });
        // Update stamp to max
        if let Some(u) = by_session.get_mut(sid) {
            u.stamp = u.stamp.max(stamp);
        }
    }

    // Source B: session-store.db
    let db_path = copilot_home().join("session-store.db");
    if db_path.exists() {
        if let Ok(sessions) = query_session_ids(&db_path) {
            for (sid, updated_at_millis) in sessions {
                let stamp_nanos = (updated_at_millis as u64) * 1_000_000;
                by_session.entry(sid.clone()).or_insert_with(|| TranscriptUnit {
                    key: format!("copilot-sqlite-{}", sid),
                    stamp: stamp_nanos,
                    source: "copilot_cli",
                });
                if let Some(u) = by_session.get_mut(&sid) {
                    u.stamp = u.stamp.max(stamp_nanos);
                }
            }
        }
    }

    by_session.into_values().collect()
}
```

**session_id_for()**: Both return `copilot-<session_id>`.

**load_content()**: Merge both sources into one JSON blob.

For sessions with events.jsonl:
1. Read `events.jsonl` line-by-line into a JSON array
2. Read `workspace.yaml` for `cwd:` line
3. If session-store.db also has the session, query turns + tokens
4. Merge: JSONL provides tool events + text, SQLite provides cleaner text
   for turns JSONL may have truncated. **JSONL text wins** when both sources
   have the same turn (JSONL is the primary record). SQLite fills turns
   that JSONL lacks.
5. Return JSON: `{"source":"merged","cwd":"...","events":[...],"sqlite_turns":[...],"tokens":{...}}`

For sessions only in session-store.db:
1. Query `sessions` + `turns` for the given session
2. Optionally query `data.db` for token aggregates
3. Return JSON: `{"source":"sqlite","session":{...},"turns":[...],"tokens":{...}}`

**parse()** dispatches on source:

```rust
fn parse(&self, content: &str) -> ParsedTranscript {
    let root: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return ParsedTranscript::default(),
    };
    match root.get("source").and_then(|s| s.as_str()) {
        Some("merged") | Some("jsonl") => parse_copilot_jsonl(content),
        Some("sqlite") => parse_copilot_sqlite(content),
        _ => ParsedTranscript::default(),
    }
}
```

Each path produces `ParsedTranscript` through the same sub-functions:

| Function | JSONL/merged path | SQLite-only path |
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

### Shared constant move (Phase 0)
- `shared_constants_in_mod` — `MAX_TURN_CHARS` accessible from `super::`
- `human_prompt_text_reused` — cursor.rs calls `human_prompt_text()` from mod.rs
- `claude_adapter_still_passes` — no regression after move
- `zed_adapter_still_passes` — no regression after move

### CursorAdapter tests
- `parse_simple_turn` — user → assistant text
- `parse_multiple_turns` — sequential turns
- `parse_tool_use_events` — tool_use lines become SynthEvents with tool_input
- `parse_injected_messages` — system/injected user messages skipped
- `parse_turn_attrs_strips_message` — message.content not in attrs
- `turn_facts_populated` — gitBranch, stop_reason from records
- `turn_facts_null_when_absent` — honest-empty, never fabricated zeros
- `units_flat_layout` — flat `agent-transcripts/<id>.jsonl` → session id from file stem
- `units_nested_layout` — nested `<id>/<id>.jsonl` → session id from directory name
- `stamp_matches_mtime` — cursor semantics
- `session_id_from_key` — `cursor-<uuid>` (both layouts)
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
- `units_otel_skips_journal_for_covered_sessions` — OTel priority applied in units()
- `units_otel_per_session` — one unit per session from DB, not one blob
- `session_id_from_key` — `vscode-<uuid>` (journal) and `vscode-<trace-id>` (otel)
- `parse_session_reconstructs_events` — UserPromptSubmit + PostToolUse + Stop
- `parse_session_none_when_empty` — empty content → None

### CopilotCliAdapter tests
- `parse_events_jsonl_turns` — user.message + assistant.message → turns
- `parse_events_jsonl_tool_use` — tool_use + tool_result → SynthEvents
- `parse_session_shutdown_tokens` — token extraction from shutdown event
- `parse_workspace_yaml_cwd` — cwd extraction
- `parse_sqlite_turns` — session-store.db turns → turns
- `parse_sqlite_limited_events` — no tool events from SQLite
- `merge_jsonl_and_sqlite_text_wins` — JSONL text preferred, SQLite fills gaps
- `units_dedup_by_session_id` — one unit per session, not two
- `stamp_is_monotonic` — both sources produce nanosecond stamps
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
| Cursor | Model not in JSONL | `extract_model` returns `None`; model captured from runtime context |
| VSCode journals | Token estimates only (chars/4 when no metadata) | Cost metrics approximate |
| VSCode transcripts | No token data at all | `SessionTokens` `None` for transcript-only sessions |
| VSCode OTel vs journal identity | OTel traceId ↔ chatSession uuid mapping unverified | Priority skip may not match; fallback: ingest all separately |
| Copilot CLI SQLite | No tool events | SynthEvents empty; turns only |
| Copilot CLI JSONL | Token counts only at `session.shutdown` | Per-turn tokens unavailable |

---

## Implementation order

0. **Hoist shared constants + `human_prompt_text` into `mod.rs`** (pure move, green suite)
1. Enum values via `dbd reconcile`
2. `CursorAdapter` — with UUID detection for session_id (fix H)
3. `CopilotCliAdapter` — with one-unit-per-session merge (fix A), monotonic stamps
4. `VscodeAdapter` — with per-session OTel units and priority in `units()` (fix B)
5. Tests
6. Integration smoke test with real local data (if available)

---

## Files to modify

| File | Change |
|---|---|
| `crates/senseid/src/transcript/mod.rs` | Hoist shared constants; add path helpers; register in `adapters()` + `adapter_for_source()` |
| `crates/senseid/src/transcript/claude.rs` | Remove moved constants, point to `super::` |
| `crates/senseid/src/transcript/zed.rs` | Remove moved constants, point to `super::` |
| `crates/senseid/src/transcript/cursor.rs` | **New** — CursorAdapter |
| `crates/senseid/src/transcript/vscode.rs` | **New** — VscodeAdapter |
| `crates/senseid/src/transcript/copilot_cli.rs` | **New** — CopilotCliAdapter |
| `database/ddl/enum/sensei/assistant_family.ddl` | Add `vscode`, `copilot` |
| `docs/backlog.md` | Mark as planned |

---

## Resolved decisions

1. **Cursor model extraction:** Accept `None`. JSONL does not carry model info.
   Model is captured from runtime context. `store.db` lookup deferred — adds
   complexity for minimal value.
2. **Copilot merge precedence:** JSONL text wins when both sources have the same
   turn (JSONL is the primary record). SQLite fills turns that JSONL lacks.
   Token counts from `session.shutdown` override SQLite estimates.
3. **VSCode session identity:** OTel `traceId` ↔ `chatSessions/<uuid>` mapping
   is assumed but unverified. Priority skip is best-effort — if it does not hold,
   fallback is to ingest all three layers separately. Verification during
   implementation with real data.
