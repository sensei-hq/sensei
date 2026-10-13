# Relay · M21

**Plane:** delivery · **Status:** [roadmap/03-delivery.md](../../../../roadmap/03-delivery.md) ·
**Serves:** team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor)),
maintainer ([persona 3](../../../03-personas.md#3--maintainer--the-curator)),
manager / delivery lead ([persona 6](../../../03-personas.md#6--manager--delivery-lead--the-roll-up))

## Purpose

Relay is the **run engine**: it takes a registered plan graph, drives an agent
(Claude, ACP, or the fallback) step by step, enforces limits (daily 5-day cap,
weekly cap, monthly cap), watches for stalls, and surfaces gates (hard blocks,
semantic review) and nudges back to the user. The Dōjō protocol (`dojo.relay_*`
tables, `/hook/{gate,nudge}`) carries the bidirectional stream. The job is to
turn a plan into **observable, interruptible execution** — the user sees every
step, every gate decision, and every nudge, and can intervene. The user feels
this as *sensei is doing the work, and I'm in the loop.*

> **Status honesty.** The run engine (`advance_run.rs`, `run_watchdog.rs`,
> `run_limits.rs`, three relay drivers), the runs API (`/api/runs*`, register
> plan, update task status, report outcome), the Dōjō gate (33 tests in
> `dojo/gate.rs`) and the Dōjō protocol (`dojo-protocol/src/relay.rs`) are
> built and tested. The flag `SENSEI_RUN_DRIVE` is **OFF by default** (not
> merged to main) — the drive loop heartbeats only. **The mobile and desktop
> relay screens are greenfield / unbuilt** (no test). There are no §12 gap IDs
> named for M21; the nearest are D-1/D-2 → T1.

## In scope / out of scope

- In: the drive loop (`drive_run`) with heartbeats, progress logs, stalled
  detection, limit parsing (5-day/weekly/monthly caps with reset/jitter), the
  three relay drivers (Claude hooks, ACP observe-only, fallback coarse observe),
  the run watchdog (5 min progress / 20 min stale / 3 recoveries), the runs API,
  the Dōjō gate (hard blocks: main-branch push, history rewrite, destructive
  deletes, deploy/publish/bump, credentials, money; semantic review for
  obfuscation), `/hook/gate` and `/hook/nudge`, `dojo.relay_*` tables.
- Out: **mobile relay screen** (greenfield); **desktop relay screen**
  (greenfield); the planner baseline/ratchet (C-1 → M20); tracker adapters
  (D-6 → T3).

## Surfaces

- **API** — `GET /api/runs` · `POST /api/runs` · `POST /api/runs/pause` ·
  `POST /api/runs/plan` · `GET /api/runs/{id}` · `POST
  /api/runs/{id}/tasks/{taskId}/status` · `POST /api/runs/{id}/outcome` ·
  `GET /api/runs/{id}/nudges` · `POST /hook/event` · `POST /hook/gate` ·
  `POST /hook/nudge`.
- **Daemon** — `tasks/handlers/advance_run.rs`; `run_watchdog.rs`;
  `run_limits.rs`; `relay_drivers/{claude.rs, acp.rs, fallback.rs, mod.rs,
  trait_def.rs}`; `dojo/gate.rs`; `api/handlers/runs.rs`; `api/handlers/sessions.rs`
  (hook endpoints).
- **App** — **mobile relay screen** (greenfield) · **desktop relay screen**
  (greenfield) · run detail in Observatory.
- **Data** — `sensei.runs`, `sensei.run_events`; `dojo.relay_inbox`,
  `dojo.relay_segments`, `dojo.relay_messages`, `dojo.relay_gates`.

## Data invariants

- **Lead.** `SENSEI_RUN_DRIVE` is OFF by default — the drive loop heartbeats
  and logs but does **not** spawn the agent. A feature flag flip enables it.
- **Lead.** The drive loop **heartbeats** every tick; progress resets the
  watchdog; stale progress → stalled; stalled + stale heartbeat → recover up to
  3×, then crash.
- **Lead.** Limits parse the exact "5-day run" string, 24h clock, weekly
  weekday, explicit month-day; reset = detected reset + jitter, else default
  backoff.
- **Lead.** The gate classifies **hard blocks** (exact string matches for
  dangerous commands) and **semantic review** (obfuscation markers, rare on
  benign, only on `bash`).
- **Lead.** The fallback driver is **observe-only, coarse** — it maps activity
  recency to `running` / `idle` / `done`.
- **Lead.** Dōjō protocol round-trips: `RelayRunStatus`, `SegmentState`,
  `GateSeverity`, `RelayInboxKind`, `RelayMessageDirection`,
  `RelayInboxStatus` all match DDL strings; `session_update` omits `None`;
  `inbox_pull` round-trips with cursor; `segments_publish` round-trips.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Run status | pending / running / paused / stalled / done / failed | `running` |
| Drive tick | heartbeat + progress log | `tick 12 · phase 2/3 · task 5/7` |
| Limit hit | daily / weekly / monthly cap | `daily cap · reset 02:00 UTC` |
| Watchdog | healthy / stalled (recover N/3) / crashed | `stalled · recover 2/3` |
| Gate decision | allow / deny (hard) / review (semantic) | `deny · destructive rm -rf` |
| Nudge | user action required | `nudge: approve gate #g-4` |
| Relay segment | phase / task / gate / tool-call | `task: add /login route` |
| Dōjō inbox | pending / acknowledged | `pending · 3 unacked` |

## Done gate

- A run created with a valid plan moves through the drive loop (with
  `SENSEI_RUN_DRIVE=1` and a stub agent) and emits heartbeats, progress, and
  task-state events.
- The watchdog stalls a run with stale progress, recovers up to 3×, then
  crashes it.
- Limits detect the exact five-day string, 24h/weekly/monthly variants, and
  compute reset with jitter.
- The gate denies `git push --force origin main`, `rm -rf /`, `aws deploy`,
  `npm publish`; flags `base64`/`eval`/`curl | sh` for semantic review.
- The ACP driver maps plan entries to segments; the Claude driver spawns a stub
  and captures exit code; the fallback driver maps recency to coarse status.
- `/hook/gate` and `/hook/nudge` accept and return the Dōjō protocol shapes.

```bash
curl -s http://localhost:7744/api/runs/$RID | jq '.status, .plan.phases|length'
SENSEI_RUN_DRIVE=1 cargo test -p senseid drive_run -- --nocapture 2>&1 | head -40
```

## Wrong gate

- **The drive loop spawns the agent with `SENSEI_RUN_DRIVE=0`** — flag gating
  regressed.
- **A run stalls but never recovers, or recovers past the cap** — watchdog
  logic regressed.
- **A `git push --force` is allowed** — hard block regressed.
- **A `base64` obfuscation slips through semantic review** — review regressed.
- **The Dōjō protocol serialises a DB string that doesn't match the enum** —
  round-trip regressed.
- **A mobile/desktop relay screen claims to show live drive progress** —
  greenfield, must read as placeholder.

## Scenarios

- [`scenarios/relay.feature`](scenarios/relay.feature) — the drive loop,
  watchdog, limits, gate, Dōjō protocol (pinned).
- [`scenarios/relay-drivers.feature`](scenarios/relay-drivers.feature) — the
  three relay drivers and the unbuilt mobile/desktop screens (`NO TEST`).

## Open gaps

- **Mobile relay screen** — greenfield, no test.
- **Desktop relay screen** — greenfield, no test.
- [C-1](../../../../reorg.md) — planner persistence feeds the run's plan graph.
- [D-6](../../../../reorg.md) — tracker adapters (Linear, Jira, GitHub Issues) →
  T3.
- The nearest §12 gaps are D-1/D-2 → T1 (not M21-specific).

## Related

- [features/06-relay.md](../../../../features/06-relay.md) — the relay dossier
  (legacy depth, folds here).
- [spec/screen/relay-*](../../../../spec/screen/) — mobile/desktop relay
  (legacy depth).
- [spec/pipeline/agent-execution.md](../../../../spec/pipeline/) — the agent
  execution pipeline (legacy depth).
- [prd/02-objectives.md §06](../../../02-objectives.md) · [prd/03-personas.md §2,3,6](../../../03-personas.md).
- Crates: `crates/senseid/src/{tasks/handlers/advance_run.rs, run_watchdog.rs,
  run_limits.rs, relay_drivers/, dojo/gate.rs, api/handlers/runs.rs,
  api/handlers/sessions.rs}`; `crates/dojo-protocol/src/relay.rs`.