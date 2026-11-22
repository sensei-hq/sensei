## ADDED Requirements

### Requirement: Index resume is pinned, not just present

Folder-state resume plus fingerprint-gated fan-out SHALL be covered by a
kill-restart proof: crash mid-scan, restart, and the scan completes with no
file re-parsed.

#### Scenario: A crash mid-index resumes without redo

- **WHEN** the daemon is killed halfway through indexing a repo and restarted
- **THEN** every file parses exactly once across both runs (fingerprint
  counts prove it) and the folder reaches indexed — no full re-walk, no
  duplicated graph writes

### Requirement: Crashed waves re-drive at dispatcher grain

Each pipeline SHALL define its boot re-drive, and the boot sequence SHALL
run it for every wave marked abandoned. Re-drive is idempotent per D1;
orphaned task rows are NEVER individually re-queued.

#### Scenario: A crash mid-backfill re-covers the missed sessions

- **WHEN** the daemon dies mid-transcript-backfill and restarts
- **THEN** the dead wave reads abandoned, a fresh backfill covers the
  un-ingested sessions without duplicating ingested ones, and the new wave
  terminals normally

### Requirement: Missed metric days backfill, sync waits its turn

Metric waves SHALL re-compute missed days via the `as_of` path; sync SHALL
NOT run a boot pass (push-ack race) and instead covers missed entities on
its next scheduled pass.

#### Scenario: A crash mid-metric-day recomputes that day

- **WHEN** the daemon dies mid-metrics-compute and restarts
- **THEN** the missed day recomputes via the backfill path and the daily
  grain shows no gap; sync entities pushed before the crash are not
  re-pushed

### Requirement: Schedules need no new recovery

Schedule self-healing (stale `running_since` clear + next-tick pass) SHALL
be asserted as sufficient: no boot re-run, no new machinery.

#### Scenario: A crash mid-pass runs clean next tick

- **WHEN** the daemon dies mid-scheduled-pass and restarts
- **THEN** no stale running state shows, and the next scheduled tick runs
  the pass normally
