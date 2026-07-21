---
type: design
---

# Remote / orchestrator — module

Behind-the-scenes design for the [Relay](../features/06-relay.md) away-from-keyboard
surface. The feature doc says what the user sees (plan · execute · watch · decide);
this says how the run engine, control channel, and (future) custom orchestrator work,
and where the code lives. Full end-to-end spec: `docs/plan/relay-engine.md`.

## Run engine (daemon-owned)

- Crate: `crates/senseid`. The tick handler is `tasks/handlers/advance_run.rs`
  (`AdvanceRun`) — one tick per active run, scheduled by
  `tasks/advance_run_scheduler.rs`. `resume_due_runs` flips a `paused` run whose
  `paused_until` has elapsed back to `running` *before* the tick runs, so
  `advance_run` never has to special-case resume.
- Backend-agnostic drive seam: `relay_drivers/trait_def.rs` (`RunDriver`,
  `DriveStep`, `DriveCapability`), with `relay_drivers/claude.rs` (`ClaudeDriver`)
  as the one shipped backend (`SENSEI_RUN_DRIVE`, off by default) and
  `relay_drivers/acp.rs` / `relay_drivers/fallback.rs` as P5.2/P5.3 stubs for
  non-Claude assistants. Selection: `relay_drivers/mod.rs::driver_for`.
- Limit parsing: `run_limits.rs` — recognizes Claude Code's CLI-text rate/weekly
  limit message (not a structured 429), computes `paused_until = reset + jitter`
  against local wall-clock. Gateway-routed calls normalize their own 429s
  separately (`gateway-embedded`, per `docs/plan/relay-engine.md` §8).
- Watchdog: `run_watchdog.rs` is the pure escalation ladder
  (`running → stalled → recover (bounded) → crashed`); `tasks/watchdog_scheduler.rs`
  is the DB-writing caller. Heartbeat-ownership invariant: `advance_run`
  heartbeats `Running`/`Blocked` runs every tick but deliberately never touches a
  `stalled` run — only the watchdog does, so staleness isn't masked.
- Run/event persistence: `db/pg_store.rs` (`activity.runs`, `run_event`,
  `reset_run_recovery`). API surface: `api/handlers/runs.rs`.
- Shipped: P3.2 (tick scaffolding + heartbeat/housekeeping), P3.3a/b (agent spawn
  + off-by-default drive), P3.4 (limit parse/pause), P3.6 (watchdog/crashed). All
  on `develop`, gated behind `SENSEI_RUN_DRIVE` — not yet enabled for a live pilot
  (that's a P2-relay exit-criterion in `docs/plan/2026-07-20-phases-1-3-plan.md`).

## Control channel — hooks as the bridge

- There is no channel *into* a running Claude session other than the hooks the
  sensei plugin already registers (SessionStart/UserPromptSubmit/PreToolUse/
  PostToolUse/PreCompact/Stop). `dojo/gate.rs` is the pure hook-gate decision
  core: parses the `SENSEI_RELAY_GATE_TOOLS` allow-list, decides which tool
  calls are gated, and maps a reply to allow/deny. **Fail-open**: no allow-list,
  no reply, or a timeout all resolve to `allow` — only an explicit human `deny`
  blocks.
- Wire contract: `crates/dojo-protocol/src/relay.rs` — `RelayRunStatus`,
  `RelayInboxItem` (payload = stripped prompt + rokkit form schema), enums kept
  in lock-step with the `dojo.*` DDL by `*_db_strings_match_ddl` tests.
- The blocking `PreToolUse` hook is task **B** in `docs/plan/relay-engine.md`
  §5/§9 — bounded by the hook's own timeout (~60s), one tier below the P3
  owned-process model (indefinite gate hold, `RunDriver` owns stdin/stdout).
- A queued **nudge** rides the same rails inverted (daemon stores it, next
  hook fire injects it) — no wall-clock cadence, only "next tool call/prompt."

## Remote surface (phone / console)

- Dōjō app: `dojo/src/lib/relay-*.ts` (`relay-data`, `relay-view`, `relay-realtime`,
  `relay-offline`, `relay-connectivity`, `relay-push` + `server/relay-push-send.ts`,
  `server/relay-push-env.ts`) and components `components/Relay*.svelte`
  (`RelayStatusBadge`, `RelayBlockedHome`, `RelayNotifyToggle`, `RelayOfflineBanner`).
- Data model projected, never authored: `Execution → Segment → Item`; segments
  roll up agent `TodoWrite` capture, summarized via gateway insight-copy — never
  raw tool logs (zero-knowledge, D10).
- Liveness = three signals, not just a status flag: `relay_sessions` presence
  heartbeat, "last progress N min ago" from the newest `run_event`, and the
  watchdog's `stalled` signal.
- Mockups: [`relay.jsx`](../mockups/Sensei/lib/relay/relay.jsx) (phone run
  dashboard/detail/gate/nudge), [`relay-planner.jsx`](../mockups/Sensei/lib/relay/relay-planner.jsx),
  [`relay-desktop.jsx`](../mockups/Sensei/lib/relay/relay-desktop.jsx).

## Gates + nudges

- Two severities only: **advisory flag** (default, async, never halts —
  reviewed PR-review style) vs **blocking gate** (rare — hard-block set only:
  merge/deploy/`make bump`, `main` changes, destructive/irreversible ops,
  money/credentials, out-of-scope). Rule-first classification; a local model
  (gemma4) is a backstop classifier, never the sole arbiter.
- Nudge has two modes: **steer** (inject into a healthy live session) vs
  **unstick** (control action on a `stalled`/`crashed` run — retry · skip ·
  resume · restart from checkpoint · force-advance).

## Future: custom orchestrator (Phase 6, design-stage — no code yet)

- Not yet built; tracked as **P6** in `docs/plan/2026-07-20-phases-1-3-plan.md`
  and `docs/plan/relay-engine.md` §7/§9 (P5 multi-assistant → P6 team relay is
  the *relay* track; the *orchestrator* is a separate, later replacement for
  manual `sensei:` playbook invocation — needs its own feature doc + story
  breakdown before build starts).
- Shape: a **Planner → Builder → Judge** repeat-until-it-works loop replacing
  ad-hoc single-shot playbook runs — the iterative Judge re-reviews and sends
  work back to Builder rather than requiring the Planner to front-load an
  exhaustively deep plan. This **simplifies the plan-depth problem**
  (`plan-depth-reviewer`, relay-engine §5 "depth bar"): depth-bar gaps that
  today must be caught *before* a run starts become recoverable *during* the
  run via Judge feedback loops.
- Model routing: gateway (`gateway-embedded`) **mix-of-models** (MoE) across
  both OAuth and API-key auth modes — task-class → model-chain policy, same
  local-first-for-structured/cloud-for-open-ended split already designed for
  the run engine (relay-engine §6.5): Opus/gemma4 for planning+judging, Sonnet/
  qwythos for building, always behind a strong reviewer.
- Base: **opencode** as the agentic-coordinator substrate, replacing the
  current Claude-only headless spawn (`ClaudeDriver`) — builds on the existing
  `RunDriver` seam so P5's ACP/fallback adapters and P6's orchestrator share one
  contract rather than forking a second execution path.
- Sequencing note (relay-engine §7, decided 2026-07-16): the multi-assistant
  orchestrator wrapper is **deferred post-stable** — P0–P4 (single-assistant,
  Claude-first) ship first; the orchestrator doesn't get built until that base
  is proven.
