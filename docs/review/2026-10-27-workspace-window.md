# Workspace window — functional review, 2026-10-27

A read-only, adversarial review of every screen in the desktop app's **workspace window**
(`app/src/routes/(workspace)/workspace/[id]/…`), written so you can work through it screen by screen.
Every claim cites a `file:line` or a live `curl` against the running daemon (`http://127.0.0.1:7744`, GET only),
taken against the **sensei** workspace (`ff1ccea2-540c-4377-a7ec-e180bc29aae4`). No code, config, database or
git state was changed: nothing was POSTed, and no write button was clicked.

**How to read each section.** *Purpose* is what the screen is for. *Shows* is what it actually renders.
*API* is every daemon call, with its live status, time and shape check. *Mockup/spec diff* compares the screen
with `docs/mockups/Sensei/assets/screens/*.png`, `docs/prd/modules/**` and `docs/spec/screen/*`. *Checklist*
is what to try by hand, written as `[ ] Do X — expect Y; code suggests Z`, each tagged **bug**, **gap**
(vs mockup or spec), **polish** or **question**.

**Caveat on timings.** The machine was heavily loaded during the review: load average 83–197, with other
agents and builds running. Timings were repeated where they mattered, and each section gives both numbers
where they differ. Re-time anything marked slow on a quiet machine before acting on the number. The *shape*
findings (a query that awaits an unbounded read, a 30-day window labelled 7 days) do not depend on load.
One re-measure at load 83: `GET /api/workspaces/{id}/mcp-tool-stats` took **39.8 s**, and `/libraries` 0.26 s
(it was 8.6 s cold).

**Related.** `docs/baseline/2026-10-27-app.md` (the crawl: the Layers freeze, the metrics ResizeObserver
warning) · `docs/roadmap/08-issue-burndown.md` (open queue; #293 screen structure, rokkit first).

**Order.** The shell first, then the sections in sidebar order: Intake, Overview, Metrics, Metric detail,
Sessions, Memories, Traceability, Diagrams (sub-nav, Code map, Structure, Layers, Cycles, Neighbourhood,
Schema), Libraries, Instruments, Patterns, Impact, Tickets, Integrations, About. The ranked **Top issues**
are at the end.

---

## Shell — titlebar, sidebar, layout

Files: `app/src/routes/(workspace)/workspace/[id]/+layout.svelte`, `+layout.ts`, `workspace-layout.ts`, `WorkspaceSidebar.svelte`, `workspace-sidebar-view.ts`, `+page.ts` (redirect), `app/src/lib/stores/windows.svelte.ts` (window creation).

**Purpose.** The frame every workspace screen sits in: a drag-region titlebar with the workspace glyph and name, a 180px sidebar (identity card, 14 section links, FTR·14d + Sessions·7d health readout, app version), and a `<main>` that the section renders into.

**Shows.**
- Titlebar: glyph, name, "· workspace window" (`+layout.svelte:36-40`), left-aligned after an 80px inset for the traffic lights.
- Sidebar identity card: glyph, FTR band dot, name, visibility chip (`WorkspaceSidebar.svelte:66-91`).
- 14 sections in this order: Intake, Overview, Metrics, Sessions, Memories, Traceability, Diagrams, Libraries, Instruments, Patterns, Impact, Tickets, Integrations, About (`workspace-sidebar-view.ts:14-29`).
- Health: FTR·14d value + delta + sparkline, Sessions·7d (`WorkspaceSidebar.svelte:110-130`), then `v{APP_VERSION}`.
- `/workspace/{id}` redirects to `/overview` (`+page.ts:5`), so Overview is the landing page even though Intake comes first in the nav.

**API.**

| Call | Where | Live result (sensei) | Notes |
|---|---|---|---|
| `GET /api/workspaces` | `workspace-layout.ts:19` via `tryListWorkspaces` (`api.ts:287`) | 200, **0.22s, 246 KB**, 133 workspaces; sensei carries 569 folders | It downloads every workspace with every folder to find one. `GET /api/workspaces/{id}` answers **405** (the route exists for another verb, `routes.rs:128`), so there is no single-workspace read to use instead. |
| `GET /api/workspaces/{id}/ftr` | `workspace-layout.ts:19` (`api.ts:307`) | 200, 0.03s: `{"ftr14d":0.875,"ftr14dPrev":1.0,"ftrTrend":[0.857,1.0],"sessions7d":0}` | Unknown id → 404 (checked). |

Failure handling is correct: a failed list is an error with its status, not a "not found" (`workspace-layout.ts:20`), and a failed FTR read becomes `ftrError` and an "unavailable" label rather than a 0 (`WorkspaceSidebar.svelte:119-120`).

Problems found in the data:
- **"Sessions · 7d" says 0 while the same workspace had a session today.** `/ftr`'s `sessions7d` is "Σdenominator over 7d" of the daily `ftr` metric rows. Those are *graded* sessions (`crates/senseid/src/db/pg_store/metrics.rs:1248-1257`). The workspace list counts every session in 7 days and returns `sessions7d: 1`, `last_session_at: 2026-10-27T21:30` for the same workspace (`pg_store/workspaces.rs:401`). The sidebar label (`WorkspaceSidebar.svelte:127`) does not say "graded", so two screens show different numbers under the same name.
- **The FTR sparkline is not a 14-day series.** The trend has one point per day *that had graded sessions* (`metrics.rs:1266-1276`, `GROUP BY day`, no calendar fill), so live it is 2 points. The holistic FTR fills 14 calendar days (`metrics.rs:1306-1311`). The workspace spark draws 2 points as if they were adjacent days.
- **The headline and the trend come from two sources.** `ftr14d` comes from `sensei.workspace_metric_daily`; `ftrTrend` comes from `activity.sessions` (`metrics.rs:1251` vs `:1268-1271`). M16's invariant is "FTR has a single source" (`docs/prd/modules/03-delivery/16-workspace-window/README.md` § Data invariants).
- **The band thresholds disagree with the workspaces index.** The sidebar dot is `accent` for 0.6–0.8 (`workspace-sidebar-view.ts:45-50`). The index's `WorkspaceDot` is `bg-ink-mute` for the same range (`(observatory)/workspaces/WorkspaceDot.svelte:10-12`). One workspace gets two colours depending on which window shows it.

**Interactions.**
- Section links are plain `<a href>` (`WorkspaceSidebar.svelte:98-105`); active state uses a prefix match (`workspace-sidebar-view.ts:32-34`). They work, and `diagrams` redirects to `DEFAULT_VIEW`.
- `diagrams/+page.ts:5-10` says Structure is the default view, but `DEFAULT_VIEW = DIAGRAM_VIEWS[0].id` is `codemap` (`diagrams-nav.ts:71`). The comment is wrong.
- **No error page.** No `+error.svelte` exists anywhere under `app/src/routes` (`find … -name +error.svelte` returns nothing). If the daemon is down or the id is unknown when the window opens, `loadWorkspaceLayout` throws (`workspace-layout.ts:20,22`) and SvelteKit's default error page renders. That page has no titlebar drag region (so the overlay-titlebar window cannot be dragged by its content), no sidebar, and no retry.
- **The sidebar does not scroll.** `<aside>` is `flex flex-col` with no `overflow-y-auto` (`WorkspaceSidebar.svelte:58-61`), inside `flex flex-1 overflow-hidden` (`+layout.svelte:42`). By its classes the sidebar needs about 740px. The window's `minHeight` is 600 (`windows.svelte.ts:48`), which leaves 564px under the 36px titlebar. At small heights the code suggests the bottom links, the Health readout and Version are clipped with no way to reach them.
- Nothing here mutates data and nothing destructive is reachable.

**Mockup / spec diff.** Mockup `assets/screens/p1-overview.png` (project window), PRD M16.
- The mockup's titlebar is centred ("先生 · Lumen Cloud · project window"); the app's is left-aligned after an 80px inset.
- The mockup's identity card has a **"⇆ switch project"** button. The app has none, so there is no way to change workspace or get back to the Observatory from this window.
- The mockup shows **per-section counts** (Sessions 28, Memories 11, Traceability 4, Libraries 5, Instruments 7, Patterns 3, Impact 2). The app shows none.
- Mockup Health has three rows: FTR·14d, Sessions·7d, **Drift watch "3 docs"**. The app has FTR (with spark and delta, not in the mock) and Sessions only; Drift watch is missing.
- The mockup footer reads "scoped to this project"; the app shows "Version vX".
- PRD M16 § Diagrams: the sidebar should carry **a flat Diagrams list of eight entries** (Structure 構, Wheel 輪, Zones 域, Dependency matrix 格, Cycles 環, Hidden coupling 縁, Complexity 繁, Database·ER 表), and Libraries should be 庫. The app has one "Diagrams" link (構) to a sub-nav that still carries Layers and Neighbourhood, which the PRD's "Wrong gate" lists as retired. Libraries is still 蔵 (`workspace-sidebar-view.ts:22`). The PRD records both as open gaps itself.
- The PRD's section list has no Intake, Metrics, Tickets or Integrations. The app adds all four, and puts Intake first.
- **#293 step 2 / burndown item 6:** the Observatory and Settings sidebars use rokkit `List` (`(observatory)/ObservatorySidebar.svelte:2`, `(preferences)/settings/SettingsSidebar.svelte:2`). The workspace sidebar is a hand-rolled `<a>` list with its own `<style>` (`WorkspaceSidebar.svelte:95-107,142-150`), so three sidebars use two implementations.

**Checklist.**
- [ ] Open the sensei workspace window. Expect "Sessions · 7d" to match the Workspaces index (1 today). Code suggests the sidebar shows **0**, because it counts graded sessions only. *(bug — curl `/ftr` → `sessions7d:0` vs `/api/workspaces` → `sessions7d:1`)*
- [ ] Look at the FTR·14d spark. Expect 14 days. Code suggests 2 points drawn as adjacent. *(bug — `metrics.rs:1266-1276`)*
- [ ] Compare the sidebar status-dot colour with the dot for the same workspace in Observatory → Workspaces, using a workspace with FTR between 60% and 80%. Code suggests accent here and grey there. *(polish — `workspace-sidebar-view.ts:48` vs `WorkspaceDot.svelte:12`)*
- [ ] Shrink the window to its minimum height (600). Expect the sidebar to scroll. Code suggests Integrations, About, Health and Version are cut off. *(bug — `WorkspaceSidebar.svelte:58-61`, `windows.svelte.ts:48`)*
- [ ] Stop the daemon, then open a workspace window (or open `/workspace/<bogus-id>`). Expect a styled error with a retry. Code suggests SvelteKit's bare default error page, which cannot be dragged and has no way back. *(bug — no `+error.svelte` in `app/src/routes`)*
- [ ] Look for a way to switch workspace or return to the Observatory from the sidebar. Expect "switch project" per the mock. There is none. *(gap vs mockup — p1-overview.png)*
- [ ] Look for counts beside each section and a "Drift watch" health row. Expect them per the mock. They are absent. *(gap vs mockup)*
- [ ] Open Diagrams from the sidebar. PRD M16 expects eight diagram entries in the sidebar. You get one link and a sub-nav with Layers and Neighbourhood, which M16 lists as retired. *(gap vs spec)*
- [ ] Note which page opens first: Overview, while Intake is the first nav item. *(question — is Intake meant to be the front door, `+page.ts:5` vs `workspace-sidebar-view.ts:15`?)*
- [ ] Watch the network panel when the window opens. Expect a single-workspace read. You get the full `/api/workspaces` (246 KB, 133 workspaces × folders). *(polish — `workspace-layout.ts:19`; `GET /api/workspaces/{id}` → 405)*
- [ ] Compare the sidebar's hover and active styling with the Observatory sidebar. Code suggests they differ, because one is hand-rolled and the other is rokkit `List`. *(gap vs #293)*

---

## Workspace index (`/workspace/[id]`)

**Purpose.** Entry URL for the workspace window; it only forwards to Overview.

**Shows.** Nothing of its own. `+page.ts:4-5` throws `redirect(307, /workspace/{id}/overview)`, and `+page.svelte` is an empty `<script>` (lines 1-2).

**API.** None of its own. The layout (`+layout.ts:7` → `workspace-layout.ts:18-25`) calls `GET /api/workspaces` (the full list: 133 workspaces, 246 KB) plus `GET /api/workspaces/{id}/ftr` before **every** pane renders.
- `/api/workspaces`: 1.1 s when the daemon was quiet. It took 17.6 s, 51.4 s and 56.4 s while other review agents were hitting the daemon. Each workspace page blocks on downloading every workspace and its folders (569 folder rows for sensei alone) just to find one.
- `/ftr`: 0.25 s.

**Mockup/spec diff.** No mockup (it is a redirect).

**Checklist**
- [ ] Open `/workspace/ff1ccea2-…` and time it to first paint. Expect a fast redirect to Overview. Code: the layout first fetches the whole workspace list (`workspace-layout.ts:19`), 246 KB, 1–56 s under load. **polish/perf**
- [ ] Open a bogus id such as `/workspace/nope`. Expect "Workspace nope not found" (404). Code: `workspace-layout.ts:22`. **question** (confirm the message reads well)

---

## Intake

**Purpose.** The front door for a chunk of work. You describe it, sensei classifies lifecycle/intent/risk and recommends a playbook, and you confirm, which records a run.

**Shows.** The guide `frame` sentence, a textarea, and "Recommend a playbook". After recommending: a card with the playbook title, rationale, opening tone, three axis chips, and "Use this playbook". After confirming: "Recorded." plus "New intake" (`intake/+page.svelte:42-110`).

**API**

| Endpoint | Status / time | Notes |
|---|---|---|
| `GET /api/playbook/guide` | 200 · fast | `{frame, axes[3], playbooks[]}`. Matches. |
| `POST /api/playbook/recommend` `{chunk, preview:true, workspace_id}` | not called | Preview |
| `POST /api/playbook/recommend` `{lifecycle,intent,risk,confirm:true,workspace_id}` | not called | Writes the run |

**Findings / interactions**
- **Auto-record without a click.** When the daemon returns `auto_select`, `recommend()` immediately calls `confirm()` (`intake.svelte.ts:55`), which records a run.
- **Double-click records two runs.** "Use this playbook" is never disabled while the confirm is in flight (`+page.svelte:104`; `confirm()` sets no loading phase, `intake.svelte.ts:59-75`).
- **A failed confirm hides the recommendation.** On failure the phase becomes `error`; the card only renders for `recommended|recorded` (`+page.svelte:72`), so the recommendation vanishes and you must re-classify.
- **Axes are read-only.** `guide.axes` (the three prompts) are loaded but never shown, and the axis chips are plain `<span>`s (`:87-91`). You cannot correct a misclassified lifecycle/intent/risk before recording.
- **"Recorded." is a dead end.** There is no run id, no link to the run or session, and no next step. You cannot tell what recording did.
- **Typed text can be wiped.** The intake state is re-created whenever `data` changes (`+page.svelte:26`), so an unrelated `invalidateAll` loses a half-typed chunk.

**Mockup/spec diff.** No mockup. PRD 16 mentions "the intake" only in scope (`16-workspace-window/README.md:33`), and there is no screen spec. **No mockup.**

**Checklist**
- [ ] Type a chunk and click Recommend. Expect a card with editable axes. The chips are not editable. **gap**
- [ ] Double-click "Use this playbook". Expect one run recorded. Code: no disable, so two POSTs. **bug**
- [ ] After "Recorded.", look for where the run went. Expect a link. None. **gap**
- [ ] Find a chunk the daemon trusts (`auto_select`). Expect a confirm step. It records with no click (`intake.svelte.ts:55`). **question**
- [ ] Stop the daemon and click Use. Expect the error *and* the card. The card disappears. **polish**

---

## Overview

**Purpose.** The workspace landing pane. It shows the FTR·14d chip, a "sensei speaks" hero with the top pending recommendation (Accept/Reject), three stat blocks, recent sessions, and two #85 additions: Hotspots·7 days and Patterns in use.

**Shows.** The header has the kanji, the "Workspace · {client|internal}" eyebrow, an `{n} repos` chip when there is more than one folder, the name, and FTR·14d (`overview/+page.svelte:72-97`). Below that:
- the hero (`:106-149`)
- stats (`:152-156`)
- recent rows that link to `/sessions#id` (`:170-186`)
- the hotspots list (`:195-216`)
- the patterns summary linking to `/patterns` (`:218-242`)

All derivations are in `overview-view.svelte.ts`.

**API** (`overview/+page.ts:12-16`, all awaited together in `Promise.all`)

| Endpoint | Status / time | Notes |
|---|---|---|
| `GET /api/workspaces/{id}/overview` | 200 · 1.1 s; 3.0 s; 8–13 s under load | Shape matches `WorkspaceOverview`. `workspace.goal` is returned but not rendered. |
| `GET /api/workspaces/{id}/hotspots?days=7` | 200 · **7.9 s, 196 s, 106 s, 90 s, 66 s** | 20 rows, 3.9 KB. Backed by the view `sensei.workspace_hotspots` (jsonb scan of `activity.assistant_events`). |
| `GET /api/workspaces/{id}/patterns` | 200 · 0.014 s | 1 followed, 2 anti |
| Accept: `GET …/recommendations/{rec}/preview` then `POST …/materialize` or `…/accept`; Reject: `POST …/reject` | preview 200 | Not POSTed (read-only review) |

Findings:
- **Hotspots blocks the whole page.** `+page.ts:12` awaits hotspots in the same `Promise.all` as the overview read, and `tryGet` has no timeout (`api.ts:139-147`). So Overview does not render until a 66–196 s query returns. One slow, optional section holds the entire landing pane.
- **Hotspots counts more than 7 days.** The view sums edits over **30 days** (`database/ddl/view/sensei/workspace_hotspots.ddl:22`). The handler only keeps files whose *last* edit falls in the window (`crates/senseid/src/db/pg_store/metrics.rs:235`). A file edited 20 times three weeks ago and once yesterday therefore shows "21 edits" under "Hotspots · 7 days".
- **"corrected" is the wrong word.** `correction_count` is *re-edits within a session* (`COUNT(*) - COUNT(DISTINCT session_id)`, `workspace_hotspots.ddl:14`), but the UI renders "· 11 corrected" (`+page.svelte:211`). The live top row is `docs/CHECKPOINT.md` with 12 edits and 11 "corrected".
- **Hotspots include agent worktrees.** Live rows include `.claude/worktrees/agent-a8cdfaacfada2a58c/crates/senseid/src/db/pg_store/memory.rs`, a disposable worktree copy counted as a project hotspot.
- **Sessions disagree with the workspace list.** Overview says `sessions7d: 0`, and its newest recent session is 2026-10-18. The `/api/workspaces` list for the same workspace says `sessions7d: 1`, `last_session_at: 2026-10-27T21:30`. The hotspots read also finds edits this week. These are two derivations of "sessions" and Overview's is stale (the burndown notes "the synced session metrics end 10-18", `docs/roadmap/08-issue-burndown.md:276`).
- **Every recent row says "session".** All four live rows have `title: ""`, so each renders as "session" (`overview-view.svelte.ts:166-168`). Their durations are 5–30 s, which looks like MCP-only stubs rather than real work.
- **Fabricated zero in the warn input.** `get_quality_signals` returns `ftr_7d: 0` when there is no row (`metrics.rs:261-263`), and `get_workspace_overview` feeds that into `is_warn` (`workspace_detail.rs:223-226`). The rule masks it today because it only warns when sessions7d > 0, but it is still a fabricated 0 on a failure-adjacent path.
- **Doc drift reads "0 of 0 referenced docs"** (live `docDrift: {open:0, referencedDocs:0}`). For a repo with dozens of docs that looks like an unscanned state presented as clean.

**Interactions**
- **Accept** (`+page.svelte:124-130` → `decideRecommendation`, `overview-view.svelte.ts:337-362`). The live top rec is `create_agent`, and preview says `consent_required: true` with path `.claude/agents/sensei-ecosystem-fragmentation-requires-unified-ag.md`. That slug is truncated mid-word. Accept shows a native `confirm()` and then writes that file into the repo.
- **Reject** has no confirmation and no undo (`:131-138`). A second click on an already-decided rec returns 409, which is shown as an error.
- **"send to {acp}"** (`+page.svelte:120-122`) has **no `onclick`**: a dead primary button. It is hidden today only because `defaultAcp` is null.
- **Recent rows** link to `/workspace/{id}/sessions#{sessionId}`, but the sessions route never reads the hash (no `hash` use under `sessions/`). The click lands on an unselected list. The spec requires the click to go through to Replay (`docs/spec/screen/project-overview.md:163`; PRD `16-workspace-window/README.md:122`).
- **Hotspot rows** are plain `<li>`, not links (`:209`).
- **Patterns card** links to `/patterns`.
- **Retry** on an overview error calls `invalidateAll()`.

**Mockup/spec diff** (`p1-overview.png`, `spec/screen/project-overview.md`, PRD 16)
- The mockup has one "send to claude-code →" CTA. The app has Accept + Reject (+ the dead send button).
- The mockup shows evidence session ids (`s-2891 · s-2889 …`). Live `evidence: []`, so the meta line is empty.
- Recent rows in the mockup carry real titles and clock-style times ("10:42", "Yesterday"). The app shows "session" and "9d".
- Hotspots and Patterns sections are not in the mockup (they came from #85).
- The mockup sidebar has a HEALTH block (FTR, Sessions·7d, Drift watch); the shell review covers it.

**Checklist**
- [ ] Open Overview for sensei and time it. Expect the page in about a second. Code: it waits for `/hotspots` (66–196 s measured) before rendering anything (`overview/+page.ts:12-16`). **bug (severe)**
- [ ] Compare "Sessions · 7d" with the workspaces-index card and the sidebar. Expect one number. Live: overview `sessions7d 0` vs list `sessions7d 1` (curl `/overview` vs `/api/workspaces`). **bug**
- [ ] Check "Recent in this workspace". Expect today's sessions. Live: newest is 2026-10-18, all titled "session". **bug**
- [ ] Click a recent row. Expect Replay of that session. Code: it goes to `/sessions#id`, which nothing reads. **gap vs spec**
- [ ] Read the Hotspots list. Expect 7-day edit counts for project files. Code: 30-day counts (`workspace_hotspots.ddl:22`), worktree paths included, re-edits labelled "corrected". **bug**
- [ ] Click Accept on the hero. Expect a confirm naming the file. Code: `confirm()` with a truncated slug `…-unified-ag.md`. Cancel it. **polish**
- [ ] Click Reject. Expect a confirm or an undo. Code: it fires immediately (`+page.svelte:137`). **question**
- [ ] Check the Doc drift card. Expect "N of M referenced docs". Live: "0 of 0". **question** (is traceability ever scanned for sensei?)
- [ ] With a rec carrying `defaultAcp`, click "send to …". Code: no handler (`+page.svelte:121`). **bug (latent)**

---

## Metrics

**Purpose.** The workspace health board. It shows the headline sentence, the composite Health score with its weekly sparkline, "What moved", a radar of all rated metrics (0–5), "What moves together" correlations, and the full signal grid grouped by family. Every cell links to the metric detail.

**Shows.** `metrics/+page.svelte:34-185`. The title is "27 signals" and the eyebrow says "Workspace · metrics · **this week**".

**API** (`metrics/+page.ts:16-62`)

| Endpoint | Status / time | Notes |
|---|---|---|
| `GET /api/workspaces/{id}/metrics` | 200 · 2.4 s quiet, 14.9 s under load | 27 rows; `narrative: {"insights":{}}` (no headline, so the deterministic one is used) |
| `GET /api/metrics/registry` | 200 · 0.21 s | 32 metrics. 5 are never computed for sensei and are not shown anywhere on this screen. |
| `GET /api/workspaces/{id}/health` | 200 · 1.6–2.8 s | `health_score 71, rated_metrics 18`, 12-week `trend` (no `coverage`, no `total_weight`) |
| `GET /api/metrics/correlations` | 200 · 1.6 s | 15 pairs (portfolio-wide) |
| `GET /api/transcripts/consent` | 200 · 0.005 s | all 6 consented, so no notice |
| `GET /api/workspaces/{id}/metrics/{key}?grain=daily` ×27 | 27 in parallel: 5.4 s | N+1: one request per metric |

**Failures shown as empty or quiet states** (the project rule is never to fabricate on failure):
- **Health:** a failed read becomes `null` (`+page.ts:70`), and the radar then says **"No rated metrics yet. Signals appear here as they are computed."** (`HealthRadar.svelte:75-77`). The hero also vanishes. A 500 reads as "not measured".
- **Correlations:** a failed read becomes `[]` (`+page.ts:75`), so the section silently disappears.
- **Series:** a failed fetch becomes `[]` (`+page.ts:59`), so the sparkline is silently missing.
- **Consent:** a failure means no notice (documented as additive at `+page.ts:24`).

**Data correctness (live curl of `/metrics`):**
- **The chip shows a different period from the value.** `value` is the latest *daily* row, while `prior`/`delta` come from the *weekly* trend (`crates/senseid/src/api/handlers/metrics.rs:233-249`). The per-point `explainer` is a third, day-over-day comparison. Live examples:
  - `ftr`: value 1.0, prior 1.0, **delta −0.125**, explainer "up 0.1429 from the prior day"
  - `churn_rate`: value 254, prior 2797, delta −2314 (254−2797 = −2543), explainer "up 25"
  - `throughput`: value 1, prior 2, **delta +6**, explainer "down 6"

  The grid chip (`SignalGridCell.svelte:22-24`) and the detail page show the weekly delta next to a daily value.
- **The Health hero has the same mismatch.** It shows `71` (live score) with delta `+6` = trend[-1] 60 − trend[-2] 54 (`lib/metrics/health-radar.ts:50-54`). The +6 is not the change in 71.
- **"This week" values are up to 40 days old.** Latest dates: `cost_per_result` 2026-09-17; `spec_depth`/`spec_deviation_rate` 2026-10-07; `rework_ratio`, `run_completion`, `incomplete_analysis_rate` 2026-10-15; most session metrics 2026-10-18. No cell shows its date (`SignalGridCell.svelte`). Health 71 is computed over these mixed-age readings, e.g. FTR rated 5 (weight 3) from one 9-second session on 10-18.
- **Absurd explainers:** `cache_read_per_day` "value 10327, **down 4918294419** from the prior day"; `tokens_in_per_day` prior **5,274,995,499**. These show in the detail drill-down (`DatapointDrilldown.svelte` explainer).
- **The spec's coverage readout is missing.** Spec: "65, from 18 of 21 metrics — never a bare number" (`docs/prd/modules/02-insight/08-metrics-and-health/README.md:129-130`). The hero renders a bare `71`. The radar says "18 rated" without "of N", and `/health` returns no `coverage`/`total_weight`.

**ResizeObserver warning, located.** The only component on this screen that uses an observer is the **RadarChart in `HealthRadar.svelte:80-91`**:
- Every sparkline (HealthHero, MoverCard, SignalGridCell, CorrelationCard) goes through `ChartCanvas`, which deliberately has no observer (`lib/components/ChartCanvas.svelte:11`).
- `RadarChart` → `Plot` → rokkit `PlotSurface.svelte:41-52` attaches a `ResizeObserver` on its container and sets `observedWidth`, which re-sizes the `<svg width=… viewBox=…>`.
- The svg is styled `width:100%; height:auto` (`PlotSurface.svelte:144-148`), so each width change briefly changes the svg's rendered height. That changes page height, toggles the scrollbar, and changes the container width again: the "loop completed with undelivered notifications" pattern.
- Side effect: `size={320}` is ignored. The radar takes the full ~1000 px column width instead of 320×320.
- Evidence level: code path only, not profiled. **Confirm by temporarily hiding the radar section and checking the console.**

**Interactions**
- Grid cells link to `/metrics/{key}`.
- **Mover cards and correlation cards are not links** (`MoverCard.svelte:12`), so the most interesting items do not drill down.
- There are no window controls.

**Mockup/spec diff** (`metrics-board.png`, PRD M8)

The mockup has:
- a **30d/60d/all window** toggle
- a **Confidence** card ("72% · 18/25 — share of metrics with a rating scale")
- a radar **by family** (6 axes: Outcome, Code quality, Velocity, Token usage, Autonomy, Knowledge) with a **prior-window overlay**
- **Gained ground / Lost ground** lists with rating steps ("+1 → 2/5 · 74%")
- a **scored-signal table ranked worst-first** with a 30-day sparkline, reading, threshold ladder and "to next"
- a Health delta stated against the prior window ("Prior window 43.3")

The app has per-metric spokes (18), no window, no confidence, no gained/lost, and a family grid not ranked by loss. The spec's wrong-gate "coverage ignored" applies (`08-metrics-and-health/README.md:173-175`).

**Checklist**
- [ ] Open Metrics and the devtools console. Expect no warnings. Expected: `ResizeObserver loop…`. Hide the "Health · all signals" radar to confirm it is the source (`HealthRadar.svelte:80`). **bug**
- [ ] Read the FTR cell: value "100%" with a "−12.5" chip. Expect the chip to be the change in the shown value. It is the weekly delta beside a daily value (`metrics.rs:233-249`). **bug**
- [ ] Read Throughput: value 1, prior 2, chip **+6**. Expect −1. **bug**
- [ ] Health hero "71 +6". Expect the +6 to be the change in 71. It is the difference between the last two weekly buckets (60−54). **bug**
- [ ] Check the dates behind "this week". Expect current values. `cost_per_result` is from 09-17 and most from 10-18, with no date shown. **bug/gap**
- [ ] Find "N of M metrics" next to Health. Expect coverage per spec. A bare number. **gap vs spec**
- [ ] Kill the daemon mid-load or force `/health` to fail. Expect an error. The radar says "No rated metrics yet". **bug (rule: never fabricate)**
- [ ] Click a "What moved" card. Expect drill-down. Not a link. **polish**
- [ ] Look for 30d/60d/all, Confidence, Gained/Lost ground, worst-first table. Expect them per `metrics-board.png`. Absent. **gap vs mockup**

---

## Metric detail (`metrics/[key]`)

**Purpose.** Master-detail view for one signal:
- the signal rail (all signals, grouped)
- a hero value with trend
- a grain toggle (daily/weekly/monthly)
- About this metric (purpose, how to read, formula)
- the chart with a start-window slider
- "Behind this datapoint" (the day's sessions, observations, transcript evidence)
- "What sensei noticed"
- Action items (pending recommendations about this metric)

**API** (`[key]/+page.ts:23-65`)

| Endpoint | Status / time | Notes |
|---|---|---|
| `/metrics`, `/metrics/registry` | as above | Refetched on every key/grain click (no reuse across navigation) |
| `GET …/recommendations?status=pending` | 200 · 0.10 s, 35 KB | **Exactly 50 rows: the SQL has `LIMIT 50`** (`crates/senseid/src/db/pg_store/patterns.rs:1368+`, "LIMIT 50"). The unfiltered call returned 44 pending + 6 accepted, so pending recs past 50 are silently cut. |
| `GET …/metrics/ftr?grain=daily\|weekly\|monthly` | 200 · 0.18 / 0.02 / 0.02 s | Shape matches (`formula`, `series[{period,value,direction,explainer}]`) |
| `GET …/metrics/nope?grain=daily` | **200** `{series:[],formula:null}` | An unknown key is a 200, not a 404. The page then says "That signal isn't computed" (acceptable, but the daemon answers an empty success). |
| `GET …/metrics/ftr/sessions?day=2026-10-18` | 200 · 0.03 s | One session, `task: ""`, summary "no files touched; no corrections" |

**Findings**
- **Action-item matching is a substring guess** (`[key]/metric-detail-state.svelte.ts:116-124`). A rec is "about this metric" if its title, why or impact text contains the metric key, its name, **or its family label**. Any rec mentioning "quality" therefore shows on every quality-family metric. Live example: "Recurring re-edits… improved code quality" (`revise_rule`). Action items also have no Accept/Reject buttons; they are read-only.
- **A 404 is reported as an old daemon.** The drill-down treats any 404 as "This drill-down isn't available on the connected daemon yet" (`datapoint-drilldown.svelte.ts:140-141`). A 404 for an unknown workspace or metric gets the same wording.
- **Drill-down sessions do not link** to the session or Replay (`DatapointDrilldown.svelte` session rows are `<div>`s).
- **Empty task lines.** The session `task` is often `""` (live), so the bold task line renders empty.
- **Value/chip mismatch.** The hero value and chip have the same daily-value/weekly-delta problem as the grid (`[key]/+page.svelte:83-87`).

**Interactions**
- The rail links, the grain links (`?grain=` reloads), "← All signals", the chart point select (`md.pick`), the range slider, the day buttons, and About-metric links to related metrics (`AboutMetric.svelte:45`) all wire up.
- The grain toggle refetches `/metrics` and `/registry` every time (`+page.ts:23-25`), which costs 2–15 s on `/metrics` under load.

**Mockup/spec diff.** No dedicated PNG; the closest is the right rail of `metrics-board.png` (NEEDS… list). The spec asks for thin metrics "behind a coverage gate, not as strong numbers" (M-8, `08-metrics-and-health/README.md:150-152`). FTR = 100% from one 9-second session renders as a strong number (`isLowN` only suppresses the chip).

**Checklist**
- [ ] Open `metrics/ftr`. The hero says 100% with chip −12.5. Expect a consistent change. **bug** (see Metrics)
- [ ] Switch Daily → Weekly → Monthly and time each. Expect an instant switch. Each refetches `/metrics` + `/registry` (`+page.ts:23-25`). **polish/perf**
- [ ] Open a quality-family metric (e.g. `churn_rate`) and read Action items. Expect recs about churn. You get any rec mentioning "quality". **bug**
- [ ] Count action items against the pending recs in the DB. Expect all. Capped at 50 by SQL (`patterns.rs` LIMIT 50). **bug**
- [ ] Click a session in "Behind this datapoint". Expect it to open. Not clickable. **gap**
- [ ] Open `metrics/cache_read_per_day` and read the explainer. Expect a sane number. "down 4918294419 from the prior day". **bug (data)**
- [ ] Open `metrics/does-not-exist`. Expect a clear not-found. "That signal isn't computed for this workspace" (daemon returns 200 empty). **polish**
- [ ] FTR at 100% from one session. Expect a thin-data gate. Shown as a strong number. **gap vs spec (M-8)**

---

## Sessions

**Purpose.** This workspace's sessions, using the same `SessionsDigestScreen` as Observatory · Sessions with workspace copy (`sessions/+page.svelte:19-31`). It shows a retro hero, one chart you can switch between five treatments, and the session list. Clicking a row goes to Replay.

**Shows.** A hero with 7d/30d/90d range chips. A mini-cycler on the right (numbers, trend, stream, constellation, bands, pulse). A totals strip ("N sessions · median M") and a first-try / corrected / abandoned tally. A chart frame with chips. The list, newest first (`lib/components/sessions/SessionsDigestScreen.svelte:74-258`).

**API.**
| Call | Curl (sensei) | Notes |
|---|---|---|
| `GET /api/sessions?range=7d&workspace={id}` (`api.ts:687-693`) | 200, 0.007s, **1 row** | `{stats, sessions, toolUsage, benchmarkPairs}`. The screen reads only `sessions` (`sessions-digest.svelte.ts:43`) |
| same, `range=30d` / `90d` | 200, 0.004s / 0.009s, 12 / 53 rows | |

Findings from the payload:
- **The range filter and the chart axis use different dates.** The daemon windows on `coalesce(completed_at, started_at)` (`crates/senseid/src/db/pg_store/sessions.rs:49`). The UI buckets by `startedAt`'s day (`sessions-digest.ts:116`, `aggregateByDay` at `:286`). With 7d, the one row returned started on 2026-10-15, which is outside the 7-day axis. The header says "1 session", the list shows 1 row, and every chart is empty.
- **One "session" covers 12 days.** Row `bbcb2965…`: `startedAt` 2026-10-15, `completedAt` moved from 21:30 to 21:45 between two curls today, `durationSecs` 567985 (6.6 days, although the field is meant to be the gap-aware active time), `tokensIn` 4,987,861,897. The newest `startedAt` in 90d is **2026-10-18**. Work since 10-18 is being merged into one long-running session, so no new sessions appear. The median and the token chart are distorted by this row.
- **The tally does not add up to the count.** Outcomes in 90d: completed 28, corrected 15, `empty` 6, `null` 3, `incomplete` 1. `quality()` maps only completed+ftr, corrected and abandoned (`sessions-digest.ts:52-57`). The other 10 sessions are "neutral" and appear in no legend or tally. There are 0 abandoned, because the daemon never emits `abandoned`.
- Errors are handled: a failed read carries `error` and renders `ScreenState` with Retry (`sessions-digest.svelte.ts:49-52`, `SessionsDigestScreen.svelte:152-157`).

**Interactions.**
- Range chips call `digest.setRange(r)`, which refetches. There is no request sequencing (`sessions-digest.svelte.ts:113-128`), so clicking 7d → 90d → 30d quickly can land a late 90d response under the 30d chip.
- The chart chips work. The code comment says "Exactly four chips" (`SessionsDigestScreen.svelte:183`), but `CHART_VARIANTS` has five, `tokens` included (`sessions-digest.ts:20`).
- A row click calls `goto('/instruments?tab=replay&session=…')` (`sessions-digest.ts:368-370`). That is an **Observatory** route in the `(observatory)` group, so the workspace window navigates out of its own shell into the Observatory layout.
- `$derived.by(() => new SessionsDigestState(...))` (`sessions/+page.svelte:19-27`) rebuilds the state whenever `data` changes, so any `invalidateAll` resets the chosen range and chart.

**Mockup/spec diff.**
- Mockup `p2-sessions.png` is a **blank image** (an empty paper-coloured canvas), so there is no visual reference.
- Spec `docs/spec/screen/project-sessions.md` Done gate asks for a folder-role chip (`web` / `backend` / `docs`) on multi-repo sessions. The wire row has no folder or repo field (curl shows the keys `id, workspace, task, summary, outcome, ftr, turns, corrections, startedAt, completedAt, agent, tokensIn, tokensOut, durationSecs, provider, model`). The screen comment cites `docs/spec/screen/workspace-sessions.md`, which does not exist (`SessionsDigestScreen.svelte:7`).
- PRD M16 wrong gate: "a row click lands in 'no tool calls'". `GET /api/sessions/012906dd…` returns `acp_id: null, turns: 0, folder_id: null`, so that row (it is in the 90d list) will replay nothing.

**Checklist.**
- [ ] Open Sessions at the default 7d. Expect a chart that matches "1 session". Code suggests the chart is empty, because the only row started 10-15, outside the axis. **bug** (sessions.rs:49 vs sessions-digest.ts:116)
- [ ] Look at the top row. Expect a session of normal length. Curl shows 6.6 days and about 5B input tokens, still growing. **bug** (session boundary or capture)
- [ ] Switch to 90d and compare first-try + corrected + abandoned with the count. Expect them to add up. Code gives 28+15+0 = 43 of 53. **gap**
- [ ] Click a session row. Expect Replay inside the workspace window. Code navigates to Observatory `/instruments`. **question** (sessions-digest.ts:368)
- [ ] Click the "Analyze sensei+dojo…" row (90d). Expect replay content. The daemon has turns 0 and acp null. **bug/gap** (PRD M16 wrong gate)
- [ ] Click 7d → 90d → 30d fast. Expect 30d rows. A race is possible. **polish** (sessions-digest.svelte.ts:122-128)
- [ ] Pick 30d, then trigger any reload (switch workspace and come back). The range resets to 7d. **polish**

---

## Memories

**Purpose.** This workspace's active memories, the "ready to share" lane (generalise → widen scope → share batches), and a What / Because / Consequence drawer.

**Shows.** A ready-to-share hero, but only when at least one memory is generalised (`memories/+page.svelte:52-76`). Proposed share batches with Approve and Reject (`:80-115`). A list with checkboxes, a "generalised" chip, type and strength. A drawer with counts, Generalise, and a widen-scope ladder (user, dōjō, collective) where the governed rungs confirm first (`:173-344`). Otherwise the empty state "Nothing to remember yet." (`:347-351`).

**API.**
| Call | Curl | Notes |
|---|---|---|
| `GET /api/workspaces/{id}/memories` (`api.ts:422-423`) | 200, **0.70s**, 23B: `{"active":[],"total":1}` | |
| `GET /api/workspaces/{id}/memory-batches?status=proposed` (`api.ts:427-429`) | 200, 0.88s, `{"batches":[]}` | `?status=bogus` also returns 200 `[]`, so the status filter is not validated |
| Writes (not run): POST memory-batches, PUT memory-batches/{b}, POST `/api/knowledge/memories/{id}/generalise`, promote | | |

- **`total: 1, active: []`.** The workspace has one memory that is not `active`. The screen ignores `total`: the loader keeps it (`memories-load.ts:20`) but the page never reads it. The result is "Nothing to remember yet." The user cannot see the memory that exists, which is probably `proposed`.
- **The daemon query caps at 100 before it filters on status.** `SELECT … LIMIT 100` and then `.filter(status == "active")` (`crates/senseid/src/db/pg_store/memory.rs:306-322`). A workspace with more than 100 memories can have its active ones pushed out by newer non-active rows. `total` is `rows.len()`, which is capped at 100 and includes non-active rows. The name suggests otherwise.
- The wire has no `name` or `kind`, although the TS type declares them (`types.ts:602-607`). The screen uses `m.title || m.name`, so it renders, but the type is wrong.
- Errors are handled: either read failing becomes the page error (`memories-load.ts:22`).

**Interactions.**
- **Approve and Reject on a share batch run in one click, with no confirmation** (`+page.svelte:93-107`). Approve shares a batch beyond the workspace.
- "widen to user" promotes at once, with no confirmation (`memories-state.svelte.ts:238-245`). Only the governed rungs confirm.
- `generalise()`: on success it awaits `reload()` before `finishGeneralise`. If the reload throws, the button stays at "generalising…" (`memories-state.svelte.ts:344-353`).
- The selection is a `SvelteSet` that is not cleared on reload, so ids that have left the list can still be counted in "N selected".
- A widen creates a `proposed` copy at the wider scope (`memories-state.svelte.ts:273-276`). This screen never lists proposed memories, so the result of the user's action is invisible here.

**Mockup/spec diff (p3-memories.png).**
- The mockup's eyebrow "THIS PROJECT · 11 MEMORIES" and its hero with "review next batch →" plus "reviewed batches · 47 / 47" are missing. The app has a plain `PageHeader title="Memories"`.
- Mockup rows show provenance ("rule · extracted from s-2891 · s-2889") and "12× used". The app shows type and strength %.
- The mockup lists "generalised" rows inline, with the 送 mark and "ready to share · next batch · in 2 days". The app has no schedule.
- Spec `project-memories.md` Done gate: the ready-to-share count must be `generalised && queued_for_batch`. The app counts `generalised` only (`memories-state.svelte.ts:121`). The spec also asks for side-by-side confirmation of a generalise before accepting it. The app writes it directly; the drawer shows original vs generalised only afterwards. The spec asks for `anonymous` credit when a client-workspace memory is widened; the app has no attribution choice.

**Checklist.**
- [ ] Open Memories for sensei. Expect the 1 memory the daemon counts. Code shows "Nothing to remember yet." **bug** (curl `total:1, active:[]`; memory.rs:319-322)
- [ ] Select a memory and press Propose batch, then Approve. Expect a confirmation before anything is shared. Code has none. **bug (destructive, no confirmation)** (+page.svelte:93-99)
- [ ] Generalise a memory. Expect to review the rewrite before accepting it. Code writes immediately. **gap vs spec**
- [ ] Widen to user. Expect the new proposed copy to be visible somewhere. It is not shown here. **gap**
- [ ] Compare hero counts with the mockup ("2 ready to share · reviewed batches"). **gap vs mockup**

---

## Traceability

**Purpose.** Doc ↔ code drift for this workspace: items that are drifted or broken, an expected vs actual signature diff, and "Scan now".

**API.**
| Call | Curl | Notes |
|---|---|---|
| `GET /api/workspaces/{id}/drift` (`api.ts:458`) | 200, 0.014s: `{"items":[],"total":0,"drifted":0,"broken":0,"indexedDocs":0}` | |
| `POST …/drift/scan` (not run) | | |

The screen is accurate about the empty state: with `indexedDocs: 0` it says "No documents are indexed … sensei does not index markdown yet" (`+page.svelte:78-84`), which is #292. In practice the **whole screen has no function** until the markdown adapter ships.

**Interactions.**
- "Scan now" stays enabled although nothing can be scanned (`+page.svelte:55-61`). It will report "Scanned 0 docs".
- `item.file` is never rendered. A row shows `detail ?? status` only (`:101`), so there is no doc path and no symbol path, and it cannot be clicked.

**Mockup/spec diff (p4-traceability.png).**
- The mockup has a hero ("docs/auth.md § Refresh flow has drifted", "run fix-drift prompt →", "checked nightly · 18 docs tracked") and rows with the doc § section, the `↪ crate/src/file.rs::symbol` target, a reason, "synced 21d ago" and "broken link". None of this is in the app.
- Spec `project-traceability.md`: a doc-coverage summary, commit context on the diff, an auto-fix bar for `resolved_auto`, and a branch strip. All are absent.

**Checklist.**
- [ ] Open Traceability. Expect the "no documents indexed" message. **gap (#292)**
- [ ] Press Scan now. Expect it to be disabled or to explain itself when no docs are indexed. Code runs a scan of 0 docs. **polish**
- [ ] Once drift exists, check that each row names its doc and file. Code renders only `detail`. **gap vs mockup**

---

## Diagrams — shared notes (read first)

Routes live under `app/src/routes/(workspace)/workspace/[id]/diagrams/`. Paths below are relative to that folder unless they start with `packages/` or `app/`.

**Timings were taken on a loaded machine.** `uptime` showed a load average of 197 / 153 / 89 during the first pass. The first pass gave layering(module) 62.2 s, layering(file) 69.2 s, structure(module) 36.2 s, codemap 21.7 s, cochange 33.8 s and schema 9.7 s. A re-run a few minutes later gave layering(module) **2.5 s**, structure(file) **1.06 s**, zones 2.3 s and codemap/module 1.0 s. Treat the first-pass numbers as "what a busy machine feels like", not as the endpoint's cost. The spec itself says the cold read costs 1.5–74 s per workspace (`docs/spec/screen/observatory-diagrams-layers.md`, Purpose).

Every diagram screen except Code map shows a plain sentence while it loads. None shows a spinner or elapsed time. A request that takes 60 s looks the same as one that has hung.

| Endpoint (workspace `ff1ccea2…`) | Status | Size | Shape check |
|---|---|---|---|
| `diagrams/codemap?groupBy=layers` | 200 | 49 KB | `level, groupBy, kinds, blocks(60), bands(210), budget, cycles(4), coverage` — matches `Level0` |
| `diagrams/codemap?groupBy=folders` | 200 | 45 KB | same |
| `diagrams/codemap/module?id=senseid/indexer` | 200 | 2.5 KB | `level, module, columns[…]` |
| `diagrams/codemap/module?id=fold:L0` | **404** | 0 | The app can request this; see Code map |
| `diagrams/codemap/cochange` | 200 | 3.8 KB | `hidden(40), pairs 581, commits 4779, minTogether 5` |
| `diagrams/zones` | 200 | 30 KB | `points[…], meanDistance` |
| `diagrams/structure?level=file\|module\|package&kinds=calls` | 200 | 1.25 MB / 72 KB / 4 KB | file has 2404 nodes / 3234 edges; module 190 / 317; package 20 / 10; `coverage.unplaced` 672 |
| `diagrams/structure?…&kinds=` | 400 | — | refused, as the state expects |
| `diagrams/layering?level=module&kinds=calls` | 200 | 100 KB | 190 nodes, 312 edges, depth 6, 4 cycles, 40 self-deps, `coverage{drawn 312, unplaced 672, units 190, unknownUnit 6}` |
| `diagrams/layering?level=file&kinds=calls` | 200 | 1.5 MB | 2404 nodes, 3234 edges, depth 10, 14 cycles |
| `/api/graph/functions?repoId=…&q=level0` | 200 | 0.4 KB, 72 ms | 3 matches `{id,name,file_path,signature,line_start}` |
| `diagrams/neighbourhood?focus=<uuid>&depth=1\|3` | 200 | 5.6 / 6.9 KB | `depth, focus, nodes(13), edges(12), coverage{drawn,cut{in,out},unplacedCallees,namedUnplaced}` |
| `diagrams/neighbourhood?…&depth=4` | 400 | — | as the spec says |
| `diagrams/neighbourhood?focus=<zero uuid>` | 404 | — | as the spec says |
| `diagrams/schema` | 200 | 656 KB | `version 2, tables 198, refs 243, entities 172, deps 1088 (410 unresolved)` |

Every field the screens read is present. None of the diagram screens swallows an error into an empty state: each one keeps loading, error and empty as separate states (`layering-state.svelte.ts:113-120`, `structure-state.svelte.ts` header, `schema-state` `view`, `neighbourhood-state` `view`). No fixture is reachable from production code.

---

## Diagrams · sub-nav (`diagrams/+layout.svelte`, `diagrams/+page.ts`, `diagrams-nav.ts`)

**Purpose.** The tab strip above every diagram, and the redirect that runs when someone opens `…/diagrams` bare.

**Shows.** Six `<a>` tabs, all built from `DIAGRAM_VIEWS` (`diagrams-nav.ts:28-71`): Code map · Structure · Layers · Cycles · Neighbourhood · Schema. The active tab gets `bg-primary` and `aria-current`. A bare `/diagrams` redirects 307 to `DEFAULT_VIEW`, which is `codemap` (`diagrams-nav.ts:75`).

**API.** None.

**Spec diff.**
- The PRD says M16's Diagrams list should be "Structure · Wheel · Zones · Dependency matrix · Cycles · Hidden coupling · Complexity · Database · ER", with the first seven opening one embedded map (`docs/prd/modules/03-delivery/17-code-map/README.md`, Surfaces). The app has a different list. The map's lenses are inside the Code map tab, and the older Structure, Layers and Cycles diagrams remain as tabs beside it. As a result "Structure" and "Cycles" each appear twice: once as a tab and once as a lens inside Code map, and they look different.
- The tabs show text only. The kanji and icon defined per view (`diagrams-nav.ts:22-25`) are used only in each page's header.

**Checklist**
- [ ] Open `/workspace/<id>/diagrams` — expect it to land on Code map. Code agrees (`diagrams-nav.ts:75`). But the comment at `diagrams/+page.ts:5-8` still says "Structure is the section's default view". **polish** (stale comment)
- [ ] Look for a "Structure" tab and a "Structure" lens inside Code map — are both wanted? They draw different pictures under the same name. **question** (PRD M17 says Code map "Replaces … Structure and Layers")
- [ ] Click each tab — expect its tab highlighted. `viewOf` matches with `endsWith` (`diagrams-nav.ts:79`), so a query string such as `?focus=` keeps the right tab active. **ok**

---

## Code map (`diagrams/codemap/+page.svelte`, `codemap-port.ts`, `packages/codemap/*`)

**Purpose.** The M17 merged map, with three levels: L0 modules → L1 files by role → L2 symbols. Seven lenses run on the L0 canvas.

**Shows.**
- **L0.** Up to 60 blocks in group bands, plus a budget line: "drawing 60 blocks for 190 modules · 210 bands for 6070 edges · 728 calls unplaced" (`CodeMap.svelte:107-109`).
- **Controls.** Grouping (Layers / Folders) and a lens tab strip (Structure, Wheel, Cycles, Matrix, Zones, Hidden coupling, Complexity) (`CodeMap.svelte:24-45`).
- **Inspector.** Selected block: group, files, symbols, weight in/out. A folded "+N more" block lists its folded modules as buttons (`CodeMap.svelte:141-159`).
- **L1.** Role columns plus "Depends on" / "Used by" lists (`FileColumns.svelte`).
- **L2.** Symbols with Callers and Callees, at most 7 a side (`SymbolView.svelte`).
- **Under 640 px.** `ModuleList` (`CodeMap.svelte:111-112`).

**API.** It reads through `codemapPort` (`codemap-port.ts:19-27`), which calls `tryGetCodemap`, `tryGetCodemapModule`, `tryGetCodemapFile`, `tryGetZones` and `tryGetCochange` (`app/src/lib/api.ts:484-491`). Each failure is turned into a reason and shown as "Could not read the code map: …" (`CodeMap.svelte:101-103`). Shapes match.

Measured on the live payload:
- 85 of the 210 L0 bands touch a fold block (`fold:L0` …), and all 85 have `direction: null`. Direction counts are null 85 · level 58 · up 31 · down 21 · skip 15.
- 12 of the 40 cycle members are folded inside "+N more" blocks. In the biggest cycle, 8 of its 25 members are folded.
- 27 of the 40 hidden couplings can be drawn. The rest fall inside one block (`lenses.ts:77`).
- `GET …/codemap/module?id=fold:L0` returns **404**.

**Mockup / spec diff** (PRD `17-code-map/README.md`). No PNG mockup exists for the map itself.
- **Navigation.** The PRD says "A single click on a module opens it (L1)". In the app a click selects and a *double*-click opens (`ModuleMap.svelte:64-65`, `WheelLens.svelte:41-42`). The inspector hint says the same (`CodeMap.svelte:157`).
- **Search** ("Search jumps to a module") is not built. `CodeMap.svelte` has no search input.
- **Lens header** ("what it shows · use it to · how to read it", collapsible) is not built, and the lens kanji marks are not shown.
- **Lenses at L1/L2.** The PRD says Structure, Cycles, Hidden coupling and Complexity "work at every level". In the app the lens bar is rendered only when `map.level === 'modules'` (`CodeMap.svelte:73-89`).
- **Group by Communities** is not offered (`CodeMap.svelte:42-45`). The PRD build-status table says it is blocked on Fix 2.
- **Band filters** (all / violations & cycles; minimum weight ≥1/10/25) are not built. **Fit / 100%** is not built.
- **Matrix colour.** The PRD wants upward red, skip/cycle amber, same-layer neutral. The app has upward red and everything else ink (`MatrixLens.svelte:25`).
- **Complexity colour.** The PRD wants amber over 10% and red over 20%. The app uses one continuous `--danger` opacity ramp (`CodeMap.svelte:40`).
- **Inspector with nothing selected.** The PRD wants a workspace summary (upward · skip · cycles · same · down counts). The app shows a hint sentence (`CodeMap.svelte:157`).
- **Breadcrumb root** reads "workspace", not the workspace's name (`packages/codemap/state.svelte.ts:23`).

**Checklist**
- [ ] Open Code map — expect L0 with the budget line and grouping set to Layers. Code: `onMount` loads L0 (`CodeMap.svelte:20-22`). **ok**
- [ ] Single-click a module block — the PRD expects its files to open. The code only selects it; you have to double-click (`ModuleMap.svelte:64-65`). **gap vs spec**
- [ ] Switch to **Matrix** and find the row labelled `fold:L0` (or another `fold:` row). Expect "+55 more". The code labels the row with the raw id: `id.split('/').slice(-1)[0]` (`MatrixLens.svelte:17`). The matrix includes fold blocks because `matrix()` filters out only hubs (`lenses.ts:31`). **bug**
- [ ] Click that `fold:` row label — expect nothing, or a list of what it folds. The code calls `onopen('fold:L0')`, which sends `GET …/codemap/module?id=fold:L0` and gets a **404** (curl above). The page then shows "Could not read the code map: …". ModuleMap and Wheel guard against this with `!folded` (`ModuleMap.svelte:65`); Matrix does not. **bug**
- [ ] **Structure** lens: look at bands to or from a "+N more" block — expect them coloured by direction. All 85 such bands have `direction: null` in the payload, so they cannot be red even if an upward call is folded inside. **gap vs spec** ("upward bands are always red")
- [ ] **Cycles** lens — expect every member of each cycle tinted, with its bands drawn. 12 of the 40 members are folded inside "+N more". Their bands have a `fold:` endpoint, and `cycleBands` drops them because it looks up `of.has(b.source)` on module ids (`lenses.ts:13-16`). The 25-member cycle is drawn with only 17 members. **bug**
- [ ] **Zones** lens with the daemon stopped — expect an error. The code shows the error line at the top, and the canvas below says "Placing components…" with nothing to clear it (`CodeMap.svelte:113-114`; `showLens` leaves `zones` null, `state.svelte.ts:64-66`). **bug** (a stuck loading message beside an error)
- [ ] **Hidden coupling** — expect "40 pairs change together … from 4779 commits, 5+ changes together". The summary counts 40 (`CodeMap.svelte:124`), but only 27 bands can be drawn (the rest are inside one block). The undrawn 13 are not mentioned. **gap** (a count beside a picture that shows fewer)
- [ ] **Complexity** — expect "N of M functions over cyclomatic 10 · 55 of 60 blocks measured". The 5 unmeasured blocks are grey and say "not measured". **ok**
- [ ] Double-click a module, then click a file — expect L1, then L2. Neither transition shows a loading indicator: the old level stays on screen until data arrives (~1 s), because `openModule` changes `level` only after the read (`state.svelte.ts:82-90`). **polish**
- [ ] At L1, look for a "+N more (k symbols)" line — expect to be able to open those files. It is plain text (`FileColumns.svelte:42`). **gap**
- [ ] Narrow the window below 640 px — expect a list. `ModuleList` is used, and fold rows are disabled (`ModuleList.svelte` `disabled={b.folded.length > 0}`). **ok**

---

## Structure (`diagrams/structure/+page.svelte`, `structure/structure-state.svelte.ts`)

**Purpose.** The hierarchical edge-bundling picture. It answers "where the code's hierarchy holds and where it breaks" (spec `observatory-diagrams-structure.md`).

**Shows.**
- Level toolbar: file / module / package, default `file` (`structure-state.svelte.ts` `LEVELS`, `level = $state('file')`).
- Edge-kind toggles, default `calls`.
- BundleControl.
- A coverage line, the `StructureDiagram`, and a selection panel: module, package, symbols, degree in / out / crossing (`structure/+page.svelte:88-107`).

**API.** `tryGetWorkspaceStructure` → `/diagrams/structure?level&kinds` (`api.ts:478-481`). File level returns 2404 nodes, module 190 (a 12.6× reduction, which passes spec gate 4), package 20. Empty kinds → 400, and the state never sends that request. Shapes match.

**Mockup / spec diff.** The spec's source mockup is `project-pages.jsx` `diag-structure`; there is no PNG. Done gates 1–6 are satisfied by the payload and code. The spec's stated route `/project/[id]/…` is stale (now `/workspace/[id]/…`).

**Checklist**
- [ ] Open Structure — the header icon should be Structure's. The code passes `icon={DIAGRAM_VIEWS[0].icon}` (`structure/+page.svelte:27`), and `[0]` is now **Code map** (`diagrams-nav.ts:30`), so Structure shows Code map's icon. **bug** (small)
- [ ] Read the header description — expect the view's question ("What is here, and what touches what"), as on every other diagram. The code passes the workspace name instead (`structure/+page.svelte:27`). **polish**
- [ ] Switch level to module, then package — each switch re-queries. File level is 1.25 MB / 2404 nodes. Expect the bundled picture to stay responsive. The baseline crawl clicked 30 of 32 controls with no issue. **ok**
- [ ] Turn off the last edge kind — expect nothing to happen (a no-op), not a 400. Mirrors `LayersState.toggleKind`. **ok**
- [ ] Select a file — expect module, package, symbols and degree. **ok**

---

## Layers (`diagrams/layers/+page.svelte`, `diagrams/layering-state.svelte.ts`) — FREEZES

**Purpose.** "Do these calls flow downward?" A layering measured from the call graph, with the climbing edges (`up`) highlighted as the arrows to cut.

**Shows.**
- Level toolbar (module / file), edge-kind toggles, and `ViolationsControl`.
- Three sentences: `sourceNote`, `breaksNote`, `coverageLine` (`layers/+page.svelte:78-80`).
- `LayersDiagram` in a `h-[70vh]` box with `legend` and `bind:showEdges`.
- A selection panel showing layer, package, files and cycle size.

**API.** `tryGetWorkspaceLayering` → `/diagrams/layering?level&kinds` (`api.ts:504-507`). Live module payload: 190 nodes, 312 edges, depth 6. Conformance counts are down 129 · level 81 · skip 63 · **up 39**. Layer sizes are L0 74, L1 46, L2 52, L3 11, L4 5, L5 2. Shapes match `LayeringPayload`.

**The freeze — what the evidence rules out, and what is left.**
1. **Not the daemon.** The request returns 200 with 100 KB in 2.5 s on a quiet machine.
2. **Not the layout algorithm.** I ran `@rokkit/graph`'s own `normalizeGraph` + `layers()` (the `dist/layout/layers.js` that ships) under bun on the **live payload**. Module grain took 5.5 ms to normalise and **3.5 ms** to lay out: 6 bands, 190 cards, 312 routed edges, size 2220×2152. File grain took 18 ms + **27 ms** for 2404 cards and a size of 2220×19224. `layers.js` is linear: one barycenter pass (`layers.js:67-80`). The Sugiyama `rank`/`order` sweeps, which do have an O(E²) crossing count (`layout/order.js:28-53`), are **not called** by `layers`. No O(n²+) layout runs here.
3. **Not `LayersState` or the page's `$effect`.** The **Cycles** page builds the *same* `LayersState`, with the *same* `$effect` (`cycles/+page.svelte:12-20` vs `layers/+page.svelte:13-23`) and the same endpoint, and it does not freeze (crawl: 8/8 clicked). The effect's tracked reads are `workspaceId`, `level` and `kinds`. `load()` writes `loading`, `failure` and `payload`, none of which the effect reads (`layering-state.svelte.ts:238-255`). No self-invalidating effect here.
4. **Not the graph renderer's size.** Structure renders 2404 nodes / 3234 edges through the same `Graph.svelte` + `GraphState` and does not freeze.
5. **What is left: the browser-only path that `LayersDiagram` adds.** jsdom does not freeze. The only parts of `Graph.svelte` that jsdom does not run are the measured-viewport parts:
   - `bind:clientWidth={vw}` and `bind:clientHeight={vh}` on the paper, which use a ResizeObserver (`Graph.svelte:185-187`);
   - the derived `frame` (`Graph.svelte:120`, `canvas/frame.js:5-18`);
   - the extent box, sized by `frame.extent`, inside an `overflow:auto` paper (`Graph.svelte:192`; CSS `[data-graph-paper]{position:absolute;inset:0;overflow:auto}`, `@rokkit/themes/dist/index.css:6688-6697`).

   `frameOf` sets `extent = max(vw, drawn + 2·pad)`, and `vw`/`vh` are the paper's *client* size, which shrinks when a scrollbar appears. On Layers the fit is height-bound: content is 2220×2152 in a ~70vh box, so the fit scale is ≈ 0.25 and `drawn.h + 56 ≈ vh` **exactly** (`frame.js:6,16`). At file grain the fit hits the 0.08 floor (`frame.js:2,6`), so the extent is taller than the viewport and the paper scrolls. Sitting exactly on the boundary like this is the classic setup for a scrollbar ↔ ResizeObserver oscillation: scrollbar appears → `vh`/`vw` change → `frame` changes → extent changes → scrollbar disappears → repeat. It only happens with classic (non-overlay) scrollbars, which the WKWebView uses when "show scroll bars" is "always" or a mouse is attached. The **workspace metrics** screen already logs `ResizeObserver loop completed with undelivered notifications` (`docs/baseline/2026-10-27-app.md`, open item), which is evidence that this app's WebView does hit RO loops.

   **This is the strongest remaining hypothesis, not a proven cause.** The fix needs one webview profile to confirm it.
6. **Smaller layers-only differences, worth bisecting in the same session:**
   - `bind:showEdges={view.showEdges}` *and* a second `ViolationsControl` outside the diagram that writes the same field (`layers/+page.svelte:49-52,87`), so two owners write one value;
   - `legend` → `GraphLegend relations`;
   - layer bands render as `[data-graph-cluster]` boxes with `pointer-events:none` (`index.css:7510`).

   None of these looked like a loop on reading.

**Suggested 10-minute bisect in the real app** (read-only for the user; this is a code experiment for whoever fixes it). In `layers/+page.svelte`:
- (a) give the diagram box a fixed pixel height, e.g. `h-[600px]`, instead of `70vh`;
- (b) drop `bind:showEdges` and keep only the `ViolationsControl` callback;
- (c) drop `legend`.

If (a) alone un-freezes it, the RO/scrollbar loop is confirmed and the fix is in `@rokkit/graph` (`frameOf` should leave a scrollbar's worth of slack, or the paper should use `scrollbar-gutter: stable`).

**Spec diff** (`observatory-diagrams-layers.md`; source mockup "Schema and Call Graph v8" views layers/cycles, no PNG).
- The spec's invariant is "Every `up` edge is a cycle's back edge … the arrows highlighted on Layers and the cut named on Cycles are the same set of facts and cannot disagree". It is **half true on live data**. All 39 `up` edges do lie inside a cycle (checked), but they are not "the cut". Layers highlights **39** arrows. Cycles names **4** cuts, and **2 of those 4 cuts are `level` edges, not `up` edges**: `sensei-cli → sensei-cli/tracker` and `sensei-dojo-web/lib → …/components` (payload `edges[].conformance` for each `cycles[].cut`). So `breaksNote` "39 calls climb a layer. Each one is a cycle's back edge — the arrow to cut." (`layering-state.svelte.ts:203`) points the reader at 39 arrows, while Cycles points at 4 different ones.
- Done gate 5's numbers (22 members, 88 inner) are stale. Live: cycles of 25 / 7 / 4 / 4 members, with 102 inner edges in the largest.

**Checklist**
- [ ] Open Layers (module grain) — expect three sentences and the banded diagram. The real app **freezes** (`docs/baseline/2026-10-27-app.md`: "a 30s script call timed out"). **bug — blocker**
- [ ] If it renders (e.g. with overlay scrollbars, or a mouse unplugged), resize the window slowly across the boundary where a scrollbar appears in the diagram — watch for flicker or a CPU spike. That is the RO hypothesis above. **bug (diagnostic)**
- [ ] Read `breaksNote` — "39 calls climb a layer … the arrow to cut". Then compare with Cycles, which names 4 cuts. Do you want 39 or 4? **question / bug in wording** (`layering-state.svelte.ts:203`)
- [ ] Toggle Violations only — expect 39 red edges. Code: rokkit filters `conformance === 'up'` (`layout/edges.js` `keepsEdge`). **ok if it renders**
- [ ] Switch to file grain — 2404 cards in a 19,224 px-tall drawing. Fit hits the 0.08 floor, so it renders as "dot" detail and scrolls. **polish** (unreadable at that grain; consider refusing file grain or defaulting to module)
- [ ] Toggle an edge kind — the diagram unmounts behind "Ranking the call graph…" for the whole request (2.5–70 s), and zoom/scroll are lost (`layers/+page.svelte:55-58`). **polish**

---

## Cycles (`diagrams/cycles/+page.svelte`, shares `layering-state.svelte.ts`)

**Purpose.** "What depends on itself, and where to cut": one card per strongly-connected component.

**Shows.**
- Level toolbar (module / file) and the coverage line.
- "N cycles, deepest-depended-on first".
- Per cycle: eyebrow, members, every inner dependency with the cut marked `cut ·` in accent, and the sentence "Cut A → B (×n, the weakest link) and the loop opens." (`cycles/+page.svelte:74-114`, `layering-state.svelte.ts:211-218`).
- A "Depends on itself" list.

**API.** The same layering call as Layers. Module grain: 4 cycles, 40 self-deps. File grain: 14 cycles, and the largest card has 612 inner rows.

**Spec diff.** The spec's cycle card is implemented as written. Unlike the PRD's Cycles *lens* there is no diagram: it is a list.

**Checklist**
- [ ] Read the first cycle's last line: "Cut senseid/adapters → senseid/tasks (×1, the weakest link) and **the loop opens**." **That is false.** I removed that edge from the cycle's 102 inner edges and recomputed the SCCs: **all 25 members are still one strongly-connected component**. For each of the three 4-member cycles, cutting the named edge leaves a 3-member cycle. No cut named on this screen opens its loop (Tarjan over `cycles[].inner` minus `cut`, live module payload). **bug** (a confident instruction that is wrong; `layering-state.svelte.ts:217`)
- [ ] Note which cuts are `level` rather than `up` edges (2 of 4). Compare with Layers' 39 highlighted arrows. **question** (see Layers)
- [ ] Switch to file grain — the largest card lists 612 inner dependencies with no collapse. **polish**
- [ ] With no cycles (another workspace) — expect a sentence, `cycles-none`. **ok** (`cycles/+page.svelte:63-67`)
- [ ] Look for an edge-kind control — Cycles has none and always asks for `calls` (`LayersState.kinds` default). Layers lets you change kinds. If you change kinds on Layers and then open Cycles, the two screens may describe different graphs. Each page owns its own state, so this is benign but may surprise. **polish**

---

## Neighbourhood (`diagrams/neighbourhood/+page.svelte`, `diagrams/neighbourhood-state.svelte.ts`)

**Purpose.** "What calls this, and what does it call": one symbol's callers and callees, 1–3 hops.

**Shows.**
- A function picker (search, 2+ characters).
- Depth toolbar (1–3 hops), shown once a focus is set.
- A focus line (name · package / module · file:line), coverage notes, a refusal line, and the `Neighborhood` diagram.
- The focus is written back to `?focus=` (`neighbourhood/+page.svelte:22-28`).

**API.**
- `trySearchFunctions` → `/api/graph/functions?repoId=…&q=` (`api.ts:659-660`): 72 ms, `{id,name,file_path,signature,line_start}`.
- `tryGetWorkspaceNeighbourhood` → `/diagrams/neighbourhood?focus&depth` (`api.ts:511-513`): 200 in ~1 s; `depth=4` → 400; unknown focus → 404, as spec gate 1 requires.
- Shapes match. A stale-response ticket protects `load()` (`neighbourhood-state` `#ticket`).

**Mockup diff (`assets/screens/arch-calls.png`, "Architecture · Calls" tab).** The mockup has:
- a hierarchy tree on the left (repository → crate → module → file → symbol);
- **Group by** Symbol / File / Module / Crate and a **Tests hidden** toggle;
- **edge-kind chips with counts** (calls 18 · references 2 · imports 1 · implements 0 · extends 0);
- "depth 2 · callers / called by / this symbol / calls / depth 2 · callees" bands;
- **Gaps** and **Correct limits** chips on the focus;
- a right inspector with fqn, file, surface, community, neighbours, unplaced use sites, and a "Gaps to close" list.

The app has only search + depth + the diagram + coverage sentences. None of the other parts are built. **gap vs mockup**. The spec (`observatory-diagrams-neighbourhood.md`) asks only for the reduced set, so the app meets its spec but not the mockup.

**Checklist**
- [ ] Open Neighbourhood — expect the picker sentence and no request. **ok** (spec gate 4)
- [ ] Type "level0" — expect `layoutLevel0`, `level0`, … with file:line. Each keystroke sends a request: there is no debounce (`neighbourhood/+page.svelte:58`, `neighbourhood-state` `search`). **polish**
- [ ] Pick a match — expect the diagram. The match list **stays open** under the search box after picking (`setFocus` does not clear `matches`). **polish**
- [ ] Click any card in the diagram — it **re-centres immediately**. There is no way to select a card just to read its details (`neighbourhood/+page.svelte:161-163`). **question** (the mockup has an inspector)
- [ ] Click an external card (e.g. `Array.filter`) — expect "… is outside this workspace". **ok** (`recentre`)
- [ ] Reload with `?focus=` — expect the same symbol. **ok** (`+page.ts:12`, `+page.svelte:20`)
- [ ] Look for an edge-kind or "tests hidden" filter, as in the mockup — absent. **gap vs mockup**

---

## Schema (`diagrams/schema/+page.svelte`, `diagrams/schema-state.svelte.ts`)

**Purpose.** "Which tables exist, and how their keys connect them": the ER picture, plus a Dependencies scope (views, functions, what reads and writes what), from the workspace's dbd `design.yaml`.

**Shows.**
- Scope toggle (Tables & keys / Dependencies).
- Schema toggle: All · 198, then activity 16, dojo 53, gateway 6, history 2, inference 6, public 1, sensei 92, staging 22.
- Coverage lines.
- `ErDiagram` or `DependencyDiagram` (`controls`, `legend`).
- `EntityView` aside for the selected table (`schema/+page.svelte:66-133`).

**API.** `tryGetWorkspaceSchema` → `/diagrams/schema` (`api.ts:521-522`): 656 KB, version 2, tables 198, refs 243, entities 172, deps 1088 (410 unresolved). Shapes match. There is one `design.yaml` (`database/design.yaml`), so all eight schemas are one database. **ok**

**Spec diff.** The spec's signals and gates are met in code. Its numbers (169 tables / 184 keys) are stale; live is 198 / 243. The spec's own "Not in scope" excludes a mockup PNG.

**Checklist**
- [ ] Open Schema — expect 198 tables · 243 keys and an ER canvas. **ok**
- [ ] Choose `dojo` — expect only its tables plus "N keys lead out of the shown schema, not drawn". **ok** (`schema-state` `#refs`)
- [ ] Switch to **Dependencies** and click a **view or function** card — expect its details. `selected` returns null for anything that is not a table (`schema-state.svelte.ts` `get selected`: `#tables.some(...)`). The diagram is controlled (`value={view.selected}`), so the click is **immediately un-selected**, and the aside keeps saying "Select a table…". **bug / gap** (nothing in the Dependencies scope can be inspected)
- [ ] Select a table, then switch to a schema that does not contain it — expect the selection to clear quietly. **ok** (`selected` is derived from the filter)
- [ ] Stop the daemon and Retry — expect the error panel, not "No database structure is described". **ok** (`view` checks `failure` before `none`)
- [ ] With All selected, use the diagram's own zoom/density controls (`controls`) — 198 cards at 656 KB. Expect it usable. The crawl clicked 23/25 controls fine. **ok**

---

## Libraries

**Purpose.** This workspace's libraries with wrapped (docs), local and version-conflict badges, search, and a filter toggle (`libraries/+page.svelte`).

**API.**
| Call | Curl | Notes |
|---|---|---|
| `GET /api/workspaces/{id}/libraries` (`api.ts:382-391`) | 200, **8.6s cold / 1.5s warm**, 8 rows | |
| `GET /api/workspaces/{id}/library-version-conflicts` (`api.ts:397-399`) | 200, 0.30s, 29 conflicts | |

- **The list and the conflicts come from different sources.** The list is "enablement" (`sensei.workspace_libraries_resolved`, `crates/senseid/src/db/pg_store/library.rs:1673-1681`). It holds 8 rows, all `cargo`, and all the workspace's **own crates** (`dojo-protocol, sensei-bootstrap, sensei-cli, sensei-config, sensei-logger, sensei-mcp, sensei-transcript-formats, senseid`), with `hasDocs: false`. The conflicts come from dependency data: 29 npm libraries such as `@rokkit/actions 1.9.0, 1.9.1 across app, dojo, website`. Results:
  - "8 total · 0 wrapped" sits beside "⚠ 29 version conflicts".
  - The **Conflicts filter returns "No libraries match this filter."**, because none of the 29 names is in the 8-row list (`+page.svelte:33`).
  - rokkit, kavach and dbd are wrapped elsewhere (burndown #341: "rokkit 15/95/5/3"), but this workspace shows 0 wrapped.
- Conflict lookup is by name only (`conflictSet` of `library_name`, `+page.svelte:24`). A name shared by two ecosystems (the baseline's `uuid` and `regex`) would flag both rows.
- The error path is handled (`+page.ts:16-22`).

**Interactions.**
- Search and the filter toggle work.
- **Rows have no click.** There is no drill-down, no docs, no wrap. The observatory Libraries screen has these (#341).
- The badges use `border-success-edge`, `border-primary-edge`, `bg-primary-soft` and `border-warning-edge` (`+page.svelte:126-136`), and `.badge` sets a literal `font-size: 10px` (`:171`). Both go against the app's token and type-scale rules (`app/CLAUDE.md` §1, scale floor `text-xs` = 11).

**Mockup/spec diff (p5-libraries.png).**
- The mockup has a "Library coverage" hero ("One library is unwrapped — tailwindcss", "wrap tailwindcss →", "4 of 5 wrapped") and rows with "detected · package.json" provenance and "14× used · wrapped". The app has neither provenance nor usage.
- Spec `project-libraries.md` Done gate: a **Wrap** action, a **Docs button** (it opens the Playground pre-filled), and a wrap-target picker for multi-repo workspaces. All are absent.

**Checklist.**
- [ ] Open Libraries. Expect the npm and cargo dependencies (rokkit, kavach, …). Code shows 8 internal crates. **bug** (library.rs:1673)
- [ ] Click the "Conflicts" filter while the banner shows 29. Expect 29 rows. Code shows "No libraries match this filter." **bug**
- [ ] Check "wrapped" for rokkit. Expect wrapped. It is absent from the list. **bug**
- [ ] Click a library row. Expect docs or a wrap action. Nothing happens. **gap vs spec**
- [ ] Time a cold open. 8.6s. **polish**

---

## Instruments

**Purpose.** This workspace's instruments: per-workspace service toggles, "Tools used" bubbles, an MCP tool table with calls, errors, average time, FTR and last use, and extensions.

**API.**
| Call | Curl | Notes |
|---|---|---|
| `GET /api/workspaces/{id}/instruments` (`api.ts:404`) | 200, **2.19s**, `{"tools":[]}` | |
| `GET /api/workspaces/{id}/mcp-tool-stats` (`api.ts:411`) | 200, **68.7s first, 39.6s second** | 20 tools, **every `calls: 0`** |
| `GET /api/workspaces/{id}/services` (`api.ts:416`) | 200, 1.04s | 3 services, all `enabledForWorkspace: false`, including sensei-mcp |
| `GET /api/workspaces/{id}/tools` (`api.ts:371`) | 200, 1.11s | 24 tools. `mcp__plugin_sensei_sensei__log_event` 124 calls, `update_phase` 78 |

- **The page takes 40–69s to open.** The loader `Promise.all`s all four reads before rendering (`instruments-load.ts:22-27`), so the slowest one, mcp-tool-stats, blocks navigation. The burndown says #298 brought workspace Instruments to 0.58s (`docs/roadmap/08-issue-burndown.md:194`). That no longer holds.
- **The MCP tools table shows a retired, hand-kept catalogue at zero.** The daemon maps a hard-coded `mcp_manifests::manifests()` list over usage rows by bare name (`crates/senseid/src/api/handlers/workspace_detail.rs:370-393`). The list includes tools that no longer exist (`get_symbol`, `get_file_tags`, `get_doc_drift`, `query`, `get_metrics`, `get_ftr_daily`, `get_hotspots`, `get_quality_signals`) and leaves out the ones in use (`log_event`, `update_phase`). The /tools read in the same page shows 124 + 78 sensei MCP calls, while the table shows 0 calls for every tool. The burndown records "the hand-kept 20-tool catalog … is retired" for the Playground (`08-issue-burndown.md:292`), but this page still uses it. `unwrap_or(json!(0))` turns "no match" into 0 calls, which is a fabricated zero.
- **Extensions is always hidden.** `/instruments` returns 0 tools, yet the repo has 8 agents in `.claude/agents` and 9 skills in `.claude/skills`. The section is guarded by `{#if data.tools.length > 0}` (`+page.svelte:180`), so the gap is silent.
- **sensei-mcp shows "Off"** for the sensei workspace (`enabledForWorkspace:false, globalEnabled:null`) while its tools are being called. Either the toggle does not mean what it says, or the state is wrong.
- `toolsCount` for sensei-mcp is 19. Elsewhere sensei lists 56 tools (burndown line 295).

**Interactions.**
- The service On/Off toggle does a PUT and reload, and shows the error on its row (`services-state.svelte.ts:27-35`). There is no confirmation, which is acceptable for a toggle.
- The kind chips (All / Queries / Actions) filter the table. They use `role="tab"` on buttons with no tabpanel (`+page.svelte:126-141`).
- Bubbles show a tooltip only; they cannot be clicked.
- `.toggle.enabled` uses `var(--success-edge)` (`+page.svelte:223`). `success-edge` is not one of the 24 canonical tokens in `app/CLAUDE.md`; check that it resolves.

**Mockup/spec diff (p6-instruments.png).**
- The mockup has a hero ("patterns.find-similar is your weakest tool here", "open replay for … →", "164 calls · 7d") and a "Project-scoped tools" list with "N calls · FTR" and the scope library. The app has no hero and no replay link.
- Spec `project-instruments.md` Done gate: "Every tab correctly scopes to the project id … Playground executions are attributed to this project … Sub-nav renders below the hero". The workspace page has **no tabs** (no Playground, Health or Replay).

**Checklist.**
- [ ] Open Instruments and time it. Expect about 1s. Curl suggests 40–69s on a blank page. **bug** (mcp-tool-stats)
- [ ] Compare "Tools used" with the MCP tools table. Expect sensei's log_event (124) in the table. Code shows 20 tools at 0, several of them retired. **bug** (workspace_detail.rs:370-393)
- [ ] Look for Extensions. Expect the repo's 8 agents and 9 skills. The section is hidden. **bug**
- [ ] Check Sensei MCP. Expect On. It shows Off. **question**
- [ ] Look for the Playground, Health and Replay tabs scoped to this workspace. They are absent. **gap vs spec**

---

## Patterns

**Purpose.** The patterns this workspace follows and its anti-patterns, each with instances, lifecycle, confidence and FTR delta. A row expands to a description, enforcement, files by directory, and an example (`patterns/+page.svelte`).

**API.**
| Call | Curl | Notes |
|---|---|---|
| `GET /api/workspaces/{id}/patterns` (`api.ts:525-526`) | 200, **2.74s**, 18KB | `followed`: 1 (`rule-candidates`, everything null). `antiPatterns`: 2 (`correction-prone` ×6 with no description; `rework hotspots` ×375 with 14 directory groups) |

- `ftrDelta` is **null on all 3**, so the FTR column is "—" everywhere. `confidence` is null on 2 of 3.
- The `rework hotspots` example names `crates/senseid/src/db/pg_store.rs`, which no longer exists (`pg_store/` is a directory now), and groups include `.worktrees` and `.claude`. The file paths are plain text and cannot be clicked.
- The loader falls back with `read.data.followed ?? []` (`+page.ts:10`). A shape change would read as "No patterns detected yet." rather than as an error.
- The TS `PatternEntry` declares `is_anti_pattern` and `members` (`types.ts:641-647`). The wire sends `isAntiPattern` and no `members`. The type is wrong but unused.
- Compare the MCP: `get_patterns` (baseline) answers with the SvelteKit route-file patterns for sensei. The Patterns screen shows none of them.

**Interactions.** A row toggle expands and collapses. Nothing else: no promote, no dismiss, and file paths are not links.

**Mockup/spec diff (p7-patterns.png).**
- The mockup has a two-tab switch ("Patterns in use 6 / Anti-patterns 6"), a list-and-detail layout, GoF families ("GoF · structural · 7 places"), lifecycle badges (rule, suggested, gap), and a code example panel with "Promoted to project rule". The app has one stacked list with an inline expand and no detail panel.
- Spec `project-patterns.md` Done gate requires a **Promote** action (it creates a rule at the chosen ladder and priority) and **Dismiss**. Neither exists.

**Checklist.**
- [ ] Open Patterns. Expect the patterns the MCP finds. Code shows 1 empty "rule-candidates". **gap** (curl)
- [ ] Expand "correction-prone". Expect its 6 instances. Code shows "No description captured for this pattern yet." and no files. **gap**
- [ ] Expand "rework hotspots" and check the example paths. `pg_store.rs` is stale. **bug (data)**
- [ ] Look for Promote or Dismiss. They are absent. **gap vs spec**
- [ ] Time the load. About 2.7s blocks navigation. **polish**

---

## Impact

**Purpose.** Measured verdicts for recommendations (FTR before vs after, MOE reasoning) plus a manual "impact log" where you record a shipped change and give it a verdict later.

**API.**
| Call | Curl | Notes |
|---|---|---|
| `GET /api/workspaces/{id}/impact` (`api.ts:450`) | 200, **1.66s**, 40KB, 96 rows | 7 `positive`, 89 `pending` |
| `GET /api/workspaces/{id}/impact-verdicts` (`api.ts:446`) | 200, 0.29s | 1 pending entry ("E2E: T2 completion", from 2026-07-02) |

- **All 7 "positive" verdicts are fabricated from a null baseline.** Each has `baselineFtr: null, currentFtr: 0.75, ftrDelta: null`, and the reasoning reads "Strong positive impact — FTR +75pp … moved from 0% to 75%". The cause is `COALESCE(r.baseline_ftr, 0)` in `measure_pending_verdicts` (`crates/senseid/src/db/pg_store/metrics.rs:71`). A missing baseline is treated as 0%, so every accepted recommendation without a baseline gets the same +75pp verdict. This breaks the project rule "Never fabricate data on a failure path". The screen then contradicts itself: the strip shows baseline "—" and delta "—" (`+page.svelte:151-158`), while the reasoning panel says +75pp.
- **89 of the 96 rows have `status: pending`.** They were never accepted, yet they are listed as "measurement pending". The header says "Each accepted recommendation gets a measurement window" (`+page.svelte:43`).
- `bucketImpact` silently drops any verdict outside the four known ones (`lib/impact.ts:145`), while `total` counts every row (`impact-load.ts:43`). A new verdict value would give a non-zero total with an empty list.
- Errors are handled (`impact-load.ts:45`).

**Interactions.**
- Selecting a verdict row works (`+page.svelte:94`).
- Log impact posts the form and keeps what was typed on error (`impact-state.svelte.ts:48-61`).
- **Success, Mixed and Failure record a verdict permanently in one click, with no confirmation or undo** (`+page.svelte:234-242`).
- The verdict list has a fixed `max-h-[560px]` and scrolls inside the page scroll (`+page.svelte:80`).

**Mockup/spec diff (p8-impact.png).**
- The mockup has a hero ("Loop closed · 2 wins this month", "see all impact reports →") and rows with "FTR 14d before 62% → after 79%", "accepted 12d ago" and "+17 pts". The app has 4 counters, a list and detail, and a separate log form.
- Spec `project-impact.md` Done gate requires an **FTR trend chart annotated at `applied_at`**. It is absent.

**Checklist.**
- [ ] Open Impact and pick any positive verdict. Expect a real baseline. The strip shows baseline "—" while the reasoning says "0% → 75%". **bug (fabrication, daemon)** (metrics.rs:71)
- [ ] Count the positive verdicts. All 7 are +75pp, which is implausible. **bug**
- [ ] Check that "pending" rows are accepted recommendations. 89 have `status: pending`. **gap**
- [ ] Press Failure on the logged entry. Expect a confirmation. Code has none, and the change is irreversible. **bug (no confirmation)**
- [ ] Look for the FTR trend chart. It is absent. **gap vs spec**

---

## Tickets

**Purpose.** For workspaces with a sprint-level board: one lane per board, the tickets with their AI usage (sessions, active time, tokens, cost), a sprint filter, unlinked sessions, and a ticket detail view (`tickets-state.svelte.ts`).

**API.** `GET /api/workspaces/{id}/trackers` returns 200 `[]` in 2.55s, so the view is `not_connected`: "This workspace has no sprint board connected." with the link "Connect one in Integrations" (`TicketsScreen.svelte:48-52`). The sprint, backlog, task and unlinked reads (`tickets-state.svelte.ts:17-33`) cannot be exercised for sensei; there is no tracker.

- Lane reads fail the whole screen rather than dropping a lane (`tickets-state.svelte.ts:323-329`). This is correct.
- `setFilter` uses a sequence guard (`:239-251`). `openTicket` does not (`:340-351`): opening ticket A and then B quickly can show A.
- `load()` runs from a `$effect` that reads `screen`, and `screen` is `$derived` from `page.params.id` (`tickets/+page.svelte:12-15`). There is no dispose, unlike Integrations, but there is no polling either, so this is harmless.

**Mockup/spec diff.** There is no PNG mockup. Spec `docs/spec/screen/workspace-tickets.md` Done gates 1–6 (searchable sprint filter, AI-only switch, `no_dates` line, "—" for unknown values, and "as <old key>") cannot be verified without a connected sprint board.

**Checklist.**
- [ ] Open Tickets for sensei. Expect the not-connected state and the Integrations link. **verify**
- [ ] Follow the link. Expect `/workspace/{id}/integrations` inside the same window. **verify**
- [ ] (With a Jira sprint board) check Done gates 1–6 of workspace-tickets.md. **question (untestable today)**
- [ ] Open two tickets in quick succession. Expect the second. A race is possible. **polish** (tickets-state.svelte.ts:340)

---

## Integrations

**Purpose.** Connect the workspace's Jira boards, each at a level (epic, sprint or task); then sync, replace the token, or disconnect. The state is client-side and polls while a first sync runs (`integrations-state.svelte.ts`).

**API.**
| Call | Curl | Notes |
|---|---|---|
| `GET /api/workspaces/{id}/trackers` (`api.ts:749-750`) | 200, **2.55s**, `[]` | slow for an empty list |
| connect / sync / disconnect (not run) | | |

- A failed read goes to `daemon_error` with Retry, never to "not connected" (`integrations-state.svelte.ts:152-162`). This is correct.
- The token is cleared after every connect attempt (`:176`). This is correct.

**Interactions.**
- Disconnect has a confirmation dialog that names what is removed (`IntegrationsScreen.svelte:304-322`). Good.
- Connect is enabled only when level, board, email and token are filled (`:124-128`). Level has no default, by design.
- "Create an API token" opens the external URL through `openUrl`, falling back to `window.open` (`IntegrationsScreen.svelte:25-27`).
- **Polling never stops** while a board stays `pending`: it reloads every 2s with no limit (`integrations-state.svelte.ts:294, 297-303`). A first sync that never finishes polls forever.
- Only Jira is offered: the radio is hard-coded and checked (`IntegrationsScreen.svelte:199`), and `cardTitle` always prefixes "Jira" (`integrations-state.svelte.ts:133`).

**Mockup/spec diff.** There is no PNG mockup. Spec: `docs/spec/screen/workspace-integrations.md`. The tracker kinds are Jira only; check the spec for others.

**Checklist.**
- [ ] Open Integrations. Expect the not-connected form quickly. The list read takes 2.5s. **polish**
- [ ] Connect a board with a bad token. Expect an alert with a link and focus moved to it. **verify**
- [ ] Disconnect. Expect the confirmation dialog. **verify (good)**
- [ ] Leave a board stuck in pending. Expect polling to stop eventually. Code polls every 2s indefinitely. **polish**

---

## About

**Purpose.** The workspace's identity document: editable status, client, goal and description; the dōjō binding; repos; stack.

**Shows.** `about/+page.svelte` contains:
- the Identity form (`:112-213`)
- Bindings (`:215-237`)
- Repos (`:239-254`)
- Stack (`:256-267`)

The header is `PageHeader title=name description=goal??vision` (`:109`).

**API** (`about/+page.ts:13-17`, plus the layout's workspace from `/api/workspaces`)

| Endpoint | Status / time | Notes |
|---|---|---|
| `GET /api/workspaces/{id}/repos` | 200 · 0.22 s | `{repos:[{id,name,path,kind}]}` — matches |
| `GET /api/workspaces/{id}/dojo-suggestion` | 200 · 0.92 s | `{suggestion:null}` → "no dōjō binding" |
| `GET /api/dojo/memberships` | 200 · 1.5 s | 2+ memberships, `bound_workspaces: []`, `sync_status: authenticating` |
| `PUT /api/workspaces/{id}` | not called | Fired on **every keystroke** |

Findings:
- **Stack never renders.** The wire sends `stack` as a flat `string[]` (live: `["rust","sveltekit","svelte","tauri","typescript","postgres","ruby","markdown"]`). The page reads `p.stack?.languages` and `p.stack?.frameworks` (`+page.svelte:262`), which are undefined on an array. The type is also wrong: `WorkspaceListItem.stack` is declared `{languages?…}` (`app/src/lib/types.ts:290`). Result: a "STACK" heading with **no chips**.
- **A cleared field never saves.** `toPatch()` drops empty strings (`about-metadata-state.svelte.ts:104-117`). If you clear Client or Goal, the status shows "saved" but the old value is still stored and comes back on reload. This is the spec's wrong-gate "edit resets on next page load" (`spec/screen/project-about.md`).
- **One PUT per keystroke with no debounce** (`+page.svelte:163-166, 184-187, 205-208`). Responses can land out of order, so the last response to arrive wins, not necessarily the last edit. The `saveTimer` only resets the label.
- **Status default is fabricated.** A null maturity is shown as `discovery` (`about-metadata-state.svelte.ts:41,96`), and any later save writes `maturity: discovery` because `toPatch` always includes it. sensei is stored as `discovery` today, while being in active daily development.
- **Typing can be wiped.** The `$effect` re-hydrate (`+page.svelte:52-54`) runs whenever layout data reloads (e.g. any `invalidateAll`) and discards text you are typing.
- **Load failures:** a repos failure shows the error but still renders an empty list header (`Repos` without a count), which is acceptable. A binding failure shows the error, which is correct.

**Interactions**
- The four fields auto-save.
- **Confirm** (inferred binding) POSTs `/dojo-binding` without a confirmation dialog. This routes future findings to a dōjō, so it is consequential.
- Repo rows are not links and have no actions.

**Mockup/spec diff** (`p9-about.png`, `spec/screen/project-about.md`)

Missing from the app:
- the left identity card with REPOS/SKILLS/LINKS/BACKLOG counts
- the section nav (Identity/Stack/Repos/Links/Guidelines/Backlog)
- an editable **Name** and **Icon** ("kanji · 雲 change")
- Stack by category with "+ add"
- repo rows with LOC, stack and **remove**
- Links, Guidelines and Backlog
- the spec's multi-repo add/remove/primary/role, **Split**, preferred-assistant default, and the client-precedence note

Added by the app: Status (maturity), Description, Bindings.

**Checklist**
- [ ] Look at the Stack section. Expect chips (rust, sveltekit…). Code: an empty heading, because of the array-vs-object mismatch (`+page.svelte:262`, `types.ts:290`). **bug**
- [ ] Type in Client, then clear it fully and reload. Expect it empty. Code: the old value returns (`toPatch` drops `''`). **bug**
- [ ] Type quickly in Goal and watch the status. Expect one save. Code: one PUT per keystroke. **polish/bug (race)**
- [ ] Check Status for sensei. Expect the real stage. Shows "Discovery" (stored), and the null default also writes "discovery". **question**
- [ ] Look for Name/Icon editing, repo remove/role, Split, Links. Expect them per the mockup/spec. Absent. **gap vs mockup/spec**
- [ ] The Bindings section reads "no dōjō binding". With 2 personal memberships and no org slugs, confirm that is expected. **question**

---

## Top issues, ranked

Ranked by how much each one damages trust or blocks use. Severity is in brackets, and the evidence is in each screen's section.

1. **[bug, blocker] Layers freezes the page.**
   - The evidence rules out the daemon (100 KB, 2.5 s), the layout (rokkit `layers()` runs in 3.5 ms on the live payload and is linear), the page's `$effect` (Cycles uses the identical state and does not freeze), and graph size (Structure renders 2404 nodes).
   - What is left is the browser-only measured viewport: `Graph.svelte:185-187` binds `clientWidth/clientHeight`, and `canvas/frame.js:6,16` sizes the extent to exactly the viewport inside an `overflow:auto` paper. On Layers, `drawn.h + 2·pad ≈ vh`, which is the setup for a scrollbar ↔ ResizeObserver oscillation.
   - This is a strong hypothesis, not proven. A three-step bisect is in the Layers section.
2. **[bug] Overview, the landing page, can take minutes to appear.**
   - `overview/+page.ts:12-16` puts `/hotspots` in the same `Promise.all`, and `tryGet` has no timeout. Measured times were 7.9 s to 196 s.
   - Hotspots also counts **30 days** under a "7 days" label (`workspace_hotspots.ddl:22`), calls re-edits "corrected", and lists `.claude/worktrees/…` copies.
3. **[bug, rule violation] Impact makes up a baseline.**
   - `COALESCE(r.baseline_ftr, 0)` (`crates/senseid/src/db/pg_store/metrics.rs:71`) turns a missing baseline into 0%. Every "positive" verdict then reads "Strong positive, +75pp, 0% → 75%", while the API's `baselineFtr` is null.
   - 89 of 96 rows were never accepted but are shown as "measurement pending".
4. **[bug] Workspace Instruments takes 40–69 s to open, and then shows zeros.**
   - It blocks on `/mcp-tool-stats`: 68.7 s, 39.6 s, and 39.8 s re-measured.
   - That endpoint reads a retired list of 20 tools matched by bare name (`workspace_detail.rs:370-393`), so every tool shows 0 calls. The same page's "Tools used" shows `log_event` with 124.
5. **[bug] Metric chips compare different periods, on stale data.**
   - Each card shows the latest *daily* value beside a *weekly* prior and delta (`handlers/metrics.rs:233-249`). Live: throughput value 1, prior 2, delta **+6**. FTR delta −0.125, while its explainer says "up 0.14".
   - The Health hero shows "71 +6", where +6 is 60−54 from the weekly trend.
   - Values under "this week" go back as far as 09-17, and no date is shown.
   - The spec's coverage readout ("from 18 of N") is missing.
6. **[bug, rule violation] Metrics shows failed reads as empty states.**
   - A failed `/health` read becomes "No rated metrics yet" (`metrics/+page.ts:70`, `HealthRadar.svelte:75`).
   - Failed correlation and series reads become `[]` with no message (`+page.ts:59,75`).
   - The ResizeObserver warning is traced to the RadarChart → rokkit `PlotSurface.svelte:41-52` (code path only, not profiled).
7. **[bug] Session counts disagree across screens, and sessions look broken upstream.**
   - The sidebar and Overview say "Sessions · 7d **0**". The workspace list says 1, with a session today (`/ftr` counts graded sessions, `pg_store/metrics.rs:1248-1257`).
   - Overview's newest recent row is 10-18, and all its rows are titled "session".
   - One session (`bbcb2965`) has run since 10-15 (6.6 days, about 5 billion input tokens), which suggests everything since is being merged into it.
   - The Sessions 7d chart is empty: the window is picked by end time and the chart places sessions by start day.
8. **[bug] About loses data.**
   - Stack never renders: the wire sends `string[]`, and the page reads `.languages`/`.frameworks` (`about/+page.svelte:262`).
   - Clearing a field says "saved" but keeps the old value (`about-metadata-state.svelte.ts:104-117`).
   - Every keystroke sends a PUT with no debounce, and a null status is written back as `discovery`.
9. **[bug] The diagrams give wrong advice.**
   - Cycles says "Cut A → B … and the loop opens". At module grain that is false for every cycle (`layering-state.svelte.ts:217`).
   - Layers marks 39 `up` arrows "to cut" (`:203`), while Cycles names different cuts.
   - The Code map Matrix lens shows `fold:L0` rows; clicking one requests `…/codemap/module?id=fold:L0`, which returns 404 and an error banner (`lenses.ts:31`, `MatrixLens.svelte:17`).
   - The Cycles lens drops folded members' bands (`lenses.ts:13-16`).
10. **[bug] Libraries contradicts itself.**
    - The list shows 8 rows: sensei's own crates, none wrapped.
    - The banner shows 29 npm version conflicts from a different source.
    - The "Conflicts" filter gives "No libraries match this filter" (`library.rs:1673` vs `libraries/+page.svelte:33`).
11. **[bug] Memories says "Nothing to remember yet." while the API returns `total: 1`.** Non-active memories are never shown, and the daemon takes 100 rows before filtering (`memory.rs:306-322`).
12. **[bug] Schema's Dependencies scope cannot inspect anything.** Clicking a view or function is undone at once, because `selected` accepts only tables and the diagram is controlled (`schema/+page.svelte:107`).
13. **[bug] The shell has no error page, and the sidebar is clipped at small heights.**
    - No `+error.svelte` exists in `app/src/routes`, so a daemon that is down or an unknown id shows SvelteKit's bare page, which cannot be dragged and has no way back.
    - The sidebar needs about 740 px and does not scroll (`WorkspaceSidebar.svelte:58-61`), while the window's minHeight is 600 (`windows.svelte.ts:48`).
14. **[bug] Single clicks with lasting effects.**
    - Intake records two runs on a double-click, and records with no click when `auto_select` is set (`intake.svelte.ts:55`).
    - These act with no confirmation: Overview Reject (`overview/+page.svelte:131-138`), Memories Approve/Reject batch (`memories/+page.svelte:93-107`), Impact verdict buttons (`impact/+page.svelte:234-242`), and About "Confirm" dōjō binding.
    - Overview's "send to {acp}" has no handler (`overview/+page.svelte:121`).
15. **[gap] Mockups and specs vs what is built (largest gaps).**
    - Shell: no "switch project", no per-section counts, no Drift-watch row. The PRD's eight-entry Diagrams list is not built, and the retired Layers/Neighbourhood tabs remain.
    - Metrics: no 30d/60d/all toggle, no Confidence card, no Gained/Lost ground lists, no worst-first table (`metrics-board.png`).
    - About: no Name/Icon/Links/Guidelines/Backlog/Split (`p9-about.png`).
    - Patterns: no Promote/Dismiss.
    - Code map: no search, single-click-to-open, lens headers or band filters.
    - Neighbourhood: none of `arch-calls.png`.
    - The workspace sidebar is still hand-rolled rather than rokkit `List` (#293 step 2).

**Cannot be checked here:**
- Tickets: sensei has no sprint board connected.
- Traceability is empty until markdown documents are indexed (#292), yet "Scan now" stays enabled.
- Sessions: `p2-sessions.png` is a blank image, so there is no visual reference.
