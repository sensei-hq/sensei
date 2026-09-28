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

## VS Code — confirmed against rajkumar's data, two bugs fixed

**rajkumar's folder IS VS Code sample data** — a `workspaceStorage` tree, 10
workspace directories, each with `workspace.json`, `state.vscdb`, `chatSessions/`
and `GitHub.copilot-chat/`. Layers present:

| Layer | Files |
|---|---:|
| 1 — `chatSessions/*.jsonl` (delta journals) | 84 |
| 2 — `GitHub.copilot-chat/transcripts/*.jsonl` | 37 |
| 3 — `agent-traces.db` (OTel) | 0, tracing not enabled |

Also: `workspace.json` IS present here, on all 10. My earlier note in
`docs/transcript-paths.md` said it was often absent — that was drawn from a
machine with VS Code but no Copilot chat data. Where chat exists, so does the
mapping. Corrected there.

### The journal format was wrong in three ways

Confirmed by parsing rajkumar's journals: the adapter reconstructed **nothing**
from any of them. Three separate mismatches, all now fixed and covered by a test
that runs against real journals when `SENSEI_VSCODE_SAMPLE` is set:

| Adapter read | Actually written |
|---|---|
| `path` (a dotted string) + `value` | `k` (an ARRAY) + `v` |
| string path segments only | segments are strings OR integers — 409 of 516 in one journal are integers, so filtering to strings corrupts nearly every path |
| `kind:2` as a replace | `kind:2` APPENDS: a reply is streamed in pieces, so `requests[N].response` grows across records. Replacing keeps only the last fragment |

The turn extraction was wrong too. It looked for a `role` field and
`responseParts`, neither of which exists: each entry of `requests[]` is ONE
exchange, with the prompt in `message.text` and the reply in untagged `response[]`
parts. Tool calls come from `toolInvocationSerialized` parts, and `modelId`
arrives namespaced (`copilot/claude-opus-4.6`).

### The configured root was ignored

`VscodeAdapter` stored `root` and never read it (clippy: `field 'root' is never
read`). `units()` iterated the built-in `VARIANTS` and resolved OS paths itself,
so the constructor argument was decorative and the adapter could only ever read
the machine it ran on.

**Fixed:** an explicit root now overrides the installed-editor scan, accepting
either a `User/` directory or a bare `workspaceStorage/` one — the shape people
actually send. Proven by a test that walks the sample, counts the `.jsonl` files
on disk and requires the adapter to find **all** of them: 121 of 121 across both
layers. Asserting merely "not empty" would have passed while missing a layer
entirely, since the two live in different subdirectories.

### Windows workspace paths did not decode

`workspace_folder` stripped `file://` and returned the rest verbatim. VS Code
stores Windows folders percent-encoded:

    file:///c%3A/Users/dev.user/Documents/workspace/sample-portal

which yielded `/c%3A/Users/...` — a path matching no directory and no repo, so
**every Windows session lost its project attribution**. All ten of rajkumar's
workspaces are Windows.

**Fixed:** percent-decode and unwrap the drive letter, so it resolves to
`c:/Users/…`. Also accepts the `$mid`-tagged object shape, which some workspaces
use instead of a plain string.

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

## What VS Code does not record

Worth knowing before building metrics on it:

* **No tool outcome.** `toolInvocationSerialized` says a tool ran, never whether
  it worked. A failure RATE computed from it is 0/N — which reads as flawless
  rather than as unknown, so the report shows `n/a`.
* **No tokens** of any kind.
* **`responseTimestamp` usually equals `timestamp`**, so turn latency is mostly
  zero and elapsed time is not measurable. Any rate derived from it describes the
  format, not the person — the report suppresses those too.

So VS Code supports pace and model mix, and nothing about friction or cost.
