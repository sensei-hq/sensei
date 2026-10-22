# Several trackers per workspace, at different levels — design (#318, D23)

**Status:** design for review (2026-10-22). No code until the questions below are answered.

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
   shows each one's sync state; the Sprints screen merges the sprint trackers.

## Questions (the user's)

- **Q1 · What is a "level" in practice?** Is it different **boards/projects** in one Jira site (a portfolio
  board for epics, team boards for sprints) — the proposal above — or something else (e.g. one board whose
  epics, sprints and tasks are read separately)?
- **Q2 · "Possibly one per task":** does a task-level tracker mean a tracker you **configure**, or the link
  sensei **derives** from a change to its ticket (proposal point 3)?
- **Q3 · Several sprint trackers in one workspace:** should the Sprints screen **merge** them (one timeline)
  or show them **side by side** (one per team)?

## Done gate

- A workspace with an epic tracker and two sprint trackers syncs all three; tasks and sprints carry their
  tracker; epics/milestones read from the epic tracker only.
- The Sprints screen shows both sprint trackers' sprints (per Q3); usage per ticket still attributes once.
- The single-tracker workspaces of today keep working unchanged (their tracker becomes `level = sprint`).
