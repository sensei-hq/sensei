# Dōjō Project Planning · T3

**Plane:** team · **Status:** [roadmap/04-team.md](../../../../roadmap/04-team.md) ·
**Serves:** manager / delivery lead ([persona 6](../../../03-personas.md#6--manager--delivery-lead--the-roll-up)),
maintainer ([persona 3](../../../03-personas.md#3--maintainer--the-curator)),
engagement / client lead ([persona 5](../../../03-personas.md#5--engagement--client-lead--the-confidentiality-guard))

## Purpose

Dōjō Project Planning connects a project to its **external tracker** — Jira, Azure
DevOps Boards or GitHub Issues, **read-only** — groups its repos, and enforces
**delta-scoped, ratchet-fair quality gates** before a sprint closes. The planner
(M20) emits a `PlanGraph`; the run engine (M21) drives execution; T3 syncs the
tracker so the plan and the task↔session links reflect the tracker's state, and
the gates read **code quality only** against the project's own history.

The rule the whole module turns on:

> **Targets gate on the code; insights only describe how it was made.**

A gate reads what was *merged* (hard-rule violations, coverage on guarded paths,
new layer/cycle/anti-pattern violations, changed-function complexity). An insight
reads how the *sessions went* (first-try rate, corrections, AI-assisted share,
gate wait time) and can never block a merge or a sprint close. The user feels
this as *my Jira board and my quality gates speak the same language — the gate
knows what the sprint actually changed, and it only ever asks for better.*

> **Status honesty.** The gate/improvement split and the own-baseline ratchet are
> designed in reorg §9.3–9.4. **The tracker adapters (Jira / ADO / GitHub Issues)
> exist only in the mockup — nothing is built** (gap T-1, P1). Of the seven gating
> targets, **1 is buildable today** (hard-rule violations), **2 need a scoped read**
> (coverage, changed-function complexity) and **4 are blocked** on the
> architecture-metrics family (#156). The project window Tasks tab (M16) stays
> unbuilt until planner persistence lands (C-1 → M20).

> **Resolved 2026-10-21.** Tracker connections are **owned by the dōjō** (one per
> project, shared by the team) and stay **read-only — the dōjō pulls**; sensei reads
> the mirror through sensei ⇄ dōjō sync. The same targets, ladder and modes also gate
> **releases** over their change set — see [T5](../05-releases-and-dora/README.md).

## In scope / out of scope

- In: **repo grouping** (a project groups many repos — any provider, or a local
  path via the CLI); **tracker sync, read-only** (Jira / ADO / GitHub Issues) →
  `PlanGraph` fields, with **one tracker owning sprints**; **task linkage by ticket
  key** (found in branch name, commit message or session prompt; unlinked sessions
  count toward no task); the **seven gating targets**, all code-derived and
  delta-scoped; the **inheritance ladder** (Dōjō → Project → Sprint → Task type →
  Task, most specific wins); the **modes** (`block` / `warn` / `off`); the
  **own-baseline ratchet** for the project-level readings that cannot be
  delta-scoped; the **sprint scorecard and close rule** (a sprint that misses a
  blocking target cannot close without a note); **notifications** (Slack hooks);
  the planner/run wiring that makes the readings available.
- Out: **writing back to the tracker** — adapters are strictly read-only; the
  planner persistence (C-1 → M20), the run engine (M21), the Instruments verdict
  classifier (M18), the architecture-metrics family (#156); **per-author metrics as
  gates** (never).

## Surfaces

- **App** — the Dōjō project-detail tabs (`dojo2-project.jsx`): **Repos · Team ·
  Integrations · Targets · Sprints · Code health**; and the org console
  `/org/[slug]/projects/[id]/planning` (tracker connection, gate config, baseline
  view, ratchet history) once the org context is wired (T-2).
- **Daemon** — `planner.rs` (v0 generate), `plan_graph.rs` (model),
  `runs.rs` (register plan, update task status, report outcome);
  `dojo/routing.rs` (client binding for tracker scope). **Tracker adapters: not
  present** (T-1).
- **Data** — `sensei.runs`, `sensei.run_events`; `sensei.metrics` (the `target`
  column is vestigial — a normalisation bound, not the gate config); **no tracker
  tables yet** (T-1); `dojo.memberships` (role floor for who can configure gates —
  maintainer and up).

## Data invariants

- **Lead.** **Targets gate on the code; insights never gate.** Every gated reading
  is code-derived; behavioural/AI-usage readings — **including FTR** — are insights
  only, and no insight may block a merge or a sprint close.
- **Lead.** A gate is **delta-scoped** — *new* violations, *changed* functions,
  *guarded* paths — **or a ratchet against the project's own trailing baseline**.
  Never a borrowed benchmark, and never an absolute project-level ratio (§9.3).
- **Lead.** The **ratchet advances only when a reading improves and holds** — a
  single spike never advances the baseline.
- **Lead.** **Inheritance is most-specific-wins** across **Dōjō → Project → Sprint
  → Task type → Task**; a target with no value at a scope **inherits** the nearest
  ancestor's.
- **Lead.** **Modes are `block` / `warn` / `off`.** Only `block` can prevent a
  sprint close; `warn` surfaces and never blocks; `off` disables the check.
- **Lead.** **Tracker sync is read-only.** An adapter never writes to the tracker.
- **Lead.** **One tracker owns sprints.** Projects, issues and task types may come
  from several trackers; the sprint calendar has exactly one owner.
- **Lead.** **A session counts toward a task only when its branch, commit or prompt
  carries the ticket key.** Sessions with no matching key are **unlinked** and
  count toward no task.
- **Lead.** A sprint that misses a **blocking** target **cannot close without a
  note** (maintainer and up).
- **Lead.** Client work uses the **client's tracker and gates**; the employer's
  mandatory rules remain a floor.

## The seven gating targets

All seven are code-derived. Values are the mockup's Dōjō default and the project's
override; the ladder (§ above) resolves which one applies.

| # | Target | Compare | Dōjō | Project | Mode | Source | Buildable today? |
|---|---|---|---|---|---|---|---|
| 1 | Hard-rule violations at merge | `= 0` | 0 | inherits | block | constitution checks (`checker.rs`) | ✅ |
| 2 | Coverage on guarded paths | `≥` | 75 % | 80 % | block | test runs (`auth/**`, `payments/**`) | 🟡 needs path-scoped read |
| 3 | New layer violations | `= 0` | 0 | inherits | block | code health · layer conformance | ⛔ #156 |
| 4 | New dependency cycles | `= 0` | 0 | inherits | block | code health · cycles | ⛔ #156 |
| 5 | Modules in the zone of pain | `≤` | 2 | 1 | warn | code health · architectural zones | ⛔ #156 |
| 6 | Changed functions over complexity 10 | `≤` | 3 | 2 | block | code health · cyclomatic, per function | 🟡 needs per-function read |
| 7 | New high-severity anti-patterns | `= 0` | 0 | inherits | warn | patterns · anti-pattern detector | ⛔ #156 |

**Overrides** may narrow a target for a sprint, a task type, or one task — e.g.
target 2 for `migration` tasks `≥ 90 %`; target 6 for one legacy task `≤ 4` with a
note. Most specific wins.

## Insights — tracked every sprint, never gated

| Insight | Reads |
|---|---|
| First-try rate | how often a session lands without correction |
| Corrections per session | how much steering a session needed |
| AI-assisted share of commits | how much of the change came through sessions |
| Gate wait time | how long sessions waited on an approval |

They describe how sessions went, not what was merged, so they can never be a merge
or sprint gate. They show beside the scorecard for context.

## Tracker sync & task linkage

- **Providers** — GitHub, GitLab, Azure Repos, Bitbucket, or a local path.
- **Read-only** — projects, issues/work items, sprints/iterations and task types
  are pulled into the `PlanGraph`. Nothing is written back.
- **One sprint owner.** Jira or ADO owns the sprint calendar; the other may still
  supply issues and task types.
- **Task types** map to sensei's kinds — e.g. `Story → feature`, `Bug → bug`,
  `Task → chore`, `label: migration → migration`.
- **Key pattern per repo** — e.g. `AUTH-\d+` for Jira (`AB#\d+` for ADO). A session
  links to a task when the key appears in the **branch name**, the **commit
  message**, or the **session prompt** (all three selectable).
- **Unlinked sessions** — those without a matching key — are reported ("*41 of 47
  sessions carry a key · 6 unlinked don't count toward any task*") and count toward
  no task.

## Notifications (Slack)

Three hooks, each independently toggled, posted to a channel:

| Hook | Fires when |
|---|---|
| `miss` | a blocking target is missed mid-sprint (once per target, with the tasks behind it) |
| `close` | a sprint closes (the note and the misses it explains) |
| `gate` | a session waits on a gate for over 30 m (and who it is waiting on) |

## Signals shown

| Signal | Value |
|---|---|
| Tracker sync | `acme.atlassian.net · project AUTH · board "AUTH Scrum" · synced 2m ago · 142 issues` |
| Sprint ownership | `Owned by Jira — 6 imported, AUTH Sprint 23 active` |
| Task linkage | `AUTH-\d+ found in branch · commit · prompt` |
| Sprint linkage | `41 of 47 sessions carry a key · 6 unlinked` |
| Target state | `Cov. ≥ 80% — blocks close` · `Pain ≤ 1 — warns` |
| Override | `migration ≥ 90%` · `AUTH-420 ≤ 4` |
| Ratchet | `coverage baseline +0.03 sustained at sprint 20` |
| Insight (never gated) | `First-try rate 81%` |

## Done gate

- A project connects to a tracker **read-only**; the plan imports from it and
  **nothing is written back**.
- The **buildable** targets (hard rules today; coverage and complexity once their
  scoped reads land) run on every run outcome and **block close on failure**.
- The ladder resolves most-specific-wins across Dōjō → Project → Sprint → Task
  type → Task; a `null` project value inherits.
- A sprint that misses a blocking target **cannot close without a note**.
- A session with no ticket key is **unlinked** and counts toward no task; the count
  is shown.
- The console shows tracker sync, gate config, baseline, and ratchet history.

```bash
# Not implemented — placeholder
curl -s -H "Authorization: Bearer $TOKEN" \
  "https://dojo.sensei-hq.com/v1/t/org/sensei-hq/projects/proj-1/planning" | jq
```

## Wrong gate

- **A gate uses a borrowed benchmark or an absolute project-level ratio** instead
  of a delta or the project's own baseline — delta-scoping regressed.
- **An insight — FTR, corrections, AI-assisted share, gate-wait — blocks a merge or
  a sprint close** — the gate/improvement split regressed.
- **The baseline advances on a single spike** — ratchet regressed.
- **A `warn` or `off` target blocks a close** — the mode model regressed.
- **A narrower scope does not override a broader one** — the ladder regressed.
- **Two trackers claim sprint ownership**, or a tracker adapter **writes back** —
  ownership / read-only regressed.
- **An unlinked session counts toward a task**, or a blocking miss closes without a
  note.
- **Client work uses the employer's tracker/gates** — client binding regressed.

## Scenarios

- [`scenarios/dojo-project-planning.feature`](scenarios/dojo-project-planning.feature) —
  the gate/insight split, the own-baseline ratchet, the seven targets, the ladder,
  the modes, the close rule, read-only sync (`NO TEST` / `# gap: T-1` for adapters).
- [`scenarios/dojo-project-planning-gaps.feature`](scenarios/dojo-project-planning-gaps.feature) —
  the unbuilt tracker adapters, task linkage, and the scoped reads
  (`NO TEST` / `# gap: T-1`, `# gap: architecture-metrics`).

## Open gaps

- [T-1](../../../../roadmap/06-gaps.md) — **tracker adapters (Jira / ADO / GitHub
  Issues) exist only in mockup — nothing built** (P1). Blocks the T3 console.
- [T-2](../../../../roadmap/06-gaps.md) — Dōjō join / membership — blocks the org
  console and the role floor for gate configuration.
- [C-1](../../../../reorg.md) — planner DB persistence + baseline + ratchet (M20).
  Blocks the project window Tasks tab (M16).
- Architecture-metrics family (#156, blocked on #146/#151/#152) — blocks 4 of 7
  gating targets (layers, cycles, zone of pain, anti-patterns) and needs scoped
  reads for 2 more (coverage, changed-function complexity).

## Related

- [reorg.md §9](../../../../reorg.md) — the gate/improvement split, own-baseline
  ratchet, and the seven targets.
- [design/integrations-and-gating-requirements.md](../../../../design/integrations-and-gating-requirements.md) —
  the full tracker/targets/gating requirements this spec distils.
- [design/dojo-web.md](../../../../design/dojo-web.md) — org context nav for
  maintainer (triage, approvals, knowledge, project planning).
- [spec/dojo/governance](../../../../spec/dojo/governance/) — the constitution
  checks target 1 reads.
- [prd/02-objectives.md §06](../../../02-objectives.md) · [prd/03-personas.md §3,5,6](../../../03-personas.md).
- Crates: `crates/senseid/src/{planner.rs, plan_graph.rs, api/handlers/runs.rs,
  dojo/routing.rs}`.
- Mockup: `docs/mockups/Sensei/lib/dojo2/dojo2-project.jsx`.
