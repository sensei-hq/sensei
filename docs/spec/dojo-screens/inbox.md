# Inbox — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.

- Route: `/you` (list rail = `(app)/you/(inbox)/+layout.svelte` + `+layout.ts`; detail = `/you/runs/[run_id]` = `(inbox)/runs/[run_id]/+page.svelte` + `+page.ts`; `/you` index `+page.svelte` = right-panel placeholder)
- Mockup: dojo2-app.jsx `ScrInbox` (L330) + `RunDetail` (L462) — board "1 · Inbox — every in-flight session, needs-you first"
- Access axis: **user/membership-primary** — canonical `entity-access-model.md` §3 row 1: "Personal inbox / runs / asks / plan (`/you`) … `owns_membership(membership_id)` … spans ALL the user's memberships/tenants". The current single-tenant read is a bug against canon (see Gap G1).
- Status: **PARTIAL** — the two-panel master-detail is built and reads REAL `/v1` relay data, but (a) it filters ONE tenant not user-wide (canon violation), (b) the list rail has no plan-pips / repo / realtime, (c) the ask card is the old `RelayGateCard`, not the mockup `AskCard`.

> **Do not duplicate — align + fill gaps.** Two pending plans already own most of this screen:
> `docs/plan/2026-07-27-dojo-inbox-mockup-fidelity.md` (element→data for the EXISTING single-tenant
> flow, no model change) and `docs/design/2026-07-27-dojo-relay-rls-membership-function.md`
> (the membership-derived RLS + user-wide read that makes it canon-correct). This spec references
> both, does not restate their tables, and adds only what they leave open (access-axis correction,
> AskCard fidelity, gate↔task linkage, conversation/pause/observatory, list-rail realtime).

## Elements → data (contract)

### A. List rail — `ScrInbox` (L330) → `InboxRow`/`K2InboxRow`
Live: `(inbox)/+layout.ts` → `guardTenantScope(tenantKey…)` → `listRuns(tk)` + `listGates(tk)` → `toKitInbox()` → `ScrInbox.svelte`.

| Element | Mockup field | Source (loader/API/table.field) | Status | Realtime? |
|---|---|---|---|---|
| SectionHead eyebrow/title | `You · in flight` / `Inbox` | static | have | — |
| header count | `D2.runs.length` | `inbox.length` (`toKitInbox` rows) | have | — |
| `{n} need you` (right) | `needTotal` | `Σ row.needs` from `listGates` (non-chat/nudge kinds) | have | via list refresh |
| filter subtabs | `K2_FILTERS` needs/running/done/all | `FILTERS` (needs/running/**finished**/all) + `filterInbox()` | have | — |
| row status dot | tone by state | `relay_sessions.status` → `inboxStatus()` | have | via realtime (G4) |
| row **repo** (top-left) | `lumen-auth` | run's project — **NOT persisted in dōjō** (`relay_sessions` has no project col; daemon sends `project_slug`, POST discards it). See fidelity-plan §4. | plumb | — |
| row age (top-right) | `2m` | `relay_sessions.last_event_at` → `age()` | have | via realtime |
| row title (2-line) | task title | `goal ?? title` (`toKitRun`); fidelity-plan A4: kill `project: current_feature ?? title` dup | bind | — |
| row why-line | `3 need you` / `no heartbeat` | gates count / `status==stalled` (`attention`) | have | via realtime |
| row **plan-pips** | per-phase colored strip | `relay_segments` phase states → `segmentsToPlan` → `PlanPips`. **Inbox load does NOT fetch segments** (only run-detail does) → rows have no pips. fidelity-plan A6/A.i: batch `getSegments(runId)` per listed run. | plumb | via realtime |
| row done/total | `5/12` | `progress_done`/`progress_total` | have | via realtime |
| rail container | pad, sticky, hairline right border, flush list | `+layout.svelte` grid `minmax(340px,400px)_minmax(0,1fr)`, `<aside>` | have | — |
| sort order | needs → stalled/blocked → running → done | `toKitInbox` `rank` sort | have | — |
| empty state | `空 Nothing here` | `EmptyState` (empty vs no-match variants) | have | — |
| error banner | — | `data.error` (guard/service failure) → `Banner tone=warning` | have | — |

### B. Detail — `RunDetail` (L462) → `/you/runs/[run_id]/+page.svelte`
Live: `runs/[run_id]/+page.ts` → `getSegments(tk,runId)` + `listRuns` + `listGates` → `segmentsToPlan/Activity`, gates filtered to run.

| Element | Mockup field | Source | Status | Realtime? |
|---|---|---|---|---|
| back header (`<md`) | `Inbox` | `back()`→`youHref()`; hidden `md:` | have | — |
| header glyph | `観` lg, accent-when-asks | `KanjiToken`, `headTone` = accent iff `gates>0` | have | subscribeRelay |
| eyebrow | `Session · S-2891` | `run.run_id.slice(0,8)` | have | — |
| title | `run.task` | `relay_sessions.title` | have | subscribeRelay |
| meta line | `repo · assistant · elapsed · edits · last activity` | built from `started_at`+`last_event_at` only; **repo (§4 gap), assistant, edits, model dropped** (fidelity-plan B4: bind model from `relay_segments.model`, drop edits, repo pending §4) | bind | subscribeRelay |
| status chip | `running` | `run.status` → `RelayStatusBadge` | have | subscribeRelay |
| goal line | — | `relay_sessions.goal` | have | subscribeRelay |
| progress "Phase X of Y · name" + bar | `pr.stage/stages/stageName` + `pct` | `planProgress(segmentsToPlan())`; bar tone accent-when-asks | have | subscribeRelay |
| tabs Needs you `{n}` / Plan | `openItems.length` | `data.gates.length` badge; per-run pick map | have | subscribeRelay |
| **ask card** | `AskCard` (see C) | `relay_inbox` gate → `RelayGateCard` (needs restyle, see C) | bind | subscribeRelay |
| plan outline + Goal line | `K2PlanOutline` | `segmentsToPlan(segments)` → `PlanOutline` | have | subscribeRelay |
| selected-task detail block | `state · agent · model · waits-on · spec_ref · summary` | `relay_segments.{state,agent,model,spec_ref,summary}` (`deps=[]`, not federated). fidelity-plan B9: render this block (currently `PlanOutline` only, no detail panel) | plumb | subscribeRelay |
| Activity feed | `K2RunActivity`, `run.feed` | `segmentsToActivity(segments)` (submitted segments, newest first) | have | subscribeRelay |
| Conversation thread | `K2ChatThread` / empty | **no chat federation** — static "Nothing said" placeholder | plumb | — |
| reply-to-sensei input | `reply to sensei…` | not wired (mockup is a static input); would POST `sendNudge` | plumb | — |
| Pause run btn | `Pause run` | not wired; MCP `pause_run`/daemon exists, no dōjō endpoint | plumb | — |
| Open in Observatory btn | `Open in Observatory` | not wired (desktop-app deep link) | plumb | — |
| empty (no asks) | `静 Nothing waits on you` / done note | `EmptyState` | have | — |

### C. Ask card — `AskCard` (L394) vs shipped `RelayGateCard`
Mockup ask = `D2.asks` filtered by run; dōjō ask = `relay_inbox` row (`RelayGate`).

| Element | Mockup field | Source | Status | Realtime? |
|---|---|---|---|---|
| kind glyph+label | `K2_ASK[kind]` 認/岐/阻/問 approval/decision/recovery/clarification | `relay_inbox.kind` (`approval`/`decision`/`chat`/`nudge`/`stall`) — mockup's 4 kinds ≠ wire kinds; needs a kind→glyph map | plumb | — |
| blocking chip | `severity==="blocking"` | `gate_severity`/`payload.category` (`toKitGate` derives blocking from `payload.category!=null`) | bind | — |
| age | `it.age` | `relay_inbox.created_at` → `relativeAge` | have | — |
| question | `it.question` | `payload.prompt` | have | — |
| context | `it.context` | `payload.context`/`evidence` | bind | — |
| **holds {task} · {taskTitle} →** | `it.task`/`taskTitle` | `relay_inbox.segment_id` → segment title (linkage exists, NOT rendered; `onShowInPlan` cross-tab jump also absent) | plumb | — |
| numbered options 1..N | `it.options` fill shared input | `payload.options` (`RelayGateCard` renders as separate buttons, not numbered-into-input) | bind | — |
| free-text `type 1–N…` + Send | typed answer | `RelayGateCard` free-text/approve-decline + `replyToGate`/offline-queue (keep the real send) | bind | — |
| hold-note line | "run holds until you answer" | static copy | plumb | — |
| answered verdict card | `了` + verdict | `invalidateAll()` re-fetch (no persisted per-ask verdict echo) | bind | subscribeRelay |

## APIs / loaders
- **list `load()`** (`(inbox)/+layout.ts`): `listRuns(tenantKey)` → `GET /v1/t/{tenant}/relay/session` (`relay_sessions`, COLS, ordered `started_at desc`); `listGates(tenantKey)` → `GET …/relay/gates` (`relay_inbox` pending). Behind `guardTenantScope` (membership-less → honest-empty). Maps via `toKitInbox`.
- **detail `load()`** (`runs/[run_id]/+page.ts`): `getSegments` → `GET …/relay/segments?run_id` (`relay_segments`); + `listRuns` (find run) + `listGates` (filter to run). `noMembership` from guard.
- **mutations**: `replyToGate` → `POST …/relay/reply` (answer a gate; run-detail routes through P4.5 offline queue). Available but unused here: `sendNudge` → `POST …/relay/nudge` (reply-to-sensei / pause). `submitReview` → `POST …/relay/review`.
- **realtime**: detail page mounts `subscribeRelay({ topic: relay:run:${runId}, onChange: invalidateAll })` (`relay-realtime.ts`, RLS-scoped `postgres_changes`, no column filter). **List rail does NOT subscribe** → new/updated runs don't stream into the rail (G4).

## Components & state (three-layer, per `sensei:ui-state-pattern`)

**Domain types** (UI-shaped; the Load layer maps wire → these):
```ts
type RelaySession = { id; runId; project: string|null; title; goal: string|null; status;
  done; total; phase: string|null; lastEventAt: string|null; needs; attention:
  'stalled'|'blocked'|'failed'|null; plan: SegmentGraph }
type SegmentGraph = { phases: { id; title; state; tasks:
  { id; title; agent?; model?; specRef?; summary?; state }[] }[] }
type Ask = { id; kind; glyph; label; blocking; prompt; context?; options: string[];
  segmentId?; taskTitle?; createdAt }
```

**State** — `relay-inbox-state.svelte.ts` → `relayInboxState`
- data: `sessions: RelaySession[]`, `selectedId`, `filter`
- `$derived`: `shown` (filter + rank), `needsCount`, `selected`
- methods: `load(sessions)`, `select(id)`, `setFilter(f)`, `patch(session)` (realtime upsert/remove),
  `subscribe()` (owns-membership channel → `patch`)
- detail sub-state `relay-run-state.svelte.ts` → `relayRunState`: the selected `RelaySession` + its
  `plan`/`asks`; `load(runId)`, `patch`, `subscribe(runId)`

**Load** — `relay-inbox.ts` → `loadRelayInbox()` (+ `loadRelayRun(runId)`)
- mock-first: hand-crafted `RelaySession[]` exercising needs / running / finished / stalled /
  empty / error (mirrors the mockup boards) → build UI + tests to fidelity NOW
- real (later, body-swap only): **user-wide** read across the user's memberships (not one tenant),
  mapping `relay_sessions` + `relay_inbox` + `relay_segments` → `RelaySession[]`

**Components** (pure, semantic, own styles + `md:` — fidelity verified per component)
- `RelayList` — left rail: `SectionHead` + filter tabs + `RelayCard[]` from `relayInboxState.shown`;
  `onselect → state.select`. (replaces `ScrInbox` wrapper)
- `RelayCard` — one `RelaySession`: dot · project · age · 2-line title · why-line ·
  `PlanPips(session.plan)` · done/total; `selected` style. **Mockup-match + `md:` live here.**
  (replaces `InboxRow`/`K2InboxRow`)
- `RunDetail` — right panel: header (session id · meta · status) · progress · `SubTabs` ·
  `AskCard[]` (needs) / `PlanOutline` + task-detail (plan) · `RunActivity` + `Conversation`. Reads
  `relayRunState`. Adapts `md:`.
- `PlanPips` · `PlanOutline` · `SegmentGraph` · `AskCard` (semantic — replaces `RelayGateCard`).
- Shell: `(inbox)/+layout.svelte` composes `RelayList` + `{@render children()}`(=`RunDetail`);
  `+layout.ts`/`+page.ts` are the Load wiring → `relayInboxState.load` / `relayRunState.load`.

**Copy** (paraglide/inlang, `m.<key>()` from `$lib/paraglide/messages` — no inline literals): the
inbox strings live in `messages/en.json`, e.g. `m.inbox_title()` "Inbox", `m.inbox_eyebrow()`
"You · in flight", `m.inbox_needs_you({n})`, filter labels `m.filter_needs/running/finished/all()`,
`m.ask_hold_note()` "The run holds here until you answer.", empty/error copy. Kanji (観/要/静) stay
`KanjiToken` brand marks, not messages. Sensei voice enforced in the catalog.

**Realtime = State**: `subscribe()` patches the arrays → Svelte re-renders (targeted `patch`, not
`invalidateAll`). **Test seams:** state methods (no DOM); `RelayCard`/`RunDetail` with a mock
`RelaySession` prop (fidelity); Load mock → shape.

## Interactions & states
- **Master-detail**: `md:` two-column grid; `<md` the list is a full pane, a row navigates to `/you/runs/[id]` which pushes the detail over it (back returns to `/you`). On `md+`, `+layout.svelte` `$effect` auto-opens `data.inbox[0]` so the right panel is never blank.
- **Filters**: `needs` (default) / running / finished / all via `filterInbox`. `needs` = `needs>0 || attention!=null`.
- **Detail tabs**: per-run pick map (`picks[runId]`) — opens on `needs` if gates else `plan`; a realtime refresh never clobbers the user's tab (pure `$derived`, no reset effect).
- **Empty/error/loading**: guard → honest-empty inbox; service/404 → empty + `data.error` banner (rail always renders). `noMembership` on detail → `DojoJoinEmpty`.
- **Responsive**: mobile mockup collapses list↔detail; matched by the `hidden md:block` pane toggle keyed off `activeRunId`.

## Gap / to-do (vs mockup), ranked
1. **Access axis (canon-blocking)** — list reads one tenant (`listRuns(tenantKey)`); canon says user-wide across ALL memberships. Fix per RLS design: `owns_membership(membership_id)` RLS + Worker read stops hard-filtering `tenant_id` for the personal surface. Also drops `relay_sessions.user_id` (re-ownership bug).
2. **Ask card fidelity** — restyle `RelayGateCard` → mockup `AskCard` (fidelity-plan B8): kanji header, numbered-options-into-input, free-text + Send-answer, "run holds until you answer". Add wire-kind→glyph map (4 mockup kinds vs 5 wire kinds).
3. **List-rail plan-pips** — inbox load must batch `getSegments(runId)` per run → `segmentsToPlan` → `PlanPips` (fidelity-plan A6). Title-dedup (A4).
4. **Repo / project name** — persist `project_slug` (daemon already sends it) as one nullable col on `relay_sessions`; return in GET; map to `KitRun.project` (fidelity-plan §4). Feeds row A2 + meta B4.
5. **Selected-task detail block** in the Plan tab (fidelity-plan B9). Model into meta (B4).
6. **Gate↔task linkage** — render "holds {task} →" from `relay_inbox.segment_id`; wire cross-tab `onShowInPlan`.
7. **List-rail realtime** — subscribe the rail so runs stream in (currently only the open detail refreshes).
8. **Conversation / reply / Pause run / Observatory** — no chat federation; buttons are dead. Lowest priority (mockup's own inputs are static).

## Open questions (for Jerry)
1. Ship the canon access-axis correction (user-wide read + drop `user_id`, per the RLS design) as part of this screen, or land mockup-fidelity first (single-tenant) and refactor after? The fidelity plan explicitly scopes NO model change; the RLS design is the model change. [JT] Yes and the tenant_id as well as per our discussion
2. Repo/project name — persist `project_slug` on `relay_sessions` now (one nullable col), or leave the row/meta repo-less until a project registry exists? (Confidentiality note: `project_slug` crossing the boundary must obey universal source-dereference, §5 — is the slug already dereferenced on the daemon publish path?) [JT] project name is fine, projects are user + tenant scoped so the RLS restriction should prevent corss tenant or use projects from appearing.
3. The 4 mockup ask kinds (approval/choice/recovery/clarification) vs 5 wire kinds (approval/decision/chat/nudge/stall) — confirm the glyph/label mapping (esp. stall→recovery 阻, nudge→? ). [JT] Use suitable solar icons instead of kanji. Users are used to simple icons. mockup ask kinds are more readable. I would suggest [approve|choose|resume|chat] stalled is state -> resume is action. Did i use verbs? consistency is key. Status and action are going to be different. 
4. Conversation tab: is human→sensei chat in scope for this screen (wire `sendNudge`), or does it stay the honest-empty placeholder until a chat channel is federated? [jT] Dojo is primarily a away from keys interface. So simple text box, one line response is what `chat` indicates. It is a short clarification or reframing of the response when choices are unclear or user wants to provide a different direction. We could use rokkit schema based forms for the bulleted options and optional freeform answer. so instead of writing 1,2, 3 in response user can select an option and behind the scenes the response is sent.
5. "Open in Observatory" — deep-link into the desktop app, or drop the button on web? [jt] deep link.
