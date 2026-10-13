---
name: Heterogeneous execution router — design note
date: 2026-07-19
status: design note / vision — NOT planned; backlog item raised
relates: docs/plan/operating-model.md §3.7 (Relay), docs/plan/relay-engine.md, docs/plan/2026-07-19-frontdoor-intake-design.md
---

# Heterogeneous execution router

A forward-looking design note (captured from a 2026-07-19 discussion, mid Plan-1 execution).
Not planned yet — this records the direction so it isn't lost.

## Problem

A subagent-based execution run through **Claude Code's Agent tool uses Claude models only** —
the tool's `model` parameter is a Claude tier (opus/sonnet/haiku/fable); there is no seam to
run a subagent on a **local model** (gemma/ollama via the gateway). So today's subagent
execution makes **zero use of local models**, even for trivial task types where a local model
would be plenty. That is a *harness* constraint, not a fundamental one.

The question: can sensei define its **own** subagent-style execution that routes tasks to
**alternative agents/models** — local where local suffices, cloud where it doesn't?

## The idea — sensei already has most of the pieces

Answer: yes, and it's the **execution-plane** of the operating model. Two existing subsystems
supply the machinery:

- **Relay (§3.7 / `relay-engine.md`)** — sensei's execution/supervision runtime. It already
  drives agent sessions through **ACP adapters** (Claude, Zed today). ACP is a protocol, so any
  ACP-speaking agent can be a *driver* — including a **local-model-backed** one (an
  ollama/aider/continue-style coding agent), or a thin **gateway-backed mini-agent** for simple
  single-shot steps.
- **The gateway** — already routes inference **local-first** across local + cloud models. The
  model layer is solved.

**Missing piece:** a **task-typed router** that, per task, picks the cheapest *capable* driver.

## Architecture

```
typed task ── router(task_type × risk × capability-contract) ── driver
                                                                  ├─ gateway inference (single-shot, no agent loop)
                                                                  ├─ local-model agent via ACP (cheap, bounded)
                                                                  └─ cloud agent via ACP (Claude)
              └─ verify result → on fail, ESCALATE to the next driver up
```

Routing table (starting point — learned/tuned over time):

| task type | driver |
|---|---|
| classify · extract · format · doc-gen · seed-author | gateway inference (local model) |
| single-file mechanical edit, exact spec | local-model agent (ACP), escalate on fail |
| multi-file integration · debugging | cloud agent |
| architecture · design · review | cloud agent |

**Escalation** mirrors patterns sensei already uses: the gateway's fallback chains and the
hook-gate's fail-open — attempt local, verify (tests/lint/spec-check), escalate to cloud on
failure. Cheap-first, correct-eventually.

## Honest limits (2026)

Gemma-class local models are reliable at **narrow** types — classification, short generation,
structured extraction — but **not yet** at autonomous multi-file *implementer/reviewer* roles.
So the first real wins are: route the cheap mechanical types local, keep reasoning/integration
on cloud, and **measure** where the boundary actually is (rather than guess).

## Measurement (prerequisite for trusting the router)

You cannot route well without data on where local models are good enough.

- **First step — already taken:** the front-door intake feature records `classified_by` +
  `model_fallback` on `sensei.playbook_run` (its `classify_chunk` is gateway-local-first with a
  heuristic fallback). That is the first datapoint: "was the local model good enough to classify
  this chunk, or did it fall back?"
- **Bigger follow-up:** a general **gateway inference-usage ledger** — one row per gateway call:
  model, chain, capability, latency, success, tokens. Today `gateway/` tables are **config-only**
  (models/routers/fallback_chain_models); only `insight_copy` persists `resp.model`; there is **no
  per-inference usage table**. The router's learning loop needs one.

## On-strategy, not a new invention

This is the execution-plane expression of the operating model's **capability contract** +
**depth-proportional-to-risk** + **§9 learning loop**. A **playbook** could even declare which of
its task types route local vs cloud. It composes with Relay (drivers), the gateway (routing), and
the intake/playbook work (typed tasks + outcome attribution).

## What it needs to build (when planned)

1. An **ACP adapter for a local-model agent** (or a gateway-backed mini-agent for single-shot tasks).
2. The **task router** (task_type × risk × capability-contract → driver) + an **escalation** policy.
3. **Outcome attribution** — the gateway inference-usage ledger + task→driver→outcome records, so
   §9 can learn the routing (which local models are good enough for which task types).

## Open questions

- Task-type taxonomy for routing — reuse the intake axes/playbook types, or a separate execution
  taxonomy?
- Verify-before-escalate: what's the cheap verifier per task type (tests, lint, spec-diff, a cloud
  judge)?
- Does the router live in Relay (drives agents) or as a gateway-adjacent service (drives inferences)
  — likely both tiers (single-shot inferences vs full agent runs)?
