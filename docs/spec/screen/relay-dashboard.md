# 家 · Relay · Dashboard

**Segment:** 05 · Relay (mobile companion)
**Route:** Relay iOS app · dashboard (no web route — phone app)
**Source mockup:** [`lib/relay/relay.jsx`](../../mockups/Sensei/lib/relay/relay.jsx) → `RelayDashboard`
**Data:** greenfield — the coordinator publishes a filtered status stream through the relay. Proposed: `GET /api/relay/agents` (snapshot: `{ counts: {needs_you, running, done}, tasks: [...] }`) + a push/SSE `GET /api/relay/stream` for live deltas; each task = `{ id, title, machine, assistant, state, status, ago, stages[] }`. Status only — no code, no diffs, no file contents.
**App file:** _greenfield — mobile app not built_
**Daemon files:** _greenfield — coordinator + relay transport not built_
**Status:** greenfield (Relay backend unbuilt) — see [architecture/relay](../../architecture/relay.md)

## Purpose

The user is away from the keyboard and opens the phone to answer one
question: *does anything need me right now?* The dashboard is a calm
3-state read — one glance tells them how many agents **need you**, how
many are **running**, how many are **done**. The single "needs you"
task leads (surfaced first, tinted vermillion), so the away-from-keyboard
user is pulled toward the one gate that matters and can ignore the rest.
Tasks are grouped by machine so a multi-machine run stays legible, and
each row carries a stage bar that shows how far the plan has walked
without ever showing what the agent is doing to the code. It should feel
like a control tower, not a log.

## Data invariants

- The screen reads **only the filtered status stream** the coordinator
  publishes: task title, owning machine, assistant family, run state,
  a short status line, relative age, and a phase/stage vector. **ZERO-KNOWLEDGE:
  never code, diffs, file contents, or transcripts** — the mockup's status
  strings ("Running — writing test_checkout.py", "Processing batch 42 / 120")
  are the *most* detail that may cross the relay, and even those are
  coordinator-composed summaries, not raw agent output.
- The three header counts are derived from the same task list —
  `needs_you = count(state == 'block')`, `running = count(state == 'live')`,
  `done = count(state == 'done')` — never fetched independently (they must
  always agree with the list below).
- Task `state` is one of exactly three visible states: **`block`** (needs
  you), **`live`** (running), **`done`**. The stage-bar segment vocabulary
  is richer — `done | live | block | todo` per phase step — but the row's
  headline state collapses to the 3-state model.
- Ordering: the single `block` task leads; `live` tasks follow; `done`
  tasks are muted (opacity ~0.66) and sink to the bottom. Grouping is by
  `machine` (e.g. `macbook-pro`, `mac-mini`).
- A `done` task shows no stage bar (the plan is complete); it shows a
  completion summary line instead (e.g. "Completed · 24 files changed" —
  a count, never the files).
- The lock glyph in the header is a live affordance into [Security](relay-security.md);
  its presence is the standing reminder that the pipe is encrypted and
  status-only.

## Signals shown

Header — 3-state counts + title:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Relay mark | `先 · RELAY` (vermillion mono) | brand + you're on the relay surface | `先 · RELAY` |
| Lock chip | tap target → Security | encrypted, status-only pipe | (lock glyph, top-right) |
| Screen title | `Agents` (display) | the surface | `Agents` |
| Needs-you count | `count(block)` in accent | gates waiting on you | `1 · needs you` |
| Running count | `count(live)` in success/jade | agents working, non-blocking | `2 · running` |
| Done count | `count(done)` in ink-faint | finished this session | `5 · done` |

Per-task card:

| Element | Value | Meaning | Example |
|---|---|---|---|
| State dot | accent (block) / jade pulsing (live) / faint (done) | headline state at a glance | pulsing jade dot |
| Machine label | `task.machine` (mono) | which machine owns the run (R7) | `macbook-pro` |
| Age | `task.ago` (mono) | relative freshness of last status | `4m`, `now`, `12m` |
| Title | `task.title` | the plan/task name | `Refactor auth module` |
| Status line | `task.status` (colored to state) | coordinator-composed one-liner | `Needs you — approve DB migration` |
| Stage bar | `stages[]` → `done\|live\|block\|todo` segments | phase progress without content | `[done,done,done,done,block,todo]` |
| Card tint | accent-soft + accent border when `block` | the one gate visually dominates | vermillion-tinted card |
| Card tap | opens [Task detail](relay-task-detail.md) | drill into plan + activity + gate | — |

Real mockup content (the four seeded rows):

- `macbook-pro · 4m` — **Refactor auth module** — *block* — "Needs you — approve DB migration" — stages `[done,done,done,done,block,todo]`
- `mac-mini · now` — **Write integration tests** — *live* — "Running — writing test_checkout.py" — stages `[done,done,live,todo,todo]`
- `mac-mini · now` — **Nightly data backfill** — *live* — "Processing batch 42 / 120" — stages `[done,live,todo,todo]`
- `macbook-pro · 12m` — **Update dependencies** — *done* — "Completed · 24 files changed" — no stage bar

## Done gate

The dashboard renders the coordinator's filtered status stream: a 3-state
header whose counts equal the grouped task list below it, the single
`needs-you` task leading and tinted vermillion, `running` tasks with a
pulsing jade dot and live stage bar, and `done` tasks muted with a
completion summary and no bar — all grouped by machine, with nothing on
screen that could not have crossed a status-only pipe.

## Wrong gate

- **Any code, diff, file path list, or transcript text appears on a card.**
  The relay leaked content — a hard zero-knowledge violation (R5). Status
  lines must be coordinator summaries, and "24 files changed" must stay a
  count, never a file list.
- **The header counts disagree with the list** (e.g. header says 2 running
  but three live rows render). Counts were fetched separately instead of
  derived from the same task set.
- **A running or done task is tinted like a gate**, drowning the one
  `needs-you` task — the "single needs-you leads" hierarchy is broken.
- **A `done` task still shows a live/pulsing stage bar.** Done tasks carry
  a completion summary and no bar.
- **Tasks are flat, not grouped by machine** — multi-machine legibility (R7)
  is lost.
- **The lock chip is decorative** (not a real route into Security) — the
  standing encryption reminder must be a live affordance.

## Related

- Objectives [R3](../../objectives.md#relay--supervising-long-runs-from-anywhere) (legible remote status · done·doing·next · nudges), [R5](../../objectives.md#relay--supervising-long-runs-from-anywhere) (zero-knowledge), [R7](../../objectives.md#relay--supervising-long-runs-from-anywhere) (multi-agent, multi-machine, grouped by machine)
- [architecture/relay](../../architecture/relay.md) — coordinator publishes filtered status; three planes
- [journeys/relay](../../journeys/relay.md) — run & supervise round-trip
- Sibling relay screens: [Task detail](relay-task-detail.md) · [Approve](relay-approve.md) · [Respond](relay-respond.md) · [Security](relay-security.md) · [Pairing](relay-pairing.md)
