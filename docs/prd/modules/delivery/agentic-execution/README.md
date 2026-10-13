# Agentic Execution · M22

**Plane:** delivery · **Status:** [roadmap/03-delivery.md](../../../../roadmap/03-delivery.md) ·
**Serves:** manager / delivery lead ([persona 6](../../../03-personas.md#6--manager--delivery-lead--the-roll-up)),
solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge))

## Purpose

Agentic Execution is the **autonomous loop**: a high-level goal → planner (M20) →
relay (M21) → verifier → iterate until done. The planner emits a `PlanGraph`; the
relay drives an agent step by step; a verifier (the `checker.rs` machinery) runs
the project's test/lint/build commands and feeds the result back; the loop
continues or revises the plan. The job is to **close the loop from intent to
verified outcome** without human-in-the-loop for each step. The user feels this
as *I state the goal, sensei plans, executes, verifies, and iterates until it
works.*

> **Status honesty.** This module is **design-only**. There is **no
> `crates/senseid/src/agents/` directory**, **no `sensei.agent_runs` table in
> DDL**, **no benchmark runner code**, and the owner files named by
> `spec/pipeline/agent-execution.md` **do not exist**. The nearest shipped
> components are the **planner v0** (`planner.rs`), the **plan graph**
> (`plan_graph.rs`), the **run engine** (`advance_run.rs`, `run_watchdog.rs`,
> `run_limits.rs`, relay drivers) and the **checker** (`checker.rs` — captures
> pass/fail/spawn-error from a command). The legacy screen spec
> `spec/screen/agent-*` and the pipeline spec `spec/pipeline/agent-execution.md`
> describe a fully-featured system — they are placeholders until the module is
> built. Gap [C-6](../../../../reorg.md) (agent/persona editors) maps here.

## In scope / out of scope

- In (when built): the agent loop (plan → drive → verify → revise); the verifier
  (reuse `checker.rs` for test/lint/build); the agent run record
  (`sensei.agent_runs`); the benchmark runner (compare agent vs human FTR);
  agent/persona editors (C-6).
- Out: the planner (M20), the relay/drive engine (M21), the checker (already in
  `crates/senseid/src/checker.rs`), the Instruments verdict classifier (M18).

## Surfaces

- **API (planned)** — `POST /api/agent/runs` · `GET /api/agent/runs/{id}` ·
  `GET /api/agent/runs/{id}/steps` · `POST /api/agent/benchmarks`.
- **Daemon (planned)** — `agents/loop.rs` · `agents/verify.rs` ·
  `agents/benchmarks.rs`; reuse `planner.rs`, `plan_graph.rs`,
  `tasks/handlers/advance_run.rs`, `checker.rs`.
- **App (planned)** — agent run detail in Observatory; agent/persona editors
  (C-6).
- **Data (planned)** — `sensei.agent_runs` (id, goal, plan_graph, status,
  started_at, finished_at, final_verdict, benchmark_id).

## Data invariants

- **Lead.** An agent run records the **goal**, the **initial plan graph**, the
  **step-by-step transcript** (drive → verify → revise), and the **final
  verdict** (pass / fail / partial).
- **Lead.** The verifier runs the project's actual test/lint/build commands via
  `checker.rs` — no synthetic proxies.
- **Lead.** The benchmark runner compares **agent FTR** vs **human baseline FTR**
  on the same task set — no fabricated comparisons.
- **Lead.** The loop has a **hard step cap** and a **wall-time budget** (reuse
  `run_limits.rs` logic).

## Signals shown (planned)

| Element | Meaning | Worked example |
|---|---|---|
| Agent run status | pending / running / verifying / revising / done / failed | `verifying` |
| Step | drive tick + verify result | `step 3: drive → test failed → revise` |
| Plan revision | the delta from the previous plan | `revised: added integration test task` |
| Verifier output | pass / fail / partial + raw output | `fail · 3 tests failed` |
| Benchmark | agent FTR vs human FTR | `agent 68% vs human 72%` |

## Done gate (planned)

- An agent run created with a goal produces a plan, drives the agent, runs the
  verifier, and either passes or revises the plan up to the step cap.
- The verifier output is the raw test/lint/build result — no interpretation.
- The final run record includes the full transcript and the final verdict.
- A benchmark run compares agent vs human on a fixed task set and stores the
  comparison.

```bash
# Not implemented — placeholder
curl -s -X POST http://localhost:7744/api/agent/runs \
  -H 'content-type: application/json' \
  -d '{"goal":"Add auth with tests","projectId":"..."}'
```

## Wrong gate (planned)

- **The agent run claims to have passed but the verifier output shows failures.**
- **The plan graph mutates without a revision record.**
- **The benchmark compares agent FTR to a fabricated number.**
- **The loop exceeds the step cap or wall-time budget without terminating.**
- **An agent/persona editor claims to edit a persona that doesn't exist in the
  DB** — C-6 unbuilt.

## Scenarios

- [`scenarios/agentic-execution.feature`](scenarios/agentic-execution.feature) —
  the designed loop, verifier, benchmarks and editors (all `NO TEST` /
  `# gap: C-6` / `# gap: agentic-unbuilt`).

## Open gaps

- [C-6](../../../../reorg.md) — **agent/persona editors** → M22.
- **`sensei.agent_runs` table** — absent from DDL.
- **`agents/` crate** — does not exist.
- **Benchmark runner** — absent (no code, no test).
- **Owner files from `spec/pipeline/agent-execution.md`** — do not exist.

## Related

- [spec/pipeline/agent-execution.md](../../../../spec/pipeline/) · [spec/screen/agent-*](../../../../spec/screen/) — legacy depth, placeholders.
- [prd/02-objectives.md §06](../../../02-objectives.md) · [prd/03-personas.md §1,6](../../../03-personas.md).
- Crates: `crates/senseid/src/{planner.rs, plan_graph.rs, checker.rs, tasks/handlers/advance_run.rs, run_limits.rs}` — the nearest shipped pieces.