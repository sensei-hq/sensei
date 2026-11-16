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

## Open Questions
- Q1: Does "By run" replace the crew grouping entirely, or do several crews (main window) show
  crew → run → tasks?
- Q2: "Agent" = the lane (one session) — or should By agent aggregate lanes by assistant/model?
- Q3: Should a done task show both implementer and reviewer, or only the implementer with the
  reviewer on the task view?
