# Change: Group Relay's lanes by run — tasks in priority and run order, attributed to agent and model (#422)

## Why
Now lists lanes grouped by crew. A person running a sprint wants to see the **run**: what is
waiting (in priority order), what is being worked and by whom, and what finished — in the order
it ran, attributed to the agent (lane) and the model that did it. The user also wants the same
work seen **by agent** as an alternative.

## What Changes
- Now's Lanes section groups by **run** (default): one group per open run — its title, state and
  progress — listing the run's tasks:
  - **Waiting** tasks in **priority order** (the backlog position of the item each came from;
    Task 0 first; ties by the plan's order);
  - **In progress** tasks with the lane holding them (codename · assistant · model), the step;
  - **Done** tasks in the **order they ran** (claimed → completed), each attributed to the lane
    and model that implemented it and the one that reviewed it.
  Lanes on no run task (idle, not responding) sit in a "Not on a run task" group after the runs.
- An alternative **By agent** view: one group per lane (codename · assistant · model · presence)
  listing the tasks it ran in this run, in the order it ran them, then what it holds now.
- A "By run | By agent" switch on the Lanes heading; the choice is the person's (kept per viewer).
- Daemon: the Now read carries each open run's tasks with what the view needs (priority, claimed
  and completed times, implementer and reviewer lanes with family/version/model) — or a run-tasks
  read the screen joins — and task events keep it live (the row-carrying events of #418).

## Impact
- Specs: `relay-now` (Lanes section grouping and views).
- Code: `database/ddl/view/relay/now.ddl` (or a run-tasks view), `crates/senseid` relay read +
  events, `packages/state/relay` (types, `toLaneGroups` → run/agent selectors, RelayState),
  `app/src/lib/relay` (LaneList/LaneRow → run groups; the switch), `/preview` fixtures, the run
  journey e2e (#426) gains run-group pins.
- Not in scope: reordering tasks from this view (priority is set in the Queue).
