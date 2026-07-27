---
title: Dōjō2 mock ↔ implementation gap analysis
description: Compares the updated dōjō2 mocks with the shipped SvelteKit app; identifies the mockup gaps to hand the LLM designer and the build changes needed to land the inbox model.
type: analysis
status: analysis-complete
created: 2026-07-27
depends_on:
  - docs/mockups/dojo2-review.md
  - docs/design/mockup-brief.md
related_issues: []
references:
  - docs/mockups/Sensei/lib/dojo2/dojo2-app.jsx
  - docs/mockups/Sensei/lib/dojo2/dojo2-kit.jsx
  - dojo/src/lib/dojo2-nav.ts
  - dojo/src/routes/(dojo2)/you/[section]/+page.svelte
  - dojo/src/routes/(dojo2)/you/runs/[run_id]/+page.svelte
  - dojo/src/lib/components/kit/
---

# Dōjō2 mock ↔ implementation gap analysis

## Objective

The updated dōjō2 mocks landed the **inbox model** (one list of in-flight sessions →
click → the session's plan + progress) and a rich run-detail flow. This analysis answers:
**where has the implementation fallen behind the updated mock, and what design work is still
missing from the mock that the LLM designer must produce** — so planning can sequence the
build and the next designer pass. No code is written here.

Method: three parallel reads — the updated mock (screens, data shapes, kit), the shipped
`(dojo2)` routes (wired vs fixture vs stub), and a component-parity diff — cross-checked
against `dojo2-nav.ts` and the relay data layer.

## Current state

### The IA diverged — the mock collapsed to an Inbox; the impl did not

The mock's personal nav is now `Inbox · Projects · Constitution · Rule packs · My dōjōs ·
Contributions`, landing on the Inbox. The implementation (`dojo/src/lib/dojo2-nav.ts`) still
ships the **pre-inbox** IA: four separate relay sections — `Live runs · Approve · Decide ·
Chat` — with a `ScrYourWork` dashboard landing. The four surfaces all read the same two
relay lists (`listRuns` + `listGates`) and split them by kind. The mock's `ScrInbox` and its
row (`K2InboxRow`) do not exist anywhere in `dojo/src`.

The org nav DID largely land: `Home · Constitution · Projects` + role-gated
`Governance / Clients / Admin` zones-with-tabs, matching the mock's flattening.

### Run detail — data is there, the rich rendering is not

Both sides have the run-detail route (`you/runs/[run_id]`) wired to real relay data
(`getSegments` + `listRuns` + `listGates`, realtime `subscribeRelay`). But:

| Mock renders | Impl renders |
|---|---|
| Plan **outline** (phases → tasks w/ agent·model·spec_ref·deps·gate chips) via `K2PlanOutline` | A **flat Phase→Step tree** built inline from `parent_id`/`seq` |
| Plan **graph** (parallel/sequential stage flow, legend) via `K2PlanGraph` | — none |
| **Run activity** feed via `K2RunActivity` | — none |
| **Ask cards** — numbered options + free-text reply (`AskCard`) | Gate answering via `RelayGateCard` (verdict/choice only) |
| **Chat** thread (sensei ↔ viewer) | Chat = newest run's segments recast as sensei turns; human turns not read back |
| Progress bar + pips (`K2PlanBar`/`K2PlanPips`) | `done/total` count only |

The daemon already authors a real plan graph → segments (`plan_graph.rs` → `plan_to_segments`),
so the hierarchy **data** is present; only the rich rendering is missing.

### Component parity — full, except the plan/inbox family

The kit is a faithful port: ~35 of the mock's `K2*` components have a `dojo/src/lib/components/kit/`
equivalent at parity. The gap is one cohesive family — the **plan / run-detail visualizers and
the inbox** — plus their shared vocab:

- **Mock-only (unbuilt):** `K2InboxRow` · `K2SubTabs` · `K2PlanOutline` (hand-rolled inline on
  the console screen, never extracted to a kit component) · `K2PlanGraph` · `K2PlanNode` ·
  `K2PlanStage` · `K2PlanPips` · `K2PlanBar` · `K2RunActivity`.
- **Not ported:** the plan-normalization vocab — `K2_NODE` (7 task states), `K2_STATE_ALIAS`,
  and helpers `k2Phases` / `k2Tasks` / `k2StageState` / `k2PlanProgress` / `k2RunFlag`.
  `kit/vocab.ts` has no task-state map. **Everything above depends on this being ported first.**
- **Divergent:** `RunCard` — impl is a flat shell; its `KitRun` type (`kit/types.ts`) has no
  `plan`/`stale`/`last` and `state` is only `running|waiting`, so the mock's seven task-states
  and `run.plan` can't be expressed. `ConfidenceBar` drops the `showN` numeric. `GateCard`
  exists in the kit but the live screen still wires the legacy `RelayGateCard`.

### Fixtures shown to real users (honesty gaps)

Every fixture accessor falls back to a single `acme` blob (`x[slug] ?? x.acme`), so any org
renders Acme's data. A real user currently sees invented data at: the `/you` landing
(`needsYou`, fake repos `lumen-auth`/`ledger-core`, hardcoded `StatBadge sub="↑ from 9"`),
Constitution (`stance`/`ladder`), Rule packs (11 fabricated packs), Contributions
(`helped: 612`), project preview (`ladder`/`conflicts`), org home/knowledge/scopes, billing
pricing/invoices, and a fabricated identity **"Rin Saito"** used for the audit "me" label and
chat author. The relay surfaces (runs/gates/segments), members, triage, audit, engagements,
incidents, health, identities, and billing seat-count are genuinely **wired**.

## Feasibility

No architectural blocker. The relay data path is proven end-to-end (daemon → Worker `/v1`
reads → dojo2 screens, realtime). The plan hierarchy already exists as segments. The work is
UI: port the plan-model vocab, build the ~9 mock-only components, re-shape the personal IA to
the inbox, and render the run detail richly. The fixture screens need Tier-3 `/v1` read-routes
(projects, contributions, constitution) — a known follow-on, not new architecture. Main risk
is ordering: the inbox row, run card, plan outline, and plan graph all depend on the shared
plan-model vocab, so that lands first or the components can't be typed.

## Mockup gaps to hand the LLM designer

The mock is complete on the happy path but specifies **no loading or error states anywhere**,
partial empty states, and models fabricated data. These are the design tasks the designer LLM
must produce (each ready to drop into the `docs/design/mockup-brief.md` task template):

1. **State coverage for every data-backed screen.** Loading (skeleton), error, and honest-empty
   for the Inbox, run detail, org consoles, projects, contributions, constitution. Today only
   filter-miss empties exist; there is no loading or error artboard at all.
2. **The five missing critical paths** as first-class Zen-Sumi surfaces (from the coverage
   audit): 404 / not-found · `+error.svelte` boundary · permission-denied (role-gated URL) ·
   session-expired / re-auth · rate-limit (429). None are designed.
3. **Honest-data variants.** Real-data + empty versions of the surfaces that currently ship
   invented data — the landing stat strip (drop "↑ from 9"), Contributions (drop "helped 612"),
   projects (empty "No projects yet"), and the `me` identity. The build is mirroring the mock's
   fabrications; the mock should model the honest version (guardrail B9).
4. **Run-detail states.** "No plan/segments yet" empty, loading, and error for the plan
   outline/graph, the activity feed, and the chat — the run detail is the richest new screen and
   has only a happy path.
5. **Inbox states.** Fetching (skeleton rows), relay-unreachable error, and the genuine
   all-clear empty ("nothing in flight") — distinct from the current filter-miss empty.

## Build gaps (what needs to change — feeds planning)

Ordered by dependency:

1. **Port the plan-model vocab to the kit** — `K2_NODE`/`K2_STATE_ALIAS` + `k2Phases`/`k2Tasks`/
   `k2StageState`/`k2PlanProgress`/`k2RunFlag` into `kit/vocab.ts`, and extend `KitRun`/`KitInbox`
   types with `plan`/`stale`/`last` and the 7-state enum. Prerequisite for everything below.
2. **Build the plan/inbox kit family** — `PlanBar` (trivial) → `PlanNode` → `PlanStage` →
   `PlanGraph`, `PlanOutline` (extract from the console inline tree), `PlanPips`, `RunActivity`,
   `SubTabs`, `InboxRow` (+ the `k2InboxRow`/`k2InboxRows` ranking logic).
3. **Re-shape the personal IA to the Inbox** — replace the `Live runs/Approve/Decide/Chat`
   sections in `dojo2-nav.ts` and the `ScrYourWork` landing with `ScrInbox` (list + filters) and
   land on it; approve/decide/chat become in-detail actions.
4. **Enrich the run detail** — render the plan outline/graph + activity + progress from the
   existing segments; bring `RunCard` to parity; adopt the kit `GateCard` (retire the legacy
   `RelayGateCard` duplication) or reconcile the two.
5. **Wire the fixture screens** (Tier-3) — projects (`list_projects`), contributions ledger,
   constitution, via `/v1` read-routes; honest empty states until they exist.
6. **Retire the legacy `(console)` group** (D-CUTOVER, gated) once the run detail reaches parity.

## Approaches

### Option A: Design-complete first, then build
Hand the designer the full gap set (all states + critical paths + honest variants) so the mock
is a complete spec, then build the inbox IA + kit family + run-detail + wiring in one phase.
- Pros: build never re-derives; one coherent design pass; honesty fixed in the spec first.
- Cons: front-loads design; the visible inbox win waits for the whole design pass; one large build phase.
- Effort: design 1 pass (5 tasks); build large, single phase.

### Option B: Build the landed IA now; design the gaps in parallel
The inbox collapse and run-detail flow are already fully designed in the mock — build them now
(port vocab → kit family → inbox IA → run-detail), the highest-value structural change, while
the designer produces the state / critical-path / honesty gaps for the next slice.
- Pros: ships the visible IA improvement fast; parallelizes design and build; builds against a
  settled spec (the plan family is well-specified).
- Cons: the state designs may lightly rework the new components; two coordinated tracks.
- Effort: build medium-large now; design medium in parallel.

### Option C: Whole-loop first (impact surface + relay revive), dojo2 second
Prioritize the under-designed Sensei app impact surface + relay revival (the cross-plane gaps
in the review), since dojo2 is largely built; treat the dojo2 inbox catch-up as a follow-on.
- Pros: closes the biggest *design* gap (the loop / impact surface), flagged most under-designed.
- Cons: leaves the dojo2 mock visibly ahead of the impl (inbox unbuilt); defers concrete,
  well-specified work.
- Effort: design-heavy; build spread across planes.

## Decision (2026-07-27)

**Option B chosen.** Build the landed Inbox IA + run-detail now (port the plan-model vocab →
kit family → re-shape the nav → enrich the run detail); the LLM designer produces the five
mockup gaps (states, critical paths, honest-data variants) in parallel; wire the fixture
screens in the next slice. Sequencing dependency stands: **the plan-model vocab ports to the
kit first.** Carry into `/sensei:plan`.

## Recommendation

**Option B.** The inbox IA and run-detail are the highest-value change *and* the most
completely specified — the mock already carries the components, data shapes, and states for the
happy path, and the relay data is wired. Building it now removes the four-surface sprawl and the
`ScrYourWork` dashboard in one coherent slice, and it sequences cleanly behind one hard
dependency: **port the plan-model vocab to the kit first**, or `InboxRow`/`RunCard`/`PlanOutline`/
`PlanGraph` can't be typed. In parallel, hand the designer the five mockup gaps (states,
critical paths, honest-data variants) — Option A's strength — so the next build slice lands them
without blocking the structural win now. Option C's impact-surface work is real but belongs in a
separate planning thread; it shouldn't hold the dojo2 catch-up, which is concrete and ready.

Trade-off accepted: the state artboards arriving after the components may cause minor rework on
the new inbox/run-detail components — cheaper than stalling the whole build behind a full design
pass.
