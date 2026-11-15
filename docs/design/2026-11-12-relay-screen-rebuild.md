---
title: Relay screen rebuild — components, state, stream, realtime, the e2e run
status: proposed
date: 2026-11-12
author: sensei (for Jerry)
---

# Relay screen rebuild — the breakdown before the build

**The bar (user, 2026-11-12):** Relay is confirmed only when a person can drive a run end to end from the app —
triage the backlog, create a run with tasks, register a crew, start the run, see the planner kick off (and answer
what it asks), see the plan become tasks, see assistants pick them up, and watch step progress as they work — with
no refresh anywhere. The dōjō does the same over Supabase realtime, runs staying local and synced.

Sources: mockup r1 `coord/mockups/relay-lanes/{Relay.dc.html,CONTRACT.md,COMPONENTS.md}` (2026-11-10), the shell
contract `coord/mockups/screen-shell/CONTRACT.md` (D87), the relay briefs in `coord/briefs/relay-*.md`, the
verification standard `coord/briefs/relay-component-verification.md`, the queue cutover shipped in
`docs/plan/2026-11-10-relay-autonomous-queue/` (develop `072ffec80`), and two code maps taken on develop today.

## 1. What the screen is today, and why it looks cobbled together

- **Shell.** `RelayScreen.svelte` and `RelayTaskScreen.svelte` wrap `ShellScreen` in a `<div data-screen="relay">`
  with no height class; rokkit's shell is `height:100%` with the body as the only scroller, so against an auto-height
  parent it grows and the window's `<main>` clips it. That is the Now-section scroll bug and #380. The other two shell
  users (`activity-logs`, `diagrams/[lens]`) give the wrapper `h-full min-h-0`; `diagrams/schema` mounts the shell
  directly. Fix once in `@sensei/ui` `ShellScreen` so no screen can get it wrong again.
- **Toolbar.** Now and Queue are toolbar toggles with no `icon`; Message is an `i-solar-*-bold-duotone` class; there is
  no Crew view. The mockup: five 36px icon buttons (pulse, layers, users-group-rounded · chat-round-line, info-circle),
  Queue and Crew with count badges, the pressed one is the view (#383).
- **Bands.** No stats band (#385). Filters are two raw `<select>`s and a `goto` with URL params, bypassing the shell's
  `FilterState` (#384). The info band is stripped on a failed read (the conformance lane's finding).
- **Sections.** Needs you, Reviews, Sprints, Lanes and Completed are ad-hoc `<section>`s; Lanes and Completed rows are
  inline markup, not components. `Pill` and `Tag` exist and are orphaned. There is no lane row, no completed row, no
  wait row, no run card, no member card.
- **Lanes.** `relay.now` lists every row of `relay.lanes` — no filter on `unregistered_at`, no filter to the active
  crew. Live today: 16 lanes over 3 crews, 5 unregistered; that is the "16 lanes". `suggested_name` is served but never
  shown (the codename, #395). Tasks are not grouped by lane; a lane row does not show its current step.
- **Actions.** The person can do four things: answer a decision, send a message, add a manual item to the backlog,
  create a sprint. No nudge, no crew create/close, no run start/cancel, no item close/reorder, no triage-item creation
  from the UI (the route exists), and a ticketed add-to-backlog is refused with 501.
- **Stream.** `/api/relay/events` is one unfiltered broadcast of `{action, entity, data}` where `data` carries ids only
  (`{op, task_id}`, `{op, lane_id, crew_id}`, `{op, run_id, crew_id}`). The crew stream ignores `action` and every
  payload and re-reads all of Now on any lane/task/relay event; `step` is ignored on the crew screen; triage/backlog
  writes emit nothing. A checkpoint carrying `steps[]` emits a `task` event, never a `step` event.
- **State.** `RelayState` is per route (`$derived.by(() => new RelayState(...))`), getters are plain `get`, and there is
  no `apply(event)`. The scan screen's pattern (a singleton whose sub-states extend `ReactiveStageContext` and apply
  `{action, entity, data}` in place) is not used.

## 2. The target: one composite state, pure selectors, components that only render

`COMPONENTS.md` §0 and §4 are the rule: components take a data object and emit callbacks; the page owns one
`RelayPageState`; selectors turn it into section props; components are fixture-tested in light/dark × wide/phone ×
kanji/icon; the page spec checks wiring only.

### 2a. Components (mockup → what exists → what is new)

| Part (COMPONENTS §) | Exists | Build |
|---|---|---|
| Shell: TitleBar, Toolbar, ToolbarItem, InfoBand, StatsBand, FormBand, Content, Footer (§2) | rokkit `ScreenShell` via `@sensei/ui` `ShellScreen`; `ToolbarItem` in `@sensei/ui` (no importer) | wire the five view/toggle buttons with badges; stats through the shell's `stats` prop; filters through `FilterSchema` (`@rokkit/forms` FormRenderer); footer items; fix the height wrapper in `ShellScreen` |
| Primitives: Pill, Tag (family/model/host), RoleMark 24/30 with status (§3a) | all three exist; Pill/Tag orphaned, RoleMark 30/status unused | adopt; snapshot fixtures |
| Now: DecisionCard, WaitList/WaitRow, LaneList/LaneRow, CompletedList/CompletedRow, SectionHeading (§3b) | `DecisionRow` (option-pick) | LaneRow, CompletedRow, WaitRow, SectionHeading new; DecisionCard = DecisionRow to the mockup (schema forms later, #389) |
| Queue: QueueRow (§3c) | `QueueRow` exists (selectable) | conform; add "+ add to backlog" for tickets (needs the daemon) |
| Task: TaskHeader, RunDetails, HandoffCard, HistoryEntry, RelayChecklist (§3d) | `HistoryEntry`, `RelayTimeline`, `RelayChecklist` exist; header, run details, handoff are inline | TaskHeader, RunDetails, HandoffCard new |
| Crew: RunCard, MemberGroup, MemberCard, AddMember, PastRunsList/Row/View (§3e) | nothing | all new (the Crew view is the landing when there is no run) |
| Compose: FormBand (§2) | `ComposeBand` exists | conform to the band (To · About · message · Queue message) |

Every new component: `app/src/lib/relay/<Name>.svelte` + `<Name>.spec.svelte.ts` fixture matrix per the verification
standard; utilities only; `data-*` hooks; callbacks out. Row and section components are candidates for `@sensei/ui`
once the dōjō consumes them (CONTRACT §5).

### 2b. States

Follow the scan/bootstrap pattern (`app/src/lib/stage.svelte.ts` `ReactiveStageContext<T>`: `items`, `add/update/
remove/set`, `apply(event)`), one sub-state per entity, one orchestrator, and put them in a shared package from the
start (#414, relay first):

```
packages/state/relay/
  lanes-state.svelte.ts     LanesState     extends ReactiveStageContext<RelayLane>      (presence, task, reviewing, step)
  tasks-state.svelte.ts     TasksState     extends ReactiveStageContext<RelayTaskRow>   (by run, by lane; completed today)
  runs-state.svelte.ts      RunsState      extends ReactiveStageContext<RelayRun>       (draft/planning/active/completed/archived)
  items-state.svelte.ts     ItemsState     extends ReactiveStageContext<RelayItem>      (triage · backlog · planned)
  decisions-state.svelte.ts DecisionsState extends ReactiveStageContext<RelayDecision>
  messages-state.svelte.ts  MessagesState  extends ReactiveStageContext<RelayPersonMessage>
  relay-state.svelte.ts     RelayState  — the composite: the six above + UI state
                            { surface, load, view, from, filter, info, compose, glyph, selected, add }
                            apply(event) dispatches by entity (the scan `#dispatch`), read(filter) hydrates with `set`
  relay-selectors.ts        pure: toStats, toNeedsYou, toWaitRows, toLaneRows, toCompletedRows, toQueueRows,
                            toRunCard, toMemberCards, toAddMember, toPastRuns, toTaskView, toToolbar, toFooter
  relay-source.ts           interface RelaySource { read(filter): Promise<RelayReads>; events(on): () => void }
  sources/sse.ts            the daemon adapter: GET /relay/now + /relay/queue + /relay/runs…, EventManager on /api/relay/events
  sources/realtime.ts       the dōjō adapter: supabase-js reads + postgres_changes → the same StateEvent (§4)
```

- `RelayState` is constructed with a `RelaySource`; the page never fetches. Specs drive `apply` with canned events
  (no DOM) and assert selectors on data.
- Task detail: `RelayTaskState` keeps its shape (view, steps in place, re-read on task change) and joins the package.
- The existing `RelayState` methods (`answer`, `send`, `addToBacklog`, `createSprint`) become source calls followed by
  an event-driven update, not a full re-read.

### 2c. The stream: what the daemon must emit, and what the state applies

Today's events carry ids only, so the UI can only re-read. The scan screen's events carry the row. Do the same for
relay: `data` is the entity row **as the `now` section already shapes it**, plus `op` for the timeline.

| entity | action | data | applied by |
|---|---|---|---|
| `lane` | add/update/remove | the `now.lanes[]` row (code, title, suggested_name, role, family, model, lease, presence, task, reviewing, current_step, steps_done/total) | LanesState |
| `task` | add/update/remove | the task row (id, run_id, lane_id, title, phase, state, round, state_changed_at, deps_status, needs_answer, waiting) + `op` | TasksState (+ RelayTaskState re-read when it is the open task) |
| `step` | add/update | `{task_id, step}` (exists) — also emitted by a checkpoint that carries `steps[]` | RelayTaskState, LanesState (step counters) |
| `run` | add/update/remove | the run row (id, crew_id, title, state, counts, started_at, completed_at) | RunsState |
| `item` | add/update/remove | the queue row (triage/backlog/planned) — new; `/relay/triage` and `/relay/backlog` emit today nothing | ItemsState |
| `decision` | add/update | the decision (question, options, recommended, task_id, blocks, answered) — today folded into `task`/`relay` ops | DecisionsState |
| `message` | add/update | the person message / nudge with receipts | MessagesState |
| `relay` | add | `{op}` for resources — unchanged | footer |

Rules: the stream takes `?crew_id=` (optional) so a window subscribes to its run; a full re-read happens only on
reconnect (the `EventManager` `onopen` after the first); every person write (answer, message, backlog, run start) is
confirmed by the event it causes, not by a re-read.

### 2d. Daemon gaps the flow needs (person-facing)

| Step of the run | Today | Needed |
|---|---|---|
| Triage the backlog | `/relay/triage` (manual item) has no client; `/relay/backlog` refuses tickets (501); no close/edit/reorder | client for triage; ticketed add; `DELETE /relay/items/{id}`, `PATCH` (position, title); events |
| Create a run with tasks | `POST /relay/runs` needs an active crew (or exactly one) and mints Task 0 at once | `draft` run without a crew; `POST /relay/runs/{id}/start` binds the crew (≥1 live lane) and mints Task 0; `cancel`; `GET /relay/runs`, `GET /relay/runs/{id}` (tasks by state, lanes) |
| Register the crew | a coordinator `register` mints the crew; the UI shows paste lines | AddMember per CONTRACT §1h: role segmented, assistants/editors seen on this machine (catalogue, `relay-crew-daemon.md`), the terminal command or paste line, "Listening"; a person route to close a crew |
| Start → planner kicks off | Task 0 is claimable by any lane; its ask reaches the person through `/relay/now.decisions` | unchanged; the run's `planning` state and Task 0's lane show on the RunCard |
| Plan → tasks | unpack on approval (shipped) | run detail lists them; `run/activated` carries counts |
| Assistants pick up tasks | `claim_next` (shipped) | lane rows re-group by run; `now` lists only the run's live lanes (`unregistered_at is null`) with presence `active|idle|not_responding` |
| Step progress | steps only on `GET /relay/tasks/{crew}/{task}`; checkpoint emits no `step` | lanes carry `current_step` + counters; checkpoint emits `step`; task stream applies in place |
| Codename | `suggested_name` stored, never shown | broker mints one when absent (the mockup's colour-star words); shown wherever a lane is named |
| Nudge | a nudge is a message with `props.nudge`; the person route has no `nudge` | `POST /relay/messages` takes `nudge: true` (+ reason) |

## 3. The e2e run, controlled from the app

One Tauri journey (`app/e2e/tests/relay-run.spec.ts`) against a fresh `sensei_e2e` (`SENSEI_DDL_DIR`, `reset-e2e-db`),
with a **fake crew driver** (`app/e2e/lib/relay-crew.ts`, the TypeScript twin of `scripts/relay-queue-smoke.sh`) that
plays the lanes over the lane HTTP API while the page is watched for each change **without a refresh**:

1. Queue: add two manual items and one from a seeded ticket; add to backlog; select; **Create run** → a draft run on
   the Crew view.
2. Crew: the screen shows "Start a run: set up your crew"; the driver registers a coordinator and two contributors
   with suggested names → MemberCards appear live with codenames; **Start** → Task 0 ready, RunCard says planning.
3. Planner: the driver claims Task 0, asks the person a decision → Needs you shows it; the test answers → the lane
   receives it; the driver reports the plan (```json block) → floats; a peer claims and approves → the plan unpacks →
   RunCard counts flip to active with N tasks.
4. Work: lanes claim tasks → lane rows show the task; checkpoints with steps → step counters and the task view's
   checklist move; a report → "Needs a reviewer"; a review approves → the gate task runs (fake runner in e2e) → done →
   Completed today.
5. Completion: the last task done → run completed → Past runs; the Now view empties into "no run".

Every step pins: the DOM change arrives from the stream (no `reload`, no retry click), the stat counters, the
toolbar badges, and one screenshot per state in both themes (true-visual backstop, thin).

## 4. The dōjō: the same screen over Supabase realtime

What exists: the `relay` schema is deployed to the dōjō database (design.yaml dōjō scope) but nothing writes it there,
its RLS keys on the machine-local `relay.daemon_user` id that no dōjō JWT matches, it is not in the realtime
publication, and nothing in `dojo/src` reads it. The dōjō's relay screens read `dojo.relay_{sessions,segments,inbox}`
from the older run/TodoWrite pipeline; `relay-realtime.ts` subscribes to `postgres_changes` on sessions and inbox and
calls `invalidateAll()`. The daemon talks to the dōjō only over HTTPS with the persona's token (no realtime).

The plan, in order (separate epic, after the app run is proven):

1. **Identity bridge.** Publish under the person's membership: the publisher resolves `relay.daemon_user` → the dōjō
   principal/membership the daemon already authenticates as. The dōjō copy of `relay.*` gains `membership_id` and RLS
   `dojo.owns_membership(membership_id)` (the `dojo.relay_*` precedent), select for `authenticated`, writes only by
   the Worker.
2. **Publisher.** A `relay_sync` task (the `dojo_sync`/`publish_segments` shape) upserts changed rows — crews, lanes,
   runs, items, tasks (with steps and checkpoint.current_step), decisions, messages, action **summaries** — through
   new Worker endpoints `/v1/t/{origin}/{org}/relay/queue/*`. Never `report_md` unless the crew's share setting allows
   (lanes.md §8); the leak guard runs on every text field. Per-target sent marks give the dōjō footer its freshness.
3. **Publication.** Add the dōjō's `relay.*` tables to `supabase_realtime` (`database/apply/after/realtime_publication.sql`).
4. **Screens.** `dojo/src/routes/(app)/you/(inbox)/relay` renders the same components with `@sensei/state`'s
   `RelayState` and the **realtime adapter**: `postgres_changes` `{eventType: INSERT|UPDATE|DELETE, new, old}` →
   `{action: add|update|remove, entity: <table>, data: new ?? old}` — the same `StateEvent` the SSE adapter yields, so
   the sub-states and selectors do not change. View ids `dojo.you.relay`, `dojo.you.relay.task`; view-states in
   `dojo/src/lib/view-states.svelte.ts` (shell contract §4).
5. **Answers back.** A dōjō answer or message writes a `dojo.relay_inbox` row (`kind: decision | chat | nudge`,
   payload `{decision_id | to, option, note, text}`, `surface: 'dojo'`). The daemon's existing inbox cursor
   (`poll_inbox`, used by the tool gate today) consumes them into the same code path as `/relay/answers` and
   `/relay/messages`; the publisher echoes the answered state. A daemon-side realtime subscription can replace the
   poll later without changing the contract.
6. **Verify.** The local recipe (Supabase + wrangler + magic link) with the fake crew driver from §3 on the desktop
   side; the dōjō page must move with no refresh at each step.

## 5. Triage — what to attack first

Order by dependency, then by what the e2e run needs:

1. **#397** bootstrap upgrade deploy drops live views — the briefs gate every live deploy and inspect run on it;
   independent of the screen; first.
2. **Daemon events + routes for the flow** (§2c, §2d) — the screen cannot be built on re-reads; the e2e cannot run
   without draft/start/cancel, item ops, codenames, live-lane filtering, step events.
3. **Relay on the shell** (§1 shell, toolbar, bands) — small, visible, closes #380 #383 #384 #385 #388 #394 and
   supersedes `lane/relay-screen-conformance` (harvest its `ShellScreen.pinned` prop and `relay-conformance.spec.ts`;
   the branch conflicts with develop since the queue cutover and is retired).
4. **`@sensei/state` relay states + SSE adapter** (§2b) — #414's first cut; closes #378 once the e2e pins it.
5. **Components by view**, each fixture-tested then integrated: Now (#379 #395), Queue, Crew (#382, the UI halves of
   #396 #393), Task (#380 #381 #388).
6. **The e2e run** (§3) — the proof; closes #387.
7. **Structured response forms** #389 — after the live chain, as the briefs order.
8. **Dōjō** (§4) — its own epic.
9. **Shell across the app and the dōjō** — #293 continues: migrate the ~40 legacy `Screen` pages to `ShellScreen`,
   adopt it in the dōjō (`AppShell` + `SectionHead` screens), one view-states store per surface.
10. **Icons and packages** — #406 (collections + vocabulary), #408 (inline svg copies), #407/#409/#410 (shims,
    AssistantCard): after the relay wave; the relay toolbar set and marks join `@sensei/glyphs`' vocabulary in step 3.

Closed by the rebuild rather than fixed in place: #386 (lanes table — replaced by LaneRow).

## 6. Issue map (filed 2026-11-12)

| Order | Issue | Scope | Closes |
|---|---|---|---|
| 1 | #397 | bootstrap upgrade deploy drops live views (unblocker, pre-existing) | — |
| 2 | #418 | daemon: events carry the row; step events on checkpoint; lanes with current step; stream crew filter | — |
| 2 | #419 | daemon: person routes for the run (draft/start/cancel, run list/detail, crew close, item ops, ticketed backlog, nudge, codename minting, live-lane now + presence) | daemon halves of #396 #393 later |
| 3 | #420 | app: Relay on the D87 shell (scroll fix in ShellScreen, toolbar icon views + badges, stats, schema filters, info band) | #380 #383 #384 #385 #388 #394; supersedes #386 + `lane/relay-screen-conformance` |
| 4 | #421 | `@sensei/state`: relay states + composite + selectors + SSE adapter (realtime seam) | #414 first cut; #378 with #426 |
| 5 | #422 | Now view components | #379 #395 |
| 5 | #423 | Queue view | — |
| 5 | #424 | Crew view | #382; UI halves of #396 #393 |
| 5 | #425 | Task view components, live steps | #381, task half of #388 |
| 6 | #426 | the e2e run from the app with a fake crew, no-refresh pins | #387 |
| 7 | #389 | structured response forms (after the live chain) | — |
| 8 | #427 | epic: the dōjō over Supabase realtime | — |
| 9 | #428 | epic: ShellScreen across the app and the dōjō (continues #293) | — |
| 10 | #406 #408 #407 #409 #410 | icons and shared-package hygiene, after the relay wave | — |
