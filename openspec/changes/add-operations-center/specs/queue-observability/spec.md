## ADDED Requirements

### Requirement: Queue listings

The daemon SHALL list pending, running, and blocked tasks with kind, path,
wait time, and blocked-by dependencies — not counts alone.

#### Scenario: An operator sees what is actually queued

- **WHEN** the queue holds work
- **THEN** a listing shows each task's kind, path, state, queue wait, and
  which dependency blocks it, paginated over a snapshot read

### Requirement: Per-kind depth and failed-task list

The daemon SHALL expose per-kind queue depth and a failed-task list with
reasons.

#### Scenario: A pile-up names its kind

- **WHEN** one kind dominates the queue or failures
- **THEN** the depth listing and failed list name the kind, the tasks, and
  their error reasons

### Requirement: Backoff and orphan visibility

Backoff-pending retries SHALL be visible before re-enqueue, and boot
orphans SHALL stay marked as pre-restart failures.

#### Scenario: A retry in backoff is not invisible

- **WHEN** a failed task waits out its backoff
- **THEN** it appears as backoff-pending with its next attempt time rather
  than vanishing until re-enqueue

### Requirement: Abandonment detail and budget signal

Abandonments SHALL be listed per task (not aggregates alone), and tasks
nearing their watchdog budget SHALL signal it.

#### Scenario: A stuck task is found before the watchdog fires

- **WHEN** a task approaches its kind budget
- **THEN** it appears as nearing-budget with elapsed time, and an abandoned
  task keeps its per-task row with the watchdog reason

### Requirement: Schedule skip reasons and pass progress

Schedules SHALL expose skip reasons and in-pass progress beyond
`running_since`.

#### Scenario: A skipped tick says why

- **WHEN** a schedule skips a tick (disabled, not due, outside window,
  wrong day)
- **THEN** the skip reason is readable on the schedule instead of living
  only in debug logs
