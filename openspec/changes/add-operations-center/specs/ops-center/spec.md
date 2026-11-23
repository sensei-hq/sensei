## ADDED Requirements

### Requirement: Domain store contract

Every background-work domain SHALL own a singleton store shaped
`{source, state, selectors, ui, fault}` after the relay/scan exemplars.

#### Scenario: A new domain plugs into the same shape

- **WHEN** the jobs, schedules, or sync domain is added
- **THEN** it ships an injected `StateStream` source with validated
  parsing, snapshot-hydrate + in-place/id-re-read apply, pure unit-tested
  selectors, explicit progress UI state, and a rendered fault slot

### Requirement: Operations center surface

One Operations surface SHALL show Now (running + progress), Queue
(listings + depth), Failures (aggregated feed), Schedules (rows + log
tails + controls), and Sync/Metrics/Transcript (state rows + triggers).

#### Scenario: One screen answers running, queued, failed, why

- **WHEN** the user opens the center during a busy, partially failing run
- **THEN** running jobs show progress and current work, queued jobs show
  kind/path/wait, failures name class and reason, and schedules show
  standing — with no second screen needed

### Requirement: Drill-down, retry, ack

Rows SHALL expand to children/attempts, failed work SHALL offer retry, and
alerts SHALL offer ack/silence.

#### Scenario: A failure is inspected and silenced

- **WHEN** the user opens a failed job, retries it, then acks a standing
  alert
- **THEN** children and attempts render, the retry re-queues, and the ack
  silences that alert class until it clears or recurs

### Requirement: Singleton ownership rule

Cross-screen domains SHALL live in singletons; page-local state is allowed
only for single-visit runs.

#### Scenario: Navigation never loses job state

- **WHEN** the user navigates away mid-run and back
- **THEN** every domain's progress, faults, and UI state survive (no
  remount resets, no lost busy flags)
