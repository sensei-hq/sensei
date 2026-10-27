# Main window review — 2026-10-27

An adversarial, read-only functional review of the desktop app's main window, written for the screen-by-screen walk on
2026-10-28. Each screen was read from its `+page.svelte`, loader, state module and the `api.ts` calls it makes. Every
endpoint it reads was called with GET against the running daemon (0.21.0, `localhost:7744`), and the screen was
compared with its mockup under `docs/mockups/Sensei/`. No code, config, database or git state was changed. Write
endpoints were traced, never called. `/api/sessions/{id}/replay` was called without `classify=true`, because that
GET writes verdict rows.

Each section has Purpose, Shows, API (endpoint → verdict), Interactions, Mockup diff and a Checklist to try by hand.
Severities: **bug** / **gap** (vs mockup) / **polish** / **question**. Paths are relative to the repo root unless they
start with a screen's own file name.

Context: `docs/baseline/2026-10-27-app.md` (crawl) and `docs/roadmap/08-issue-burndown.md` § "UI walk". Settings has
its own window and is not covered here.

Spot-checked by hand after the section reviews: the fixed "daemon · running" text
(`ObservatorySidebar.svelte:45`), `COALESCE(r.baseline_ftr, 0)` (`crates/senseid/src/db/pg_store/metrics.rs:71`),
`ftr?: number` in `app/src/lib/types.ts:489` against the live `/api/sessions`, which returns `ftr` as booleans, and
`LIMIT 200` on pending recommendations (`crates/senseid/src/db/pg_store/patterns.rs:1067`).

## Shell — `+layout.svelte`, `ObservatorySidebar`, command palette, native menu

**Purpose.** The frame every main-window screen sits in: the rail (rokkit `List`), the ⌘K command palette, the
settings-window redirect, and (from the root layout) the DB-degraded banner and the sign-in overlay.

**Shows.** A 220px rail: Today, Workspaces (badge = workspace count), group "Needs you" (Insights, Memories, Impact,
Traceability, Upgrades), group "Review" (Sessions, Libraries, Consolidation, Instruments, Logs, Dōjō, Sharing, Sources,
Share review), separator, Settings (`observatory-nav.ts:73-115`). Footer: `daemon · running` / `port 7744`
(`ObservatorySidebar.svelte:44-47`). The palette carries "Go to <rail entry>" for every rail link plus "Open workspace ·
<name>" for every workspace (`+layout.svelte:23-64`).

**API.**
- `GET /api/config` (appState.load, via `(observatory)/+layout.ts:9`) → 200. On failure `appstate.svelte.ts:162-170`
  sets `config = {}` and carries on: every preference (glyph mode, etc.) silently reverts to defaults with only a
  `console.warn`. Deliberate (boot gate owns routing), but it is a default substituted for a failed read.
- `GET /api/workspaces` → 200, 133 rows, fields `id,name,client,…` match what the palette and badge read. Called twice on
  mount (`+layout.svelte:48` via `loadWorkspaceCount`, and `+layout.svelte:51`) — same list fetched back to back.
  A failed read: badge hidden (`appstate.svelte.ts:188`), palette workspace commands absent, error only in console.
- No health endpoint is read by the rail: the footer's "daemon · running" is a literal string.

**Dead-link check.** Every rail href resolves to a route under `routes/(observatory)/` (or `/settings`, which
`+layout.svelte:78-84` intercepts and opens as the settings window at `/settings/general`). Not in the rail or palette:
`/help` (only Help → Keyboard Shortcuts, `src-tauri/src/lib.rs:343`), `/logs` (⌘⇧L, `routes/+layout.svelte:95-97`),
`/health` (View → Health).

**Interactions.**
- Rail links → client navigation; active entry resolved by longest prefix (`observatory-nav.ts:138-149`).
- ⌘K → palette (`+layout.svelte:31-39`, `global: true`, so it fires inside text fields too).
- Palette "Open workspace · X" → `openWorkspaceWindow` (new window). 133 entries, unfiltered by watched root.
- Native menu: View → Toggle Sidebar (`lib.rs:78`) and Help → What's New (`lib.rs:110`) have no handler — they fall
  into `_ => {}` (`lib.rs:251-288`). App → Check for Updates… emits `update-check-requested` (`lib.rs:257-258`), which
  nothing in `app/src` listens for (ripgrep: no match) — click, nothing happens, no feedback.

**Mockup diff** (`assets/screens/06-today.png` rail; `hq/_sidebar.png` is a slideshow frame of the same rail).
- Mockup shows counts on most entries (Projects 5, Sessions 41, Insights 6, Memories 24, Upgrades 5, Impact 3,
  Traceability 4, Libraries 14, Instruments 7). Live: only Workspaces carries a count; the others were removed as
  fabricated placeholders (`observatory-nav.ts:55-59`) and never wired to real counts.
- Mockup nests Memories → Anatomy / Sharing / Consolidate and Instruments → Playground / Replay / Health. Live is flat;
  Consolidation and Sharing are separate Review entries, and the Instruments sub-screens are page tabs.
- Mockup order: Today, Projects, Sessions, Logs, Insights, … Collective intel. Live regroups into "Needs you" / "Review"
  and renames Collective intel → "Sharing".
- Live "Memories" rail label routes to `/learnings` — the URL and the label disagree.

**Checklist.**
- [ ] **bug** — Stop the daemon (`brew services stop sensei` or kill senseid) while the app is open and look at the rail
  footer. Expect a down/unknown state; code always prints `daemon · running` (`ObservatorySidebar.svelte:45`, and the
  spec pins it: `ObservatorySidebar.spec.svelte.ts:102`). A status that cannot be false.
- [ ] **bug** — App menu → Check for Updates…. Expect a result or progress; nothing listens for the event
  (`src-tauri/src/lib.rs:257-258`, `commands/update.rs:51-61` is a stub).
- [ ] **bug** — View → Toggle Sidebar; Help → What's New. Expect an effect; neither id has a handler (`lib.rs:78,110`,
  match falls to `_ => {}`).
- [ ] **question** — ⌘⇧L opens `/logs` (the (health)-group log viewer), while the rail's "Logs" opens
  `/activity-logs`. Two different "Logs" screens — is that intended?
- [ ] **question** — ⌘K, type "help": no Help entry (palette is built from rail links only, `+layout.svelte:23-30`).
  The only way to /help is the Help menu.
- [ ] **polish** — ⌘K, scroll the Workspaces group: all 133 workspaces, unwatched included (`+layout.svelte:51-63`),
  where Instruments' "Runs in" was narrowed to watched roots (burndown UI walk #10).
- [ ] **gap vs mockup** — Rail counts: only Workspaces has a badge (133). Mockup has per-entry counts.
- [ ] **question** — Break `/api/config` (e.g. daemon restarting) and reload: preferences such as glyph mode silently
  revert to defaults (`appstate.svelte.ts:162-170`), console warning only.
- [ ] **polish** — `GET /api/workspaces` fetched twice on every main-window mount (`+layout.svelte:48` and `:51`).

## `/` — Today

**Purpose.** The daily landing page: one headline teaching (hero koan), supporting insights, what sensei has adopted,
recent sessions, and the 14-day First-Try-Right header.

**Shows.** `+page.ts:12` fetches Today + FTR in parallel; `todayPageData` (`today-view.svelte.ts:103-113`) keeps each
read's error. Header: date eyebrow, greeting, FTR % with delta arrow, 14-bar strip (`+page.svelte:26-61`). Body: HeroKoan,
"Also worth noticing" (InsightCard ×≤3), "System has learned" (AdoptedCard ×≤5), RecentSessions (≤5). A failed Today read
renders `ScreenState status="error"` with retry (`+page.svelte:63-66`); a failed FTR read shows "unavailable"
(`+page.svelte:39-40`). Error handling on the client is correct.

**API.**
- `GET /api/observatory/today` → 200. Keys `greeting, today, dataMaturity, hero, insights, adopted, recentSessions`
  — all read by the page; shapes match. Live: `dataMaturity: "mature"`, hero = top pending recommendation (a security
  dependency update), 3 insights (all `label: "Recommendation"`, `tag: "high"`), 5 adopted, 5 recent sessions.
  Content issues are in the daemon:
  - `hero.source` and `hero.noticed` are both `""` live. `noticed` is hard-wired empty: `mature_hero(top, &sources, "")`
    (`crates/senseid/src/api/handlers/observatory.rs:1041`). `source` is empty whenever the rec has no session ids in
    `evidence` (live: `evidence: []`). So the hero provenance line never shows.
  - `hero.action` is always the literal `"Open insights"` (`observatory_home.rs:80`); the button goes to `/insights`
    (`HeroKoan.svelte:14,37`), not to the recommendation itself.
  - Insight cards and the hero carry no workspace; `/api/insights` has the recommendation's workspace `name`, Today
    drops it (`observatory_home.rs:113-121`). A "Security: update X" card does not say which workspace.
  - `adopted[].source` is filled with the memory's `impact` text, or the invented string `"adopted by sensei"` when that
    is empty (`observatory.rs:1106-1109`); `scope` defaults to `"workspace"` when absent (`:1105`). Live rows show the
    rationale text as the "source", and one row literally "adopted by sensei". A default substituted for a missing
    value (the CLAUDE.md "plausible default" rule).
  - `adopted[].scope` renders as the word "workspace" (`AdoptedCard.svelte:14`), not the workspace's name.
- `GET /api/observatory/ftr` → 200 `{"ftr14d":0.9,"ftr14dPrev":0.818…,"ftrTrend":[0.0,0.875,0.0,0.0,1.0,0.0,1.0,0.0,…],
  "sessions7d":0}`. Headline null-safe. **The trend is 0-filled for days with no scored session**
  (`crates/senseid/src/db/pg_store/metrics.rs:1341`, `v.unwrap_or(0.0)`), and FtrStrip draws `rate 0` as a minimum
  bar (`FtrStrip.svelte:37`). A day with no sessions renders the same as a day where every session needed rework. Live:
  10 of 14 points are 0.0. Also the headline counts every `ftr IS NOT NULL` session while the daily points exclude
  `outcome = 'empty'` (`metrics.rs:1313-1333`), so the bars and the headline use different populations.

**Interactions.**
- "all insights →" → `/insights`; "all teachings →" → `/learnings`; "all sessions →" → `/sessions` — all exist.
- Hero button → `/insights` (generic, see above).
- InsightCard → `/insights` (every card, same target; `InsightCard.svelte:12`).
- AdoptedCard: not clickable, no link to the memory (`AdoptedCard.svelte:7-20`).
- Recent session row → `/sessions#<id>` (`RecentSessions.svelte:60`). No element on `/sessions` has that id and no
  code reads `url.hash` (ripgrep over `routes/` and `lib/`: none) — lands at the top of Sessions, session not selected.
- Error state Retry → `invalidateAll()`.

**Mockup diff** (`assets/screens/06-today.png`).
- Greeting: mockup "Good morning, Aiko." — live has no name ("Good afternoon.").
- FTR strip: mockup labels "14d ago" / "today" and accents today's bar; live has no axis labels.
- Hero: mockup has a vertical "SENSEI SPEAKS" label, a specific action ("Draft a persona"), projected impact
  ("Projected FTR +14% in Lumen Cloud") and provenance ("from s-2891 · s-2889 · s-2886 · noticed 2 days ago"). Live:
  generic "Open insights →", `impact: null`, no provenance, no label.
- Insights: mockup mixes PATTERN RECURRING / TEACHING ADOPTED / DRIFT DETECTED with varied tags ("3rd time", "+7% FTR",
  "low urgency"). Live: only `Recommendation` cards with an urgency tag — the daemon only feeds pending recommendations
  (`observatory.rs:1068`).
- Adopted: mockup shows the workspace name ("lumen-studio") and a source ("from session s-2890"). Live shows the literal
  scope word and the memory's rationale text.
- Recent sessions: matches in shape (dot, workspace, task, corrections, duration, time).

**Checklist.**
- [ ] **bug** — Look at the FTR strip in the header. Days with no sessions show as short bars indistinguishable from
  0% days; expect a gap/"no data" mark. Evidence: `metrics.rs:1341` (`unwrap_or(0.0)`), live trend has 10 × `0.0`.
- [ ] **bug** — Click a Recent sessions row. Expect that session opened/highlighted on Sessions; code links to
  `/sessions#<id>` (`RecentSessions.svelte:60`) and nothing handles the hash — you land at the top of the list.
- [ ] **bug** — Read the "System has learned" cards: the grey line under each is the memory's rationale, or
  "adopted by sensei" (invented, `observatory.rs:1106-1109`); the accent label says "workspace" instead of which
  workspace (`AdoptedCard.svelte:14`).
- [ ] **bug** — Recent session dot colour: a session whose `ftr` is unknown (null) is painted amber like a reworked
  one (`RecentSessions.svelte:57,65-68`: `(s.ftr ?? 0) >= 1`).
- [ ] **gap vs mockup** — Hero: no provenance (source/noticed), action is always "Open insights" and opens the general
  Insights board, not the recommendation (`observatory.rs:1041`, `observatory_home.rs:80`, `HeroKoan.svelte:14`).
- [ ] **gap vs mockup** — Insight cards and hero do not name the workspace the recommendation is for, though
  `/api/insights` has it (`observatory_home.rs:113-121`).
- [ ] **gap vs mockup** — Only recommendation-type insights ever appear; no pattern/teaching/drift cards.
- [ ] **polish** — Recent session duration for the long-running sensei session renders as hundreds of hours
  ("297h …"): `formatDuration` has no day unit and uses wall-clock span, not `durationSecs`
  (`RecentSessions.svelte:26-39`). The time column shows only HH:MM of `startedAt` with no date, so a 12-day-old
  session reads like today (`RecentSessions.svelte:41-45,88`).
- [ ] **polish** — When there are no recent sessions the whole section disappears (`RecentSessions.svelte:48`) — no
  empty state, unlike the two cards above.
- [ ] **question** — The header shows `ftr14d` 90% while `sessions7d` is 0; the bars and the headline count different
  session populations (`metrics.rs:1313-1333`). Do the numbers read right to you?
- [ ] **question** — In the early state the two insight cards ("Listening", "Calibrating") are fixed copy from the daemon
  (`observatory_home.rs:124-137`), not data. Acceptable as status copy?

## /workspaces

**Purpose.** The index of every workspace: find one and open it in its own window.

**Shows.** A page header (場, "All the places you work.", a "⌘K to jump" hint). Below it: filter chips (All / Active / Dormant / Archived, each with a count), a grid/list toggle, a search over name and client, and an "N of M" count. Then cards or rows. An active card shows ftr / repos / libs. A dormant or archived card shows repos / libs / last session. Each card has a status dot, a client pill and a status pill. Clicking a card opens `/workspace/{id}` in a new Tauri window (`routes/(observatory)/workspaces/+page.svelte:48-50`, `lib/stores/windows.svelte.ts:28-58`). Status comes from the data: archived when `maturity === 'archived'`, active when `sessions7d > 0`, otherwise dormant (`buckets.ts:69-74`).

**API**

| Endpoint | Verdict |
|---|---|
| `GET /api/workspaces` (`api.ts:287`) | Shape matches. Every field the card reads is present: id, name, client, maturity, icon{kind,value,source,available}, vision, repos_count, libs_count, last_session_at, sessions7d. A failed read shows as an error with Retry (`workspaces-load.ts:24`, `+page.svelte:137-138`), as the project rule requires. **Data:** 133 workspaces. All 133 have `maturity: discovery`, so the Archived chip is always 0. All have `client: null` and `vision: null`. 129 of the 133 have `repos_count: 0`. In `sensei.folder_workspaces`, only **4** workspaces have any folder linked (psql). The other 129 have no folder at all, yet 83 of them still carry `libs_count > 0` (for example "Charts": 23 libs, `/repos` → `[]`). |
| `GET /api/workspaces/{id}/ftr` (`api.ts:307`), active workspaces only | Shape OK (`ftr14d`). Read failures surface as "unavailable" on the card (`buckets.ts:50`, `WorkspaceCard.svelte:74-79`). **Inconsistent with the list:** the list says sensei has `sessions7d: 1`, but `/ftr` says `sessions7d: 0`. The list counts by `coalesce(completed_at, started_at)` (`crates/senseid/src/db/pg_store/workspaces.rs:401`). The ftr window counts by `started_at`. |
| `GET /api/workspaces/{id}/quality-signals` (`api.ts:602`) | Shape OK. **The daemon fills a missing value with 0:** `ftr_7d` is `COALESCE(ftr.ftr_7d, 0)` in the `sensei.workspace_quality_signals` view, and the store's miss branch also returns `"ftr_7d": 0` (`crates/senseid/src/db/pg_store/metrics.rs:261-263`). The app's `isWarning` turns any `ftr_7d < 0.6` amber once `sessions7d > 0` (`buckets.ts:113-119`). Live result: sensei (ftr14d 0.875) and gateway (ftr14d 1.0) both read `ftr_7d: 0.0`, because no session with an outcome started in the last 7 days. So **both active workspaces show an amber "needs attention" dot and an amber FTR figure**, even though both are healthy. |

**Interactions**

- The filter chips, view toggle and search are pure client state (`workspaces-view.svelte.ts`). The view choice persists to localStorage; the filter and the query do not.
- Card or row click calls `openWorkspaceWindow`. If that fails, the error goes only to `console.error` and the user sees nothing (`+page.svelte:49`).
- If the window is already open, the click focuses it (`windows.svelte.ts:32-35`). The route `/workspace/{id}` exists (`routes/(workspace)`), so the link resolves.
- The "⌘K to jump" hint matches a real palette command, `workspace.open:{id}` (`routes/(observatory)/+layout.svelte:42-45`). The page has no "+ new workspace" button.

**Mockup diff** (`07-projects.png`, `o4-first-entry-projects.png`)

- The mockup has a **"+ new project"** primary button at top right. The app shows a "⌘K to jump" chip there, and you cannot create a workspace from this screen.
- Mockup active cards show **FTR · 7D · REPOS·LIBS** ("2·9"). The app shows ftr · repos · libs, and the 7-day session count is not shown.
- Mockup dormant/archived cards are a single quiet line ("1 repo · 4 libs … last · 2mo ago") plus a DORMANT/ARCHIVED label. The app uses a 3-cell stat grid.
- The mockup shows the client ("internal", "external") as a sub-line under the name. The app uses a pill, but with `client` null everywhere it never appears.
- The app adds a grid/list toggle that is not in the mockup.
- `o4` shows a first-session onboarding banner above the index ("Give sensei something to watch", "Sessions watched 0/3"). The app has nothing like it.
- At real scale the mockup's 6-card grid becomes 133 cards, 129 of them dormant.

**Checklist**

- [ ] Look at the sensei and gateway cards. Expect a green dot and a plain FTR (88% and 100%). Code and API suggest an **amber dot and amber FTR** labelled "needs attention", because `ftr_7d` is a fabricated 0 (view `COALESCE(ftr_7d,0)`; `pg_store/metrics.rs:261`; `buckets.ts:118`). **bug**
- [ ] Count the cards. Expect only workspaces under a watched root. You will get 133, and 129 of them have no folder linked at all and read "0 repos" (the `folder_workspaces` count). `?watched=true` also returns 133. Burndown item 10 narrowed "Runs in" to watched roots, but this index was not narrowed. **gap / question:** should orphan workspaces be pruned or hidden?
- [ ] Open a dormant card such as "Charts". Expect repos and libs to agree. You get "0 repos · 23 libs": libraries are enabled for a workspace that has no folder (`workspaces.rs:396-398`). **bug (data)**
- [ ] Click the Archived chip. It is always 0, because every workspace is `discovery`, and nothing on this screen can archive one. **question**
- [ ] Compare sensei's "7d sessions" between this index (1) and its workspace window (`/ftr` says 0). The two endpoints use different windows (`workspaces.rs:401` vs the ftr query). **bug (polish)**
- [ ] Click a card while the Tauri window API fails (for example in a plain browser). Nothing happens and the error goes only to the console (`+page.svelte:49`). **polish**
- [ ] Look for the mockup's "+ new project" button. It is missing (`+page.svelte:72-76`). **gap vs mockup**
- [ ] Look at an active card for the "7D" sessions stat. It is missing (`WorkspaceCard.svelte:72-85`). **gap vs mockup**
- [ ] Letter-fallback icons (39 workspaces have `source: letter_fallback`, such as "A" for a workspace named A…) render as a Latin letter in the kanji slot. **polish / question**
- [ ] A stale comment says buckets are "active by FTR", but they are sorted by recency (`+page.svelte:33` vs `buckets.ts:100`). **polish**

## /sessions

**Purpose.** A retrospective across all workspaces: how sessions went over 7, 30 or 90 days, with one chart you can flip between views, and each session linking to Replay.

**Shows.** A hero (録, "The shape of your week.") with 7d/30d/90d range chips. On the right is a mini cycler (numbers → trend → stream → constellation → bands → pulse). Below that: a totals strip ("N sessions across W workspaces · median X" plus first-try / corrected / abandoned counts). Then a chart with **five** chips: trend, stream, constellation, bands, tokens. Then the session list (newest first). A row click calls `goto('/instruments?tab=replay&session={id}')` (`lib/components/sessions/SessionsDigestScreen.svelte`, `lib/sessions-digest.ts:368-370`).

**API**

| Endpoint | Verdict |
|---|---|
| `GET /api/sessions?range=7d\|30d\|90d` (`api.ts:687-692`) | The `sessions[]` shape matches `SessionRow`: id, workspace, task, summary, outcome, ftr, turns, corrections, startedAt, completedAt, agent, tokensIn/Out, durationSecs, provider, model. `folderRole` is never sent, so the row's folder chip is dormant (as the comment at `types.ts:520-523` admits). A failed read shows as an error with Retry (`sessions-digest.svelte.ts:49-52, 122-128`). Live: 7d → 2 rows, 30d → 29, 90d → 156. |
| Range filter semantics | The daemon keeps sessions **active** in the window: `coalesce(completed_at, started_at) >= now() - N days` (`crates/senseid/src/db/pg_store/sessions.rs:48`). The client buckets the charts by `dayKey(startedAt)` over the last N calendar days (`sessions-digest.ts:216-226, 286-292`), but computes totals over every row (`computeTotals`, `:348`). Live 7d: both rows started **before** the 7-day axis (sensei on 2026-10-15, gateway on 2026-10-20; the axis covers 10-21 to 10-27). The header therefore says "2 sessions · 1 first-try · 1 corrected" while **every chart is empty**. |
| Duration / median | `mins` is wall-clock time (`completedAt - startedAt`, `sessions-digest.ts:66-72`), and the median and row duration use it. The live sensei row ran from 10-15 to 10-27, about 17,970 minutes, so the 7d median reads about "174h 43m". `durationSecs` (active time) is 567,985 s, about 158 h, which is also implausible as "active work". |
| Tokens | The sensei row has `tokensIn: 4,987,861,897` (5 billion). `compactTokens` has no "B" unit and renders "4994M" (`sessions-digest.ts:92-100`). The size suggests cache reads are being summed per turn. **The data needs checking, not just the display.** |
| `?range=<unknown>` | Returns 200 with **no filter**: 403 rows, unbounded (`handlers/sessions.rs:14-21`). The app never sends it. **Low.** |
| `?workspace=<unresolvable name>` | Resolves to None, which means **no scope**: all sessions come back as if they were that workspace's (`handlers/sessions.rs:28-35`). This affects the workspace tab, not this screen. |
| Ingestion | Note for the walk: the newest `started_at` in `activity.sessions` is **2026-10-20**. No session has started in the last 7 days (psql), despite daily use. Either the transcript capture stopped or sessions are being folded into long-running ones (the sensei row's `completedAt` is today). **question** |

**Interactions**

- The range chips call `setRange`, which refetches; "updating…" shows while the request is in flight. A failure drops the old rows and shows the error (`sessions-digest.svelte.ts:113-128`).
- The chart chips and mini cycler are pure client state.
- A row click deep-links to Replay. `instruments/+page.svelte:29-34` reads `?tab` and `?session`, and `GET /api/sessions/{id}/replay` answers 200. The link works.
- There is no workspace filter on the observatory view, no search, and no paging. The daemon limit is 500 rows (`handlers/sessions.rs:47`).

**Mockup diff.** `09-sessions.png` is a **blank image**: there is no usable mockup. The component cites `docs/mockups/Sensei/lib/sessions-zen.jsx` (`SessionsDigestScreen.svelte:8`), but no `sessions*` file exists under `docs/mockups/Sensei/lib/`. **No mockup.**

**Checklist**

- [ ] Open /sessions on 7 days. Expect the charts to show the two sessions. Code suggests the header says "2 sessions" and **all five charts are empty**, because rows are filtered by completion and drawn by start day (`pg_store/sessions.rs:48` vs `sessions-digest.ts:290`). **bug**
- [ ] Read the median in the totals strip. Expect a few minutes. It shows wall-clock spans like "174h 43m", because `durationMinutes` ignores `durationSecs` (`sessions-digest.ts:66-72`). **bug**
- [ ] Switch to the "tokens" chart or read the sensei row's token column. Expect a sane number. You get "4994M", meaning 5 billion input tokens for one session; `compactTokens` has no B unit (`:92-100`). **bug (data + display)**
- [ ] Count the chart chips. A comment says "Exactly four chips. pulse is never here", but there are five, with tokens added (`SessionsDigestScreen.svelte:183`, `sessions-digest.ts:20`). **polish**
- [ ] Look for today's sessions. None started after 2026-10-20 (psql `max(started_at)`). **question:** is capture stalled?
- [ ] Rows with outcome `empty` (26 in 90d) or null (5) render as neutral and are counted in "N sessions". **question:** should empty sessions be listed at all?
- [ ] One 30d row has `workspace: null` and shows "unlinked" (`SessionRow.svelte:61`). Its title comes from `summary`, which for the sensei row is a worktree path ("…/.claude/worktrees/agent-…/dojo/src/lib/server"), and `task` is empty. **polish**
- [ ] Click a session row. Expect Replay to open on that session. The wiring checks out (`instruments/+page.svelte:31`, replay 200). **ok**
- [ ] Flip range 7d → 90d → 7d. Expect "updating…" each time; a failed read should show an error, not old rows (`sessions-digest.svelte.ts:125`). **ok by code**

## /impact

**Purpose.** "Did sensei's advice actually work?" For each accepted recommendation, compare FTR before and after it was accepted, and show the verdict with its reasoning.

**Shows.** A header with counts (positive / neutral / negative / pending). Below it, an unread-workspaces notice if any read failed. Then a two-column body. On the left is a list ordered negative → positive → neutral → pending, each row with verdict, delta, title and workspace. On the right is the detail: workspace · status, title, verdict pill, a 4-cell strip (FTR baseline/current/delta, Status, Verdict, Workspace) and the reasoning panel (`routes/(observatory)/impact/+page.svelte`).

**API**

| Endpoint | Verdict |
|---|---|
| `GET /api/workspaces` then `GET /api/workspaces/{id}/impact` **for every one of the 133 workspaces**, all at once (`lib/impact-load.ts:24-28`, `lib/fan-out.ts:16`) | Shape matches `WorkspaceImpactWire`. Per-workspace failures are named in `unread` and a list failure is an error, as the rule requires. **Cost:** one sensei `/impact` call took 5.3 s, and a serial sweep of all 133 took 52 s. Every visit fires 133 requests, and 123 of them return `[]`. There is no aggregate endpoint. |
| What the rows are | 237 rows: **228 `verdict: pending` with `status: pending`**, meaning recommendations that were **never accepted**, plus 9 accepted/positive and 1 dismissed. The daemon returns any rec with a reasoning trace. The header says "Each accepted recommendation gets a measurement window", and `impact.ts:23-24` says pending means "accepted recs whose measurement window hasn't closed". Both are wrong for 96% of the list. In practice this screen is the full recommendations inbox. |
| Verdict data | All 9 "positive" verdicts have `baselineFtr: null` and `ftrDelta: null`, yet the reasoning says "FTR moved from 0% to 75%… FTR +75pp". The measurement query uses `COALESCE(r.baseline_ftr, 0)` (`crates/senseid/src/db/pg_store/metrics.rs:71`), then derives both the verdict and the narrative from that **fabricated 0% baseline**. Seven sensei recs share an identical "0% → 75%". **Every positive verdict on this screen is an artifact.** |
| `reasoning.modelsUsed` | It is `[]` on the positives. The panel (`MoeReasoningPanel`) gets no per-model notes, while the header copy still says "The MOE panel writes the reasoning". |

**Interactions**

- A list row click sets `openId` and the detail switches; nothing else is clickable.
- There are no actions: no Revise, Keep monitoring or Dismiss, no link to the rec or its workspace, and no filter by verdict or workspace.
- A row whose verdict falls outside the four known values is dropped from the buckets but still counted in `total` (`impact.ts:145`, `impact-load.ts:28`). If every row were unknown, the page would show "no measurements yet" while `data-impact-total > 0` (`+page.svelte:82`). **Low.**

**Mockup diff** (`15-impact.png`; `p8-impact.png` is the workspace-scoped tab)

- The mockup lists only **measured** items (1 positive, 1 neutral, 1 negative), each with "10 days · 31 sessions". The app lists 237 items, mostly unaccepted proposals, labelled with the workspace name.
- The mockup detail eyebrow is "R-PROMOTE-ADAPTER · LUMEN · ACTED 16 APR · MEASURED 26 APR". The app shows workspace · status, with no acted or measured dates.
- The mockup metric strip is **First-try-right (before → after, ±pp), Corrections/session, Avg session, Tool-usage shift**. The app shows FTR, then Status, Verdict and Workspace, which repeats the eyebrow.
- A **Tool usage delta** section (per-tool bars) is missing.
- The mockup's MOE panel has a per-model verdict list ("claude-haiku 好 …", a "2 positive · 1 neutral · 0 negative" tally). The app has a headline/body only (deliberately, per the #109 audit in `impact.ts:6-11`), but the header copy still promises a MOE panel.
- The **Revise / Keep monitoring / Dismiss** action buttons are missing.
- The app adds a fourth "pending" count to the header.

**Checklist**

- [ ] Open /impact and time it. Expect a quick load. It makes 133 parallel `/impact` reads, about 5 s each for the busy workspaces (`impact-load.ts:26`). **bug (performance)**
- [ ] Open any "positive" row. Expect a baseline FTR and a delta. You get baseline "—", delta "—", and current 75% or 100%, while the reasoning claims "+75pp from 0%". The baseline is fabricated in `pg_store/metrics.rs:71` (`COALESCE(r.baseline_ftr, 0)`). **bug (fabricated data, violates the project rule)**
- [ ] Scroll the list. Expect only accepted, measured recommendations. You get 228 never-accepted "pending" proposals (`status: pending`). **bug / question:** filter to accepted, or rename the screen?
- [ ] Read the header copy "The MOE panel writes the reasoning". `modelsUsed` is `[]` and no panel vote exists (`+page.svelte:48`, `impact.ts:6-11`). **polish (copy)**
- [ ] Look for the Revise / Keep monitoring / Dismiss actions. They are absent: you cannot act on a verdict here (`+page.svelte:130-203`). **gap vs mockup**
- [ ] Look for acted/measured dates, sessions-in-window, corrections and avg-session deltas. All are absent; the wire carries only FTR fields (`impact.ts:53-64`). **gap vs mockup**
- [ ] The detail strip repeats Status, Verdict and Workspace, which are already in the eyebrow and pill (`+page.svelte:179-190`). **polish**
- [ ] Check the dismissed dbd row. It appears under "pending" with status "dismissed", a dismissed rec shown as awaiting measurement. **bug (minor)**
## /insights

**Purpose.** Cross-workspace triage board: what needs a decision now, what is emerging, what is settled.

**Shows.** Header with three counts (now / soon / settled); a workspace filter strip ("All", the 3 most recent workspaces, a search box); three scrolling columns. Now = violated memories, high-urgency recommendations (Apply / Review / Dismiss), the top 3 recurring corrections. Soon = medium recommendations, suggested patterns, proposed memories. Settled = in-force memories (strength bar + scope), rule patterns. Load: `insights/+page.ts:11-14`; state: `insights/insights-board.svelte.ts`.

**API.**
- `GET /api/insights` (`api.ts:563`) → 200, keys `counts, workspaces, memories, recommendations, patterns, corrections`. Every field the view-models read is present (`id, workspace_id, name, title, why, impact, column`; memories `violated_count, strength, scope`; corrections `text, count`). Shape OK. **The content is wrong, though:**
  - Recommendations are capped at `LIMIT 200`, ordered high-urgency first (`crates/senseid/src/db/pg_store/patterns.rs:1062-1067`). The live DB holds **27,663 pending recs** (high 1,015 · medium 23,368 · low 3,280; read-only `SELECT` on `inference.recommendations`). The response carries 200, all `high`. So the board says `now 203`, and Soon says "nothing brewing." with 23k medium recs pending. The cap is only logged on the daemon (`patterns.rs:1075`); the UI does not mention it.
  - 162 of the 200 are `Security: update <dep> …` recs (e.g. `@angular/common 8.2.14 → 22.2.0` for a client workspace). That makes 836 pending security recs in total, so the Now column is effectively a dependency-advisory feed.
  - Corrections hold noise: the top one is `"task notification task id afcad528… tool use id toolu_014… output file private tmp claude …"` ×122. That is a harness notification, not a user correction (daemon side, `get_insights_corrections`).
  - Memory `strength` is on a 0–5 scale on the wire (challenged memories 2.5 and 3.0, active ones 1.0, per `/api/knowledge/memories`). The client treats it as a 0–1 fraction and clamps it (`insights-board.svelte.ts:149-150`), so every memory at 1.0/5 renders a **full, success-coloured bar** (`MemoryRow.svelte:15-16`). The Memories screen shows the same memory as "★ 1.0 / 5".
- `POST …/recommendations/{id}/accept` and `…/reject` (`api.ts:552-555`) were not invoked. On failure the handler restores the card and shows a `role=alert` banner (`insights-board.svelte.ts:288-307`). That path is correct.
- On a failed read the page shows `ScreenState` error with Retry (`+page.svelte:193-198`), not three quiet columns. Correct. The stale comment at `api.ts:559-562` still says the fallback is "the empty board".

**Interactions.**
- Apply → `accept`. It only flips the rec to accepted and enqueues MeasureVerdicts (`crates/senseid/src/api/handlers/workspace_detail.rs:516-537`). It does not update the dependency or materialize a rule; `tryMaterializeRecommendation` exists at `api.ts:543` but this screen never calls it. "Apply" overpromises.
- Dismiss → `reject`, with no confirmation and no undo. The card disappears optimistically.
- Review → opens the workspace window at `/workspace/{id}` (its root, not the recommendation). A failure only goes to `console.error` (`+page.svelte:37`).
- Workspace chips and search refetch with `?workspace=`. `loading` is set (`insights-board.svelte.ts:274`) but the page never reads it, so there is no in-flight feedback.
- Violation, pattern, memory and correction cards are display-only. They have no click target and no link to the memory.

**Mockup diff** (`10-insights.png`).
- Header KPIs differ. Mockup: "6 TO ACT · 11 MEMORIES · +8% FTR LIFT". App: now/soon/settled counts. There is no FTR lift.
- Mockup violation cards carry Reinforce / Review / Mute plus a "reinforces it in <file>" line. The app's are display-only (`ViolationCard.svelte:7-10`).
- Mockup Soon recs are typed (SKILL / PROJECT / RULE) with "Scaffolds <file>"; the app has no type.
- Mockup pattern cards show "FTR +18%"; the app shows an instance count only.
- Mockup Settled rows show scope + path (`project · lumen-cloud · tests/integration/*`); the app shows scope only.
- The project chip strip matches in spirit (All + chips + search).

**Checklist.**
- [ ] **bug.** Count pending recommendations in the DB against the board. Expect the true count, or "showing 200 of N". The code caps at 200 high-first with no UI signal, so Soon reads "nothing brewing." over 23k medium recs (`patterns.rs:1067`, `+page.svelte:139-141`).
- [ ] **bug.** Look at a Settled memory's strength bar. Expect a partial bar for 1.0/5. The code clamps 0–5 into 0–1, so it is full and green (`insights-board.svelte.ts:150`, `MemoryRow.svelte:15-16`).
- [ ] **bug.** Read the Now corrections. Expect real user corrections. Live: "task notification task id … tool use id …" ×122 (daemon `get_insights_corrections`).
- [ ] **question.** Click Apply on a "Security: update X" rec. Expect the dependency to be updated, or a rule written. The code only marks the rec accepted (`workspace_detail.rs:521`). Should the verb be "Accept", or should it call materialize?
- [ ] **gap.** Click Dismiss. Expect a confirm or an undo. The card is removed immediately, with no undo (`insights-board.svelte.ts:299-307`).
- [ ] **polish.** Click a workspace chip. Expect a loading indication. `board.loading` is unused in the template.
- [ ] **gap vs mockup.** Expect Reinforce/Mute on violation cards and FTR lift on patterns and in the header. Neither exists.
- [ ] **polish.** Review on a rec. Expect the workspace window to open on that recommendation; it opens the window root.

## /learnings  (sidebar label "Memories")

**Purpose.** Review and maintain memories: triage AI proposals, keep the active set healthy, browse the archive.

**Shows.** Header "Memories"; tabs Triage / Active / Archive with counts; list on the left, detail pane on the right (scope/status/tags, content, impact, 7-day usage strip, evidence, examples, recent outcomes). State: `lib/memoryState.svelte.ts` (a module singleton). The scope is `appState.activeWorkspaceId`, which is `null` live (`/api/config` has no `active_workspace`), so the lists are global (`learnings/+page.svelte:16`).

**API.**
- `GET /api/knowledge/memories?status=<s>&limit=500` (`api.ts:1025-1029`) is called once per status: 7 calls (`memoryState.svelte.ts:56-60,107-109`). Live counts: proposed 0, active 12, challenged 2, rejected 2, others 0. Keys include everything the rows read (`scope, scope_filter, status, tags, title, strength, applied_count, violated_count, triage_signal`). OK. The response has **no `total`**, so a status bucket past 500 truncates silently.
- `GET /api/knowledge/memories/{id}` (`api.ts:1031`) → `{memory, evidence, examples, outcomes, related, loaded_last_7d, followed_last_7d, skipped_last_7d}`. Shape OK. `related` is returned but never rendered (`MemoryDetail.svelte`). An unknown id → 404.
- If any list read fails, `listError` → `ScreenState` with Retry (`+page.svelte:57-65`). Correct.
- Writes (not invoked): `acceptProposal`, `rejectProposal`, `reinforceMemory`, `challengeMemory`, `archiveMemory`, `dismissMemory` (`api.ts:1077-1112`).

**Interactions.**
- **Accept / Reject (Triage) swallow failures.** `if (!res.ok) return;` (`memoryState.svelte.ts:133,141`): no message, no busy state, no double-submit guard (`TriageList.svelte:22-23`). The Active-tab verbs do have per-row errors and busy guards (`:159-174`), so the two tabs are inconsistent.
- **A failed detail read is swallowed.** `if (res.ok) this.detail = …` (`memoryState.svelte.ts:126-127`). `selected` moves to the new row, which gets the highlighted border, while the pane keeps showing the *previous* memory. No error appears.
- Archive and Dismiss (the latter red, i.e. reject) have **no confirmation** (`ActiveList.svelte:45-58`). Challenge and Dismiss are disabled for terminal statuses; that is correct.
- Archive rows can be opened in detail but offer no restore action (`ArchiveList.svelte`).
- On first mount the screen jumps to Active when Triage is empty (`+page.svelte:17`). Live, Triage is 0, so the user lands on Active.

**Mockup diff** (`11-memory.png`; `p3-memories.png` is the workspace-window version).
- Mockup header: "Every memory has the same anatomy." plus a memory-strength histogram. App header: "Memories", no histogram.
- The mockup's project chip filter, "search projects", "search memories" and "11 of 11 memories" are all absent. There is no search at all.
- Mockup list rows carry a type kanji (作/技/則/令) and title only. App rows are bordered cards with chips, ★/✓/✗ and four buttons each.
- The mockup detail has category · status · surfaced-as (Agent, `tests.agent.md`) · applies-in (workspace + path glob) · "In the codebase" file links with ✓/✗ · strength bar · actions **Promote to rule / Enrich / Apply elsewhere**. The app has none of surfaced-as, applies-in or codebase links. Promote and generalise exist only in the workspace window (`(workspace)/workspace/[id]/memories/memories-state.svelte.ts:346,367`).
- The mockup has no Triage/Active/Archive tabs.
- Styling: the components use `<style>` blocks with literal rem/px and `var(--text-muted)`, a token that does not exist (`ActiveList.svelte:74`, `ArchiveList.svelte:24`). This breaks the app's no-`<style>`, token-only rule (app/CLAUDE.md §1).

**Checklist.**
- [ ] **bug.** With the daemon stopped (or a stale id), click Accept on a proposal. Expect an error. The code returns silently (`memoryState.svelte.ts:133`).
- [ ] **bug.** Select a memory whose detail read fails. Expect an error in the pane. The code keeps the previous memory's detail beside the new highlight (`memoryState.svelte.ts:126-127`).
- [ ] **bug.** Compare the strength of the same memory here ("1.0 / 5") and on Insights (full bar). The two screens disagree (see /insights).
- [ ] **gap.** Click Dismiss or Archive on an active memory. Expect a confirm. The code acts immediately (`ActiveList.svelte:45-58`).
- [ ] **gap.** Double-click Accept. Expect one request. Nothing guards it (`TriageList.svelte:22`).
- [ ] **gap vs mockup.** Expect search, a workspace filter, Promote to rule, Enrich and Apply elsewhere. None exist on this screen.
- [ ] **question.** Is the global (unscoped) view intended? `activeWorkspaceId` is never set by any caller (`appstate.svelte.ts:31`; no `setActiveWorkspaceId(` call outside the store).
- [ ] **polish.** The archive empty text reads in `var(--text-muted)`, an undefined token (`ArchiveList.svelte:24`), so it falls back to the inherited colour.
- [ ] **polish.** The route is `/learnings`, the sidebar says "Memories" and the eyebrow says "Observatory". Expect one name.

## /consolidation

**Purpose.** Governance tier-2: merge the global raw rules into one deduped ruleset via a model, then approve it.

**Shows.** Header stats (version, sections, status); id + model eyebrow; a "before → after" diff strip; the merged ruleset split on `#` headings; Approve and Re-consolidate. If nothing has ever been consolidated, an empty state with "consolidate now".

**API.**
- `GET /api/knowledge/rules/consolidated` (`api.ts:1126`) → 200 `{id, version:3, content(948 chars), conflicts:[], model:"gemma4", status:"proposed"}`. Shape matches `ConsolidatedRuleset` (`consolidation-view.ts:13-22`). On failure the load returns `error` → `ScreenState` with Retry (`+page.ts:15`, `+page.svelte:62-63`). Correct. The loader uses `res.error.message` rather than `failureReason`, which is inconsistent with other screens.
- Writes (not invoked): `POST /api/knowledge/rules/consolidate`, `POST …/consolidate/{id}/approve`. Failures surface in `notice` (`consolidation-state.svelte.ts:26-28,42-44`). Correct.
- Live content check: the merged ruleset holds 2 Mandatory rules, and Required, Recommended and Advisory each read "*(No rules in this set meet …)*", plus a "Conflicts: None" section. Whether a 948-char merge of all global rules is plausible is worth a look; the screen gives no way to see the inputs.

**Interactions.**
- "re-consolidate" asks a model (it spends) with **no confirmation and no cost/model hint** (`+page.svelte:149-157`).
- "approve · merge & keep" changes governance with **no confirmation** (`+page.svelte:139-147`). The page's own description says "Nothing changes until you approve".
- Busy labels ("consolidating…", "approving…") and skip/failure notices exist. A successful run gives no "done" notice; it relies on the reload.

**Data shown.**
- The diff strip is always "— → N". `sourceCount` is hard-wired to `null` (`+page.svelte:24`), so there is no "before" side and no rules-on-disk delta. That leaves 3 cells in a 4-column grid (`+page.svelte:99`), so one cell slot is empty.
- The "sections" stat counts the document title too: 6 with the live content, including "Coding Assistant Governance Ruleset" and "Conflicts".
- The ruleset renders as raw markdown inside `<pre>` (`+page.svelte:130`), so `*   **Verification…**` shows literal asterisks.
- The raw input rules are never listed. "One coherent ruleset from many raw rules" cannot be checked on screen. (`GET /api/knowledge/rules` needs `folder` or `workspace`; without them it returns 400 "folder or workspace required".)

**Mockup diff.** No mockup.

**Checklist.**
- [ ] **gap.** Click "approve · merge & keep". Expect a confirm naming what changes. The code fires immediately (`+page.svelte:144`).
- [ ] **gap.** Click "re-consolidate". Expect a model/cost hint or a confirm. The code fires immediately (`+page.svelte:154`).
- [ ] **bug.** Look at the diff strip. Expect before → after. Every "before" is "—" and one grid slot is empty (`+page.svelte:24,99`; `consolidation-view.ts:114-148`).
- [ ] **polish.** Read the ruleset. Expect rendered markdown; the code shows raw `**`/`*` in a `<pre>` (`+page.svelte:130`).
- [ ] **question.** Should the screen list the raw rules that fed the merge? The current proposed v3 has 2 rules and three empty tiers.
- [ ] **polish.** The "sections" stat counts the H1 title (6 instead of 5) (`+page.svelte:50`).

## /knowledge-sources  (sidebar "Sources")

**Purpose.** Register federation ("hive-mind") endpoints that rules and memories sync from and to.

**Shows.** An add form (URL + Name + Add), then either a list of source cards (name, url, direction, `seq`, enabled) with Sync and Remove, or an empty state. State: `lib/knowledge-sources.svelte.ts`.

**API.**
- `GET /api/knowledge/sources` (`api.ts:1147`) → 200 `{"sources":[]}` live. The daemon serializer emits `id, kind, name, url, namespace_id, direction, last_seq, enabled` (`crates/senseid/src/api/handlers/knowledge.rs:1205-1208`), which matches `KnowledgeSource` (`types.ts:871-880`). OK.
- `GET /api/knowledge/sources/{id}/status` exists (`routes.rs:518`) but has **no caller in the app**, so no sync status or last error is ever shown.
- Writes (not invoked): `POST /api/knowledge/sources`, `DELETE …/{id}`, `POST …/{id}/sync`.

**Interactions / failure handling.**
- **A failed load renders "No knowledge sources."** The template checks `state.sources.length === 0` before the error (`+page.svelte:55-62`). The error string is printed as small text at the bottom (`:100-102`), with no Retry. The empty state claims there are no sources while the read failed. This breaks the project's "an error must show as an error" rule.
- Remove is **destructive with no confirmation** (`+page.svelte:84-88` → `knowledge-sources.svelte.ts:91-95`).
- Sync has no busy state, no double-click guard, and no success feedback; the `SyncStats` result is discarded (`knowledge-sources.svelte.ts:97-101`).
- Remove and Sync errors go to the same bottom `error` line and are cleared only by the next load.
- Add validates https or loopback http and requires a name (client mirror at `:23-38,62-71`). Add errors show beneath the form. Good.

**Mockup diff.** No mockup.

**Checklist.**
- [ ] **bug.** Stop the daemon and open Sources. Expect an error with Retry. The code shows the "No knowledge sources." empty state plus a small red line (`+page.svelte:57`).
- [ ] **gap.** Click Remove. Expect a confirm. It deletes immediately (`knowledge-sources.svelte.ts:92`).
- [ ] **gap.** Click Sync. Expect progress, then a result (items pulled/pushed). Nothing visible happens apart from `seq` maybe changing (`:97-101`).
- [ ] **gap.** Expect each source's sync health or last error. The `/status` endpoint exists but is unused.
- [ ] **polish.** The eyebrow is "Federation", the sidebar says "Sources" and the kanji differs (header 連, sidebar 源; `observatory-nav.ts:105` vs `+page.svelte:22`).
## /instruments (shell shared by the three tabs)

**Purpose.** One screen for the MCP tools assistants can use: try them (Playground), replay what an assistant called (Replay), and judge toolset health (Health).

**Shows.** A hero (kanji, "Instruments · <tab>", tagline, sub-line from `instruments-view.ts:14-31`) and a tab bar (`+page.svelte:183-200`). The tab comes from `?tab=` (`instruments-view.ts:9-11`), so the Sessions digest can open Replay at `?tab=replay&session=<id>` (`+page.svelte:31,150-155`), and `?workspace=<id>` preselects "Runs in" (`+page.svelte:33,137`). There is no `+page.ts`: each tab reads on its first open (`+page.svelte:132-147`). State lives in `instruments-state.svelte.ts` and the global `mcp` store (`lib/state/mcp.svelte.ts`).

**Layout.** `<Screen>` is used **without `fill`** (`+page.svelte:168`). The shared `Screen` (`packages/ui/Screen.svelte:7-8,29`) makes the body the only scroll owner unless `fill` is set. Both two-pane tabs therefore scroll as one page (see Replay).

---

## /instruments — Playground

**Purpose.** Pick any tool from any discovered MCP server, fill in its inputs (built from its schema), and run it through the server itself.

**Shows.**
- **Left rail:** a search box. Sensei's own server comes first and is expanded. Below it, "MCP servers", grouped by assistant (Claude, Cursor, Zed, "Added in sensei"), each with an on/off switch, and a remove button on hand-added servers. A footer reads "N MCPs · M tools" with a "+ scan" button (`+page.svelte:216-382`).
- **Right pane:** the tool name, kind badge, description, a "Runs in" workspace select, an inputs form, a Run/Query button, and the response `<pre>` (`+page.svelte:385-527`).

**API**

| Endpoint (GET unless stated) | Verdict |
|---|---|
| `/api/instruments/mcp-servers` | 200, 6 servers in 1.2s. The shape matches `McpServerRow`. The two zed servers come back `connection_state: "error"` with `last_error: null`. The rail never shows `connection_state` (only `enabled` as "on"), so a broken server looks the same as a healthy one until it is expanded. |
| `/api/instruments/mcp-servers/{id}/tools` (no `probe=false` from the page, so a first expand may spawn the server) | Checked with `?probe=false` for sensei: 56 tools, `stale: true`, `age_seconds: 2625`. Each tool carries only `name, description, inputSchema`; there are no annotations. The UI never shows `stale` or `probed_at`. |
| `/api/workspaces?watched=true&folders=roots` ("Runs in") | **200 in 16.7s, 133 workspaces; only 4 carry folders.** The fix 3b8017ed ("only the workspaces under a watched root", burndown UI-walk item 10, marked done) is not in any tag. The running daemon is 0.21.0, ignores `watched=true`, and returns all 133. The fix is committed but not running. |
| POST `/api/instruments/mcp-servers/{id}/call` (Run) | Not invoked (writes / side effects). Traced: `ins.execute`, then `tryCallServerTool`. Failures show as `tool-error` (`instruments-state.svelte.ts:126-133`). Correct error surfacing. |
| PUT `.../enabled`, DELETE `.../{id}`, POST `.../refresh`, POST `/api/instruments/mcp-servers` | Not invoked. Each returns a message on failure (`mcp.svelte.ts:219-252`). |

**Interactions**
- **Tool row** opens the tool and pre-fills defaults (`+page.svelte:290`). It works.
- **Run / Query:** the label comes from the *inferred* kind (`+page.svelte:495`). The kind always comes from a name regex; any declared kind is ignored (`playground-groups.ts:102`, `inferKind(t.name, undefined)`; regex at `:67-68`). Against the live sensei manifest:
  - 23 of 56 tools come out as `other`, so they get "Query →" and the hint "returns information" (`+page.svelte:393`). They include writes (`save_memory`, `propose_memory`, `promote_memory`, `accept_proposal`, `reject_proposal`, `log_event`, `add_library`, `register_plan`, `record_outcome`, `pause_run`, `report_run_outcome`, `accept_playbook_rule`) and model calls that cost money (`infer`, `embed`, `consensus`, `generate_image`, `plan`).
  - `run_status`, a read, comes out as `action`.
  - No tool asks for confirmation before running.
- **Server on/off switch** (`+page.svelte:256-265`) writes to the daemon. On failure it shows `servers-error`. It works, but there is no confirmation and no explanation of what "off" means for the assistant's own config.
- **Remove** (manual servers only, `+page.svelte:267-272`) deletes immediately, with **no confirmation**.
- **+ add / Add server** form (`+page.svelte:321-342`) validates through `parseServerForm`, closes only after the daemon accepts, and shows the error otherwise. OK.
- **+ scan** (`+page.svelte:375-380`) shows "scanning…" while running and an error on failure. OK.
- **Search** only filters tools that are already loaded (`instruments-view.ts:34-43`). Non-sensei groups start collapsed and their tools are not read until expanded (`+page.svelte:83-90,118-126`). Searching "browser" finds nothing until Playwright is expanded.
- **Footer count** "N MCPs · M tools" (`+page.svelte:374`) also counts only loaded manifests: it shows 56 tools when the servers hold 56+25+7+4+….
- **No loading indicator** for the server list: the `idle`/`loading` states render nothing (`+page.svelte:347-355`).

**Mockup diff** (13-instruments.png)
- The tagline differs. Mockup: "Try any tool before you trust it." App: "Sensei · what your instruments can do." (`instruments-view.ts:28`).
- The mockup shows an "OFF" badge plus a tool count on each collapsed group. The app shows an "on/off" pill plus a count.
- Mockup footer: "5 MCPs · 22 tools · + add MCP". App: "+ scan" in the footer and "+ add" under the section header.
- Response preview: the mockup shows a sample answer, the app shows "—". That is acceptable: no fabricated preview.
- Otherwise the layout (300px rail, inputs card, response pane) matches.
- p6-instruments.png is the workspace-scoped view: a "Tool effectiveness — X is your weakest tool here" card with "open replay for X →", and per-tool rows with "calls · FTR". The app has no workspace-scoped Instruments view and no per-tool FTR.

**Checklist**
- [ ] **bug**: Pick `save_memory` or `generate_image` in the sensei group. Expect an "action" badge and "Run →" (or a confirmation). Code suggests "Query →" with "returns information" (`playground-groups.ts:67-68,102`; `+page.svelte:393,495`). Do **not** press it on a real workspace.
- [ ] **bug**: Pick `run_status`. Expect "query". Code labels it "action" (the regex matches `run`).
- [ ] **bug**: Open "Runs in". Expect only sensei, dbd, gateway and torii. The live daemon returns 133 workspaces after a 16.7s read; 3b8017ed is not installed.
- [ ] **gap**: Type "browser" in the tool search before expanding Playwright. Expect Playwright tools. Code: no match (`instruments-view.ts:34-43`).
- [ ] **gap**: Look at the zed servers (context7, postgres). Expect an error state. The rail shows "on" like a healthy server; the daemon says `connection_state: error` with `last_error: null`.
- [ ] **bug** (destructive, no confirm): Add a throwaway manual server, then click "remove". It is deleted at once (`+page.svelte:267-272`).
- [ ] **polish**: Footer count. Expect total tools across servers. It counts only expanded or loaded servers (`+page.svelte:374`).
- [ ] **polish**: A stale manifest (`stale: true`, 44 min old) is not flagged anywhere.
- [ ] **question**: What does "off" on a discovered Claude server do? Does it only hide the server in sensei, or does it edit the assistant's config?

---

## /instruments — Replay

**Purpose.** Pick a captured assistant session and step through its tool calls in order, with the request, response, timing and verdict (used / partial / ignored).

**Shows.** A left rail with every session from `/api/sessions` (8-char id, FTR badge, task, start time; `+page.svelte:555-591`). The right pane has a summary strip (task, id, calls, used, ignored), a timeline list, and the selected call's request and response (`+page.svelte:594-710`).

**API**

| Endpoint | Verdict |
|---|---|
| `/api/sessions` | 200, 403 sessions, 186 KB. **`ftr` is a boolean on the wire** (True 319, False 72, None 12). `SessionData.ftr` is typed `number \| null` (`lib/types.ts:489`), while the sibling `SessionRow` correctly types it `boolean` (`types.ts:514`). **398 of 403 sessions have `task: ""`.** The newest `startedAt` is 2026-10-20, so nothing has been captured for 7 days. That is outside this screen but visible here. Order is `coalesce(completed_at, started_at) DESC` (`pg_store/sessions.rs` list_all_sessions), while the rail shows `startedAt`, so dates look unordered (10-15, 10-20, 10-15, 10-18…). |
| `/api/sessions/{id}/replay?classify=true` | The page always sends `classify=true` (`instruments-state.svelte.ts:174`), which makes this **GET write** verdict rows (`handlers/sessions.rs:166-180`). I checked without `classify`. **The timeline is capped at 200 calls with no indication:** the handler defaults to `limit=200` (`sessions.rs:149`), the app passes no limit, and `summary` counts the whole session. Live: session bbcb2965 returns `count 200` with `summary used 12528 / ignored 776 / total 13386`. 7 of the 12 most recent sessions have 0 calls. The `request`/`response` payloads are the entire hook envelope (`cwd, transcript_path, permission_mode, prompt_id…`), not just `tool_input` / `tool_response`. |

**Interactions**
- **Session row** runs `selectReplaySession` (`+page.svelte:565`). There is **no stale-response guard**: `instruments-state.svelte.ts:168-181` writes whichever answer arrives last, unlike Libraries, which checks `selected?.id` (`libraries-state.svelte.ts:65,80`). Clicking A then B quickly can show A's calls under B's header.
- **Timeline row** sets the selected call (`+page.svelte:644`). The detail renders at the top of the right column.
- **Whole-screen scroll (UI-walk item 8), confirmed in code:**
  - `<Screen>` has no `fill` (`+page.svelte:168`).
  - The two-pane grid is `min-h-[600px]` with no height cap (`+page.svelte:553`).
  - The aside and main are `overflow-auto` but have no bounded height, so they never scroll on their own (`:555,594`).
  - With 403 sessions and up to 200 calls, the page body scrolls. Clicking call #150 shows its request and response at the top of the column, out of view.
- **Error / empty states:**
  - List error: `ScreenState` with Retry (`:538-545`). Good.
  - Detail error (`:630-631`). Good.
  - "No sessions recorded yet" appears only on a genuinely empty list. Good.

**Mockup diff.** No PNG for Replay. The page cites `docs/mockups/Sensei/lib/instruments.jsx` (InstrumentsReplay), not reviewed as an image. The burndown item asks for "an easier way to read the history of calls"; today's request and response are raw hook envelopes.

**Checklist**
- [ ] **bug**: Open Replay and look at the FTR badges. Expect "ftr", or a correction %, for non-first-try sessions. Code: `ftr` is boolean, so `1 - false` gives **"100%c" on every non-FTR session** (`+page.svelte:572-580`; type mismatch at `types.ts:489`).
- [ ] **bug**: Open session bbcb2965 (sensei, 13k calls). Expect a call count that matches the summary, or "showing 200 of 13,386". Code: "calls 200", "used 12528" side by side (`+page.svelte:610,614`; `sessions.rs:149`).
- [ ] **bug** (UI-walk 8): Scroll the timeline to a late call and click it. Expect the detail to stay visible. The whole body scrolls and the detail sits at the top (`+page.svelte:168,553`).
- [ ] **bug**: Click two sessions in quick succession. Expect the second session's calls. There is no guard against the first answer landing last (`instruments-state.svelte.ts:168-181`).
- [ ] **gap**: Rail rows. Expect a task line. 398 of 403 are blank, so a row shows only an 8-char id and a date. The header title is blank too, because `?? selectedSessionId` does not catch `""` (`+page.svelte:604`).
- [ ] **gap**: Many sessions have 0 calls (7 of the top 12). Expect a filter or a "has calls" hint. Code: you can only click and see "No tool calls in this session."
- [ ] **polish**: The request panel shows `cwd`, `transcript_path`, `permission_mode` and similar fields. Expect `tool_input` only (and `tool_response` for the response).
- [ ] **polish**: Rail dates look unordered. The daemon orders by completed-or-started; the rail prints `startedAt`.
- [ ] **question**: Should opening a session (a GET) write verdicts (`classify=true`)? It also makes every first open slower.
- [ ] **question**: No sessions captured since 2026-10-20. Is capture broken? (Outside this screen; flag it to the Sessions reviewer.)

---

## /instruments — Health

**Purpose.** Show how the toolset is doing: which tool sources are connected, what share of each source's tools is used, and per-tool call, error and verdict detail.

**Shows.**
- **L1** (`+page.svelte:752-816`): 4 KPIs (servers connected, tool coverage, total calls 14d, first-try rate 14d) and a card per source (`ToolHealthCard.svelte`): share-invoked %, bar, calls 14d.
- **L2** (after clicking a card, `+page.svelte:719-732,818-1053`): "← all sources", then 4 KPIs (total calls, errors, signals, dormant), signal cards, and a per-tool usage table with expandable insight rows.

**API**

| Endpoint | Verdict |
|---|---|
| `/api/instruments/tools-health` | 200, 7 sources, **12.5s to 26.7s**. The loading state shows meanwhile (`:757-758`). Fields match `ToolHealthSource`. **"svelte" appears twice:** `plugin_svelte_svelte` (off, not probed, 21 calls) and `svelte` (on, 4 tools, 0 calls). The usage lands on the ghost row. Only `claude` sources are returned; the zed servers do not appear. |
| `/api/observatory/tool-usage` | 200, 4 to 16s. **Hard `LIMIT 50`, all-time** (`pg_store/mcp.rs:228-229`). **All 50 rows have `avg_duration_ms: null`.** It mixes assistants' built-ins (`Bash` 163,493; Zed's `edit_file`, `read_file`, `terminal`). |
| `/api/observatory/tool-signals` | 200, 4 to 10s. 2 signals, both with `tool_name: ""`: "20 tools dormant" (unused) and "30 workhorse tools" (win). |
| `/api/observatory/tool-insights` | 200, 101 rows, 46 KB. Matches `ToolInsight`. |
| `/api/observatory/ftr` | 200 `{ftr14d: 0.9, …}`. Used for the first-try KPI. OK. |

`loadCatalog` fails as a whole if either usage or signals fails, and shows an error with Retry (`mcp.svelte.ts:71-77`). That is the correct direction: no empty fallback.

**Interactions**
- **Source card** opens L2 (`ToolHealthCard.svelte:38`). **L2 is not scoped to the source:** the header literally says "· all sources · scoping per source coming" (`+page.svelte:728-730`). Clicking "semgrep" (0 calls) shows a table led by `Bash`.
- **Table row** expands the insight snapshot or shows "No cached snapshot yet" (`:932-1048`). OK.
- **← all sources** goes back. OK.

**Mockup diff.** No PNG for Health. p6-instruments.png (workspace-scoped) shows per-tool "N calls · FTR%" and a "weakest tool" card with "open replay" deep link. The app has no FTR per tool and no link from Health to Replay.

**Checklist**
- [ ] **bug**: L2 "dormant" KPI. Expect 20 (the signal says "20 tools dormant"). Code counts *signals* with variant `unused`, so it shows **1**, with a hardcoded hint "0 calls" (`+page.svelte:841,882-885`).
- [ ] **bug**: L2 header vs L1. L1 "total tool calls · 14d" ≈ 21.5k. L2 "total calls" sums the all-time top-50 (>250k) with no window label (`+page.svelte:835,850`; `mcp.rs:228`).
- [ ] **bug**: "Per-tool usage · 50 tools · sorted by calls" (`+page.svelte:917`). The daemon truncates at 50 without saying so (`mcp.rs:229`).
- [ ] **gap**: "Avg ms" column is "—" on every row (all `avg_duration_ms` null live).
- [ ] **gap**: Click the "semgrep" card. Expect semgrep's tools. You get every tool across all sources (`+page.svelte:729`).
- [ ] **bug** (data): Two "svelte" cards; the one with usage is "off" and unprobed. Expect one svelte source.
- [ ] **gap**: Zed's servers (context7, postgres) are absent from the grid, but Zed built-ins are counted in the usage table.
- [ ] **polish**: First open waits 12 to 27s on `tools-health` (plus 4 to 16s on usage and signals).

---

## /libraries

**Purpose.** List every library detected in the watched code, and for one library show where it is used and what it provides to assistants (docs, skills, agents from its `sensei.library.json`).

**Shows.** A page header, a search box, and a list of library cards (name, ecosystem, "N repos", skills and agents count). Selecting one shows a sticky detail: name, ecosystem, repo count, doc pages, Packages chips, "Provided by the library" (expandable skills, agents, docs), "Used in" chips, and "Usage by folder" (`libraries/+page.svelte:21-154`). No `+page.ts`; `onMount` loads (`:13`). State: `libraries-state.svelte.ts`.

**API**

| Endpoint | Verdict |
|---|---|
| `/api/libs?` | 200, 209 libs, 0.5s. Fields match `LibEntry`. `repos` holds **folder/crate names** (`app`, `cli`, `senseid`, …): serde_json has `repoCount: 29` made of 29 crates, not repositories. Only 3 libs have `pageCount > 0` (rokkit, dbd, kavach). Sorted by repoCount desc. |
| `/api/libs/{id}/usage` | 200. **`import_count` is 0 on every row** (serde_json 29/29, rokkit 57/57). Rows are per *package* (`library_name: "@rokkit/actions"`) and per folder, with `workspace_id`. |
| `/api/libs/{id}/docs` | 200. rokkit: 95 pages, **481 KB, 6.1s**, fetched in full (with content) on every selection. serde_json: `[]`. |
| `/api/libs/{id}/skills`, `/agents` | 200. rokkit 5 skills / 3 agents with bodies; serde_json `[]`. |

Errors: a list failure shows `ScreenState` with Retry (`+page.svelte:39-40`). A usage or provided failure shows an inline error (`:109-110,131-132`). The race guard exists (`libraries-state.svelte.ts:65,80`). Good.

**Interactions**
- **Library card** selects the library and fires 4 reads (`+page.svelte:58`). It works.
- **Search** filters by name client-side (`:16-18`).
- **`<details>`** expands a skill, agent or doc body (`:163-169`).
- There are no other actions: no add library, no filters, no links (for example to a workspace).

**Mockup diff** (12-libraries.png)
- The mockup has a hero ("Tools the student uses. Kept close.") and a "+ add library" button. The app has a bare "Libraries" header and no add.
- The mockup has filter chips (All / Code / Services; LANG: Rust, TS, docs, MCP) and an "N of M" count. The app has only a search box.
- Mockup list rows show an avatar, version, source manifest ("Cargo.toml · lumen-api"), a docs status badge ("docs indexed / partial / schema only"), "N× calls" and "last used". The app shows name, ecosystem, "N repos", and skills/agents only.
- The mockup detail has a description, version · language · status · manifest, a narrative line, **Top symbols** with counts, **Used at** (file:line with code snippet), **Rules attached**, and "What sensei can do with this library". The app has packages, provided, used-in chips, and usage by folder. Top symbols, file-level usage, rules and description are missing (the API returns `description: null`).
- p5-libraries.png (workspace-scoped) shows a coverage hero ("One library is unwrapped — tailwindcss" / "wrap tailwindcss →") and per-library "N× used · wrapped/unwrapped". The app has nothing comparable.

**Checklist**
- [ ] **bug**: Type "zzzz" in search. Expect "no libraries match". Code shows **"No libraries indexed."** with setup advice (`+page.svelte:41-46`).
- [ ] **bug**: Select serde_json. Usage rows read "0 imports" on all 29. Expect real counts or no column. The daemon returns `import_count: 0` everywhere (`+page.svelte:144`).
- [ ] **bug**: Select rokkit. "Usage by folder" lists `site` 17 times and `app` 10 times with no package name (rows are per package; `+page.svelte:136-146` drops `library_name`). Same-named folders from different workspaces (`cli` ×2 for serde_json) are indistinguishable.
- [ ] **bug** (label): "29 repos" for serde_json, and "Used in" chips `app, bootstrap, cli…`. These are crates/folders, not repositories (`+page.svelte:66-68,123-125`).
- [ ] **gap**: Select a library with usage but nothing returned. When usage is `[]` there is no "not used" line; the section silently disappears (`+page.svelte:133`).
- [ ] **polish**: Select rokkit. The detail waits about 6s for 481 KB of doc bodies just to list 95 titles.
- [ ] **polish**: Error text uses `text-error` (`+page.svelte:110,132`) while the rest of the app uses `text-danger`. It resolves through a rokkit alias (`rokkit.config.js:107`), but it is inconsistent.
- [ ] **gap vs mockup**: No version, docs status, calls or last-used in the list. No filters or count. No add library. No top symbols, file-level usage or rules in the detail.
## /activity-logs

**Purpose.** The operator's screen: what the daemon's background workers are doing, and the structured log, for when a number or a signal is not updating.

**Shows.** A "Background tasks" table (16 workers: last run, interval, next run, schedule, avg, health, a per-worker "log" tail), then filters (Level, Source, Module, a search box, 15m/1h/24h/All, Limit), a count line ("showing N logs · capped at…"), and the log rows. A row expands to show its `error`/`context`/`data`. Files: `activity-logs/+page.ts:12-26`, `+page.svelte`, `activity-logs.svelte.ts`, `scheduled-tasks.svelte.ts`.

**API**
| Endpoint | Verdict |
|---|---|
| `GET /api/logs?level&source&module&since&limit` (`api.ts:701`) | 200, top-level array; fields `id, level, source, module, logged_at, message, context, data, error` match `LogRow` (`types.ts:1469`). A failed read shows "Could not read the log: …" (`activity-logs.svelte.ts:212`), not an empty list. OK. `since=bogus` → 400, which surfaces as that error. |
| `GET /api/tasks/scheduled` (`api.ts:714`) | 200, `{tasks:[16]}`; every field the row reads is present (`name, description, enabled, interval_secs, window, days, last_run_at, last_ok, last_error, next_run_at, avg_ms, running_since`). All 16 `last_ok: true`. The error path is handled (`+page.svelte:80`). OK. |
| `GET /api/tasks/scheduled/{name}/log?lines=50` (`api.ts:717`) | 200, `{worker, file, lines[]}`. A failed read shows an error (`scheduled-tasks.svelte.ts:150-155`). OK. |

**Interactions.** Level (native `<select>`), Source and Module (rokkit `Select`), the since chips and Limit all call `apply()`, which runs `goto(...)` with `invalidateAll` (`+page.svelte:54-61`). This works end to end because the loader reads the URL. Search filters client-side. A row expands only when it has a payload; otherwise it is `disabled` (`LogRow.svelte:32`). "log" on a worker toggles the tail. There are no write actions. The header comment says the schedule is "editable via PATCH /api/tasks/scheduled/{name}" (`+page.svelte:77-78`), but this screen has no edit control.

**Mockup diff.** No mockup.

**Checklist**
- [ ] Open the screen with the default filters (All, limit 200) and look for errors. Expect: the 5 `task_failed` errors from today (17:01–19:49, including `import_dbd_schema … Too many open files (os error 24)`) to be findable. What the code does: 200 rows cover only about the last minute. `curl /api/logs?limit=1000` spans 21:37→21:41 (4 minutes), 879 `tasks` info rows and 121 `narration_cache` warns, so errors from earlier in the day never appear unless you pick Level = error. **gap** (`activity-logs.svelte.ts:26-29`, default `since=all`, `limit=200`)
- [ ] Set Level = warn. Expect warn **and above**. What the code does: an exact match only (`+page.svelte:115` title "exact level match"), so errors disappear. **polish/question**
- [ ] Open the Source dropdown. Expect daemon/cli/mcp/app. What the code does: options come only from the rows already loaded (`activity-logs.svelte.ts:169-174`). Live, all 1000 rows are `daemon`, and `source=mcp|cli|app` each return 0 rows. So either nothing but the daemon writes `public.logs`, or the other writers never reach it. Module is likewise only `tasks` and `narration_cache`. **question** (the description at `+page.svelte:72` promises "Daemon, cli, mcp and app activity")
- [ ] Watch the log for a minute. `narration_cache: model reply failed validation after retry — copy not cached` fires many times a minute (121 in 4 minutes). It is out of scope for this screen, but it is the noise that pushes everything else off the page. **bug (daemon)**
- [ ] Compare the Level control with Source and Module. Level is a native `<select>`; the other two are rokkit `Select` (`+page.svelte:112` vs `130,142`). **polish**
- [ ] Look for a way to change a worker's schedule. The comment says it is editable; there is no control. **gap** (`+page.svelte:77-78`)
- [ ] Consider a daemon that returns 0 tasks. The section simply vanishes (`+page.svelte:82`), with no "no background tasks" line. **polish**

## /traceability

**Purpose.** Cross-workspace doc↔code drift: documents that mention symbols that no longer exist (broken) or whose signature has changed (drifted).

**Shows.** A rollup headline and one group per workspace (broken-first). Each row is the drift detail plus a status chip; a row opens the workspace in its own window (`+page.svelte:21-31`). When nothing is indexed it shows the empty state "no documents indexed … This is not a clean bill of health" (`+page.svelte:68-74`). Files: `+page.ts`, `traceability-load.ts:24-39`, `traceability-state.svelte.ts`.

**API**
| Endpoint | Verdict |
|---|---|
| `GET /api/workspaces` (`api.ts:287`) | 200, 133 workspaces. A failure is the page error (`traceability-load.ts:33`). OK. |
| `GET /api/workspaces/{id}/drift` ×133 (`api.ts:457`) | 200 for all 133. Every one is `{items:[], total:0, drifted:0, broken:0, indexedDocs:0}`. A failed workspace is named in `unread` (`fan-out.ts:12-21`). OK. **Truncation:** the daemon caps items at `LIMIT 200` per workspace (`crates/senseid/src/db/pg_store/workspaces.rs:790-792`), and the page's rollup counts `items` (`traceability-load.ts:35`). The loader comment says "no cap: a silent truncation would hide drift" (`traceability-load.ts:19`); the cap is in the daemon. It is latent today because there are no rows. The wire also carries `expectedSignature`, `actualSignature` and `detectedAt`, which the screen drops (`traceability-state.svelte.ts:55-66`). |

**Why the baseline counted 0 clickables.** The only clickables are drift rows, and there are none: 0 docs are indexed in all 133 workspaces (the `indexedDocs` sum is 0). The screen shows the honest empty state. That is correct behaviour, not a wiring fault.

**Interactions.** Drift row → `openWorkspaceWindow` (new window). Retry on the error state → `invalidateAll`. Nothing else.

**Mockup diff** (`p4-traceability.png`, which is the per-workspace version in a project window). The mockup has: a hero card for the top drift ("docs/auth.md § Refresh flow has drifted…") with a **"run fix-drift prompt →"** button; "checked nightly · 18 docs tracked"; per-row doc path § section, symbol path (`lumen-auth/src/refresh.rs::rotate_token`) and "synced 21d ago"; and a "broken link" marker. The observatory screen has none of the hero, the fix action, the symbol path, the age or the docs-tracked count. It shows only `detail` and a status chip, grouped by workspace.

**Checklist**
- [ ] Open Traceability. Expect "no documents indexed". Code and live data agree: 0 doc nodes in any workspace. This screen cannot show anything until markdown is indexed. **gap**
- [ ] Read the header. It says "Every doc-to-symbol link, checked nightly" (`+page.svelte:57`), while the empty state says "sensei does not index markdown yet" (`+page.svelte:73`). The two contradict each other, and the scan actually rides the analyzer's due signal (`crates/senseid/src/tasks/analyzer_scheduler.rs:66-72`), not a nightly job. **polish**
- [ ] Time the page load. It makes one drift read per workspace with no batching: 133 requests (`traceability-load.ts:34`). **polish (perf)**
- [ ] Once drift exists, check whether a workspace with more than 200 drift rows reports its true count. The daemon `LIMIT 200` (`workspaces.rs:792`) silently caps it. **bug (latent)**

## /upgrades

**Purpose.** Practice to adopt. Lane 1 is the Dōjō downstream inbox (apply/pin/mute). Lane 2 is the local analyzer's pending recommendations that map to an installable artifact (skill/agent/rule/lint).

**Shows.** Lane 1, "From your Dōjō": cards with type pill, state chip, scope, attribution and Apply/Pin/Mute. Lane 2, "From your analyzer": buckets of cards; each card links to the workspace. Files: `upgrades-load.ts:43-61`, `buckets.ts`, `dojo-upgrades-state.svelte.ts`, `+page.svelte`.

**API**
| Endpoint | Verdict |
|---|---|
| `GET /api/upgrades` (`api.ts:1196`) | 200 `{"artifacts":[],"unread_count":0}`. The shape matches `dojoUpgrades`/`dojoUnread`. An error shows `ScreenState` (`+page.svelte:218`). OK. |
| `GET /api/workspaces` | 200, 133. OK. |
| `GET /api/workspaces/{id}/recommendations?status=pending` ×133 (`api.ts:532`) | All 200, but **slow**: about 5.7s each (sensei: 50 recs, 35 KB), and **19s** for the whole fan-out at 6-way concurrency (`xargs -P 6`). The wire key is `actionType` (camelCase); the loader handles both spellings (`upgrades-load.ts:30-32`). 4,483 pending recommendations in total: `library_update` 4202, `create_agent` 90, `audit_stale` 75, `enrich_memory` 41, `write_skill` 31, `revise_rule` 28, `promote_pattern` 15, `cross_workspace` 1. The screen keeps the installable 265 and silently drops the other 4,218 (`upgrades-load.ts:16-18,58`). |
| `POST /api/upgrades/{id}/apply`, `/mute`, `/pin` (`api.ts:1206-1210`) | Traced only, not called. Success → `invalidateAll`; failure → `actions.error` is shown (`dojo-upgrades-state.svelte.ts:118-130`, `+page.svelte:214`). |

**Interactions.** Lane 1: Apply, Pin and Mute call the daemon directly, with no confirmation. Apply "lands it into your local rules and memory" (`+page.svelte:210-211`). Lane 2: each card is an `<a href="/workspace/{id}/overview">` (`+page.svelte:125-127`). This is a plain anchor in the **main** window. The observatory layout intercepts only settings paths (`(observatory)/+layout.svelte:78-84`), and Traceability deliberately opens workspaces in their own window ("never replaces the observatory main window", `traceability/+page.svelte:18-20`). So clicking an Upgrades card replaces the observatory with a workspace page inside the main window. Lane 2 has no accept or dismiss action at all.

**Mockup diff** (`14-upgrades.png`). The mockup is a master/detail layout. It has a header stat strip (5 received · weekly cadence · +9% avg FTR lift), kind filter chips (all/Agents/Skills/Commands/Lints), and a left list with source count, FTR lift and "battle-tested". The detail pane has "why for you", contributors, adoptions, FTR lift, stack, numbered "what it does" steps, and **Install for ‹workspace› / Preview in sandbox / Defer / Dismiss**. The app has a two-lane card grid instead: no filter chips, no detail pane, no FTR/adoption evidence, no Preview/Defer, a "Commands" kind that does not exist, and local cards with no action except a link out.

**Checklist**
- [ ] Open Upgrades. Expect a quick load. What the code does: it waits on 133 recommendation reads at about 5.7s each, so the page takes tens of seconds (19s measured with 6 concurrent curls). **bug (perf)** (`upgrades-load.ts:56`)
- [ ] Click any analyzer card. Expect the workspace window to open. What the code does: the main window navigates to `/workspace/{id}/overview` and the observatory rail is replaced. **bug** (`+page.svelte:125-127`)
- [ ] Scroll Lane 2. 265 cards with no filter, no paging and no dismiss: Agents 90, Lints 75, Rules 69, Skills 31. **gap**
- [ ] Look for the "Other" bucket. It can never fill: the loader keeps only the 5 `INSTALLABLE_ACTION_TYPES`, and all 5 map to a named bucket (`upgrades-load.ts:16-18` vs `buckets.ts:39-47`). The `other` bucket and its copy "surfaces the row instead of silently dropping it" are dead, and `promote_pattern` and `cross_workspace` recs **are** silently dropped. **bug**
- [ ] Read Lane 1's empty state: "Connect a Dōjō to receive shared learnings." Seven Dōjō memberships already exist (`GET /api/dojo/memberships`), so the copy points the user at the wrong cause. **polish** (`+page.svelte:221-225`)
- [ ] When a Dōjō artifact arrives, click Apply. It is applied immediately with no confirmation or preview, and there is no undo on the screen. **question** (`+page.svelte:95-101`)

## /share-review

**Purpose.** The upstream publish gate: the next approved-but-unsent memory-share batch, with what each item looks like after redaction, which items are held, and a Publish button.

**Shows** (live). The policy bar shows Destination chips `organization/<employer-org>`, `organization/<family-org>`, `organization/sensei-hq`, `organization/sensei-hq` (**duplicated**), cadence `manual`, "0 to ship", "1 held", and a disabled "Nothing to publish" button. Below it is one held item: type `principle`, empty title, the body "Content withheld — flagged for residual identifier risk.", attribution named, "won't ship this batch". Files: `+page.ts`, `ShareReviewScreen.svelte`, `share-review-state.svelte.ts`.

**API**
| Endpoint | Verdict |
|---|---|
| `GET /api/share-review/next-batch` (`api.ts:1225`) | 200 with batch `e8041370-…` (approved **2026-08-29**, the only approved batch in `sensei.memory_share_batches`) and one item, `state:"held"`, `title:""`, `body:""`. A load error is shown with Retry (`+page.ts:11`, `ShareReviewScreen.svelte:154-155`). OK. |
| `POST /api/share-review/{batch}/publish` (`api.ts:1229`) | Traced only. The handler maps "not found" → 404 and "not approved" → 409 (`crates/senseid/src/api/handlers/share_review.rs:53-62`). |

**Why the baseline counted 0 clickables.** The crawler runs `/share-review`, `/dojo/*` and `/upgrade*` links-only (`app/e2e/tests-inspect/baseline-crawl-inspect.spec.ts:22`), and on top of that the only button here is disabled because 0 items are shippable. It is a crawler artifact plus a real stuck state (below).

**Interactions.** Publish fires on the first click when there are 10 or fewer shippable items; only a batch of more than 10 asks for confirmation (`share-review-state.svelte.ts:22,100-102`; `ShareReviewScreen.svelte:53-61`). There is no per-item hold, edit or remove, no link to the memory, and no way to dismiss a batch.

**Mockup diff.** There is no dedicated mockup. `16-collective.png` links to it as "Review next batch (4 insights) →" from Sharing. The app has no such link; Share review is a separate rail entry (`observatory-nav.ts:106`).

**Checklist**
- [ ] Open Share review. You see a batch approved on 2026-08-29 with its only item held. It can never publish, and it stays "next" forever: `next_unsent_approved_batch` returns the oldest approved batch with any unsent member, `LIMIT 1` (`crates/senseid/src/db/pg_store/dojo.rs:256-270`). Any later approved batch is hidden behind it, and the screen offers no way out. **bug**
- [ ] Read the held item. It says "flagged for residual identifier risk" (`ShareReviewScreen.svelte:99-101`). The preview also holds items for "nothing generalised for this memory yet" (`nothing_shareable`, `crates/senseid/src/dojo/contribute.rs:925-931`; `types.ts:1348-1349`), and with an empty title and body you cannot tell which memory it is or why it is held. **bug (wrong reason, no link)**
- [ ] Count the destination chips: `organization/sensei-hq` appears twice. Two memberships (`7cc4c667…`, `e8857504…`) share the tenant key; the preview does not de-duplicate (`contribute.rs:915-920`), and the chips are keyed by index (`ShareReviewScreen.svelte:193`). **bug**
- [ ] Note that the destination list includes `organization/<employer-org>` (an employer) and `personal`/dev registries at `localhost:5173`/`127.0.0.1:5173`. Confirm that these are the intended targets before anything publishes. **question**
- [ ] With 1–10 shippable items, click Publish. It sends immediately with no confirmation, and leaving the machine cannot be undone. **bug (irreversible, no confirm)** (`share-review-state.svelte.ts:100-102`)
- [ ] Look for the item's `example`. The wire type carries `example`, "the synthetic example shipping alongside body" (`types.ts:1342-1345`), and the daemon preview fills it (`contribute.rs:955`), but the screen never renders it (`ShareReviewScreen.svelte:93-102`). Part of what leaves is never shown for review. **bug**
- [ ] Ask whether what you review is what ships. The preview uses `NoLlm`, the deterministic strip only (`contribute.rs:850-862`, `share_review.rs:37`). Publish uses `GatewayGeneralizer`, a model rewrite (`contribute.rs:832`). The text published can differ from the text reviewed. **question**
- [ ] Set Sharing → both destinations off ("Nothing leaves the machine while both destinations are off", `dojo/sharing/+page.svelte:112-114`), then come here. Publish is still live: `contribute_batch` never reads `collective_preferences` (`contribute.rs:826-846`). Only the scheduler honours `destination=none` (`crates/senseid/src/tasks/contribute_scheduler.rs:19-23,108`). **bug (promise not kept)**

## /dojo/connections

**Purpose.** Read-only list of the Dōjō tenants this machine's personas belong to, joined by signing in.

**Shows.** The headline "7 connections · 0 healthy." Seven cards, each with the tenant key, kind pill, dojo_url, sync chip, role, attribution, "no workspaces bound" and the org slugs. A "Sign in to a Dōjō" link sits in the header. Files: `+page.ts`, `+page.svelte`, `dojo-connections-state.svelte.ts`.

**API**
| Endpoint | Verdict |
|---|---|
| `GET /api/dojo/memberships` (`api.ts:1165`) | 200, a 7-element array. Fields read (`id, tenant_key, kind, dojo_url, sync_status, role, attribution_default, bound_workspaces, org_slugs`) are all present. An error shows `ScreenState` (`+page.ts:11`). The shape is OK. **Content:** all 7 have `sync_status: "authenticating"`, `last_heartbeat_at: null`, `last_seq: 0` and `bound_workspaces: []`. `organization/sensei-hq` appears twice (ids `7cc4c667`, `e8857504`). Registries mix `http://localhost:5173` and `http://127.0.0.1:5173`. There is a `personal/hi` membership via `sso`. |

**Interactions.** Only "Sign in to a Dōjō" (`href="/settings/dojo"`, `+page.svelte:76-80`), which `beforeNavigate` turns into the Settings window (`(observatory)/+layout.svelte:78-84`). Cards have no actions: you cannot disconnect, bind a workspace, re-authenticate or set a default.

**Mockup diff.** No mockup for this screen. `hq/_dojo-teams*.png` is the marketing "For teams · Dōjō" page (the loop Contribute → Accumulate → Triage → Approve → Distribute), not an app screen.

**Checklist**
- [ ] Read the headline. It will always say "0 healthy": no code writes `last_heartbeat_at` and the `sync_status` setter has no callers. The source says so itself (`dojo-connections-state.svelte.ts:5-10`). Meanwhile the `dojo_sync` worker reports `last_ok: true` and "cycle complete … allowed=4 denied=1 mapped=5 unmapped=152 pushed=0" (`GET /api/tasks/scheduled/dojo_sync/log`). Every chip reads "authenticating" while sync is in fact running. **bug**
- [ ] Count the cards. There are 7, including two identical `organization/sensei-hq` cards and dev-registry rows (`localhost:5173` vs `127.0.0.1:5173`, `personal/hi`). The duplicates also show up as duplicate Share-review destinations. **bug / question** (is this stale test residue?)
- [ ] Look for "bound workspaces". All read "no workspaces bound", and nothing on this screen lets you bind one. **gap**
- [ ] Click "Sign in to a Dōjō". Expect the Settings window to open at Dōjō, with the main window staying put (`+layout.svelte:78-84`). **verify**

## /dojo/sharing

**Purpose.** The collective-sharing preferences: destination (global commons / company Dōjō), cadence, which categories may leave, and the default attribution.

**Shows** (live). Both destination switches are off, with "Nothing leaves the machine while both destinations are off." Cadence `manual`; all 7 categories on; attribution **Anonymous** selected; "using defaults · never changed". Files: `+page.ts`, `+page.svelte`, `collective-sharing-state.svelte.ts`.

**API**
| Endpoint | Verdict |
|---|---|
| `GET /api/preferences/collective` (`api.ts:1239`) | 200 `{"destination":"none","cadence":"manual","categories":{…7 true},"attribution_default":"anonymous","updated_at":null}`. The shape matches `CollectivePreferences`. OK. |
| `PUT /api/preferences/collective` (`api.ts:1242`) | Traced only. Whole-object read-modify-write, adopting the server's response; on failure it shows `state.error` and the control reverts (`collective-sharing-state.svelte.ts:275-287`). |

**Why the baseline counted 0 clickables.** The crawler is links-only for `/dojo/*` (`baseline-crawl-inspect.spec.ts:22`) and skips `role=switch` (`:73`). The screen does have about 12 live controls. It is a crawler artifact.

**Interactions.** Every switch and chip saves immediately with a PUT, and shows "saving…" or an error. Turning on "Global commons" is a one-click opt-in to sending data outward, with no confirmation. The Retry on the load error runs `invalidateAll`, but the controller is built once from `untrack(() => data.preferences)` (`+page.svelte:24-25`), so after a failed first load `state` stays `null` and the screen stays on the error even when the retry succeeds.

**Mockup diff** (`16-collective.png`, "Settings · Collective intelligence"):
- **Header stats.** The mockup has 184 lifetime · 612 users helped · 12-week streak; the app has none.
- **Sharing mode.** The mockup has a 3-way Auto-share / **Review before sharing** / Off; the app has two destination switches.
- **Cadence.** The mockup has daily/weekly/**monthly** plus "Review next batch (4 insights) →"; the app has manual/daily/weekly and no link to Share review.
- **Categories.** The mockup has Patterns, Anti-patterns, Corrections, FTR signals, Tool usage, Stack stats, Model preferences, Skills authored, in two columns with checkboxes. The app has Memories, Patterns, Rules, Prompts, Guards, Skills, Agents in one column with switches.
- **Sharing history.** The mockup has a table (date · count · categories · "helped N"); the app has none.
- **Attribution.** The app has a section the mockup does not.

**Checklist**
- [ ] Read the attribution options. The copy says Dereferenced is "the safest, and the default" (`collective-sharing-state.svelte.ts:170-173`). The daemon default is `anonymous` (`crates/senseid/src/collective/preferences.rs:41`), and the live never-saved row shows Anonymous selected. **bug (copy contradicts data)**
- [ ] Leave both destinations off, then go to Share review. Publish there ignores this setting (see /share-review). The promise "Nothing leaves the machine" holds only for the scheduler. **bug**
- [ ] Toggle "Global commons" on. It saves instantly, with no "are you sure" for an outward data flow. **question**
- [ ] With cadence = manual, the subtitle says "Manual pauses batching until you ask for one" (`+page.svelte:120`). Look for where you ask for one. Batches are created in a workspace window's Memories (`routes/(workspace)/workspace/[id]/memories/memories-state.svelte.ts` uses `tryCreateMemoryShareBatch`/`tryDecideMemoryShareBatch`). Nothing here links there or to Share review. **gap**
- [ ] Stop the daemon, open Sharing, start the daemon, and click Retry. Expect the form. What the code does: it stays on "Failed to load sharing preferences" until you navigate away and back (`+page.svelte:24-25,70-72`). **bug (minor)**
- [ ] Compare the switches with rokkit's Switch. They are hand-rolled `role=switch` buttons with inline geometry (`+page.svelte:30-49`), the same family as UI-walk item 5. **polish**
## `/help`

**Purpose.** Static help: quick start, the keyboard-shortcut table, a short FAQ.

**Shows.** Three static sections (`help/+page.svelte:12-84`); the shortcut table comes from `SHORTCUTS`
(`lib/shortcuts.ts:14-19`), which `shortcuts.spec.ts` holds to the real bindings.

**API.** None.

**Interactions.** None on the page. Reached only from Help → Keyboard Shortcuts (⌘?) (`src-tauri/src/lib.rs:343`); not
in the rail or the palette.

**Mockup diff.** No mockup (`p9-about.png` is the workspace window's About page, not this screen).

**Checklist.**
- [ ] **bug** — FAQ "What is the daemon?" says it runs at `localhost:7749` (`help/+page.svelte:75`). The daemon is on
  7744 (`appstate.svelte.ts:16`, rail footer, live curl).
- [ ] **bug** — FAQ defines FTR as "First-Try Rate" (`help/+page.svelte:65-68`); Today's header calls it
  "First-Try-Right" (`+page.svelte:37`).
- [ ] **gap** — The shortcut table omits ⌘, (Preferences…, `lib.rs` `PREFERENCES_ACCELERATOR`), which opens the new
  settings window.
- [ ] **question** — Not reachable from the rail or ⌘K; only from the Help menu. Add a palette entry?
- [ ] **polish** — Content is left-aligned at `max-w-[640px] px-8` (`help/+page.svelte:10`) while other screens centre
  their column (`mx-auto`, e.g. `+page.svelte:29`).

## Top issues (ranked)

1. **bug — fabricated baseline on /impact.** `COALESCE(r.baseline_ftr, 0)` (`crates/senseid/src/db/pg_store/metrics.rs:71`)
   turns a missing baseline into 0%, so all 9 "positive" verdicts read "0% → 75%, +75pp" while the API's own
   `baselineFtr` is null. This breaks the project rule against fabricating data.
2. **bug — Share review: data can leave against the user's setting, and the queue is stuck.** Publish calls
   `contribute_batch` without reading `collective_preferences` (`crates/senseid/src/dojo/contribute.rs:826-846`), so
   Sharing's "Nothing leaves the machine" is not enforced on this path.
   - Batches of 10 or fewer publish on one click, without confirmation.
   - The item's `example` ships but is never shown.
   - The preview text is produced without the model, while publish rewrites it with one.
   - The oldest approved batch is held and blocks every later batch (`pg_store/dojo.rs:256-270`).
3. **bug — the Playground labels writes and paid model calls as queries.** Kinds are guessed from tool names
   (`instruments/playground-groups.ts:67-68,102`), so `save_memory`, `accept_proposal`, `infer`, `consensus` and
   `generate_image` get "Query →", the hint "returns information", and no confirmation.
4. **bug — FTR missing is drawn as 0.**
   - Today's 14-day strip uses `unwrap_or(0.0)` (`pg_store/metrics.rs:1341`); 10 of the 14 live points are 0.0.
   - Workspaces' quality signals use `COALESCE(ftr_7d, 0)` (`pg_store/metrics.rs:261`), so sensei (88%) and gateway (100%)
     show amber "needs attention" (`workspaces/buckets.ts:118`).
5. **bug — Insights hides almost everything.**
   - Recommendations are cut at 200 (`patterns.rs:1067`) out of 27,663 pending, and "Soon" reads "nothing brewing"
     beside 23k medium ones.
   - Memory strength (0–5) is drawn as 0–1 (`insights/insights-board.svelte.ts:150`), so every memory shows a full bar.
   - "Apply" only marks a recommendation accepted; it never materializes anything.
6. **bug — Replay.** `ftr` is a boolean on the wire but a number in the type (`app/src/lib/types.ts:489`), so the rail
   badge reads "100%c" for every session that was not first-try. The timeline stops at 200 calls without saying so
   (`handlers/sessions.rs:149`), and the whole screen scrolls as one page (UI walk item 8, confirmed at
   `instruments/+page.svelte:168,553`).
7. **bug — status that is always the same, whatever the truth.**
   - The rail footer always reads "daemon · running" (`ObservatorySidebar.svelte:45`).
   - Dōjō connections always shows "0 healthy · authenticating", because nothing writes heartbeats
     (`dojo-connections-state.svelte.ts:5-10`).
8. **bug — /sessions counts and charts disagree.** The daemon filters by completion time (`pg_store/sessions.rs:48`) and the
   charts by start day (`lib/sessions-digest.ts:290`), so 7d reads "2 sessions" over empty charts.
   - The median duration is wall-clock (~174h).
   - One session shows ~5B input tokens as "4994M".
   - No session has been captured since 2026-10-20.
9. **bug — failures shown as empty or as stale data.**
   - Sources shows "No knowledge sources." when its load fails, with no Retry (`knowledge-sources/+page.svelte:55-62,100`).
   - A Libraries search with no hits says "No libraries indexed." (`libraries/+page.svelte:41-46`).
   - A failed memory detail leaves the previous memory on screen (`lib/memoryState.svelte.ts:126-127`).
   - Triage Accept/Reject fail silently.
   - Sharing's Retry never recovers (`dojo/sharing/+page.svelte:24-25`).
10. **bug — performance fan-out.** /workspaces, /impact and /upgrades each send one or two requests per workspace for
    all 133 workspaces (`workspaces-load.ts:29`, `lib/impact-load.ts:26`, upgrades ~19s). Health waits 12–27s on
    `/api/instruments/tools-health`. "Runs in" fix 3b8017ed is not in any release: the installed daemon ignores
    `watched=true` (133 workspaces, 16.7s).
11. **gap — destructive or consequential actions without confirmation.** Insights Dismiss; Memories Archive and Dismiss;
    Consolidation approve and re-consolidate (which spends on a model); Sources Remove; Playground server remove;
    Share review Publish.
12. **bug — dead navigation.**
    - Recent sessions rows link to `/sessions#<id>`, which nothing reads (`RecentSessions.svelte:60`).
    - The Upgrades analyzer cards replace the observatory with `/workspace/{id}/overview` in the main window
      (`upgrades/+page.svelte:125-127`).
    - The native menu items Check for Updates, Toggle Sidebar and What's New do nothing (`src-tauri/src/lib.rs:78,110,257`).
    - Help gives the daemon port as 7749 instead of 7744 (`help/+page.svelte:75`).
13. **question — Activity logs.** The default view (All, 200 rows) covers about a minute, because a repeating
    `narration_cache` warning fills it. It hides today's `task_failed` errors, including "Too many open files (os error 24)"
    in `import_dbd_schema`. Every row's source is `daemon`.
14. **gap — vs mockups.**
    - Today's hero has no source or "noticed" line, and its cards name no workspace.
    - Workspaces has no "+ new" button and no 7D stat.
    - Impact has no Revise, Keep monitoring or Dismiss.
    - Memories has no search, no workspace filter and no Promote/Enrich.
    - `09-sessions.png` is blank, so Sessions has no usable mockup.
