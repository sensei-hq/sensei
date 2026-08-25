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
    // variant = "Code" | "Code - Insiders" | "VSCodium"
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

### Algorithm

```
units():
  glob ~/.cursor/projects/*/agent-transcripts/*/*.jsonl + flat *.jsonl
  stamp = mtime_ns (like Claude)

parse(content):
  for each JSONL line:
    if type == "user" and not injected → start new turn
    if type == "assistant" → append text to current turn's assistant_text
    if type == "tool_use" → record as SynthEvent (best-effort)
  return ParsedTranscript { turns, cwds: [], events, model, tokens: None }
```

- `tokens`: Always `None` (Cursor does not expose token counts)
- `model`: Extract from `store.db` metadata if available, else `None`
- `cwds`: Empty (Cursor JSONL does not reliably carry cwd)

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

### Workspace resolution

Each `chatSessions/<uuid>.jsonl` has a sibling `workspace.json` with the folder
URI. Resolve `file://` URIs to local paths; handle `vscode-remote://wsl+<distro>/`
by extracting the Linux path.

### Algorithm

```
units():
  for each variant in ["Code", "Code - Insiders", "VSCodium"]:
    root = vscode_user_root(variant)
    glob workspaceStorage/*/chatSessions/*.jsonl
    glob globalStorage/emptyWindowChatSessions/*.jsonl
    glob workspaceStorage/*/GitHub.copilot-chat/transcripts/*.jsonl
    glob globalStorage/github.copilot-chat/agent-traces.db (single file, all sessions)
  stamp = mtime_ns

parse(content, source_layer):
  if source_layer == "journal":
    reconstruct from kind:0/1/2 mutations → requests[]
  elif source_layer == "transcript":
    parse event stream → user.message + assistant.message pairs
  elif source_layer == "otel_spans":
    parse spans table → turns with real token counts
  return ParsedTranscript
```

- `tokens`: Populated from `agent-traces.db` when available; estimated from
  `result.metadata.promptTokens/outputTokens` in journals; `None` for transcripts
- `model`: From `selectedModel` (journal) or span attributes
- `cwds`: From `workspace.json` sibling

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

### Algorithm

```
units():
  glob ~/.copilot/session-state/*/events.jsonl → stamp = mtime_ns
  query session-store.db → stamp = max(updated_at) as epoch_nanos
  dedup by session_id

parse_from_jsonl(content, workspace_yaml):
  parse events.jsonl line-by-line
  user.message → turn start
  assistant.message → turn text
  tool_use → SynthEvent (tool name + input)
  tool_result → SynthEvent (tool output — available here, unlike Cursor!)
  session.shutdown → extract SessionTokens
  return ParsedTranscript

parse_from_sqlite(db_path, session_id):
  SELECT turns WHERE session_id = ?
  SELECT sessions WHERE id = ? (for metadata)
  return ParsedTranscript (limited — no tool events, no tokens)
```

- `tokens`: From `session.shutdown` event (JSONL) or `data.db` aggregates
- `model`: From `session.model_change` event or `sessions` table
- `cwds`: From `workspace.yaml` (`cwd:` scalar line)

### Session ID

`copilot-<session-id>` where `<session-id>` is the UUID directory name.

---

## Phase 5 — Tests

Each adapter follows the existing pattern (Claude adapter has 35+ tests):

### CursorAdapter tests
- `parse_simple_turn` — user → assistant text
- `parse_multiple_turns` — sequential turns
- `parse_tool_use_events` — tool_use lines become SynthEvents
- `parse_injected_messages` — system/injected user messages skipped
- `units_flat_layout` — flat `agent-transcripts/<id>.jsonl`
- `units_nested_layout` — nested `<id>/<id>.jsonl`
- `stamp_matches_mtime` — cursor semantics
- `session_id_from_key` — `cursor-<uuid>`

### VscodeAdapter tests
- `parse_journal_mutation` — kind:0/1/2 reconstruction
- `parse_transcript_event_stream` — session.start/user.message/assistant.message
- `parse_otel_spans` — span attributes → turns with tokens
- `workspace_resolution` — file:// and vscode-remote:// URIs
- `units_cross_variant` — Code + Insiders + VSCodium
- `units_empty_window_sessions` — globalStorage/emptyWindowChatSessions
- `priority_otel_over_journal` — agent-traces.db wins
- `session_id_from_key` — `vscode-<uuid>`

### CopilotCliAdapter tests
- `parse_events_jsonl` — full event stream
- `parse_session_shutdown_tokens` — token extraction
- `parse_workspace_yaml` — cwd extraction
- `parse_sqlite_turns` — session-store.db fallback
- `dedup_jsonl_and_sqlite` — merge behavior
- `units_from_both_sources` — glob + SQLite query
- `session_id_from_key` — `copilot-<uuid>`

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
| Cursor | No `tool_result` content | SynthEvents lack output; no PostToolUse events |
| Cursor | No token counts | `SessionTokens` always `None` |
| Cursor | No reliable cwd | `cwds` empty; project resolution falls back to folder hash |
| VSCode journals | Token estimates only (chars/4 when no metadata) | Cost metrics approximate |
| VSCode transcripts | No token data at all | `SessionTokens` `None` for transcript-only sessions |
| Copilot CLI SQLite | No tool events | SynthEvents empty; turns only |
| Copilot CLI JSONL | Token counts only at `session.shutdown` | Per-turn tokens unavailable |

---

## Implementation order

1. DDL enum extension + migration
2. CursorAdapter (simplest — mirrors Claude's file-per-session model)
3. VscodeAdapter (most complex — 3 sources, variant support, workspace resolution)
4. CopilotCliAdapter (dual-source dedup)
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
