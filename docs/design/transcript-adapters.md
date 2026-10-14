# Transcript Adapters — state and remaining gaps

> **Source:** 2026-08-26 Copilot adapter review — kept with the shared transcripts, not in this repository (it names the people who shared them)
> **Status:** assimilated 2026-10-13 — fixed items are in code; remaining gaps are in
> [`roadmap/06-gaps.md`](../roadmap/06-gaps.md) as C-16..C-24.

## Current state by adapter

### Copilot CLI (`crates/senseid/src/transcript/copilot_cli.rs`) — **FIXED**

| Issue | Was | Now | Evidence |
|---|---|---|---|
| Event names | `tool_use` / `tool_result` (0 occurrences) | `tool.execution_start` / `tool.execution_complete` | 77,139 events across 9 sessions |
| Model extraction | `session.model_change` only (2/77,139) | `assistant.message` + `assistant.turn_start` + `session.model_change` | Every turn names model |
| `extract_jsonl_model` scan | `?` operator abandoned file on one missing model | `continue` — next event almost certainly has it | Verified against real data |
| Test fixture | Invented `tool_use` / `tool_result` | Real wire format from live transcript | `EVENTS_JSONL` const |
| Integration test | None | `parses_a_real_copilot_session_when_one_is_available` (env `SENSEI_COPILOT_SAMPLE`) | Asserts tools>0, prompts>0, model resolved, tokens>0 |

**Parses correctly today:**
- User prompts and assistant text
- Tool calls (`tool.execution_start` → `PostToolUse` with `toolName`/`arguments`)
- Model per turn
- Token totals from `session.shutdown` (cumulative, last wins; summed across models; camelCase usage keys; `requests.cost` for premium)
- Session IDs and cwds

### VS Code (`crates/senseid/src/transcript/vscode.rs`) — **FIXED**

| Issue | Was | Now | Evidence |
|---|---|---|---|
| Journal format | `path` + `value` | `k` (array) + `v` | Confirmed by parsing participant-b's journals |
| Path segments | Strings only | Strings **or integers** (409/516 integers in one journal) | Fixed |
| `kind:2` | Replace | **Appends** — reply streamed in pieces | Fixed |
| Turn extraction | Looked for `role` + `responseParts` | Each `requests[]` entry = one exchange; prompt in `message.text`, reply in untagged `response[]` | Fixed |
| Root parameter | Stored, never read (clippy: never read) | **Explicit root overrides installed-editor scan** | Accepts `User/` or bare `workspaceStorage/` |
| Windows paths | `file:///c%3A/...` → `/c%3A/...` (no match) | Percent-decoded, drive unwrapped → `c:/Users/…` | 10/10 Windows workspaces now resolve |
| OTel sessions | Flat | Per-session units keyed `<db>#<session-id>` | Blocking plan issue resolved |

**Three storage layers covered:**
1. Chat session journals (delta journals, 84 files in sample)
2. Newer transcript event streams (37 files)
3. OTel span store (SQLite, `agent-traces.db`)

### Cursor (`crates/senseid/src/transcript/cursor.rs`) — **DESIGN ONLY**

| Aspect | Status |
|---|---|
| Layout handling | ✅ Both nested (`<id>/<id>.jsonl`) and flat (`<id>.jsonl`) via `session_id_for` heuristic |
| Event parsing | ✅ `user`, `assistant`, `tool_use`, `system` (skipped) |
| SynthEvent | ✅ `UserPromptSubmit`, `PostToolUse`, `Stop` |
| Token facts | ✅ `gitBranch`, `stop_reason`, `input_tokens`, `output_tokens` |
| **Model info** | ❌ Cursor JSONL does not carry model |
| **Tool result** | ❌ Known limitation — no `tool_result` content |
| **Real sample** | ❌ No sample data in review — nobody uses Cursor |

---

## Remaining gaps (from 2026-08-26 review)

These are **not fixed in code** and are tracked in [`roadmap/06-gaps.md`](../roadmap/06-gaps.md):

### C-16 (P1) — Copilot CLI: `SynthEvent` lacks `tool_success` field
- **Signal:** `tool.execution_complete` carries `success: false` (234 of 10,507 = 2.2% failed)
- **Impact:** The only direct measure of agent friction is discarded. `SynthEvent` has no field for it.
- **Blocker:** Requires `SynthEvent` schema change + DB column + all adapter parsers.

### C-17 (P2) — Copilot CLI: turn duration not captured
- **Signal:** `assistant.turn_start` → `turn_end` timestamps available
- **Impact:** Latency per turn and its long tail not measurable.

### C-18 (P2) — Copilot CLI: `codeChanges` not captured
- **Signal:** `session.shutdown` includes `codeChanges` (real lines added/removed, files touched)
- **Impact:** Real diff stats per session discarded.

### C-19 (P2) — Copilot CLI: per-model token usage with cache read/write not fully captured
- **Signal:** `session.shutdown` carries `totalPremiumRequests`, per-model `cacheReadTokens`, `cacheWriteTokens`, `reasoningTokens`
- **Impact:** Cost and context-reuse signals partially parsed but not all fields.

### C-20 (P3) — Copilot CLI: `session.permissions_changed` not captured
- **Signal:** Every point the agent stopped and waited for a human
- **Impact:** Permission friction not measurable.

### C-21 (P2, known limitation) — VS Code: no tool outcome recorded
- **Signal:** `toolInvocationSerialized` says a tool ran, never whether it worked
- **Impact:** Failure rate computed from it is 0/N — reads as flawless rather than unknown.

### C-22 (P2, known limitation) — VS Code: no tokens of any kind recorded

### C-23 (P2, known limitation) — VS Code: turn latency not measurable
- **Signal:** `responseTimestamp` usually equals `timestamp`, elapsed time not measurable
- **Impact:** Any rate derived from it describes the format, not the person.

### C-24 (P1) — Cursor adapter: no real sample data to verify against
- **Status:** Only adapter with no real data behind it. `session_id_for` handles both layouts but untested against real transcripts.

---

## What would fix the highest-value gaps

| Gap | What it needs |
|---|---|
| **C-16** `tool_success` | 1. Add `success: Option<bool>` to `SynthEvent` (mod.rs) 2. Add column to `assistant_events` 3. Parse `tool.execution_complete.success` in Copilot CLI 4. Parse OTel `gen_ai.tool.success` if present 5. Update all adapter tests |
| **C-17** turn duration | Add `turn_duration_ms` to `TurnFacts` or `TranscriptTurn`; parse from `assistant.turn_start`/`turn_end` |
| **C-24** Cursor real data | Acquire a Cursor transcript sample; add `SENSEI_CURSOR_SAMPLE` integration test |

---

## Related

- [`roadmap/06-gaps.md`](../roadmap/06-gaps.md) — C-16..C-24
- [`crates/senseid/src/transcript/`](../../crates/senseid/src/transcript/) — three adapter files
- [`docs/reorg.md`](../reorg.md) §12 — gap register framework