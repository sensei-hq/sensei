# 期 · Project window · Sprints

**Segment:** 04 · The project window
**Route:** `/project/[id]/sprints`
**Source mockup:** none. Layout follows the project window's list + detail screens
(Sessions); numbers follow the Sessions digest's formatters.
**App files:**
- `app/src/routes/(project)/project/[id]/sprints/+page.svelte`
- `sprints-state.svelte.ts` (state, words, actions) and `SprintsScreen.svelte` (template)
- the sidebar entry `{ id: 'sprints', kanji: '期', label: 'Sprints' }` in
  `project-sidebar-view.ts` `SECTIONS`, after `impact` and before `integrations`
- `app/src/lib/api.ts` wrappers: `tryListSprints`, `tryListTrackerTasks`,
  `tryGetTrackerTask`, `tryListUnlinkedSessions`

**Daemon contract:** [`spec/daemon/tracker-jira-linkage.md`](../daemon/tracker-jira-linkage.md)
R7 (usage), R8 (`GET …/sprints`, `…/tracker-tasks[?sprint=]`, `…/tracker-tasks/{key}`,
`…/sessions/unlinked?sprint=`). No new daemon route.

## Purpose

The day-to-day view of the project's tracker: **which sprint, which tickets, and how
much active AI time and how many tokens went into each**, from AI sessions linked to
tickets by key (R6). Connecting the board is the Integrations screen's job; this screen
only reads.

**Eyebrow:** `期 · SPRINTS`. **Header:** the board name (`board_name`, else
`board #<board_ref>`). **Sub-line:** "Active AI time and tokens per sprint and ticket,
from AI sessions whose branch or prompt names the ticket. Read from Jira; nothing is
changed there."

## States

**Loading.** "Reading sprints…".

**Not connected** (`GET …/sprints` → 404 `not_connected`): "This project has no tracker
connected." and a link **Connect one in Integrations** → `/project/[id]/integrations`.
Never shown for a failed read.

**Daemon error** (any other failure, including the daemon not answering): the mapped
message (same words as the Integrations screen's table; one shared mapping) and **Retry**.

**Ready.** Two columns (stacked under `md`):

1. **Sprint list** (newest first, as the daemon orders it). Each row: name, state as
   text (`active` / `closed` / `future`), dates `Jun 15 – Jun 25`, `N tickets`. The
   selected row is `aria-current="true"`. Default selection: the `active` sprint if there
   is one, else the first row.
2. **Sprint detail** for the selected sprint:
   - **Usage strip.** With a window: "N sessions in this sprint's window", then three
     parts — **Linked to this sprint**, **Linked to other tickets**, **Not linked** —
     each with sessions, **active AI time**, fresh tokens. Without a window, one line in
     place of the strip: `not_started` → "Not started. Usage counts from the sprint's
     start."; `no_dates` → "Jira has no dates for this sprint, so its sessions can't be
     placed in it." Never zeros.
   - **Not linked** is a button when its count is > 0: it opens the list of those
     sessions (started, task text), each linking to the session replay.
   - **Ticket table** (`GET …/tracker-tasks?sprint=<id>`): ticket (key above its summary) · status ·
     sessions (branch/prompt) · active AI time · fresh tokens · out tokens · cost. A
     ticket whose current sprint is another sprint shows "now in <name>". Rows are
     buttons that open the ticket.
   - (The Backlog row, below, uses the same ticket table.)
   - **Shared note**: when any ticket's `usage.shared > 0`, under the table: "A session
     linked to several tickets counts in full for each; the sprint total counts it once."
2b. **Backlog** — the last row of the sprint list, "Backlog · N tickets" (from
   `GET …/backlog`), selectable like a sprint. Its detail: "N tickets in no sprint",
   then one usage line — **Linked to backlog tickets**: sessions, **active AI time**,
   fresh tokens — all-time (the backlog has no window), each session once; then the
   same ticket table (`GET …/tracker-tasks?backlog=true`). No "Not linked" part (that
   needs a window). A failed backlog read is the daemon error, never "0 tickets".
3. **Ticket detail** (replaces the table; **Back to Sprint N** returns): key, summary,
   status, the usage strip for the ticket, **By repository** (name, sessions, active AI
   time, fresh tokens), and **Sessions** (started, the links as text — "branch", "prompt
   ×2" — and the key as written when it differs from the ticket's current key: "as
   AUTH-420"), each linking to the session replay.

## Numbers

- Active AI time: `compactDuration(activeMinutes(active_secs))` ("1h 04m"); tokens:
  `compactTokens`; cost: `$1.50`. An unknown value is "—", never `0`. The column header
  says **active AI time**, never "hours" or "time spent".
- A sprint's total is its own `linked_sprint` usage, never a sum of its tickets (shared
  sessions would count twice).
- `tokens_in` is not shown.

## Done gate

1. Sidebar shows **Sprints** (期) between Impact and Integrations; the route renders.
2. A connected project lists its sprints newest first; the active sprint is selected by
   default; selecting another loads its tickets.
3. A project without a tracker shows the not-connected state with the Integrations link;
   a failed read shows the daemon error with Retry, never the not-connected state.
4. A future sprint shows "Not started…" and no zeros; a sprint without dates shows the
   `no_dates` line.
5. Every unknown number renders "—"; a genuinely zero session count renders `0`.
6. A ticket opens to its by-repository breakdown and its sessions, each linking to the
   replay; a session that wrote an old key shows "as <old key>".
6b. The Backlog row lists exactly the tickets no sprint has (a discovered backlog ticket appears there at once); its total counts a session once.
7. a11y: list rows and ticket rows are buttons with names; the selected sprint is
   `aria-current`; axe finds no serious/critical violations in the four states.

## Wrong gate

- "Not connected" shown because a read failed.
- A `0` (or `0m`, `$0.00`) for a value the daemon reported as `null`.
- A sprint total computed by adding its tickets.
- "hours" or "time spent" without "active AI time".
- Any assignee, author or other person attribute shown.
- Any request that changes Jira, or any write route called from this screen.

## Related

- Connection: [`project-integrations.md`](project-integrations.md)
- Daemon: [`spec/daemon/tracker-jira-linkage.md`](../daemon/tracker-jira-linkage.md)
