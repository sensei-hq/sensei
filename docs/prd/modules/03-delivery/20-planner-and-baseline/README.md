# Planner & Baseline · M20

**Plane:** delivery · **Status:** [roadmap/03-delivery.md](../../../../roadmap/03-delivery.md) ·
**Serves:** team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor)),
maintainer ([persona 3](../../../03-personas.md#3--maintainer--the-curator)),
manager / delivery lead ([persona 6](../../../03-personas.md#6--manager--delivery-lead--the-roll-up))

## Purpose

The planner turns a **goal + depth bar** into a **phased, dependent task graph** the
run engine (M21) can execute. v0 is stateless: `POST /api/planner/generate` takes
the goal, the depth bar, and the project context, returns a `PlanGraph` (phases →
features → tasks with acceptance criteria and dependencies). The graph is
validated (DAG, no duplicate ids, no dangling/self deps, no cycles) and can be
registered on a run (`POST /api/runs/{id}/plan`). The baseline/ratchet is the
missing piece: a persisted `plan_baseline` that records the FTR at plan start,
and a ratchet that only advances the baseline when FTR **improves and holds**
(never on a single spike). The user feels this as *I describe what I want, sensei
plans it, and the baseline only moves when we actually got better.*

> **Status honesty.** The planner v0 (stateless generate + graph validation), the
> `PlanGraph` model, and the runs API (`POST /api/runs`, register plan, update
> task status, report outcome) are built and tested. **There are no plan tables,
> no baseline, no ratchet** — `rg ratchet` / `definition_of_done` = 0 hits. The
> Tasks section in the project window (M16) stays unbuilt until planner
> persistence lands (gap [C-1](../../../../reorg.md) → M20). The legacy screen
> spec `spec/screen/project-tasks.md` names a fully-featured Tasks tab — it is a
> placeholder.

## In scope / out of scope

- In: `POST /api/planner/generate` (stateless v0); `PlanGraph` model (phases,
  features, tasks, acceptance criteria, dependencies, `parent_seq` ordering);
  graph validation (DAG, no duplicates, no dangling/self deps, no cycles);
  `POST /api/runs/{id}/plan` (register graph on a run); `PlanGraph` round-trips
  through JSON with state; task progress (terminal / total).
- Out: **plan persistence** (no `sensei.plans`, `plan_baselines`,
  `plan_tasks` tables — C-1); **baseline FTR at plan start** (unbuilt);
  **ratchet** (only advance baseline when FTR improves and holds — unbuilt);
  **definition of done** per task (unbuilt); tracker adapters (Linear, Jira, GitHub
  Issues — D-6 → T3); architecture-metric targets (D-7 → T3·M8).

## Surfaces

- **API** — `POST /api/planner/generate` · `POST /api/runs` · `POST /api/runs/{id}/plan`
  · `GET /api/runs/{id}` · `PATCH /api/runs/{id}/tasks/{taskId}/status` ·
  `POST /api/runs/{id}/outcome`.
- **Daemon** — `planner.rs` (v0 prompt, parse, render); `plan_graph.rs` (model,
  validation, state rollup, progress); `api/handlers/planner.rs`;
  `api/handlers/runs.rs`; `db/pg_store/metric_reads/planner.rs` (placeholder,
  returns empty).
- **App** — project window Tasks tab (placeholder until C-1); run detail in
  Observatory.
- **Data** — **no plan tables yet**; runs use `sensei.runs`, `sensei.run_events`.

## Data invariants

- **Lead.** The planner is **stateless v0** — no DB write on generate; the caller
  decides whether to register the graph on a run.
- **Lead.** `PlanGraph` validates: DAG, no duplicate ids, no dangling deps, no
  self deps, no cycles — `validate_rejects_a_cycle`.
- **Lead.** Phase state rolls up from tasks: `failure` and `block` dominate.
- **Lead.** Task progress = terminal / total.
- **Lead.** `parent_seq` orders siblings; phases project to phase-then-tasks order.
- **Lead.** Round-trips through JSON with state preserved.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Goal + depth bar | what and how deep | `Add auth · depth=3` |
| Phase | top-level milestone | `Phase 1: Scaffold` |
| Feature | shippable slice | `Feature: login flow` |
| Task | atomic work with AC | `Task: add /login route · AC: 200 + session` |
| Dependency | DAG edge | `task-2 depends on task-1` |
| Task state | pending / running / done / blocked / failed | `running` |
| Phase rollup | derived from tasks | `blocked` if any task blocked |
| Progress | terminal / total | `3 / 7` |
| Baseline FTR | FTR at plan start | `baselineFtr: 0.62` (unbuilt) |
| Ratchet | baseline only advances on sustained improvement | `ratchet: +0.03 sustained` (unbuilt) |

## Done gate

- `POST /api/planner/generate` returns a valid `PlanGraph` for a realistic goal.
- The graph passes all validators (DAG, no dupes, no dangling/self, no cycles).
- `register_plan` stores the graph on a run; `list_and_get` returns it with
  events.
- `update_task_status` flips state; progress counts terminal over total.
- `report_run_outcome` marks terminal or rejects.

```bash
curl -s -X POST http://localhost:7744/api/planner/generate \
  -H 'content-type: application/json' \
  -d '{"goal":"Add auth","depth":3,"projectId":"..."}' | jq '.phases|length'
```

## Wrong gate

- **The planner writes to the DB on generate** — v0 must be stateless.
- **A graph with a cycle passes validation** — cycle rejection regressed.
- **Task progress counts running as terminal** — progress formula regressed.
- **Phase rollup doesn't propagate `blocked`/`failed`** — rollup logic regressed.
- **A registered plan disappears after a restart** — persistence missing (C-1).
- **The Tasks tab claims to show plan phases** — unbuilt until C-1 lands.

## Scenarios

- [`scenarios/planner.feature`](scenarios/planner.feature) — the v0 planner
  generate, parse, render and the graph model (pinned).
- [`scenarios/baseline.feature`](scenarios/baseline.feature) — the runs API,
  the unbuilt baseline/ratchet (`NO TEST` / `# gap: C-1`).

## Open gaps

- [C-1](../../../../reorg.md) — **planner DB persistence + baseline + ratchet**
  → M20 (blocks M16 Tasks tab).
- [D-6](../../../../reorg.md) — tracker adapters (Linear, Jira, GitHub Issues) → T3.
- [D-7](../../../../reorg.md) — architecture-metric targets blocked
  (#156 ← #146/#151/#152) → T3·M8.

## Related

- [spec/screen/project-tasks.md](../../../../spec/screen/) · [spec/pipeline/agent-execution.md](../../../../spec/pipeline/) — legacy depth, folds here.
- [prd/02-objectives.md §06](../../../02-objectives.md) · [prd/03-personas.md §2,3,6](../../../03-personas.md).
- Crates: `crates/senseid/src/{planner.rs, plan_graph.rs, api/handlers/planner.rs, api/handlers/runs.rs, tasks/handlers/metrics/planner.rs}`.