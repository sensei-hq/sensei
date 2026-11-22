## ADDED Requirements

### Requirement: Ack dispositions persist

Ack state SHALL survive app reloads and daemon restarts: acking writes
through to `activity.alert_acks`, attach hydrates it, and a client-observed
clear records the clear (feeding escalation memory) instead of deleting the
row outright.

#### Scenario: An ack survives a reload

- **WHEN** the user acks a standing alert and reloads the app
- **THEN** the alert still stands, still marked acked, with its count
  continuing — not reset, not re-toasted

### Requirement: Recurrence memory persists

A condition that clears and recurs SHALL escalate even across restarts:
`seen_clears` persists per (class, key) and the derivation treats a
previously-cleared key as a recurrence.

#### Scenario: A flapping job escalates across a restart

- **WHEN** a job fails, clears, the app reloads, and the job fails again
- **THEN** the new alert renders escalated (one level up), not flat

### Requirement: Stale dispositions prune themselves

Disposition rows SHALL NOT grow unboundedly: cleared rows untouched for 90+
days delete in the PUT path. No new worker, no scheduled pruner.

#### Scenario: Old clears fade away

- **WHEN** a disposition sits cleared and untouched for 90 days and any ack
  writes through
- **THEN** the stale row is gone; a later recurrence alerts flat (memory
  faded honestly, not kept forever)
