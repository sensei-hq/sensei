# 札 · Workspace window · Tickets

**Segment:** 04 · The workspace window
**Route:** `/workspace/[id]/tickets` (was `/sprints` before 2026-10-22: tickets first, sprint a filter)
**Source mockup:** none. Layout follows the workspace window's list + detail screens
(Sessions); numbers follow the Sessions digest's formatters.
**App files:**
- `app/src/routes/(workspace)/workspace/[id]/tickets/+page.svelte`
- `tickets-state.svelte.ts` (state, words, actions) and `TicketsScreen.svelte` (template)
- the sidebar entry `{ id: 'tickets', kanji: '札', label: 'Tickets' }` in
  `workspace-sidebar-view.ts` `SECTIONS`, after `impact` and before `integrations`
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

**Eyebrow:** `札 · TICKETS`. **Header:** the board name (`board_name`, else
`board #<board_ref>`). **Sub-line:** "Active AI time and tokens per sprint and ticket,
from AI sessions whose branch or prompt names the ticket. Read from Jira; nothing is
changed there."

## States

**Loading.** "Reading tickets…".

**Not connected** (`GET …/sprints` → 404 `not_connected`): "This workspace has no tracker
connected." and a link **Connect one in Integrations** → `/workspace/[id]/integrations`.
Never shown for a failed read.

**Daemon error** (any other failure, including the daemon not answering): the mapped
message (same words as the Integrations screen's table; one shared mapping) and **Retry**.

**Ready.** One column: a toolbar, an optional usage strip, the ticket table.

1. **Toolbar.**
   - **Sprint filter** — a type-to-search `Select` (rokkit, `filterable`), labelled
     "Sprint": **All tickets** (default), each sprint newest first as "<name> · <state>",
     then **Backlog · N tickets** (from `GET …/backlog`).
   - **Only tickets with AI sessions** — a labelled switch, off by default.
   - A count line: "N tickets", or with the switch on "M of N tickets has AI sessions".
2. **Usage strip** — only when a sprint or the Backlog is chosen (All tickets has none):
   - a sprint with a window: "N sessions in this sprint's window", then three parts —
     **Linked to this sprint**, **Linked to other tickets**, **Not linked** — each with
     sessions, **active AI time**, fresh tokens; **Not linked** is a button when > 0 and
     opens those sessions, each linking to the replay. Without a window, one line:
     `not_started` → "Not started. Usage counts from the sprint's start."; `no_dates` →
     "Jira has no dates for this sprint, so its sessions can't be placed in it." Never zeros.
   - the Backlog: "N tickets in no sprint", then one part — **Linked to backlog
     tickets** — all-time (the backlog has no window), each session once.
3. **Ticket table** (`GET …/tracker-tasks`, `?sprint=<id>`, or `?backlog=true`): ticket
   (key above its summary) · **sprint** (its current sprint, or "Backlog") · status ·
   sessions (branch/prompt) · active AI time · cycle time · fresh tokens · out tokens ·
   cost. Rows are
   buttons that open the ticket. **Shared note** under it when any ticket's
   `usage.shared > 0`: "A session linked to several tickets counts in full for each; the
   sprint total counts it once."
4. **Ticket detail** (replaces the table; **Back to <filter>** returns): key, summary,
   status, the usage strip for the ticket, a **Time** line (active AI time split as
   "Claude 30m · you 10m", session time, and cycle time with its first and last dates:
   "2d… · Mon 10:00 → Wed 17:20"), **By repository** (name, sessions, active AI
   time, fresh tokens), and **Sessions** (started, the links as text — "branch", "prompt
   ×2" — and the key as written when it differs from the ticket's current key: "as
   AUTH-420"), each linking to the session replay.

## Numbers

- Active AI time: `compactDuration(activeMinutes(active_secs))` ("1h 04m"); tokens:
  `compactTokens`; cost: `$1.50`. An unknown value is "—", never `0`. The column header
  says **active AI time**, never "hours" or "time spent".
- Claude / you: `claude_secs` / `user_secs`, formatted like active AI time; they add up
  to it. Session time: `wall_secs`, the sessions' clock time, idle inside them included.
- Cycle time: `durationMinutes(first_at, last_at)` through `compactDuration` ("55h 20m"),
  from the first linked session's start to the last one's end; "—" when `last_at` is
  `null`.
- A sprint's total is its own `linked_sprint` usage, never a sum of its tickets (shared
  sessions would count twice).
- `tokens_in` is not shown.

## Done gate

1. Sidebar shows **Tickets** (札) between Impact and Integrations; the route renders.
2. A connected project lists every ticket first, each naming its sprint (or Backlog);
   the sprint filter offers All tickets, each sprint and Backlog, searchable by typing;
   choosing one loads its tickets and its usage strip; the AI-only switch narrows the list.
3. A project without a tracker shows the not-connected state with the Integrations link;
   a failed read shows the daemon error with Retry, never the not-connected state.
4. A future sprint shows "Not started…" and no zeros; a sprint without dates shows the
   `no_dates` line.
5. Every unknown number renders "—"; a genuinely zero session count renders `0`.
6. A ticket opens to its by-repository breakdown and its sessions, each linking to the
   replay; a session that wrote an old key shows "as <old key>".
6b. The Backlog filter lists exactly the tickets no sprint has (a discovered backlog ticket appears there at once); its total counts a session once.
7. a11y: the filter and the switch are labelled; ticket rows are buttons with names; axe
   finds no serious/critical violations in the four states.
8. A ticket row shows its cycle time; its detail shows Claude / you adding up to the
   active AI time, the session time, and the cycle time with its first and last dates.
   A ticket whose sessions have no hook events shows "—" for each, never `0m`.

## Wrong gate

- "Not connected" shown because a read failed.
- A `0` (or `0m`, `$0.00`) for a value the daemon reported as `null`.
- A sprint total computed by adding its tickets.
- "hours" or "time spent" without "active AI time".
- A cycle time computed by adding session times, or session time by adding active times.
- Any assignee, author or other person attribute shown.
- Any request that changes Jira, or any write route called from this screen.

## Related

- Connection: [`project-integrations.md`](project-integrations.md)
- Daemon: [`spec/daemon/tracker-jira-linkage.md`](../daemon/tracker-jira-linkage.md)
