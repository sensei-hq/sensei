# Several trackers per workspace, at different levels — design (#318, D23)

**Status:** decided 2026-10-22 (Q1–Q3 answered by the user: D40). Ready to build.

## The decision this serves

D23: a workspace has **several trackers, at different levels** — a tracker for an epic, separate ones for
sprints, possibly one per task. A new task's tracker is scoped to what it changed; incremental changes are
read against the whole system and what changed. D14: the dōjō owns the integration's credential. D22: each
daemon pulls what it needs.

## What exists (PR #274)

- `sensei.trackers` — one row per workspace (`unique (workspace_id)`), plus `unique (kind, base_url,
  board_ref)`: a Jira board connected to one workspace.
- Everything below hangs off a **tracker**, not off the workspace: `tracker_tasks (tracker_id,
  external_id)`, `sprints (tracker_id, external_id)`, `tracker_task_sprints`, `tracker_task_keys`. So widening
  to several trackers per workspace does not reshape the task and sprint tables.
- The daemon's sync, the CLI (`sensei tracker connect|sync|sprints|usage --workspace`), the app's
  Integrations and Sprints screens all assume one tracker per workspace.

## Proposal

1. **A tracker serves a level.** Add `level` (`epic | sprint | task`) to `sensei.trackers`; drop
   `unique (workspace_id)`; keep one tracker per `(workspace_id, level, board_ref)`. A workspace may have one
   epic tracker (e.g. a portfolio board), one or more sprint trackers (team boards), and task-level ones.
2. **Reads follow the level.** Epics and milestones (T3, V11) read from the epic tracker; sprints and their
   tasks from the sprint trackers (merged per workspace, de-duplicated by issue key); a task-level tracker
   ties one change to its ticket.
3. **"A new task's tracker is scoped to what it changed; incremental changes are read against the whole
   system and what changed"** — a task-level link is made from the change (branch / commit / PR → ticket key,
   as `tracker_task_keys` does today), not configured by hand; the sprint and epic trackers give the whole.
4. **The credential stays the workspace's** (D14): one credential per tracker *site* (`base_url`), shared by
   every tracker on that site in the workspace.
5. **Screens:** the workspace's Integrations screen lists its trackers by level, adds one at a level, and
   shows each one's sync state; the Sprints screen shows the sprint trackers side by side, one lane each (D40).

## Decisions (user, 2026-10-22 — D40)

- **Q1 · A level is a separate board** (a portfolio board for epics, team boards for sprints): each is its own
  tracker on the workspace, tagged with its level.
- **Q2 · A task-level link is derived from the change** (branch / commit / PR → ticket key), never configured.
- **Q3 · Several sprint trackers show side by side, one lane per team**, not merged.

## Done gate

- A workspace with an epic tracker and two sprint trackers syncs all three; tasks and sprints carry their
  tracker; epics/milestones read from the epic tracker only.
- The Sprints screen shows the two sprint trackers side by side, one lane each; usage per ticket still attributes once.
- The single-tracker workspaces of today keep working unchanged (their tracker becomes `level = sprint`).

## Build plan (mapped against develop, 2026-10-22)

What the code assumes today, and the change for each — one tracker per workspace is enforced in five places:

| Layer | Today | Change |
|---|---|---|
| DDL `sensei.trackers` | `unique (workspace_id)`; `unique (kind, base_url, board_ref)` | add `level tracker_level` (enum `epic\|sprint\|task`, default `sprint` so today's rows become sprint trackers); drop `unique (workspace_id)`; **keep** the board key — a board is one tracker at one level, and `(workspace_id, level, board_ref)` alone would collide one board id across two sites |
| Credential (D14) | per tracker row: `secret_service = com.sensei.tracker.<kind>.<uuid>`, fresh per connect, tombstoned by a row trigger | new `sensei.tracker_sites (workspace_id, kind, base_url, account, secret_service)`, unique `(workspace_id, kind, base_url)`; trackers reference their site; the tombstone triggers move to the site, so removing one of several trackers never revokes a shared secret |
| Store | `tracker_for_workspace` (`fetch_optional`), `connect_tracker` replaces the old tracker, `delete_tracker_for_workspace` deletes all | `trackers_for_workspace` → `Vec`; connect adds (or re-keys the same board); disconnect takes a tracker id |
| HTTP | `/api/workspaces/{id}/trackers` returns `[]` or `[one]`; sync/sprints/tasks resolve the one tracker | list returns all with `level`; connect takes `level`; sync/disconnect take `/trackers/{tracker_id}`; sprints return per tracker (lanes); ticket lookup searches the workspace's trackers |
| CLI | `list[0]`; no tracker selector | `--level` on connect; `--board` selector where a workspace has several |
| App | `data[0]` on Integrations and Sprints | Integrations lists trackers grouped by level with "add at a level"; Sprints renders one lane per sprint tracker |

**Epics and milestones (T3, V11) have no reader yet** — the Jira adapter fetches neither, no table holds them. R9 makes
an epic-level tracker sync its board like any other (its issues carry `issue_type`); the epic/milestone *read*
for the planning graph is T3's slice, which reads from `level = epic` trackers only.
