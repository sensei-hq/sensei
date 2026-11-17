## ADDED Requirements

### Requirement: Lanes grouped by run
The Now view SHALL group its Lanes section by open run by default: one group per run in the
order the person created them, headed by the run's title, state and progress, then a group of
lanes on no run task.

#### Scenario: A run with waiting, active and done tasks
- **GIVEN** an active run with two waiting tasks, one task held by a lane, and two done tasks
- **WHEN** the person opens Now
- **THEN** the run's group lists the waiting tasks first, in priority order
- **AND** then the held task with its lane's codename, assistant, model and current step
- **AND** then the done tasks in the order they ran, each naming the lane and model that did it and the one that reviewed it

#### Scenario: A lane on no task
- **GIVEN** a live lane that holds no task of an open run
- **WHEN** the person opens Now
- **THEN** it is listed under "Not on a run task" after every run group, with its presence

#### Scenario: The main window with two crews
- **GIVEN** the main window holds two crews, each with an open run
- **WHEN** the person opens Now
- **THEN** each crew is an outer group under its repository heading
- **AND** inside each, that crew's run groups, then its lanes on no task

#### Scenario: No open run
- **GIVEN** a crew with lanes and no open run
- **WHEN** the person opens Now
- **THEN** the lanes are listed under "Not on a run task" and no run group is drawn

### Requirement: Priority order for waiting tasks
Waiting tasks of a run SHALL be ordered by priority: the backlog position of the item each task
came from, Task 0 before the plan's tasks, and the plan's order breaking ties.

#### Scenario: Items reprioritised in the backlog
- **GIVEN** two waiting tasks from backlog items at positions 3 and 1
- **WHEN** the person views the run group
- **THEN** the task from position 1 is listed before the task from position 3

### Requirement: Run order and attribution for done tasks
Done tasks SHALL be listed in the order they ran (by when each was claimed, then completed), and
each SHALL name the implementing lane (codename, assistant and version, model) and the reviewing
lane; a value the daemon did not report reads "not reported", never a default.

#### Scenario: Two tasks done by different agents
- **GIVEN** task A claimed at 10:00 by cobalt-lyra (Claude Code 2.1, claude-opus-5-5) and reviewed by coral-spica, and task B claimed at 09:40 by coral-spica
- **WHEN** the person views the run group
- **THEN** B is listed before A
- **AND** A reads "cobalt-lyra · Claude Code 2.1 · claude-opus-5-5 — reviewed by coral-spica"

#### Scenario: A model not reported
- **GIVEN** a done task whose lane reported no model
- **THEN** its attribution reads "model not reported"

### Requirement: By agent view
The Lanes section SHALL offer a "By agent" view: one group per lane (codename, assistant, model,
presence) listing the tasks it ran in this run in the order it ran them, then the task it holds now.
The person's choice between "By run" and "By agent" SHALL be kept for that person.

#### Scenario: Switching to By agent
- **GIVEN** the Lanes section in "By run"
- **WHEN** the person picks "By agent"
- **THEN** each lane is a group with its run tasks in run order
- **AND** reopening Now later shows "By agent" again

### Requirement: Live from the stream
Run groups and agent groups SHALL update from the relay event stream without a reload: a claim,
a step, a report, a review and a merge each move the task where it belongs.

#### Scenario: A claim moves a waiting task
- **GIVEN** the run group with a waiting task
- **WHEN** a lane claims it
- **THEN** the task moves to in progress under that lane with no reload
