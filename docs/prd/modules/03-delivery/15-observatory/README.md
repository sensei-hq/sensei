# Observatory · M15

**Plane:** delivery · **Status:** [roadmap/03-delivery.md](../../../../roadmap/03-delivery.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)),
team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

The Observatory is where the user lives day to day. Every surface observes the work,
forms **one finding**, and offers **one action** (Apply / Review / Dismiss). Today
(`/`) surfaces the single thing that needs a decision — a hero *koan* with its
provenance — plus at most three supporting insights, what sensei adopted on the
user's behalf, a first-turn-resolution (FTR) chip, and the most recent sessions.
Early on it says *"Still listening…"* and offers **no action at all**; once there is
enough signal it switches to a mature mood. The user feels this as *one thing today,
with its receipt* — not a dashboard to maintain.

> **Status honesty.** Today, the projects index, insights, sessions, memories and
> logs are real. Two surfaces are **mock-only / roadmap**: **Consolidation**
> (`/consolidation`) and **Insights Reasoning** (`/insights/{id}/reasoning`) — neither
> has a route in `routes.rs`. And the doc-declared `sensei.ftr_daily` table is absent
> (FTR rolls up from `sensei.metrics`); the doc's `sensei.reasoning_traces` is really
> `inference.reasoning_traces`, and `sensei.dojo_downstream_inbox` is really
> `sensei.dojo_inbox`.

## In scope / out of scope

- In: **Today** (`/`) — one hero koan + ≤3 supporting insights + the adopted lane +
  FTR chip/trend + ≤5 recent sessions, with the early/mature mood switch; **Projects**
  (`/projects`) — the index of projects with health, filter chips
  (All/Active/Dormant/Archived), grid/list toggle, search, open → project window;
  **Insights** (`/insights`) — tri-column Now/Soon/Settled server bucketing with
  one-decision-one-default triage; **Memories** (`/memories`) — human curation of the
  memory pool (What/Why/How/Where anatomy + a 7-day usage strip + the promotion
  ladder); **Sessions** (`/sessions`) — a digest with range chips (7d/30d/90d/all) and
  four chart variants (trend/stream/constellation/bands) plus the `pulse` mini-cycler;
  **Collective/Sharing** (`/dojo/sharing`), **Dōjō connections** (`/dojo/connections`),
  **Upgrades** (`/upgrades`) — opt-in give-back and downstream accept; **Logs**
  (`/logs`) — operator diagnostics.
- Out: **Consolidation** (`/consolidation`) and **Insights Reasoning**
  (`/insights/{id}/reasoning`) — routes are not in `routes.rs`; roadmap/mockup only
  (see gaps). Any knowledge-base browsing UX — Insights is action-first, not a KB.
  The deep project window (M16), Atlas (M17), Instruments (M18) and Impact (M19) are
  peer surfaces with their own specs.

## Surfaces

- **API — Today** — `GET /api/observatory/today` · `/ftr` · `/ftr-daily` ·
  `/tool-usage` · `/tool-signals` · `/tool-insights` · `/model-effectiveness`.
- **API — Insights** — `GET /api/insights`; Apply → `POST
  /api/projects/{project_id}/recommendations/{rec_id}/accept`; Dismiss → `…/reject`.
- **API — Projects** — `GET /api/projects` (index) · `/{id}/ftr` · `/quality-signals`
  · `/maturity` · `/ftr-daily` · `/teachings` · `/hotspots`.
- **API — Sessions** — `GET /api/sessions` · `/{id}` · `/{id}/tool-timeline` ·
  `/{id}/replay`.
- **API — Memories** — `GET|POST /api/knowledge/memories`; `/{id}` + `/promote`,
  `/generalise`, `/archive`, `/reinforce`, `/challenge`, `/dismiss`, `/merge`;
  `GET /api/projects/{id}/memories`.
- **API — Collective / Upgrades / Logs** — `GET|PUT /api/preferences/collective`;
  `GET|POST /api/dojo/memberships`; `GET /api/dojo/sync-state`; `GET /api/upgrades`
  (+ `/apply`, `/mute`, `/pin`); `GET|POST /api/logs`; `GET /api/tasks/scheduled` +
  `PATCH /api/tasks/scheduled/{id}`.
- **App** — `app/src/routes/(observatory)/{today,projects,insights,sessions,memories,
  logs,dojo,upgrades}/+page.svelte`.

## Data invariants

- **Lead.** Maturity is a daemon decision, reused. `dataMaturity = maturity_signal(
  watched, has_insights, MATURITY_TARGET)` aggregated across active projects;
  `mature ⇔ watched ≥ target AND has_insights`; never a UI heuristic or a hardcoded
  count (`crates/senseid/src/maturity.rs`).
- **Lead.** Early is a feature, not an empty state. With insufficient signal the
  screen renders "Still listening…" and the action button is **absent**, not disabled.
- **Lead.** FTR types are distinct. `recentSessions[].ftr` is a per-session boolean
  (pass/fail dot), **not** the 0–100 header metric; `corrections` is an integer; the
  header chip comes from `/api/observatory/ftr`, the trend from `ftr14d − ftr14dPrev`.
- **Lead.** The adopted lane is in-force memories. Sourced from `list_active_memories`
  (`status='active'`, `strength >= 1.0`), strongest first, ≤5 — never
  `inference.detected_patterns`.
- **Lead.** Provenance resolves. Any session id in `hero.source` resolves to a real
  `activity.sessions` row (client-session-id → `activity.sessions.client_session_id`).
- **Lead.** Bucketing is one source. A rec lives in exactly one of Now/Soon/Settled by
  server rule (rec `urgency`; memory violation → Now always; pattern `lifecycle`); the
  client trusts the label.
- **Lead.** No synthetic rows. Sessions lists and totals contain only real captured
  sessions.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Hero koan | the day's one teaching (≤60-char title + 2–3 sentences) | `"The AI does not know your auth."` |
| Koan source | clickable provenance | `from s-2891 · s-2889 · s-2886` |
| Insight card (≤3) | a supporting observation | `繰 · Pattern recurring · Cache invalidation missed again` |
| Adopted lane (≤5) | what sensei did on the user's behalf | `Canvas smoothing pattern → rule · lumen-studio` |
| FTR chip | 14-day rolling first-turn resolution | `78%, up 6 pts` |
| Triage stats | Now · Soon · Settled counts | `{now} · {soon} · {settled}` |
| Project status dot | warning if `warn`; success if `!warn && ftr14d ≥ 0.8`; else ink-mute | `warning` |
| Session row | time · project · duration · corrections · ftr · outcome | `09:14 · sensei · 42m · 0 corrections · ✓ FTR · completed` |

## Done gate

- Loading `/` shows **either** early **or** mature based on `dataMaturity` (never
  both); the koan is specific and the early action is absent.
- Every `hero.source` id resolves via `GET /api/sessions/{id}`; the adopted lane shows
  real rows or the honest early string.
- `/projects` chips sum to All; dormant/archived cards show repos/libs/last-session;
  the grid/list preference persists (`localStorage` key `sensei:projects:view`).
- `/insights` renders real Now/Soon/Settled bucketing; every card has one highlighted
  recommended verb; Accept schedules `MeasureVerdicts`.
- `/sessions` range chips reduce the set; a row click calls
  `goto('/instruments?tab=replay&session={id}')`; every FTR badge agrees with the
  underlying column.

```bash
curl -s http://localhost:7744/api/observatory/today | jq '{maturity, koan: .hero.title, source: .hero.source}'
curl -s http://localhost:7744/api/insights | jq '{now: (.now|length), soon: (.soon|length), settled: (.settled|length)}'
```

## Wrong gate

- **Both early and mature copy visible** — the maturity switch collapsed.
- **Koan is generic** ("Nothing to report today") — should be the early state or absent.
- **The adopted lane is populated with items not in `memories`, or empty while in-force
  rows exist** — a read-path bug either way.
- **A rec appears in both Now and Soon** — the server bucketing rules collided.
- **`FTR ✓` badge while Replay says "no tool calls"** — a session-id vs client-session-id
  join regression (a known repeat offender).
- **Chip counts don't sum to All** / a card can't be opened (Tauri capability regression).
- **Usage strip always `0 · 0 · 0`** — telemetry tables not written, or reading a static
  fixture.

## Scenarios

- [`scenarios/observatory-today.feature`](scenarios/observatory-today.feature) — hero,
  adopted lane, mood switch (pinned).
- [`scenarios/observatory-insights.feature`](scenarios/observatory-insights.feature) —
  Now/Soon/Settled bucketing and narration copy (pinned).
- [`scenarios/observatory-surfaces.feature`](scenarios/observatory-surfaces.feature) —
  memories, sessions, collective, Dōjō, logs, scheduled tasks (pinned + `NO TEST`).

## Open gaps

- [C-4](../../../../reorg.md) — chat guidance (roadmap only) → M15.
- [G8](../../../../reorg.md) — net-new surfaces; residual text only → M15/M16.
- [C-8](../../../../reorg.md) — impact-alert full-screen (fold into Impact, M19) — a
  sibling surface that also touches M15.
- [D-4 / D-5](../../../../reorg.md) (T2) — seven dojo2 sections are fixture-only and the
  cutover is human-gated; these undercut the Collective / Dōjō screens.
- [D-11](../../../../roadmap/06-gaps.md) (P2) — the website presents unbuilt work as
  shipped; the honesty gate for Observatory feature claims.
- **Consolidation + Insights Reasoning** — no route and no test; the clearest unbuilt
  surfaces here, not yet registered (Phase 3).

## Related

- [features/03-observatory.md](../../../../features/03-observatory.md) — the UX half
  (legacy depth; stays live — spans Delivery and Team).
- [spec/screen/observatory-*](../../../../spec/screen/) — today, projects, sessions,
  insights, memories, collective, dojo-connections, upgrades, logs, consolidation;
  plus `insights-reasoning.md`.
- [spec/pipeline/insights](../../../../spec/pipeline/insights.md) — narration-cache and
  bucketing copy.
- [design/mockup-brief](../../../../design/mockup-brief.md).
- [prd/02-objectives.md §Observatory](../../../02-objectives.md) · [prd/01-vision.md](../../../01-vision.md#the-core-loop).
- Crates: `crates/senseid/src/{observatory_home.rs, maturity.rs, insights.rs,
  collective/, api/handlers/observatory.rs, api/handlers/knowledge.rs,
  api/handlers/sessions.rs, api/handlers/logs.rs, api/handlers/upgrades.rs,
  api/handlers/dojo.rs}`.
