## Context
Now reads `relay.now` (lanes, run summaries with counts) and the stream; `GET /relay/runs/{id}`
already returns a run's tasks by state with their last lane. Tasks link to backlog items
(`item_id` → `items.position`), carry `assigned_at`/`completed_at`, and `relay.actions` records
who claimed, reported and reviewed.

## Goals / Non-Goals
- Goals: the run as the unit on Now; priority and run order; attribution to agent and model; a by-agent view.
- Non-Goals: reordering from Now; historical runs (Past runs has its own view).

## Decisions
- Data path: extend the Now read with a `run_task` section (one row per task of an open run)
  rather than a second read per run — Now stays one read plus the stream, and task events
  already carry rows (#418).
- Priority: item position at the time the run started is not stored; use the current position
  (the person may reprioritise the backlog; the run reflects it).

## Resolved (user, 2026-11-16)
- Several crews (main window): **crew → run → tasks** — each crew stays the outer group under its
  repository heading, its runs inside, then its lanes on no task. The workspace window (one crew)
  shows the runs directly.
- "Agent" in By agent = **each lane** (one session): codename, assistant, model, presence.
- A done task names **implementer and reviewer** on the row.

## Built (2026-11-16)
- `run_tasks` is a query in `relay_now` (JSON per row, times via `iso()`), not a `relay.now` view section.
- The reviewer is the lane of the task's latest `reviewed` action (the review claim clears after a review).
- A run group is headed by the existing `SprintBand`; the standalone Sprints section shows only By agent.
- Lanes on no run task are labelled "Not on a run task" (a lane may hold a task outside any run); with no
  open run the lanes are drawn alone, as before. `toLaneGroups` is superseded by `toRunGroups`.
- By agent rows name only their reviewer: the group already names the lane, its assistant and model.
- The view choice lives in `RelayState.laneView`, kept through an injected store (the app's localStorage).
