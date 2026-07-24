---
type: design
status: draft (pending review) — supersedes nothing; extends remote.md + the heterogeneous-execution-router note
---

# Local-agent coordinator — module

Behind-the-scenes design for a **superpowers-like coordinator** that orchestrates a
**mix of subagents** — including subagents whose reasoning loop runs on a **local
model** (gemma / qwen via the gateway or the embedded llama.cpp engine) plus external
(Claude / cloud) agents. The near-term posture is **local-first**: a v1 local-only
extension, then a later local+cloud mix.

This composes with — and does not replace — three existing modules:
[`remote.md`](remote.md) (the daemon-owned run engine + `RunDriver` seam + Phase-6
orchestrator), [`assistants.md`](assistants.md) (ACP adapters, the `RunDriver`
contract, MCP `infer`/`consensus`/`embed`), and [`playbook.md`](playbook.md) (typed
chunks + `classified_by`/`model_fallback` outcome attribution). It is the concrete
design for the [heterogeneous-execution-router note](../plan/2026-07-19-heterogeneous-execution-router.md)
(status there: *design note — NOT planned*), pulled forward into a phased build.

> Grounding convention: every claim below is tagged **EXISTS** (built + wired today)
> or **BUILD** (net-new). File refs are `crate/file` or `path:line`.

---

## 1. The superpowers coordinator pattern, and how sensei mirrors it

**Superpowers** (the `superpowers:*` plugin) is a **meta-skill router**: `using-superpowers`
routes a task to phase skills — `brainstorming → writing-plans → executing-plans` (plan-then-execute,
separate review-checkpointed session) or `subagent-driven-development` (same session), with
`dispatching-parallel-agents` for fan-out of 2+ independent tasks, and discipline gates woven
in (`test-driven-development`, `systematic-debugging`, `verification-before-completion`,
`requesting/receiving-code-review`, `using-git-worktrees`, `finishing-a-development-branch`).
It is **Claude-in-harness and single-machine** — every subagent is a Claude tier.

Sensei already mirrors the *shape* of this pattern, part built, part designed:

| Superpowers piece | Sensei equivalent | State |
|---|---|---|
| meta-skill router (`using-superpowers`) | `/sensei:intake` front door + playbook recommend-and-confirm (`crates/senseid/src/playbook.rs`) | **EXISTS** |
| `writing-plans → executing-plans` | `/sensei:plan` + Phase-6 **Planner → Builder → Judge** loop (`remote.md` §"Future") | plan built / loop **BUILD** |
| `dispatching-parallel-agents` | graph-safe parallelism — disjoint features run parallel, overlapping serialize; worktree-isolated | **BUILD** (design only) |
| `subagent-driven-development` | the P3 run engine's per-feature gated loop | seam **EXISTS**, loop **BUILD** (single-shot today) |
| discipline gates (TDD/debug/verify/review) | `.claude/agents/*` reviewer + mindset agents + per-phase cadence | agents **EXIST** (Claude-only, pull-based); auto-invoker **BUILD** |
| `using-git-worktrees` | worktree-isolated parallel backfill | **BUILD** |
| `finishing-a-development-branch` | commit/push-per-feature + never-merge-`main` hard-block | partially **EXISTS** (hard-block set), auto-loop **BUILD** |

**Sensei's four differentiators over superpowers** — all four are why a *sensei* coordinator
is worth building rather than just installing superpowers:

1. **Daemon-owned** — the coordinator survives session death (the vacation-run requirement),
   because the run engine is a daemon task (`crates/senseid/src/tasks/handlers/advance_run.rs`),
   not a harness skill. **EXISTS** (the engine; the coordinator loop is **BUILD**).
2. **Graph-derived** parallelism/overlap instead of human-declared independence. **BUILD**.
3. **Remote supervision** (phone / Dōjō, zero-knowledge) via relay. **EXISTS** (relay P0–P5).
4. **Heterogeneous models** — local + cloud, not Claude-only. **This doc's subject.** **BUILD**.

The coordinator is deliberately "orchestration of capabilities sensei already has"
(`docs/plan/operating-model.md` §"the vision is orchestration") — not a new runtime.

---

## 2. Can a subagent run on a LOCAL model? The seam and what's missing

There are **two distinct "agent" notions** in the repo, and **neither** gives a subagent a
local-model brain today:

- `crates/senseid/src/agent_spawn.rs` — a process-supervision primitive only (spawn + hard
  timeout + no-zombie). **No** gateway/model wiring. **EXISTS** as a primitive.
- `crates/senseid/src/tasks/handlers/advance_run.rs` `drive_run` — spawns **one** headless
  `claude -p` step. `agent_cmd` defaults to `"claude"`, overridable via `SENSEI_RUN_AGENT_CMD`
  (`advance_run.rs:150,159`). The spawned agent carries the sensei plugin, so it can *call* the
  MCP `infer` tool — but its **own reasoning loop is Claude**. **EXISTS**, single-shot, drive
  OFF by default (`SENSEI_RUN_DRIVE`).

**What local inference EXISTS today** (fully production-ready, local-first — MAP 3):

- **Embedded in-process llama.cpp** — the preferred local chat leg, router `embedded-llama`
  (`url: embedded://embedded-llama`). Registered when built `--features embedded-llama-cpp`
  (the shipped default, `Makefile:57/76`) **on the default instance only** (skipped for named
  instances, `gateway_init.rs:160-211`). Embedded chat = `gemma2:2b`; model bytes resolve via
  `ChainedResolver` (managed dir → Ollama cache), on-demand pull otherwise (`model_provision.rs`).
- **Ollama** (HTTP localhost:11434) — widest local set: `gemma4`, `gemma3:27b/12b`,
  `qwen3:14b/8b`, `phi-4`, `llama4-scout`. Probed at startup (`gateway_init.rs:214`).
- **Gateway chains** — the real routing today. `classify` / `reasoning` / `summarize` /
  `insight-copy` chains are all **local-first** with a cloud tail (`fallback_chain_models.jsonl`).
  Consumers pin a named chain: `gateway.execute(&InferenceRequest{ chain: Some("classify"), .. })`.
- **Agent-callable surface** — the sensei MCP tools `infer` / `embed` / `consensus` /
  `gateway_status` (`crates/mcp/src/lib.rs:423-444` → `POST /api/gateway/*`). A Claude subagent
  *can* run one local completion via `infer` — but that is a "brain for one call," not "a
  subagent whose loop runs on gemma."

**The seam** (where a local-model subagent plugs in) is the same one `remote.md` names:

- **Driver level — `RunDriver`** (`crates/senseid/src/relay_drivers/trait_def.rs`): `id()`,
  `capability() → {Hooks|Acp|Fallback}`, `drive_step(DriveStep) → AgentOutput`, `observe_update()`.
  Selection is `relay_drivers/mod.rs::driver_for` — **hardcoded to always return `ClaudeDriver`**
  (`mod.rs:29-31`), with a `// FOLLOW-UP` to "dispatch on a driver id → `Box<dyn RunDriver>`".
  **This is the exact insertion point for a task-typed router.** **EXISTS** (seam), **BUILD** (router).
- `relay_drivers/acp.rs` `AcpObserveDriver` is **observe-only** — `drive_step` returns
  "unsupported"; drive-over-ACP is deferred (P5.2b, `assistants.md` §ACP). **EXISTS** (observe),
  **BUILD** (drive).

**What is missing to make a subagent's loop run on gemma/qwen** (MAP 3 §4, MAP 4 §4, ranked):

1. **A local-model agent DRIVER** — the core blocker. Either (a) an **ACP-drive backend for a
   local coding agent** (ollama/aider/continue-style behind `RunDriver`), or (b) a
   **gateway-backed mini-agent** driver: a small bounded read→infer→act→verify loop whose steps
   are gateway `infer` calls on a local chain. `agent_spawn.rs` is only the process primitive; it
   has no gateway/ACP wiring. **BUILD**.
2. **A task-typed router** at `driver_for` — `router(task_type × risk × capability-contract →
   driver)`. **BUILD**.
3. **An inference-usage ledger** — no per-inference usage table exists (`gateway.*` tables are
   config-only; only `insight_copy` persists `resp.model`; intake persists `classified_by` +
   `model_fallback` on `sensei.playbook_run`). Without it the router has nothing to learn from.
   **BUILD**.
4. **Honest capability limit (2026)** — gemma-class models are reliable at *narrow* types
   (classify / short-gen / structured-extract), **not** autonomous multi-file implementer/reviewer
   roles. So even with drivers, only cheap mechanical types route local. (A constraint, not a task.)

---

## 3. The routing model — classify a task, pick the cheapest capable driver

The coordinator routes **per typed task**, not per run. The router is a pure function feeding
`driver_for`:

```
typed task ── router(task_type × risk × capability-contract) ── driver
                                                                  ├─ gateway inference (single-shot, no agent loop)   [classify · extract · format · doc-gen · seed-author]
                                                                  ├─ local-model agent (ACP or gateway-mini)          [single-file mechanical edit, exact spec — escalate on fail]
                                                                  └─ cloud agent (Claude, ACP)                         [multi-file integration · debugging · architecture · review]
              └─ verify result (tests / lint / spec-diff / cloud judge) → on fail, ESCALATE to the next driver up
```

Design commitments, each grounded:

- **Reuse the intake taxonomy, don't invent one.** The router's `task_type × risk` axes reuse the
  playbook `Axes` (`Lifecycle`/`Intent`/`Risk`) already classified at the front door
  (`crates/senseid/src/playbook.rs`), and the **capability contract** is the operating-model's
  ≥80% gate. A **playbook may declare** which of its task types route local vs cloud. **EXISTS**
  (axes + classifier), **BUILD** (the local/cloud declaration + the router).
- **Cheap-first, correct-eventually escalation** mirrors two patterns sensei already uses: the
  gateway's fallback chains (`fallback_chain_models.jsonl`, local→cloud tail) and the hook-gate's
  fail-open (`dojo/gate.rs`). Attempt local, **verify**, escalate to cloud on failure. **EXISTS**
  (both precedents), **BUILD** (the escalation policy binding them to a task result).
- **Reviewer / gate tier is pinned to a strong cloud model, exempt from local-first** (relay-engine
  D12): a review/gate step waits under a limit, it never downgrades to a local model. The
  classify-the-blocking-gate step is **rule-first, local-model-second** — a local model (gemma4) is
  a *backstop* classifier, never the sole arbiter (`remote.md` §Gates+nudges). **EXISTS** (the
  rule-first gate design), **BUILD** (wiring it as a router exemption).

**MoE / panel via gateway consensus** — a mixed-model panel (gemma vs qwen debating a decision)
is *one config step* from working, but is **not wired**:

- Gateway v0.4.6 has full native machinery — `PanelConfig{slots,distinct_by,strict}`,
  `ConsensusConfig{panel,synthesizer,judge,judge_quorum}`, `DistinctBy::{None,Model,Family}`
  (`kernel/src/types/config.rs`). **The family axis is READY end-to-end**: `ModelConfig.family`
  threaded DB→config (`gateway.models.family` column, comment: "picked to fan-out to DIVERSE
  families"), so `distinct_by: family` (gemma vs qwen) would work **if a panel were configured**.
- **But** `GatewayConfig.panels`/`.consensus` are seeded **empty everywhere**, there is **no DDL
  table** for panels/consensus, and the shipped MCP `consensus` tool does **not** call the gateway's
  native `execute_consensus` — it hand-rolls a sequential 3-step proposer→challenger→synthesizer
  `Purpose` (`handlers/gateway.rs:194-331`) with **no family-distinctness enforcement, no true
  fan-out, no judge quorum**. **EXISTS** (native machinery + family threading + a sequential
  stand-in), **BUILD** (DDL for panels/consensus + swap the MCP tool onto `execute_consensus` +
  seed a `distinct_by:family` local panel).

**Attribution wired today** (the only datapoint the router could learn from): `playbook_run.classified_by`
+ `model_fallback` (`database/ddl/table/sensei/playbook_run.ddl:14-15`), surfaced via
`GET /api/playbook/model-stats`. This is the **template to extend** to `task → driver → outcome`. **EXISTS**.

---

## 4. How it composes with the execution model

The coordinator is a **thin layer over three seams that already exist** — it adds a router and a
driver, not a new execution path.

- **Playbooks / runs (front door).** Intake already classifies a chunk into `Axes` and records a
  `playbook_run` (`playbook.md`). The coordinator consumes that classification as the router's
  input, and extends the same outcome-attribution pattern (`classified_by`/`model_fallback` →
  `task → driver → outcome`) so the §9 learning loop can tune routing. **A `playbook_run` is a
  recommendation, not an executed agent run today** — the coordinator is what turns a recommended
  playbook's typed steps into routed, driven tasks. **EXISTS** (classification + attribution seam),
  **BUILD** (the recommendation→driven-run link).
- **Relay run engine.** The coordinator **is** the body of `drive_run`, replacing today's
  single-shot `claude -p` on the run's `goal` (`advance_run.rs:256-259`, explicitly MVP) with the
  Planner→Builder→Judge loop whose per-step driver is chosen by the router at `driver_for`. It
  inherits, for free: heartbeat/housekeeping, the status machine, limit-parse/auto-pause
  (`run_limits.rs`), the watchdog escalation ladder (`run_watchdog.rs`), crash recovery, and durable
  `activity.runs` + `run_events` persistence. It stays behind `SENSEI_RUN_DRIVE` (OFF by default)
  and remains daemon-owned (survives session death). **EXISTS** (all the engine plumbing), **BUILD**
  (the loop + router body).
- **Task-tool subagents (Claude Code).** These remain the cloud tier — dispatched inside the Claude
  harness reading `.claude/agents/*.md` (`done-gate-verifier`, `spec-doc-reviewer`, mindset
  reviewers). The coordinator does **not** try to replace them; it **routes to** the cloud driver
  (which is a Claude Task/`claude -p` run) for the task types local models can't do yet
  (multi-file integration, architecture, review), and reserves the **local** driver for the cheap
  mechanical types. The existing reviewer/mindset agents are the discipline gates the loop invokes.
  **EXISTS** (the agents), **BUILD** (the executor that auto-invokes them from the loop).

Net composition: **intake types the work → the router picks a driver per task → the run engine
drives + supervises → attribution feeds the learning loop → the router improves.** Every arrow but
"router" and "local driver" already exists.

---

## 5. Where the coordinator lives, and the MCP surface

Two placement options (see Open Decisions):

- **Daemon (recommended)** — the router is a pure fn plus a `driver_for` dispatch inside
  `crates/senseid/src/relay_drivers/`, and the loop is the body of `advance_run::drive_run`. This
  keeps it **daemon-owned** (survives session death), reuses the run engine wholesale, and puts the
  router next to the gateway it routes to. Matches the "daemon-owned coordinator" differentiator.
- **Plugin / harness** — a `superpowers`-style meta-skill that dispatches Task-tool subagents. This
  is *Claude-only by construction* (the Task tool's `model` param is a Claude tier — the exact
  harness constraint this whole design routes around) and dies with the session. **Rejected for the
  coordinator itself**; the plugin stays the *cloud driver's* execution substrate.

**MCP surface** (extend, don't fork — the tools already exist as the local-inference surface):

| Tool | State | Role in the coordinator |
|---|---|---|
| `infer` (`mcp/src/lib.rs:423`, `POST /api/gateway/infer`) | **EXISTS** | the gateway-mini driver's per-step call (local chain) |
| `consensus` (`handlers/gateway.rs:194`) | **EXISTS** (sequential stand-in) | swap onto native `execute_consensus` for a family-distinct local panel |
| `gateway_status` / `embed` | **EXISTS** | readiness probe / embeddings |
| `start_run` / `run_status` (`mcp/src/lib.rs:538`) | **EXISTS** | create + observe a coordinated run (drive still gated) |
| `route_task` (new) | **BUILD** | classify a typed task → chosen driver + rationale (dry-run + execute) |
| model-stats extension (`GET /api/playbook/model-stats`) | **EXISTS**, extend | `task → driver → outcome` router-learning readout |

---

## 6. Phased build plan

**v1 — local-only extension (prove the seam, no external mix).** Ship the local driver + a
minimal router behind `SENSEI_RUN_DRIVE`, default OFF.

1. **Gateway-mini local driver** — a `RunDriver` impl whose `drive_step` runs a bounded
   read→infer→act→verify loop via gateway `infer` on a **local** chain (embedded `gemma2:2b` /
   Ollama `gemma4`/`qwen3`). Scope to the honest-limit task types only: single-file mechanical
   edit, structured extraction, doc-gen. (Alternative/parallel: an ACP-drive local agent — heavier,
   deferred.)
2. **Task router** at `relay_drivers/mod.rs::driver_for` — pure `router(Axes × capability-contract)`;
   v1 routes only "cheap mechanical + local-model-confident" tasks to the local driver, **everything
   else to `ClaudeDriver`** (the current default, unchanged).
3. **Inference-usage ledger + `task → driver → outcome` attribution** — the new DDL table +
   extend the `classified_by`/`model_fallback` pattern. Prerequisite for trusting any routing.
4. **Verify-before-escalate** — wire the cheap per-task verifier (tests/lint/spec-diff); on
   local-driver failure, **escalate to `ClaudeDriver`** (reuses the run engine's existing outcome map).
5. **`route_task` MCP tool** (dry-run first) so intake/playbooks can preview a task's routing.

Exit criteria: a local-only run completes a cheap task end-to-end, escalates correctly on failure,
and the ledger shows where the local/cloud boundary actually is.

**v2 — local + external mix (the real router).** Turn on the full taxonomy.

6. **Cloud driver via the same seam** — route multi-file integration / debugging / architecture /
   review to Claude; pin **reviewer/gate tier to a strong cloud model, exempt from local-first** (D12).
7. **MoE / panel** — add the `panels`/`consensus` DDL, swap the MCP `consensus` tool onto the
   gateway's native `execute_consensus`, and seed a `distinct_by:family` panel (gemma + qwen) for
   decision/judge steps.
8. **Router learning loop** — feed the ledger + `task→driver→outcome` into a `learn_playbooks`-style
   pass so routing tunes itself; let a playbook declare per-task-type local/cloud preferences.
9. **Planner → Builder → Judge loop** (Phase 6) — replace single-shot `drive_run` with the iterative
   loop, each step routed by the v2 router; needs its own feature doc + story breakdown, deferred
   post-stable (P0–P4 single-assistant Claude-first ship first — relay-engine §7, decided 2026-07-16).

---

## 7. Open decisions (for Jerry)

1. **Which local models?** v1 default: embedded `gemma2:2b` (in-process, no Ollama) for the cheapest
   types + Ollama `gemma4`/`qwen3:14b` for slightly heavier local tasks? Or Ollama-only to avoid the
   embedded build-gate (default-instance-only, `--features embedded-llama-cpp`)?
2. **Coordinator placement — daemon vs plugin?** Recommendation: **daemon** (survives session death,
   reuses the run engine, next to the gateway). Confirm we are *not* building a harness-side
   superpowers clone for the coordinator itself.
3. **Local driver kind — gateway-mini vs local ACP agent?** v1 leans **gateway-mini** (a bounded
   infer-loop, no new external dependency, unit-testable). The ACP-drive path (ollama/aider) is
   heavier and blocked on P5.2b drive-over-ACP — defer, or fund it in parallel?
4. **Task-type taxonomy** — reuse the intake `Axes` (Lifecycle/Intent/Risk) + capability contract,
   or a separate execution taxonomy? (Reuse is the on-strategy default.)
5. **Verify-before-escalate** — what is the cheap verifier per task type: tests, lint, spec-diff,
   or a cloud judge? (Determines escalation trustworthiness.)
6. **MCP surface** — add a `route_task` tool now (dry-run routing preview), or keep routing internal
   to the run engine until v2?
7. **Activation & safety** — v1 stays behind `SENSEI_RUN_DRIVE` (OFF). What is the gate to enable a
   local-driver pilot, and does the reviewer/gate exemption (never-local) hold from day one?
8. **Panel/consensus DDL** — build the `panels`/`consensus` tables + swap the MCP `consensus` tool
   onto native `execute_consensus` in v2, or sooner (the family axis is already threaded end-to-end)?

---

## 8. Status

| Piece | State |
|---|---|
| Local inference (embedded llama.cpp `gemma2:2b`, Ollama `gemma4`/`qwen3`, chains) | **EXISTS**, local-first, agent-callable via MCP `infer` |
| `RunDriver` seam + `driver_for` insertion point | **EXISTS** (`driver_for` hardcoded to `ClaudeDriver`) |
| ACP observe / ACP drive | observe **EXISTS** (pure mapping); drive **BUILD** (P5.2b) |
| Gateway native panels/consensus + `family` axis | machinery **EXISTS**, configs empty; MCP `consensus` is a sequential stand-in |
| Outcome attribution (`classified_by`/`model_fallback`) | **EXISTS** on `playbook_run`; `task→driver→outcome` **BUILD** |
| Inference-usage ledger | **BUILD** (no per-inference usage table today) |
| Local-model agent driver (gateway-mini or ACP) | **BUILD** — the core blocker |
| Task-typed router | **BUILD** |
| Planner → Builder → Judge coordinator loop | **BUILD** (Phase 6, design-only; single-shot today) |
