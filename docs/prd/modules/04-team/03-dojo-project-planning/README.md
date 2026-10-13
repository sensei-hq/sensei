# Dōjō Project Planning · T3

**Plane:** team · **Status:** [roadmap/04-team.md](../../../../roadmap/04-team.md) ·
**Serves:** manager / delivery lead ([persona 6](../../../03-personas.md#6--manager--delivery-lead--the-roll-up)),
maintainer ([persona 3](../../../03-personas.md#3--maintainer--the-curator)),
engagement / client lead ([persona 5](../../../03-personas.md#5--engagement--client-lead--the-confidentiality-guard))

## Purpose

Dōjō Project Planning connects a project to its **external tracker** (Jira, ADO
read-only) and enforces **delta-scoped, ratchet-fair quality gates** before a
change merges. The planner (M20) emits a `PlanGraph`; the run engine (M21)
drives execution; T3 wires the tracker so the plan reflects the tracker's
state, and the gates enforce the project's own baseline (never a borrowed
benchmark). The job is **plan ↔ tracker sync + gates that only tighten**. The
user feels this as *my Jira board and my quality gates speak the same language —
the gate knows what the plan promised, and it only ever asks for better.*

> **Status honesty.** The gate/improvement split and own-baseline ratchet are
> designed in reorg §9.3–9.4. **The tracker adapters (Jira / ADO) exist only in
> the mockup — nothing is built** (gap T-1, P1). 2 of 7 gating targets are
> buildable today, 2 need scoped reads, 3 are blocked on the architecture-metrics
> family (#156). The project window Tasks tab (M16) stays unbuilt until planner
> persistence lands (C-1 → M20).

## In scope / out of scope

- In: the tracker sync (Jira / ADO read-only) → `PlanGraph` fields; the
  **gate/improvement split** — a gate is a mandatory threshold (blocks merge),
  an improvement is advisory (shows in Observatory, never blocks); the
  **own-baseline ratchet** — the baseline is the project's FTR at plan start, it
  only advances when FTR **improves and holds** (never on a single spike); the
  7 gating targets (see table); the project-planning console in the org context
  (maintainer/admin surface).
- Out: the planner persistence (C-1 → M20), the run engine (M21), the
  Instruments verdict classifier (M18), the architecture-metrics family (#156).

## Surfaces

- **App** — `/org/[slug]/projects/[id]/planning` (maintainer/admin console):
  tracker connection, plan import/export, gate config, baseline view, ratchet
  history.
- **Daemon** — `planner.rs` (v0 generate), `plan_graph.rs` (model),
  `runs.rs` (register plan, update task status, report outcome);
  `dojo/routing.rs` (client binding for tracker scope).
- **Data** — `sensei.runs`, `sensei.run_events`; **no tracker tables yet** (T-1);
  `dojo.memberships` (role floor for who can configure gates).

## Data invariants

- **Lead.** A gate is **mandatory** — it blocks merge; an improvement is
  **advisory** — it surfaces in Observatory, never blocks.
- **Lead.** The **baseline is the project's FTR at plan start** (own-baseline,
  not a borrowed benchmark).
- **Lead.** The **ratchet only advances when FTR improves and holds** — a
  single spike never advances the baseline.
- **Lead.** Gating targets are **delta-scoped** — they measure change from the
  baseline, not absolute thresholds.
- **Lead.** Client work uses the **client's tracker and gates** — the owning
  org's rules win ordinary conflicts; the employer's mandatory rules are a floor.

## The 7 gating targets

| # | Target | Type | Buildable today? | Blocker |
|---|---|---|---|---|
| 1 | FTR ≥ baseline + δ | gate | ✅ | — |
| 2 | No new hard blocks (gate) | gate | ✅ | — |
| 3 | No new semantic review flags | gate | ◐ | needs scoped read |
| 4 | Architecture metric: coupling ≤ baseline | gate | ✗ | #156 |
| 5 | Architecture metric: cohesion ≥ baseline | gate | ✗ | #156 |
| 6 | Architecture metric: cycles = 0 | gate | ✗ | #156 |
| 7 | Doc drift: referenced docs ≤ baseline | improvement | ◐ | needs scoped read |

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Tracker sync status | connected / stale / error | `Jira · synced 2m ago` |
| Plan phases | imported from tracker | `Phase 1: Scaffold (3/7)` |
| Gate status | pass / fail / pending | `FTR gate · pass (72% ≥ 65%)` |
| Improvement | advisory, shown in Observatory | `Coupling advisory · +2% over baseline` |
| Baseline FTR | FTR at plan start | `baselineFtr: 0.65` |
| Ratchet history | when baseline advanced | `ratchet +0.03 sustained at run #47` |

## Done gate

- A project connects to Jira (or ADO read-only); the plan imports from the
  tracker and exports status back.
- The 2 buildable gates (FTR, hard blocks) run on every run outcome and block
  merge on failure.
- The baseline is recorded at plan start; the ratchet advances only on
  sustained improvement.
- The project-planning console shows tracker sync, gate config, baseline, and
  ratchet history.

```bash
# Not implemented — placeholder
curl -s -H "Authorization: Bearer $TOKEN" \
  "https://dojo.sensei-hq.com/v1/t/org/sensei-hq/projects/proj-1/planning" | jq
```

## Wrong gate

- **A gate uses an absolute threshold instead of delta-from-baseline** — gate
  type regressed.
- **The baseline advances on a single FTR spike** — ratchet regressed.
- **An advisory improvement blocks merge** — gate/improvement split regressed.
- **Client work uses the employer's tracker/gates** — client binding regressed.
- **A tracker adapter claims to write back** — read-only regressed.

## Scenarios

- [`scenarios/dojo-project-planning.feature`](scenarios/dojo-project-planning.feature) —
  the gate/improvement split, own-baseline ratchet, the 7 targets
  (`NO TEST` / `# gap: T-1` for tracker adapters).
- [`scenarios/dojo-project-planning-gaps.feature`](scenarios/dojo-project-planning-gaps.feature) —
  the unbuilt tracker adapters and scoped reads (`NO TEST` / `# gap: T-1`,
  `# gap: architecture-metrics`).

## Open gaps

- [T-1](../../../../roadmap/06-gaps.md) — **tracker adapters (Jira / ADO)
  exist only in mockup — nothing built** (P1). Blocks T3 console.
- [C-1](../../../../reorg.md) — planner DB persistence + baseline + ratchet
  (M20). Blocks project window Tasks tab (M16).
- Architecture-metrics family (#156) — blocks 3 of 7 gating targets.

## Related

- [reorg.md §9](../../../../reorg.md) — gate/improvement split, own-baseline
  ratchet, the 7 gating targets.
- [design/dojo-web.md](../../../../design/dojo-web.md) — org context nav for
  maintainer (triage, approvals, knowledge, project planning).
- [prd/02-objectives.md §06](../../../02-objectives.md) · [prd/03-personas.md §3,5,6](../../../03-personas.md).
- Crates: `crates/senseid/src/{planner.rs, plan_graph.rs, api/handlers/runs.rs,
  dojo/routing.rs}`.