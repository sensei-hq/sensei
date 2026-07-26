---
name: Automated runs — phase 1 (Claude-CLI-driven)
description: >-
  Skills + commands that let Claude Code plan, register, and orchestrate a multi-session
  automated run relayed via Dōjō — with the daemon as the durable spine. Phase 1 of D-EXEC-TEAM
  (the daemon-owned execution team is phase 2+).
type: design
status: design
created: 2026-07-26
related:
  - docs/design/agentic-execution-team.md   # D-EXEC-TEAM — the daemon-owned end-state (phase 2+)
  - docs/design/local-agent-coordinator.md  # D-COORD — task-typed router (phase 2 insertion point)
  - docs/plan/relay-engine.md               # relay run engine (P0–P6) — the substrate we ride
  - docs/plan/operating-model.md            # §3.4 Planner · §3.7 Execution · §3.11 three planes
---

# Automated runs — phase 1 (Claude-CLI-driven)

> **Phase framing.** In phase 1 the **Claude Code controller owns planning and execution**;
> the sensei daemon is the durable **spine** (persist the plan, mirror it to Dōjō, inject
> governance, host the report/nudge contract). This is the deliberate bootstrap of
> **D-EXEC-TEAM** ([`agentic-execution-team.md`](agentic-execution-team.md)) — that design
> (sensei OWNS the loop, models are interchangeable workers) stays deferred as the phase-2+
> end-state. The one hard constraint: **phase 1 reports into the same relay substrate** so
> phase 2 inherits the Dōjō view and the usage data for free. `SENSEI_RUN_DRIVE` stays **OFF**;
> the daemon never drives an agent.

## 1. Problem

We want to hand sensei a goal and get a **multi-day / multi-session automated run**: a plan
decomposed into a graph of phases → tasks (each task carrying how to build it, which skills
and conventions apply, how to verify/secure/qualify it, an assigned agent, an assigned model,
and a reference to its spec), executed by a resilient loop that dispatches parallel workers and
stays **watchable and steerable from Dōjō** on the phone. Today the pieces are scattered:
the planner is stateless, the plan is a flat markdown, execution has no orchestration skill,
and the coordinator↔daemon reporting contract is only half-built.

## 2. Scope

**In (phase 1):**
- Three command→skill pairs: `/sensei:analyze` → `analyzer`, `/sensei:plan` → `planner`,
  `/sensei:execute` → `executor`.
- An authored **plan graph** with per-task `agent`/`model`/`spec_ref` + typed dependency edges.
- **Registration to Dōjō** by authoring the plan into `dojo.relay_segments` (+3 columns) via a
  new `register_plan` MCP verb, reusing the shipped run-federation path.
- A **minimal coordinator MCP contract**: `register_plan`, `update_task_status`,
  `report_run_outcome`, `get_pending_nudges`.
- `analyzer`: grill-for-depth, conflict detection, decision recording.

**Out (phase 2+, deferred — see §11):** daemon-owned executor loop; local models as autonomous
coding workers; full `sensei.plans` relational tables; the inference-usage ledger (#49);
`SENSEI_RUN_DRIVE` ON.

## 3. Packaging — three command→skill pairs

Follows the shipped **command = thin phase/logging orchestrator, skill = project-agnostic
procedure** pattern (`mockup` → `building-app-mockups`, `spec` → `reverse-engineering`).
Specs carry the *project-specific* detail; the skills carry the *how-to-run-well* frame.

| Command | Skill (new) | Collision note |
|---|---|---|
| `/sensei:analyze` (extend) | `analyzer` | avoids the taken `analyze` skill (codebase health-check) |
| `/sensei:plan` (extend) | `planner` | distinct from the `plan` MCP tool (D-PLANNER) |
| `/sensei:execute` (new) | `executor` | new; boundary vs `/sensei:build` drawn below |

**Reconciliation (Decision AR-1).** The existing `/sensei:plan` and `/sensei:analyze` commands
are tied to the human doc pipeline (`ideas → analysis → blueprints → plans` + GitHub issues).
We **do not fork** the workflow into parallel commands. The new autonomous-run capability lives
in the **skills**; the commands invoke them and preserve their existing human-pipeline behavior.
A plan produced for an autonomous run is the executable **graph** (this doc); the GitHub-issue
decomposition remains the human-tracked mode of the same command.

- `init` is **dropped** — constitution/rule injection is already automatic (D-INJECT SessionStart
  + PreCompact re-push; `materialize_global_rules` at startup). Any real gap (proactively *calling*
  `get_project_summary`/`get_patterns`/`context_pack` rather than instructing the agent to) folds
  into `/sensei:intake` or the session-start hook — not a fourth initializer beside `sensei init`
  (CLI) and `sensei scaffold`.
- **Boundary vs `/sensei:build`:** `build` implements **one** feature (TDD + review). `execute`
  drives the **whole plan graph** as a monitored relay run, dispatching per-task workers (which may
  themselves do build-style TDD).
- `log_event` is a **dead no-op** (events sink retired #68). New surface does **not** call it; the
  progress channel is `update_phase` + the new verbs.

## 4. The plan graph (authored artifact)

`/sensei:plan` (Claude, guided by the `planner` skill) authors the graph as files. A plan is a
**self-contained unit** under `docs/plan/<plan-id>/`:

```
docs/plan/<plan-id>/            # <plan-id> = YYYY-MM-DD-<slug>
  plan.md                       # the graph: goal, phases, tasks, edges, per-task metadata
  tasks/<task-id>.md            # the detailed spec for each task (what spec_ref points at)
```

`plan.md` carries, per **task** node:

| Field | Meaning |
|---|---|
| `id` | stable task id (unique within the plan; used by edges + segments) |
| `phase` | owning phase |
| `title` | short label |
| `what` | intent — the observable outcome |
| `how` | approach / implementation notes |
| `skills[]` | which sensei skills to apply (e.g. `zero-errors-policy`, `test-gen`) |
| `conventions` | refs to conventions/patterns to follow |
| `spec_ref` | path#anchor into `tasks/<task-id>.md` — the task's detailed spec |
| `agent` | subagent **role** to dispatch (e.g. `general-purpose`, `sensei-security-reviewer`) |
| `model` | Claude tier for execution (`opus`/`sonnet`/`haiku`/`fable`) + optional local gateway sub-step |
| `verify` | **observable** acceptance criteria (the gate) |
| `security` | security checks for this task |
| `quality` | quality gates (lint/test/checker) |
| `deps[]` | **typed edges** over task ids → parallelism is derived, not free-text |

This upgrades today's planner shape (`Feature { title, acceptance_criteria, scope,
dependencies: Vec<String> }` — a *list* of free-text names) into a real **DAG** with stable ids.
The `planner` skill self-reviews the graph for completeness against the shipped
`sensei-plan-depth-reviewer` bar (observable criteria, no TBDs, deps resolve, no cycles) before
registration — a bad plan is the most expensive failure.

**Model semantics (phase 1).** The `model` field is **recorded** for every task (including local
models) for Dōjō visibility + phase-2 routing, but phase-1 **execution dispatches Claude subagents
only** (CC subagents are Claude-only). Local models (qwen/qwythos/gemma4) are used only for bounded
gateway sub-steps (a review lens, a classification) via MCP `infer`/`consensus` where they add
value. The recorded assignment is the first data the phase-2 router + usage ledger will learn from.

## 5. Persistence + Dōjō registration

**Decision AR-2: reuse the relay tree; author it from the plan.** The plan file is the working
source of truth. `register_plan` projects the graph into the already-shipped
`dojo.relay_segments` tree so the **whole plan is visible in Dōjō — with assigned models —
before execution starts**, and the executor flips segment state as tasks run.

- **Reuse** the shipped federation path: `activity.runs` (daemon-local, email/git-identity
  attribution) → background `publish_run` → `dojo.relay_sessions` + `relay_segments`
  (upsert on `tenant_id + run_id`, membership-routed).
- **Add 3 columns** to `dojo.relay_segments`: `agent text`, `model text`, `spec_ref text`
  (labels only). No new table set, no new `/v1` route.
- **Projector source-of-truth rule (avoids the two-projector clash):** a run **seeded from a
  registered plan** uses the *authored* segment tree (phase = top segment, task = child via
  `parent_id`, `seq` = order). Ad-hoc runs keep deriving segments from `run_events`
  (`plan_events_to_segments`) as today. The two never write the same run.
- **Zero-knowledge (D10) holds:** only labels/status/`agent`/`model`/`spec_ref` (a path, not the
  body) cross to Dōjō. Task spec bodies, code, and diffs never leave the daemon.

Deferred: full `sensei.plans`/`plan_features` tables — YAGNI until a second consumer (the deferred
project-window Tasks tab, or the phase-2 daemon coordinator) needs a queryable relational graph.
The file + the authored relay tree cover phase 1.

**Implementation notes (refine AR-2, recorded 2026-07-26 during build):**
- **Authored-graph store = a nullable `plan_graph jsonb` on `activity.runs`** (daemon-local, additive
  — *not* a new table, consistent with AR-2). `register_plan` stores the structure + per-task state;
  `publish_run` branches: `plan_graph` present → author segments from it, else derive from
  `run_events` (the single-source guard). It is fetched on demand, off the 16-column `RUN_SELECT`.
- **Phase-1 authors segments FLAT** (phases + tasks as one ordered `seq` list, `parent_id = None`).
  The Worker segments upsert keys on `(session_id, seq)` and does not honor a client-assigned `id`,
  so `parent_id` can't link children in one publish (FK). Visual Phase/Step nesting is deferred to a
  later Worker change (honor client ids). All per-task `agent`/`model`/`spec_ref` still cross.
- **Worker segments route gains `agent`/`model`/`spec_ref`** in its COLS (GET) + row-map (POST), or
  the new columns never populate.
- **Per-task state is event-fed for liveness AND stored on the graph for fidelity:**
  `update_task_status` updates the task's state in `plan_graph` (full 7-state fidelity) *and* appends
  a feature-class `run_event` so the progress clock stays fresh (keep-alive) — never `heartbeat_at`.

## 6. Executor loop (`/sensei:execute` + `executor` skill)

A resilient Claude-Code orchestration loop. Drive stays OFF — this loop *is* the controller.

1. **Load** the plan graph from `docs/plan/<plan-id>/`.
2. **Register** the run if not already: `register_plan(goal, project, plan_ref)` → run + authored
   segment tree + `run_id`.
3. **Ready set:** tasks whose `deps` are all `done`.
4. **Dispatch** the ready set as **parallel subagents** (one per task), each on its assigned Claude
   model, fed `what`/`how`/`skills`/`conventions`/`spec_ref`/`verify`/`security`/`quality`. Each
   worker does TDD + `zero-errors-policy` and returns a **structured result** (pass/fail + artifacts
   + notes).
5. **Aggregate** (the coordinator = the CC main loop is the *sole MCP caller* — workers report to
   it via their return value, sidestepping the missing worker-id column on `run_events`):
   - task done → `update_task_status(run_id, task_id, done)` (flips the segment + appends a
     `feature_done` run event);
   - phase complete → `update_phase(phase=…)` (rides the shipped bridge →
     `advance_run_phase_for_project` → run events → relay segments);
   - task fail → retry within a **loop-bound**, else escalate (mark the segment `needs_review`
     + surface a relay flag for human steer).
6. **Nudge/health poll:** each iteration `get_pending_nudges(run_id)` — the "daemon initiates a
   check" contract, expressed as **agent-pull** (MCP is one-directional; the daemon can't call the
   agent). Respond via MCP.
7. **Keep-alive** rides the **progress clock** (a progress-class event via `update_phase` inside
   the 5-min window). It **never touches `heartbeat_at`** — that stays daemon-owned so the watchdog
   can still escalate a genuine stall (heartbeat-ownership invariant).
8. **Terminal:** all tasks done/failed → `report_run_outcome(run_id, done|failed, summary)`.

**Resilience / resume.** Loop-bound per task; a crashed worker retries or marks the task `blocked`.
Because task state is persisted (relay segment state + run events), a **new session resumes** by
reading `run_status` and recomputing the ready set — the file + the run are the checkpoint.
One `activity.runs` run per plan; parallelism capped by `max_concurrency` (or the CC subagent cap).

## 7. MCP contract additions (minimal — §Decision AR-3)

Thin wrappers over existing daemon primitives + the relay inbox. Reuse
`start_run`/`update_phase`/`pause_run`/`run_status` unchanged.

| New MCP verb | Daemon route | Does |
|---|---|---|
| `register_plan(goal, plan, project?, plan_ref?)` | `POST /api/runs/plan` | validate the graph (unique ids, deps resolve, DAG) + create the run with the graph stored in `plan_graph`; `publish_run` then authors the relay segments from it. `plan` is the structured graph (JSON string at the MCP boundary); `plan_ref` is the human plan-doc path. Returns `run_id` |
| `update_task_status(run_id, task_id, state, note?)` | `PATCH /api/runs/{id}/tasks/{task_id}` | flip the task's segment state + append the matching run event |
| `report_run_outcome(run_id, outcome, summary?)` | `POST /api/runs/{id}/outcome` | set the terminal run status (`done`/`failed`) — the one agent-settable terminal transition |
| `get_pending_nudges(run_id)` (+ ack) | `GET /api/runs/{id}/nudges`, `POST …/nudges/{id}/ack` | agent-facing read of the human→agent relay inbox + reply |

**Trust-boundary validation (never skipped):** `project` must resolve to a known project;
`run_id` must belong to the caller's attribution/membership; `state` ∈ `segment_state` enum;
`task_id` must exist in the registered plan; `outcome` ∈ {`done`,`failed`}; `get_pending_nudges`
is RLS-scoped to the run's membership (own-rows). `spec_ref` is stored verbatim as a label — never
dereferenced server-side, never sent to Dōjō as a body.

## 8. `analyzer` skill (extend `/sensei:analyze`)

Deep analysis that precedes planning:

1. **Grill for depth** — targeted questions grounded in graph lookup (`search`, `get_callers`,
   `get_callees`, `get_patterns`, `get_project_summary`), one theme at a time, until the objective
   is fully specified.
2. **Write/extend docs** under `docs/` per conventions (design → `docs/design/`).
3. **Depth + clarity self-pass** — no TBDs, observable criteria, ambiguities resolved.
4. **Conflict detection** across docs — contradictions between this analysis and existing docs.
5. **Resolve + record** — on conflict, ask the user to decide, then record the resolution as a
   **decision** (`D-<NAME>` / `AR-n` row in `docs/decisions.md`) and update the affected docs.

**Gate:** `/sensei:plan` (autonomous-run mode) requires an analysis reference; if missing it offers
to run `/sensei:analyze` first (soft prompt, not a hard block).

## 9. Reuse map — ride, don't rebuild

| Need | Already shipped — reuse |
|---|---|
| Progress → Dōjō | `update_phase` → `advance_run_phase_for_project` → run events → relay segments (the old "no external phase-append" gap is **closed**) |
| Run registration + attribution | `start_run` → `activity.runs` → `publish_run` → `relay_sessions`, email/git-identity keyed, membership-routed |
| Constitution/rule injection | D-INJECT SessionStart + PreCompact + `materialize_global_rules` (fully automatic) |
| Depth bar | `sensei-plan-depth-reviewer` skill/agent |
| Checker/quality gate | D-CHECKER (`run_checkers`, `sensei.rule_check_runs`) |
| Worker roles | mindset subagents (security/perf/ux/persona/acceptance/developer) |

## 10. Security & safety

- **Drive OFF.** The executor is the CC controller; `SENSEI_RUN_DRIVE` stays off. The daemon
  publishes + logs steer, never spawns/drives.
- **Zero-knowledge (D10).** Only labels/status cross to Dōjō; no code, diffs, tool output, or spec
  bodies.
- **Heartbeat invariant.** Agent keep-alive feeds the *progress* clock only; `heartbeat_at` is
  daemon-owned so stall→recover→crash escalation stays honest.
- **Input validation** on every new MCP verb at the trust boundary (§7).
- **Bounded terminal authority.** An agent may set `done`/`failed`/`blocked`/`paused` only; the
  watchdog retains independent stall/crash authority.

## 11. Deferred → phase-2 handoff (D-EXEC-TEAM)

Phase 1 is designed so phase 2 is additive, not a rewrite:
- The **recorded per-task `model`** seeds the phase-2 complexity→tier router + the inference-usage
  ledger (#49, build-first per D-EXEC-TEAM §9).
- The **authored relay segment tree** is exactly the seed the daemon-owned coordinator will drive.
- Moving the loop into the daemon (`RunDriver` / `driver_for`) swaps *who calls* the same MCP/relay
  surface — the Dōjō view and the contract are unchanged.

## 12. Build order (TDD throughout)

1. **DDL** — add `agent`/`model`/`spec_ref` to `dojo.relay_segments` (full-file DDL, `dbd`
   reconcile per repo convention). *(Dōjō scope, prod-apply gated per D-TIER3-DDL.)*
2. **Daemon** — `register_plan` (author segments from a plan-ref/graph), `update_task_status`,
   `report_run_outcome`, `get_pending_nudges` handlers + routes; the authored-vs-derived projector
   guard.
3. **MCP** — the four new tool wrappers in `crates/mcp`.
4. **Skills + commands** — `planner`, `analyzer`, `executor` skills; extend `/sensei:plan` +
   `/sensei:analyze`, add `/sensei:execute`.
5. **Verify** — live smoke: plan a small goal → register → watch it appear in Dōjō with assigned
   models → execute → tasks flip state → run completes. `zero-errors-policy` at both ends.

## 13. Decisions recorded

- **AR-1** — Autonomous-run capability lives in the skills; commands preserve their human-pipeline
  behavior. No forked commands. `init` dropped (injection already automatic).
- **AR-2** — Register the plan by authoring `dojo.relay_segments` (+3 columns) via `register_plan`;
  reuse `publish_run`. No new `sensei.plans` tables in phase 1.
- **AR-3** — Minimal coordinator MCP contract (4 verbs); keep-alive feeds the progress clock, never
  `heartbeat_at`; drive stays OFF.
- **AR-4** — Phase-1 execution is Claude-subagent-only; `model` is recorded (incl. local) for Dōjō
  + phase-2, local models used only for bounded gateway sub-steps.
