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

---

## Review notes (2026-08-26)

Checked against the code at `79cce5c3`. The overall shape is right — the trait,
`ParsedTranscript` and `SynthSession` all match, and `rusqlite` (0.32, bundled)
plus `dirs` (6) are already dependencies, so no new crates are needed. The
SQLite open flags quoted above are exactly what `zed.rs:62` uses.

Six things are wrong or unbuildable as written, and two designs conflict with how
ingestion actually works. Ordered by how much rework they cause if found late.

### A. Two units for one session silently overwrite each other (Phase 4)

**This is the big one.** `ingest_one` (`mod.rs:262`) processes ONE unit at a time
and has no view of a sibling unit. Turns are written with
`ON CONFLICT(source, session_id, turn_index) DO UPDATE` (`transcript.rs:25`).

The plan emits two units per Copilot session — `<dir>` for events.jsonl and
`copilot-sqlite-<id>` for the DB — and both resolve to the same `session_id` under
the same `source`. So they do not merge; they **overwrite by turn index**, and
which one wins depends on unit ordering. Worse, if JSONL yields 10 turns and
SQLite 8, the SQLite pass rewrites 0–7 and leaves 9–10 from JSONL: a spliced
session that matches neither source.

"If both exist → merge: take richer turn data" describes a step the architecture
has nowhere to put.

**Fix:** emit ONE unit per session and merge inside `load_content()`, which is
free to read both sources and return a single blob. `units()` dedups by session
id (key = the session dir, or a `copilot:<id>` synthetic key), `stamp_for()`
returns `max(jsonl mtime_ns, sqlite updated_at)`, and the merge happens where the
plan already wants it — in the JSON it hands to `parse()`.

Note the two stamps are also on different scales as written (`mtime_ns` vs epoch
millis). Whatever `stamp_for` returns must be monotone in the same unit, or the
watermark comparison `prev >= stamp` misfires.

### B. The OTel unit cannot resolve a session id (Phase 3)

`session_id_for(key) -> Option<String>` returns exactly one session per key, and
`ingest_one` bails when it is `None`. The plan makes `agent-traces.db` a **single
unit per variant** whose content bundles "ALL sessions' turns". There is no
session id for that key, so the unit can never be ingested.

**Fix:** enumerate sessions in `units()` (query distinct trace/session ids) and
emit one unit each, keyed `<db path>#<session-id>`, so `session_id_for` can split
the key. `load_content()` then queries just that session's spans.

The same constraint kills the **priority order** as described: "skip
journal+transcript for that session" cannot be decided inside `parse()`, which
only sees one unit's content. It has to be done in `units()` — which is fine,
because `units()` sees all sources at once and can drop journal/transcript units
for sessions the OTel DB already covers.

### C. The enum change is declarative here, not a migration (Phase 1a)

The `ALTER TYPE … ADD VALUE IF NOT EXISTS` snippet and the "DDL enum extension +
migration" step do not match this project. dbd is declarative and pre-release:
edit `database/ddl/enum/sensei/assistant_family.ddl`, add the values, then
`dbd reconcile`.

Verified — with `vscode` and `copilot` appended to that file, `dbd diff` emits:

    ~ alter  sensei.assistant_family
        ALTER TYPE sensei.assistant_family ADD VALUE 'vscode';
        ALTER TYPE sensei.assistant_family ADD VALUE 'copilot';

Two things to know: dbd emits `ADD VALUE` WITHOUT `IF NOT EXISTS` (safe, because
it diffs first), and `ALTER TYPE … ADD VALUE` cannot run inside a transaction
block — so it is a standalone statement, not part of a batched apply.

`cursor` is indeed already in the enum, as the Scope table says.

### D. `parse()` as written does not compile (Phase 4)

    fn parse(&self, content: &str) -> ParsedTranscript {
        let root: serde_json::Value = serde_json::from_str(content).ok()?;

`?` on an `Option` in a function returning `ParsedTranscript`. Use a `let … else`
returning `ParsedTranscript::default()`, matching the `_ =>` arm already in the
match below it.

### E. `human_prompt_text()` is private to `claude.rs`

`claude.rs:449` — not `pub`, not re-exported. The Cursor turn-parsing step calls
it. Same for `INJECTED_MARKERS` (`claude.rs:38`).

### F. The "shared constants" are not shared, and copying them again breaks a hard rule

`MAX_TURN_CHARS` is already declared separately in `claude.rs` AND `zed.rs`.
`MAX_TRANSCRIPT_BYTES` / `MAX_LINE_BYTES` exist only in `claude.rs`. The plan
re-declares them in `cursor.rs` (and implies the same for the other two), which
would leave five copies of `MAX_TURN_CHARS`.

CLAUDE.md is explicit: *"Three near-identical lines are a sign to refactor — not
a reason to add a fourth."*

**Do this first, as its own commit:** hoist `MAX_TURN_CHARS`,
`MAX_TRANSCRIPT_BYTES`, `MAX_LINE_BYTES`, `INJECTED_MARKERS` and
`human_prompt_text()` into `mod.rs`, repoint `claude.rs`/`zed.rs` at them, and
confirm the suite is still green. A pure move, easy to review, and it unblocks E
as a side effect.

### G. `dirs::data_dir().unwrap()` panics on a failure path (Phase 1b)

The Windows arm of `vscode_user_root`. `data_dir()` returns `Option` precisely
because it can fail. Panicking in the daemon over a missing known-folder is
exactly what the never-fabricate/fail-closed rule forbids.

Make it `fn vscode_user_root(variant: &str) -> Option<PathBuf>` and let `units()`
skip a variant it cannot locate — an absent editor is a normal state, not an
error.

### H. Cursor's `session_id_for` contradicts its own `units()`

`units()` globs both the nested `<id>/<id>.jsonl` and the flat
`agent-transcripts/<id>.jsonl`. `session_id_for` says the session id is "the
directory name (NOT the file stem)" — but for the flat layout the directory is
literally `agent-transcripts`, which would make every flat session collide under
one id.

Needs both cases: use the parent directory name when it parses as a UUID,
otherwise the file stem.

### Smaller notes

* **Test count.** `adapters()` currently returns 3 (`mod.rs:239`), so the
  `backfill_all_includes_new_adapters` expectation of 6 is right.
* **Source strings.** `adapter_for_source` matches on `"claude_code"` etc.; the
  new arms are `"cursor"`, `"vscode"`, `"copilot_cli"` per the Scope table. The
  `copilot_cli` source / `copilot` family split mirrors `claude_code` / `claude`.
* **`crate::paths::home()`** exists (`paths.rs:15`) and is the right helper.
* **Cursor `store.db`.** `extract_model` is specced to read it, but `parse()`
  only receives `content` — the JSONL. Either fold the `store.db` lookup into
  `load_content()` (return a blob carrying both, like the Copilot design) or drop
  model extraction for Cursor and let it stay `None`.

### Suggested order (revised)

0. **Hoist the shared constants + `human_prompt_text` into `mod.rs`** (pure move,
   green suite) — fixes E and F before they multiply.
1. Enum values via `dbd reconcile` (C).
2. `CursorAdapter` — after resolving H and the `store.db` question.
3. `CopilotCliAdapter` — with the one-unit-per-session merge from A.
4. `VscodeAdapter` — with per-session OTel units and priority applied in
   `units()` (B).
5. Tests.

### Open questions — no longer none

1. **Cursor model extraction** — fold `store.db` into `load_content()`, or accept
   `None`? (smaller note above)
2. **Copilot merge precedence** — when JSONL and SQLite disagree on the same
   turn's text, which wins? A says merge in `load_content()`, but not what to
   prefer. Suggest: JSONL text (it is the primary record), SQLite only to fill
   turns JSONL lacks.
3. **VSCode session identity across layers** — does the OTel `traceId` equal the
   `chatSessions/<uuid>` stem? The priority order in B assumes it does. If not,
   there is no way to tell that two layers describe one session, and the whole
   priority scheme collapses to "ingest all three separately".

