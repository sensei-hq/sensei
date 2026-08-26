# Transcript adapter review — Copilot CLI, VS Code, Cursor

**Date:** 2026-08-26
**Checked against:** real transcripts shared by five people
(`~/Developer/sensei-hq/transcripts`, not in this repo)

The sample turned out to cover three different tools, which is more useful than
it first looked:

| Person | Tool | Shape |
|---|---|---|
| manoj | **Copilot CLI** | 9 sessions, `events.jsonl` + `workspace.yaml` + `session.db` |
| rajkumar | **VS Code Copilot Chat** | `chatSessions/*.jsonl`, the `kind:0/1/2` delta journal |
| Balaji, chandra, dipti | **Claude Code** | `projects/<path-slug>/*.jsonl` + `history.jsonl` |

---

## Copilot CLI — the adapter ingested no tool activity at all

**Every event name it looked for was wrong.** Counted across all 9 sessions,
77,139 events:

| Adapter expected | Occurrences | Actually present | Occurrences |
|---|---:|---|---:|
| `tool_use` | **0** | `tool.execution_start` | 10,520 |
| `tool_result` | **0** | `tool.execution_complete` | 10,507 |
| `session.model_change` | 2 | `assistant.message` (carries `model`) | 7,586 |
| `session.shutdown` | 72 | — | — |

So the adapter produced **zero** `PostToolUse` events from real data, and left
`model` NULL on all but two sessions. Prose turns were fine; everything about
what the agent DID was lost.

The unit tests passed throughout, because the fixture used the invented names.

### Fixed

- `tool.execution_start` → `PostToolUse`, reading `toolName` / `arguments`
  (was `name` / `input`).
- Model resolved from `assistant.message` / `assistant.turn_start` as well as
  `session.model_change`.
- `extract_jsonl_model` used `?` inside its scan loop, so one matching event
  without a `model` field abandoned the search for the whole file. Now continues.
- Test fixture replaced with the real wire format.
- Added `parses_a_real_copilot_session_when_one_is_available`, which runs against
  a real session when `SENSEI_COPILOT_SAMPLE` points at one and asserts a session
  yields tool events, prompts and a model. Verified it FAILS when the event name
  is reverted — the guard actually guards.

### Signals we are still discarding

Present in the transcript, not carried into `ParsedTranscript`:

| Signal | Where | Worth having because |
|---|---|---|
| `success: false` on completion | `tool.execution_complete` | The only direct measure of agent friction. 234 of 10,507 calls (2.2%) failed |
| Turn duration | `assistant.turn_start` → `turn_end` | Latency per turn, and its long tail |
| `codeChanges` | `session.shutdown` | Real lines added/removed and files touched |
| `totalPremiumRequests`, per-model token usage incl. cache read/write | `session.shutdown` | Cost, and how well context is being reused |
| `session.permissions_changed` | — | Every point the agent stopped and waited for a human |

`SynthEvent` has no field for tool success, so the failure signal cannot be
represented today. That is the single most valuable addition — see the
`session-report` tool, which reads it directly.

---

## VS Code — the configured root is ignored

`VscodeAdapter` stores `root` and never reads it (clippy: `field 'root' is never
read`). `units()` iterates the built-in `VARIANTS` and resolves OS paths itself,
so the constructor argument is decorative.

Consequence for the work in hand: the adapter **cannot be pointed at a sample
folder**, so rajkumar's shared transcripts cannot be ingested without editing the
code. Every other adapter takes its root as a parameter and honours it.

Otherwise the structure follows the plan review — per-session OTel units keyed
`<db path>#<session-id>`, so `session_id_for` can resolve them. That was the
blocking issue in the plan and it has been handled.

---

## Cursor — handles both layouts correctly

`session_id_for` uses the parent directory when it looks like a UUID and the file
stem otherwise, which covers both the nested `<id>/<id>.jsonl` and flat
`<id>.jsonl` forms. This was point H of the plan review and it is resolved.

No sample data to verify against — nobody in the sample uses Cursor.

---

## Suggested order

1. **Add tool success to `SynthEvent`.** Copilot CLI reports it, and it is the
   friction signal the metrics want. Without it, ingestion knows what was
   attempted but not what worked.
2. **Make `VscodeAdapter` honour its root**, so it can be tested against shared
   samples like every other adapter.
3. **Get a Cursor sample** before trusting that adapter — it is the only one with
   no real data behind it.
