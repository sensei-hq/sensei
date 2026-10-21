# Capture & Sessions · M3

**Plane:** foundation · **Status:** [roadmap/01-foundation.md](../../../../roadmap/01-foundation.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)), team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

Capture is the front door of the whole system — nothing sensei knows about the developer's
work exists without it. Four paths feed the pipeline: a one-shot filesystem scan that turns
real folders into repos and multi-repo workspaces; a continuous root-watcher that keeps the
graph current without manual re-scans; git branch-switch handling that flips the view to the
checked-out branch; and assistant-event capture that turns every session into a stream of
events (session start, prompt, tool call, tool result, session end). Sessions materialise
from those events and every downstream signal — FTR, insights, memories — falls out of them.
Each assistant's adapter also reports what the work **consumed**: tokens and cost, under one
contract that every assistant meets ([§Usage & cost capture](#usage--cost-capture--the-adapter-contract)).
The user's experience: *point sensei at a folder once and it just keeps knowing*.

## In scope / out of scope

- In: root-based folder/project scan (incremental, content-hash); multi-repo workspace
  detection + suggestion + merge/split; the root-watcher with restart reconciliation; branch
  versioning (active-only default, copy-on-switch); assistant-event capture via per-family
  adapters (Claude Code live, Zed read-only, custom via MCP `log_event`); event
  secret-redaction; session materialisation on the analyzer tick; **usage & cost capture**
  (tokens, cost, context occupancy, plan headroom) through the per-family adapter contract.
- Out: the code-graph *indexing* semantics (M4 — capture *finds and queues*; M4 *parses and
  resolves*); FTR computation and memory/insight generation (later planes); dojo
  auto-discover join (T1); the preferences surface for roots (M2).

## Surfaces

- **API** — `POST /api/scan/roots`, `GET /api/projects/suggestions`, `POST /api/projects/merge`
  (+ split), `crates/senseid/src/api/handlers/sessions.rs::materialise_session`,
  `crates/senseid/src/api/handlers/sessions.rs` (`ingest_hook_event`, `hook_gate`,
  `hook_nudge` — the `/hook/*` event ingest), MCP `log_event`.
- **Daemon** — scanner (`crates/senseid/src/indexer/scan_repo.rs`,
  `crates/senseid/src/tasks/handlers/scan.rs`), watcher (`watcher/root_watcher.rs`).
- **Adapters** — `crates/senseid/src/adapters.rs`'s trait with `adapters/{config,manifest}`
  children; Claude Code hook plugin at `marketplace/plugins/sensei/hooks/`; the Claude Code
  usage mod `marketplace/plugins/sensei/hooks/usage.ts`; transcript readers implementing
  `TranscriptAdapter` (`crates/senseid/src/transcript/{claude,opencode,zed,copilot_cli,cursor,vscode}.rs`).
- **App** — Observatory workspace list shows discovered workspaces and multi-repo suggestions;
  Sessions list reads `activity.sessions`.

## Data invariants

- `sensei.folders_to_watch` holds the user's watch roots; a folder whose `content_hash` is
  unchanged since the last scan is skipped end-to-end (incremental).
- `sensei.folders` — one row per discovered repo/folder under a root
  (`path, name, kind, project_id?, scan_state, content_hash, stack, manifest_summary`);
  `sensei.projects` is the logical grouping — a folder reaches its workspace only through the
  junction (`folders.repository_id → repositories_in_projects → projects`), never a column of
  its own.
- Watch roots have **one watcher per active root**; file events are debounced (default
  ~500ms) and coalesced (a bulk checkout collapses to one reconciliation pass).
- Restart-safety: on boot the watcher compares each root against the last-persisted content
  hashes; any drift becomes a reconciliation pass.
- Branch versioning: `sensei.nodes`/`sensei.edges` carry a `branch` column; the active-branch
  view is a filter; switch = copy-on-switch (reuse unchanged rows with the new branch tag,
  enqueue adds, retract removes from the active view, re-parse modified).
- Assistant events are the atoms: `activity.assistant_events` — one row per event with
  `client_session_id, assistant_family, assistant_id, event_kind, tool_name, ts_ms, payload,
  duration_ms?, error?`. No aggregation at capture time.
- Session materialisation: every session with hook events gets an `activity.sessions` row
  after the next analyzer tick, with `analyzed_at`, `ftr`, `corrections` after enrichment.
- Session-id: `activity.sessions.id` is a sensei UUID; `assistant_events.session_id` is the
  adapter's client-side id; every join resolves through `sessions.client_session_id`.
  (Regression trap, explicitly.)
- **No PII/secrets egress**: payloads are stored locally only; payloads matching a secret
  pattern list are redacted at capture with a logged redaction.

## Usage & cost capture — the adapter contract

What a session *consumed* is captured per assistant, at the most authoritative source that
assistant exposes, into one shape — so every metric that reads tokens or cost (M8) works the
same whichever assistant ran the session. **Any new assistant (ACP) adapter or extension
adopts this contract**; it is how Claude Code was brought up to it (#238) and the checklist
for the rest.

### What an adapter reports

| Figure | Shape | Lands in | Rule |
|---|---|---|---|
| **Tokens, split** | fresh input · cache read · cache write · output · reasoning (where the source separates it) | `activity.sessions.{tokens_fresh, cache_read, cache_write, tokens_out, tokens_reasoning}`; `tokens_in` stays the folded total | Never fold cache into fresh: cache reads are 96–98% of input and price ~10× cheaper (#177). |
| **Model** | the model id the provider reported | `activity.sessions.model`; per turn on `TurnComplete.usage.model` | The real id, never a default family. |
| **Billed cost** | what the user is actually charged, in currency units | `activity.sessions.metered_cost` | `0.0` on a subscription is a **real reading**, distinct from NULL (not reported). |
| **List-price cost** | the assistant's own priced estimate (e.g. Claude Code's `/cost`) | `activity.sessions.list_cost_usd` | **Not** billed spend; never written into `metered_cost`. Keeps the largest total reported for the session. |
| **Per-turn usage** | the four token counts + model for one turn | `TurnComplete` event payload (`activity.assistant_events`) | Optional where the source is per-session only. |
| **Context occupancy** | `{ tokens, window, percent }` | `SessionMeasure` event payload | Event-only today; no metric reads it yet. |
| **Plan headroom** | rate-limit windows `[{ kind, percentUsed, resetsAt }]` | `SessionMeasure` event payload | Event-only today; empty off a subscription. |

A figure the source does not have is **absent** (NULL / key omitted), never zero and never
a default — the no-fabrication rule applies at capture.

### Where it comes from — one source per figure

Per assistant, each figure has **one** owning source. Different figures may come from
different sources (Claude Code's tokens from its transcript, its cost from the mod), but a
figure is never read from source A and then source B if A misses: a fallback makes the
provenance of a number unknowable.

Sources, most authoritative first — an adapter uses the highest one its assistant offers:

1. **In-process extension at the source** — the assistant's own event API, running inside it.
   Claude Code: the usage mod (`session.measure`, `turn.complete`). The shape new extensions
   (an OpenCode plugin, a Copilot/VS Code extension, a Cursor hook) should emit.
2. **Assistant hook payloads** — when the assistant's hooks carry usage (Claude Code's do
   not).
3. **Transcript adapter** — `TranscriptAdapter::parse` returns `ParsedTranscript.tokens`
   (`SessionTokens`: the split, plus `cost` = **billed** cost where the source records it).
4. **MCP `log_event`** — for an assistant with no hooks and no readable transcript.

### The wire shape (sources 1, 2, 4)

An extension posts to `POST /hook/event` (dead-lettering to `~/.sensei/events.jsonl` when the
daemon is unreachable, as the hook scripts do). The daemon stores every event as-is and folds
what it says about the session through one step (`capture_drain::apply_to_session`) on both
the live and the drain path:

```jsonc
// once per turn and whenever a figure moves — cumulative per session
{ "hook_event_name": "SessionMeasure", "session_id": "…", "assistant_family": "<family>",
  "ts": 1760000000000,
  "context": { "tokens": 62980, "window": 1000000, "percent": 6 },
  "rate_limits": [{ "kind": "five_hour", "percentUsed": 8, "resetsAt": "…" }],
  "cost": { "usd": 0.42 },            // list-price estimate; omitted when none
  "changed": ["context", "cost"] }

// once per turn
{ "hook_event_name": "TurnComplete", "session_id": "…", "assistant_family": "<family>",
  "ts": 1760000000000, "turn_id": "…", "agent_id": "…" /* subagent turns only */,
  "reason": "answer", "duration_ms": 8123,
  "usage": { "input_tokens": 2, "output_tokens": 4, "cache_read_input_tokens": 10327,
             "cache_creation_input_tokens": 52651, "model": "claude-opus-5-5" } }
```

A **billed** figure an extension knows (metered API usage) needs its own field and its own
write into `metered_cost` — it must not ride `cost.usd`, which means list price.

### Coverage today

| Assistant | Tokens split | Model | Billed cost | List cost | Per-turn / context / headroom |
|---|---|---|---|---|---|
| Claude Code | transcript | transcript | — (plan; modelled by `cost.subscription`) | **mod** | **mod** |
| OpenCode | transcript | transcript | transcript (`0.0` on a plan) | — | — |
| Zed | transcript | transcript | — | — | — |
| VS Code | transcript (OTel sessions only; C-22) | — | — | — | — |
| Copilot CLI | partial (C-19) | — | — | — | — |
| Cursor | — (C-24) | — | — | — | — |

Closing a dash is adapter work against this contract, not a new design ([C-25](../../../../roadmap/06-gaps.md)).

## Signals shown

| Signal | Source | Consumer |
|---|---|---|
| Discovered repo/project rows | scan | every screen; Workspace window Overview |
| Multi-repo suggestion card | scan analyzer | Observatory Workspaces pane |
| `activity.assistant_events` rows | adapters / hooks | analyzer materialisation |
| `activity.sessions` rows (ftr, corrections, analyzed_at) | materialisation | FTR, Sessions, replay |
| Branch indicator ("main / feat-x") | branch scan-mode | code-graph + impact views |
| Session tokens (split) + model | transcript adapters | M8 usage/cost metrics, Sessions |
| `metered_cost` (billed) / `list_cost_usd` (list price) | transcript adapter / usage mod | M8 cost metrics |
| `SessionMeasure` / `TurnComplete` events | usage mod (extensions) | per-turn usage, context occupancy, plan headroom (future M8 signals) |

## Done gate

- A fresh scan of `~/Developer` + `~/Work` turns every discovered repo into a
  `sensei.folders` row within the incremental scan window; nested roots do not get
  double-indexed (outer subsumes).
- Multi-repo groups appear as proposal cards; accept merges via `POST /api/projects/merge`;
  dismiss is remembered for the 7-day staleness window.
- Every Claude Code session with the plugin installed produces `assistant_events` rows
  continuously (not batched at session end).
- Every session resolves by either the observatory UUID or the `client_session_id` — both
  return the same session.
- A change that landed while the daemon was down is discovered on next boot with no user
  action (restart reconciliation).
- A prompt payload containing a secret never lands verbatim in `assistant_events.payload`.
- A Claude Code session with the plugin's usage mod produces `SessionMeasure` and
  `TurnComplete` events, and its `activity.sessions.list_cost_usd` equals the session's
  `/cost` total (`claude -p --output-format json`'s `total_cost_usd`).

```bash
# events flowing in the last minute?
psql -A -t -c "select count(*) from activity.assistant_events
               where ts_ms > (extract(epoch from now()) - 60) * 1000" -d sensei
# suggestions pending?
curl -s http://localhost:7744/api/projects/suggestions | jq '.suggestions'
# usage captured at the source? (list cost per Claude Code session, last day)
psql -A -c "select client_session_id, list_cost_usd, metered_cost from activity.sessions
            where started_at > now() - interval '1 day' and acp_id = 'claude'" -d sensei
# id resolution consistent?
curl -s http://localhost:7744/api/sessions/$UUID | jq '.client_session_id'
curl -s http://localhost:7744/api/sessions/$CLIENT_ID | jq '.id'   # same session
```

## Wrong gate

- **Every folder becomes its own workspace** — multi-repo detection never fires.
- **Suggestion cards re-appear immediately after dismiss** — no staleness persistence.
- **Events arrive but sessions never materialise** — analyzer tick or the client-session
  join is broken.
- **`assistant_events` grows past retention with no pruning** — the append-only stream slows
  the scanner.
- **A file added while the daemon was down is invisible until the periodic tick** — restart
  reconciliation missing.
- **Branch switch leaves the old branch's rows in the active view** — copy-on-switch didn't
  fire or the branch tag/filter is broken.
- **Two adapters collide on `client_session_id`** — id namespace must be `{family}:{raw_id}`.
- **A secret-bearing prompt persisted verbatim** — redaction pattern list missed it.
- **A list-price figure written as billed cost** (or vice versa) — `metered_cost` and
  `list_cost_usd` mean different things; a subscription session would read as API spend.
- **A missing token or cost figure stored as `0`** — absence must stay NULL.
- **A figure read from a second source when the first misses** — provenance unknowable.
- **Cache reads folded into fresh input** — inflates any cost read off tokens ~8×.
- **Merging two workspaces loses FTR history** — session project_id attribution must survive a merge.

## Scenarios

- [`scenarios/capture.feature`](scenarios/capture.feature) — scan, watcher, branch flips,
  event capture, materialisation, redaction.

## Open gaps

- [C-11](../../../../roadmap/06-gaps.md) — transcript-scanning review not scoped as a task (P1).
- [C-13](../../../../roadmap/06-gaps.md) — Claude "mods" supersede plugin + hooks; install path
  and migration not verified (P1). The usage mod ships in the plugin (`hooks/hooks.json` →
  `modules`); it reaches installs at the next marketplace publish.
- [C-14](../../../../roadmap/06-gaps.md) — multi-assistant expansion: Claude + OpenCode first,
  others later; the extension seam is not specified (P1). For usage & cost the seam **is**
  specified — §Usage & cost capture.
- [C-25](../../../../roadmap/06-gaps.md) — usage & cost contract not yet met by OpenCode
  (no list cost / per-turn), Zed, VS Code, Copilot CLI, Cursor (P2).
- Branch scan-mode UI (`active-only` / `pinned` / `all`) is a spec gap; deferred to the
  Preferences Folders pane.

## Related

- [spec/pipeline/capture](../../../../spec/pipeline/capture.md) — the four paths, branch
  versioning, adapter trait, locked decisions (legacy depth, folds here).
- [features/04-project.md](../../../../features/04-project.md) — workspace grouping story.
- [prd/01-vision.md §The core loop](../../../01-vision.md#the-core-loop) —
  [prd/02-objectives.md](../../../02-objectives.md) §03 Observatory.
- Crate: `crates/senseid/src/{scan,watcher,adapters}`; memories: project_incremental_indexing,
  feedback_no_silent_errors, project_stale_folder_reconcile.