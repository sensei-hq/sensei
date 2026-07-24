---
title: Governance + Execution Gap Analysis
date: 2026-07-24
status: analysis (observed facts + gaps, not decisions)
scope: governance/constitution · skills-agents stickiness · local-model inference · orchestration/execution · docs-framework · built dojo2/senseid/Worker
method: read-only sweep of docs + DDL + crates/{senseid,mcp,cli} + marketplace/plugins/sensei + dojo/ (6 area maps)
supersedes: nothing — this is a point-in-time delta doc
---

# Governance + Execution Gap Analysis — 2026-07-24

> **What this is.** A subsystem-by-subsystem map of PREMISE (what the vision / design docs assume)
> vs IMPLEMENTATION (what is actually built and wired) vs GAP (what is missing), with each gap
> ranked by whether it **BLOCKS a full unattended auto build-out** (a "vacation run" where sensei
> drives the work end-to-end with no human in the loop). This is **analysis, not decisions** —
> recommendations are kept light and open questions are flagged inline with **[OPEN]**.
>
> **Blocking legend:**
> - **BLOCKS** — an unattended build-out cannot correctly complete or would silently do the wrong thing.
> - **DEGRADES** — the run proceeds but a core promise (governance stickiness, local-first, provenance) is unmet.
> - **COSMETIC** — drift / hygiene that misleads a reader but does not stop the run.

---

## 0. Executive frame

Two orthogonal ambitions run through the docs:

1. **Governance is push, non-negotiable, always-on** — "context/mindsets/metrics injected into the work," a `mandatory` constitution tier that "cannot be overridden by a more specific scope" (`docs/architecture/concepts/governance.md:36`, `docs/plan/operating-model.md:65`).
2. **Sensei drives** — a daemon-owned coordinator that runs an unattended build-out mixing local + cloud subagents, survives session death, and learns which model to route where (`docs/plan/operating-model.md:19-38`, `docs/plan/relay-engine.md` §5).

The observed reality is that **the deterministic data-model machinery for both is largely built and unit-tested, but the delivery/enforcement/drive edges are pull-based, off-by-default, or unwired.** The constitution resolves correctly but reaches nobody automatically; the run engine ticks correctly but drives a single-shot `claude -p` behind an OFF switch; the heterogeneous router that would mix local models is a captured design note marked *NOT planned*. The net effect: **an unattended build-out today would run as a single Claude process with no enforced governance, no local-model legs in the agent loop, and no learning ledger** — the opposite of the stated end-state.

---

## 1. Governance / Constitution / Instruction-injection

### Premise
- Two-axis model: **scope** (`general < user < organization < client < technology < team < project < repository`, most-specific-wins by integer `level`) × **enforcement** (`advisory < recommended < required < mandatory`; `mandatory` = non-overridable constitution tier). `docs/architecture/concepts/governance.md:29-36`; `database/ddl/enum/sensei/enforcement.ddl:8-9`.
- A rule **is a memory** — no parallel rules table; `sensei.memories` carries `namespace_id`, `enforcement`, `origin`, `source_id`. `database/ddl/table/sensei/memories.ddl:12-15`.
- Governance is **DB-owned**; `~/.sensei/rules.md` is a read-only materialized view; there is **no** per-repo `.sensei/rules.md`. `docs/features/default-constitution.md:18-35`; `governance.md:152`.
- A default constitution (~30 mandatory/guardrail/guideline rows) is seeded into `org/global-dojo` and distributed down. `docs/features/default-constitution.md:18-35,102-107`.
- Provenance origins are `authored | promoted | remote`. `governance.md:110-112`.

### Implementation (built + wired, verified)
| Capability | Evidence |
|---|---|
| Two-axis data model (`scopes`, `namespaces`, `folder_namespaces`, memories governance cols, `enforcement` enum) | `database/ddl/table/sensei/{scopes,namespaces,memories}.ddl`; `enum/sensei/enforcement.ddl` |
| Tier-1 deterministic resolve (dedup + mandatory-lock, strongest-first) | `crates/senseid/src/governance.rs:55-128`; `crates/senseid/src/db/pg_store.rs:7325` (`resolve_rules_raw`), `:7393` (`resolve_global_rules`) |
| `~/.sensei/rules.md` materialize on boot + on-demand endpoint | `crates/senseid/src/api/server.rs:303-321`; `crates/senseid/src/api/handlers/knowledge.rs:193-217` |
| CLAUDE.md durable pointer upsert | `crates/senseid/src/api/server.rs:310-315` |
| `get_rules` MCP tool (full per-repo resolution, all scopes) | `crates/mcp/src/lib.rs:134-137,417`; `knowledge.rs:155-188` |
| Tier-2 LLM consolidation (table + prompt + approval; materialize prefers approved row) | `consolidated_rulesets` table; `build_merge_prompt`; `knowledge.rs:198-202`; `tasks/handlers/consolidate.rs` |
| Federation pull (rules land as memories, boot loop every 300s) | `crates/senseid/src/federation/mod.rs:157-206`; `server.rs:363` |
| SessionStart hook injection | `marketplace/plugins/sensei/hooks/session-start:49-172` |
| Dōjō Worker `dojo.shared_rules` registry + `/v1/rules` routes | `dojo/src/routes/v1/rules`; `database/ddl/table/dojo/shared_rules.ddl` |

### Gaps
| # | Gap | Evidence | Blocking |
|---|---|---|---|
| G1.1 | **Default constitution is never seeded or distributed.** `dojo.seed_default_governance()` has **zero callers** (grep across Rust / `design.yaml` seed steps / Makefile / Worker migrations). The `dojo` schema is **excluded from the daemon deploy** (`excludes: [dojo]`). Fresh install renders `_No global rules yet_`. | `database/ddl/procedure/dojo/seed_default_governance.ddl`; `database/design.yaml:44`; `governance.rs:99-101` | **BLOCKS** — the mandatory baseline reaches no session. |
| G1.2 | **No auto-registration of the global-dōjō as a `knowledge_source`.** `run_pull_loop` only pulls sources already in `sensei.knowledge_sources`; nothing seeds that row on install. Even if the constitution were seeded upstream, there is no default subscription to pull it. (Push-side const `GLOBAL_DOJO_TENANT_KEY` exists in `dojo/contribute.rs:53`; no pull counterpart.) | `federation/mod.rs`; `server.rs:363` | **BLOCKS** — closes the last mile of G1.1. |
| G1.3 | **Scoped/per-project rules are pull-only, not pushed at session start.** SessionStart injects only `user`+`general` (`~/.sensei/rules.md`) plus a *textual nudge* to call `get_rules`. Org/project/tech/repo rules — the actual governance value — arrive only if the model **chooses** to call `get_rules`. "Scoped per project, at session start" is structurally unmet. | `session-start:45-57,119`; `knowledge.rs:155`; `governance.rs:94-96` | **BLOCKS** — governance is advisory-in-practice for an unattended agent that may never pull. |
| G1.4 | **Mandatory ≠ enforced at the boundary.** The mandatory-lock only affects ordering/dedup *inside a resolved set* (`governance.rs:63,75`); it does not block a session or a tool call. Delivery is model-discretionary. A mandatory rule is honored on paper, skippable in practice. | `governance.rs:63,75` | **BLOCKS** — "non-negotiable" is not enforceable without a gate. |
| G1.5 | **`origin` value drift + partial promotion.** DDL/code use free-text `origin` (`learned`/`promoted`/`federated`); doc specifies enum `authored\|promoted\|remote`. Pull writes `'federated'`, never `'remote'`. Promotion→push is "partial." | `federation/mod.rs:199`; `memories.ddl:14`; `governance.md:110-112`; `docs/features/05-governance.md:108` | DEGRADES — provenance inconsistent with spec; promoted rules don't reliably climb. |
| G1.6 | **Dual rules-file models coexist.** SessionStart still reads repo-local `${PROJECT_ROOT}/.sensei/rules.md` and `sensei init` still creates one, contradicting the DB-owned "no per-repo file" target. | `session-start:31-43`; `crates/cli/src/main.rs:598-611`; `governance.md:152` | DEGRADES — drift risk; contradicts the single-source target. |
| G1.7 | **Post-compaction gap.** SessionStart does not re-run; only the CLAUDE.md pointer (global) survives, pointing at the global file — not per-repo scoped rules. | `server.rs:310`; `session-start` (no re-run) | DEGRADES — scoped governance is lost until the model re-pulls. |
| G1.8 | **Stale transport in concept doc.** `governance.md:230-254` describes the hive-mind as embedded Postgres; superseded by the Worker/D1 (Supabase) Dōjō. | `governance.md:230-254` | COSMETIC. |
| G1.9 | **README-frontmatter identity sync claimed, unverified.** Namespace creation "at scan time from README frontmatter" is documented in depth but a live frontmatter parser was not confirmed in the sweep — treat as claimed, unverified. **[OPEN]** does a live parser exist? | `namespaces.ddl:20-22`; `governance.md:170-207` | **[OPEN]** — if absent, org/project namespaces never populate → G1.3 has nothing to resolve. |

**Anchor files:** `crates/senseid/src/governance.rs`; `db/pg_store.rs:7325/7393`; `api/handlers/knowledge.rs:155-217`; `api/server.rs:303-363`; `federation/mod.rs:157-225`; `marketplace/plugins/sensei/hooks/session-start`; `database/ddl/procedure/dojo/seed_default_governance.ddl`; `database/design.yaml:44`.

---

## 2. Skills / Agents stickiness

### Premise
- Instructions (rules, mindsets, disciplines) should **shape behavior across a whole session**, not just at turn 0. Governance is "push not pull" (`operating-model.md:65`).
- A durable pointer in CLAUDE.md/AGENTS.md is the hedge for "survives across ACPs and post-compaction states where the hook doesn't run." `governance.md:165`.

### Implementation
- Four artifact types ship in one plugin (`marketplace/plugins/sensei/`): **skills** (14, on-demand, only the frontmatter is indexed — body loads when the model self-elects), **commands** (20, user-invoked), **agents/mindsets** (8, on-demand; mindsets *are* agents, `mindsets.md:9`), **hooks** (event-driven, the one always-on channel). Plugin manifest `plugin.json:23-122`; `marketplace.json:7-18`.
- **The one push channel is SessionStart** — builds a `<sensei-session>` `additionalContext` block once (`session-start:98-172`) with workflow state, global+project rules, a *lean* mindset reminder ("apply Analyst → Developer → Acceptance Tester … run `/sensei:agent`"), personas, MCP tool guide (`session-start:59-64`). PreCompact re-injects a **thinner** `<sensei-refocus>` (`pre-compact:31-56`).
- `nudge` (PreToolUse) is capped once-per-session and informational-only (`nudge:16-20`); `gate` (PreToolUse blocking) exists but is opt-in, fail-open, unregistered (`gate:11-19`); all telemetry hooks fail-open.

### Gaps
| # | Gap | Evidence | Blocking |
|---|---|---|---|
| G2.1 | **Skills/agents are pull, not push — the model must self-elect.** A skill body loads only when the model matches its `description`. `zero-errors-policy`, `test-gen`, `knowledge-capture`, `plan-depth-review` never load if the model doesn't recognize the trigger. No mechanism forces a skill in. | `docs/architecture/marketplace.md:14,56-64` | **BLOCKS** — disciplines an unattended run depends on (TDD, zero-errors) may never activate. |
| G2.2 | **SessionStart injects once, then decays up-context.** The block sits at transcript position 0; nothing re-asserts it. Rules/mindset/tool-preference are stated exactly once. | `session-start:98-160` | DEGRADES — attention drops over a long autonomous run. |
| G2.3 | **Compaction is the cliff.** PreCompact re-inject is a *thinner* block (60-line rules head, mindset one-liner, 4 tool bullets); global rules, personas, full tool guide are dropped. The governance doc names this exact gap. | `pre-compact:31-56`; `governance.md:165` | **BLOCKS** — a long build-out compacts repeatedly; governance thins each time. |
| G2.4 | **Mindset subagents run in isolated context — insights don't persist back.** An agent returns only its report; the *mindset* it wore does not transfer to the main thread. | `agents/analyst.md:43`; `mindsets.md:82-84` | DEGRADES — main-thread reverts to the lean pointer after each subagent. |
| G2.5 | **Everything fails open.** `gate`, `nudge`, `forward`, and SessionStart telemetry are fail-open; if the daemon is down, `get_rules`/`get_patterns` return nothing and the model proceeds ungoverned. | `marketplace.md:47-51`; `gate:18-19`; `nudge:29-31` | **BLOCKS** — stickiness is contingent on daemon uptime for the entire unattended run. |
| G2.6 | **`gate` is the only enforcement primitive and it is inert.** Blocking PreToolUse enforcement is the sole way to make a mandatory rule bind, but `gate` is opt-in, fail-open, and not registered. | `gate:11-19` | **BLOCKS** — pairs with G1.4; no live path from "mandatory rule" to "denied tool call." |
| G2.7 | **Competition for the context budget.** The sensei block competes with `~/.claude/CLAUDE.md`, `sensei-hq/CLAUDE.md`, project CLAUDE.md, `MEMORY.md`, and ~200 skill descriptions, with no priority. | (observed in this very session's context) | DEGRADES. |

**Anchor files:** `marketplace/plugins/sensei/.claude-plugin/plugin.json:23-122`; `hooks/{session-start,pre-compact,nudge,gate,forward}`; `docs/architecture/concepts/mindsets.md`; `docs/architecture/marketplace.md`.

**[OPEN]** Which truly-non-negotiable mandates are *tool-observable* (and thus gate-able) vs prose-only? The gate can only enforce the former.

---

## 3. Local-model inference + gateway

### Premise
- Local-first inference across local + cloud, routed by task-class → model-chain. `docs/plan/relay-engine.md:360-366`.
- A future state where **a subagent's autonomous loop runs on a local gemma/qwen model** (heterogeneous execution), cheap-first with escalation. `docs/plan/2026-07-19-heterogeneous-execution-router.md`.
- Native MoE panels/consensus with family-distinct fan-out (gateway #13/#14).

### Implementation (production-ready for *inference*)
- **Three local backends** funneled through `Gateway::execute`, local-first: **Ollama** (localhost:11434, widest seed set), **embedded llama.cpp** (`embedded://embedded-llama`, the shipped daemon build — Makefile `CRATE_FEATURES` defaults to `--features senseid/embedded-llama-cpp`; default-instance-only), **embedded ORT/fastembed** (ONNX embeddings only, off in shipped build). `crates/senseid/src/api/gateway_init.rs:160-211,214,830-842`; `gateway_embedded.rs`; `Makefile:57,76`.
- On-demand provisioning pulls GGUFs from the Ollama registry over HTTP+sha256 into `~/.sensei/models` (never auto-pulls at startup). `crates/senseid/src/model_provision.rs:29`; `api/model_provisioning.rs`.
- Shipped chains all local-first: `classify`, `reasoning`, `summarize`, `embed` (384-dim all-minilm contract), `insight-copy` (**local-only, no cloud legs**). `database/import/staging/fallback_chain_models.jsonl`.
- Agent-callable inference surface via MCP: `infer`, `embed`, `consensus`, `gateway_status`. `crates/mcp/src/lib.rs:423-444`; `api/handlers/gateway.rs:51`.
- **Family axis ready end-to-end**: `ModelConfig.family` threaded DB→config; `family_for_baseline`. `gateway_init.rs:32-56`; `gateway_config_loader.rs:100-101,188-191`.

### Gaps
| # | Gap | Evidence | Blocking |
|---|---|---|---|
| G3.1 | **No local-model agent DRIVER (core blocker).** Run engine only drives `claude -p` (`agent_cmd` defaults `"claude"`). `agent_spawn.rs` is a process primitive only — grep for gateway/infer/model/gemma/qwen returns nothing. No ACP adapter for a local coding agent; no gateway-backed mini-agent driver. A subagent's *loop* cannot run on gemma/qwen today. | `tasks/handlers/advance_run.rs:150,159`; `agent_spawn.rs`; `heterogeneous-execution-router.md:91` | **BLOCKS** — "mix local + external subagents" is impossible without a driver. |
| G3.2 | **No task-typed router.** No `router(task_type × risk × capability-contract → driver)` + escalation policy to send cheap types local and reasoning/integration to cloud. | `heterogeneous-execution-router.md:37,92` | **BLOCKS** — nothing decides local-vs-cloud per task. |
| G3.3 | **No inference-usage ledger.** `gateway.*` tables are config-only; `database/ddl/table/inference/` has no per-inference usage row (model/latency/success/tokens). Only `insight_copy` persists `resp.model`; intake persists `classified_by`+`model_fallback` on `sensei.playbook_run` — the sole datapoint. | `database/ddl/table/gateway/*`; `database/ddl/table/inference/`; `database/ddl/table/sensei/playbook_run.ddl:14-15` | **BLOCKS the learning loop** — a router can't be trusted/tuned without it. |
| G3.4 | **Native MoE panels/consensus not wired (#13/#14).** Gateway v0.4.6 has `execute_consensus`/`PanelConfig`/`distinct_by:family`, but `GatewayConfig.panels`/`.consensus` are seeded **empty everywhere**; there is **no DDL table** for panels/consensus. The shipped `consensus` MCP tool is a hand-rolled sequential 3-step Purpose (proposer→challenger→synthesizer) — **no family-distinctness enforcement, no true fan-out, no judge quorum**. | `gateway_init.rs:729-731,906`; `gateway_config_loader.rs:247-249`; `api/handlers/gateway.rs:194-331`; kernel `config.rs:103-193` | DEGRADES — a family-distinct local panel is one config-plumbing step away but unbuilt. |
| G3.5 | **Honest capability limit (2026).** Gemma-class local models are reliable at narrow types (classify/short-gen/structured-extract) but not autonomous multi-file implementer/reviewer roles. Even with drivers, only cheap mechanical types route local. | `heterogeneous-execution-router.md:62-67` | Constraint (not a defect) — bounds what G3.1/G3.2 can safely route. |
| G3.6 | **Embedded path is build-gated + default-instance-only.** The preferred in-process `gemma2:2b` brain needs `--features embedded-llama-cpp` (a plain `cargo build --workspace` skips it) and the DEFAULT instance; named instances degrade to Ollama/cloud, gated on the resolver finding bytes. | `gateway_init.rs:160-211`; `Makefile:57` | DEGRADES — non-default instances silently lose the local brain. |

**Status of the router doc:** `heterogeneous-execution-router.md` is a **design note marked `status: NOT planned`** (`docs/backlog.md:62`). It is a captured vision, not scheduled work.

**Anchor files:** `crates/senseid/src/api/{gateway_init,gateway_config_loader,gateway_embedded,model_provisioning}.rs`; `src/{model_provision,agent_spawn}.rs`; `api/handlers/gateway.rs`; `crates/mcp/src/lib.rs`; `database/ddl/table/gateway/*`; `docs/plan/2026-07-19-heterogeneous-execution-router.md`.

---

## 4. Orchestration / Execution

### Premise
- A **daemon-owned coordinator** runs an unattended build-out: per-feature gated loop (`spec-doc-reviewer → implement → done-gate-verifier → … commit/push`), graph-safe parallelism (disjoint features parallel, overlapping serialized, worktree-isolated), remote supervision, heterogeneous models. `docs/plan/relay-engine.md` §5/§3.7; `operating-model.md:228-237`.
- A Planner→Builder→Judge orchestrator (P6). `docs/design/remote.md:85-111`.
- Superpowers-shape coordinator but daemon-owned, graph-derived, heterogeneous. (map:orchestration)

### Implementation (three decoupled layers, different maturity)
| Layer | State | Evidence |
|---|---|---|
| **A. Front-door playbooks + runs** | SHIPPED — but a `playbook_run` is a **recommendation record, not an executed agent run**. No agent is spawned; no `playbook → subagent → outcome` link. Learning loop (`LearnPlaybooks`, hourly) is real. | `crates/senseid/src/playbook.rs`; `api/handlers/playbook.rs`; `tasks/handlers/learn_playbooks.rs`; `database/ddl/table/sensei/playbook_run.ddl` |
| **B. Relay run engine** | BUILT to P3.6 — tick/heartbeat/limits/watchdog/crash-recovery are real and durable. **Drive is OFF by default** (`SENSEI_RUN_DRIVE`) and, when on, **single-shot** on the run's `goal` (spawns one `claude -p`). The per-feature reviewer loop is the documented *target only*. | `tasks/handlers/advance_run.rs:144-166,256-289`; `run_watchdog.rs`; `run_limits.rs`; `database/ddl/table/activity/{runs,run_events}.ddl`; `crates/mcp/src/lib.rs:538-549` |
| **C. Internal analyzer task queue** | SHIPPED — durable queue dispatching `TaskKind` handlers that call the gateway for **single-shot inference**, not subagents. This is where local inference actually happens. | `crates/senseid/src/tasks/` |

**Honest answer on subagent dispatch:** there is **no sensei-owned subagent dispatch**. The only real multi-agent execution is the **Claude Code Task tool** reading `/sensei:*` commands + `.claude/agents/*.md` — Claude-only by definition. The gateway does single-shot inference; the run engine spawns exactly one headless `claude -p` per tick behind an OFF switch.

**Routing seams that exist (the router that uses them does not):**
- **`RunDriver`** (agent-level): `relay_drivers/trait_def.rs:59-100`; backends `ClaudeDriver` (live), `AcpObserveDriver` (observe-only), `FallbackDriver` (coarse). Selection `driver_for` is **hardcoded to `ClaudeDriver`** with a `// FOLLOW-UP` marking the exact insertion point. `relay_drivers/mod.rs:27-31`.
- **Gateway routing** (inference-level): live, local-first, config-only tables.

### Gaps
| # | Gap | Evidence | Blocking |
|---|---|---|---|
| G4.1 | **Run drive is OFF by default and never enabled for a live pilot.** | `advance_run.rs:144-166`; `docs/design/remote.md:37-38` | **BLOCKS** — no unattended build-out runs at all until flipped. |
| G4.2 | **Drive is single-shot, not a per-feature gated loop.** Uses the run's `goal` as the whole prompt; the `spec-doc-reviewer→implement→done-gate→commit/push` loop (P3.5/P3.7) is unbuilt. | `advance_run.rs:256-259`; `relay-engine.md:156-164` | **BLOCKS** — no decomposition, no gates, no per-feature commits during a run. |
| G4.3 | **Planner→Builder→Judge orchestrator is design-only.** "No code yet"; deferred post-stable. | `docs/design/remote.md:85-111,108-111`; `roadmap/phases.md:25` | **BLOCKS** the "repeat until it works" loop. |
| G4.4 | **`driver_for` hardcoded to Claude; ACP is observe-only.** The multi-assistant seam exists but the router that selects a driver does not; drive-over-ACP (P5.2b) deferred. | `relay_drivers/mod.rs:29-31`; `relay_drivers/acp.rs:31-38`; `docs/plan/decisions.md:507` | **BLOCKS** heterogeneous dispatch (ties to G3.1). |
| G4.5 | **No graph-safe parallelism.** Runs are serial per run; `max_concurrency` is a throttle-to-1 field, not multi-feature fan-out; no overlap detection / worktree isolation / backfill. | `advance_run.rs` (one tick/run); `database/ddl/table/activity/runs.ddl`; `relay-engine.md:350-354` | DEGRADES — build-out is sequential; slower, no isolation. |
| G4.6 | **Mindset auto-invoke from blast-radius unbuilt.** Mindset agents exist as `.claude/agents/*.md` (Claude-only, pull-based); no executor pushes them from graph blast-radius. | `operating-model.md:237` | DEGRADES. |
| G4.7 | **No task→driver→outcome attribution.** Only `playbook_run.classified_by`/`model_fallback` exists; the router's learning loop can't exist (mirrors G3.3). | `playbook_run.ddl`; `activity/run_events.ddl` | **BLOCKS the learning loop.** |

**Anchor files:** `crates/senseid/src/relay_drivers/{mod.rs,trait_def.rs,acp.rs}`; `tasks/handlers/advance_run.rs` (`drive_run`); `agent_spawn.rs`; `database/ddl/table/{gateway/*,sensei/playbook_run.ddl,activity/run_events.ddl}`; `docs/design/remote.md`; `docs/plan/relay-engine.md` §5/§7; `docs/plan/operating-model.md` §3.7/§3.11.

---

## 5. Docs framework

### Premise
- A canonical stage tree (`docs/README.md:14-26`): vision → objectives → personas → journeys → roadmap → design → mockups → **features/<name>/ dossier (source of truth)** → architecture; `spec/` transitional, `plan/*` transient, `requirements/` retired.
- Per-feature **dossier** = `brief.md · design.md · plan.md · tests/ · decisions.md · mockup-ref.md`. `features/README.md:81-86`.
- Governance is deliberately **NOT** a dossier slot — it's a live plane resolved via `get_rules`. `operating-model.md:216-222`.
- Decisions land in `docs/decisions.md` (top-level, append-only). `README.md:38`; `crates/cli/src/scaffold.rs:63-69`.

### Implementation
- `sensei scaffold` writes the canonical layout; `sensei scaffold feature <name>` writes the dossier (shipped `develop` `0f15480e`/`8723520a`). `crates/cli/src/scaffold.rs:33-50`.
- `plan/README.md` is a well-maintained living gap analysis (G1-G10 with commit trail); `plan/decisions.md` is a dense ADR + discarded + deferred log.

### Gaps (all COSMETIC for a build-out, but they mislead an unattended builder)
| # | Gap | Evidence | Blocking |
|---|---|---|---|
| G5.1 | **Two `decisions.md`; the framework points at the empty one.** Canonical `docs/decisions.md` is header-only; the real ADR log is `docs/plan/decisions.md` (misfiled in the transient tier, orphaned from the canonical slot). New decisions have no obvious single home → "verbal without landing." | `docs/decisions.md:1-10`; `docs/plan/decisions.md`; `README.md:38` | COSMETIC (but the top doc-system defect). |
| G5.2 | **Dossier adoption ≈ 2 of 13 features.** Only `features/front-door/` (full 6-slot) and `features/governance/feature.md` (wrong shape). The other 11 are flat numbered files predating the convention; "source of truth = dossier" is aspirational. | `features/README.md:13-19,59`; `features/dojo/README.md` | COSMETIC — but an auto-builder reading a flat file may miss the design contract. |
| G5.3 | **Three competing phase/roadmap numberings** that collide (P2 = Relay vs P2 = differentiator vs a 9-step sequence). No reconciliation doc. | `roadmap/phases.md:18-25`; `plan/README.md:124-190`; `operating-model.md:616-630` | COSMETIC — but sequencing an auto build-out off the wrong map misorders work. |
| G5.4 | **`operating-model.md` is `status: draft (pending review)` and self-supersedes `vision.md`**, but the reframe was never folded back or accepted. Strategy-canon and vision-canon knowingly diverge. | `operating-model.md:4-6`; `vision.md:10-22` | COSMETIC. |
| G5.5 | **Stale-doc drift, un-annotated in-place.** dojo-mind retirement is DONE but `plan/decisions.md:46` / `plan/README.md:169` / `2026-07-22-dojo-ia-rebuild-plan.md:44` still reference `crates/dojo-mind`; the dojo1 IA plan and `design/dojo-web.md` are superseded by dojo2 with no "SUPERSEDED-BY" header. `personas/` and `design/` are canonical stages left empty/stub. | as cited | COSMETIC — but see §6 for the *dangerous* subset. |
| G5.6 | **Governance docs scattered across 5+ files** despite "governance is never a docs folder." | `features/05-governance.md`; `features/governance/feature.md`; `design/governance.md`; `spec/governance/`; `spec/pipeline/governance.md` | COSMETIC. |

**Where a new gap-analysis / decision / design doc belongs (per framework):** gap-analysis → `plan/README.md` or a dated `plan/*` (this doc lives in `docs/analysis/` per the request, which is a *new* home not yet in the framework — **[OPEN]** should `analysis/` be formalized as a tier?); decisions → *should* be `docs/decisions.md` (live content misfiled in `plan/decisions.md`); design → `features/<name>/design.md` (per-feature) or `design/<module>.md` (cross-cutting).

---

## 6. Built dojo2 / senseid / Worker (built-vs-premise)

### Premise
- dojo2 IA cut over to prod; org + personal consoles backed by real `/v1` data; Tier-3 (projects/scopes/billing/stance/ladder/constitution authoring) wired. (MEMORY dojo2)
- Governance is authored in the dōjō and injected into the work ("push not pull," `operating-model.md:65`).

### Implementation
- **dojo2** (`dojo/src`): `(dojo2)` route group live (`/you`, `/org/[slug]`, `+projects/[id]`); old `(console)` group **still present and routable** in parallel. ~55 kit components + 26 `Scr*` screens + `fixtures.ts` (1393 lines). **Tier-1/Tier-2 data REAL** via `/v1` (relay, memberships, org triage/approvals/members/policies/audit/identities/incidents/engagements/health), guarded by `guardTenantScope`, degrading to empty on 403/404/fail. **Tier-3 still fixture-backed** with inline `// Tier 3, needs DDL` comments.
- **Worker `/v1`**: backing store is **Supabase (service-role, `dojo` schema), NOT Cloudflare D1** — `dojo-supabase.ts` `createClient(...{db:{schema:'dojo'}})`. Real endpoint groups: rules, artifacts, relay/*, triage, members, policies, identities, engagements, incidents, audit, health. **NOT built: projects / scopes / billing / stance / ladder / constitution / rule-pack catalog** (no tables, no routes). Two auth planes: device-token (daemon) + Supabase JWT (console).
- **senseid**: governance resolve/materialize real; federation retargeted to Worker tenant path; `dojo-mind` crate **deleted**.
- **DDL**: `dojo` schema has 24 tables; **missing 7 for Tier-3** (projects, scopes, billing, stance, ladder, constitution(+sections), rule_packs). `sensei` schema *does* have `projects/scopes/namespaces` but those are daemon-side per-install, not tenant-scoped.
- **Branch state:** dojo2 is entirely on `develop`, **117 commits ahead of `main`** (v0.6.0). Cutover + merge are Jerry-gated.

### Gaps
| # | Gap | Evidence | Blocking |
|---|---|---|---|
| G6.1 | **"D1" naming trap.** Retire-plan + commit messages say "D1"; the store is **Supabase Postgres via service-role**, and "D1/D2/D3" = **Decision-1/2/3**. An auto-builder will assume Cloudflare D1 and author D1 bindings/SQL. Tier-3 tables must be `database/ddl/table/dojo/*.ddl` via `dbd deploy` + staging import. | `dojo/src/lib/server/dojo-supabase.ts`; `docs/plan/2026-07-23-retire-dojo-mind.md` | **BLOCKS** — high-confidence wrong path for any Tier-3 data work. |
| G6.2 | **Stale `daemon/` path in CLAUDE.md.** Global/project CLAUDE.md says DDL lives in `daemon/database/`; it's actually `database/ddl/` at repo root (per-object files, no combined `.sql`). A doc-following builder will `cd daemon/` and fail. | `CLAUDE.md` (project); repo layout | **BLOCKS** — docs-first rule points at a non-existent path. |
| G6.3 | **Fixtures render as "working" — silent premise trap.** Every Tier-3 surface renders full, real-looking data (billing totals, scope owners, ladder rungs) through the *same return shape* as real data. Browser-verifying "does the org console work?" shows green. Only signal = inline comments / DDL absence. | `dojo/src/routes/(dojo2)/org/[slug]/[section]/+page.ts:78-80,205-206`; `fixtures.ts` | **BLOCKS** — an auto-builder concludes "done" on unbuilt surfaces. |
| G6.4 | **Governance is resolve-only; no author path, no injection target.** Rules enter *only* via daemon federation-publish; the dōjō has **no rule-authoring write-route**, and `relay_drivers/claude.rs` does **not** inject resolved rules into a spawned agent (grep `context_pack\|inject\|get_rules` → nothing). "Push not pull" is not literally true for driven agents. | `dojo/src/routes/v1` (no author route); `relay_drivers/claude.rs`; `operating-model.md:65` | **BLOCKS** — "governance authoring" work finds no table, no route, no injection seam; must build all three; stance store is **user-scoped, not tenant-scoped** (easy to get wrong). |
| G6.5 | **Federation `seq`-on-update divergence (correctness).** Republish/retract keep prior `seq` (PostgREST can't `nextval()` inline on UPDATE), so a re-published/retracted rule **won't re-surface in a puller's delta**. Needs an in-DB trigger (precedent `dojo.relay_inbox_seq_bump`). | `dojo/src/lib/server/rules-data.ts:126-133,266-270`; `federation/mod.rs:181` | DEGRADES — "edit a rule → daemon re-pulls" silently fails; correctness bug for an auto-test. |
| G6.6 | **Hook-gate OFF + fail-open + unregistered.** The blocking PreToolUse "stickiness" gate exists but activation is Jerry-gated; it always returns `allow` unless `SENSEI_RELAY_GATE_TOOLS` is set AND a human denies. | `dojo/gate.rs:11-16`; MEMORY beta+relay | **BLOCKS** enforcement (ties to G1.4/G2.6). |
| G6.7 | **Two parallel IAs, no cutover; 117 commits unmerged.** `(console)` + `(dojo2)` both live on `develop`; nothing redirects old→new; prod does not serve dojo2. | `git rev-list main..develop`; `orgs/+page.svelte:17` | DEGRADES — a builder may edit the wrong (legacy) group or assume prod = dojo2. |

**Anchor files:** `dojo/src/routes/(dojo2)/{you,org}/[section]/+page.ts`; `dojo/src/lib/server/{dojo-auth,dojo-supabase,rules-data,artifacts-data}.ts`; `dojo/src/lib/components/kit/fixtures.ts`; `crates/senseid/src/{federation/mod.rs,governance.rs,dojo/gate.rs,relay_drivers/claude.rs}`; `database/ddl/table/dojo/` (24 tables, 7 missing); `docs/plan/2026-07-23-retire-dojo-mind.md`.

---

## 7. Blocking gaps — ranked (for a full unattended auto build-out)

Ranked by how completely each stops (or silently corrupts) a vacation-run. The first four are **run-doesn't-happen / run-does-the-wrong-thing** blockers; the rest are **promise-unmet** blockers the run would expose.

| Rank | Gap | Subsystem | Why it blocks | Refs |
|---|---|---|---|---|
| **1** | **Run drive is OFF by default; and when on, it is single-shot on `goal` — no per-feature gated loop, no Planner→Builder→Judge.** | Orchestration (G4.1/G4.2/G4.3) | No unattended build-out actually runs; even flipped, it is one `claude -p` on the whole goal with no decomposition, gates, or per-feature commits. | `advance_run.rs:144-166,256-289`; `remote.md:85-111` |
| **2** | **Default constitution never seeded/distributed + no default pull subscription.** | Governance (G1.1/G1.2) | The mandatory baseline reaches no session; a fresh install runs with `_No global rules yet_`. The run is ungoverned from turn 0. | `seed_default_governance.ddl` (no callers); `design.yaml:44`; `federation/mod.rs` |
| **3** | **Scoped/per-project rules are pull-only + mandatory is not enforced at the boundary + the one enforcement primitive (`gate`) is OFF/fail-open/unregistered.** | Governance + Stickiness (G1.3/G1.4/G2.6/G6.6) | Even a seeded constitution is model-discretionary; "non-negotiable" cannot bind an unattended agent that never pulls and is never blocked. | `session-start:45-57`; `governance.rs:63,75`; `gate:11-19`; `dojo/gate.rs:11-16` |
| **4** | **"D1" naming trap + stale `daemon/` path in CLAUDE.md.** | Built dojo2/Worker + Docs (G6.1/G6.2) | Docs-first rule sends an auto-builder to `cd daemon/` (fails) and to author Cloudflare-D1 SQL (wrong store is Supabase). High-confidence wrong path for the most likely first task (Tier-3 data). | `dojo-supabase.ts`; project CLAUDE.md |
| **5** | **Fixtures render as "working" for every Tier-3 surface.** | Built dojo2 (G6.3) | Browser/self-verification shows green on unbuilt surfaces; the auto-builder marks tasks done that have no table/route. Defeats "verify your work." | `org/[slug]/[section]/+page.ts:205`; `fixtures.ts` |
| **6** | **No local-model agent driver + no task-typed router.** | Local-model + Orchestration (G3.1/G3.2/G4.4) | The run is Claude-only; the stated heterogeneous (local+cloud) mixing is impossible; `driver_for` is hardcoded and ACP is observe-only. | `agent_spawn.rs`; `relay_drivers/mod.rs:29-31`; `heterogeneous-execution-router.md:91` |
| **7** | **No inference/task usage ledger.** | Local-model + Orchestration (G3.3/G4.7) | Without per-inference/per-task attribution, the router's learning loop cannot exist; the run can't self-tune or be trusted to route local-vs-cloud. | `database/ddl/table/{gateway/*,inference/*}`; `playbook_run.ddl:14-15` |
| **8** | **Governance is resolve-only — no author path in the dōjō and no injection into driven agents.** | Built dojo2 + Governance (G6.4) | Org-authored governance has no table/route; the daemon does not inject resolved rules into a spawned agent (no `context_pack` in `claude.rs`), so "push not pull" is false for the exact agents a build-out spawns. Stance store must be user-scoped (easy to get wrong). | `relay_drivers/claude.rs`; `dojo/src/routes/v1`; `operating-model.md:65` |

**Cross-cutting observation.** Ranks 2, 3, 6, 7, 8 all converge on **one missing seam pair**: (a) a **push/inject + enforce** path that puts resolved governance into a driven agent's context and blocks on `mandatory` violations, and (b) a **route + record** path that selects a driver per task and writes a usage ledger. The deterministic cores for both sides (governance resolve; gateway routing; `RunDriver` trait) already exist and are unit-tested — the gaps are at the **edges** (seed, subscribe, inject, enforce, drive-loop, route, record), not in the models.

### Compaction of the top-8 into "what a run needs before it can be trusted unattended"
1. Turn the drive loop on and make it a real per-feature gated loop (Rank 1).
2. Get the constitution into every session and make `mandatory` actually bind (Ranks 2, 3, 8).
3. Stop the docs/fixtures from lying to the builder (Ranks 4, 5).
4. Only then does heterogeneous routing + a learning ledger add value (Ranks 6, 7).

---

## Open questions (flagged inline above, collected)
- **[OPEN] G1.9** — Does a live README-frontmatter → `folder_namespaces` parser exist? If not, org/project namespaces never populate and per-repo resolution has nothing to return.
- **[OPEN] G2 anchor** — Which mandates are tool-observable (gate-able) vs prose-only? Only the former can be enforced by `gate`.
- **[OPEN] G3.4** — Is a family-distinct local panel (gemma+qwen) wanted as the first MoE use, given it is "one config-plumbing step away"?
- **[OPEN] G5** — Should `docs/analysis/` be formalized as a docs tier, or fold into `plan/`? And which of the three phase maps is authoritative for sequencing an auto build-out?
- **[OPEN] §6** — Is the intended Tier-3 store confirmed Supabase (not D1) for all new tables, and is stance user-scoped (not tenant-scoped) the accepted model?
