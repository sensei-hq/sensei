## ADDED Requirements

### Requirement: Orphaned tasks re-queue as new tracked tasks

Every crash-orphaned execution row SHALL return as a new task: new session
ID, `retry_number` carried forward, `retried_from_execution_id` pointing at
the prior attempt's uuid PK. Bounded to rows carrying the orphan marker,
retryable kinds, and `retry_number < MAX_RETRIES`. The drill-down SHALL show
the lineage ("retried from #old-id after crash").

#### Scenario: A crash returns its orphans with history intact

- **WHEN** the daemon dies with 5 retryable tasks in flight and restarts
- **THEN** 5 new tasks enqueue with carried retry numbers, each attempt row
  names the exact prior row it retries, and the 5 orphan rows stay failed
  with their marker — nothing overwritten, nothing lost

### Requirement: Recovery layers dedup, never double-run

Boot recovery SHALL run layered — orphan reconcile, abandon open waves,
wave re-drive, folder resume, orphan sweep — with each enqueue through
`enqueue_unique`, so work covered by an earlier layer is a no-op in a later
one.

#### Scenario: A re-driven file is not also re-queued

- **WHEN** a crashed folder's files return via folder resume and the orphan
  sweep runs after
- **THEN** each file processes exactly once; the sweep's duplicate enqueues
  dedup away and the boot log counts them as deduped, not run

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

### Requirement: Missed metric days backfill, sync boot pass is safe

Metric waves SHALL re-compute missed days via the `as_of` path; sync SHALL
run its existing boot pass, safe because marks land only on acked pushes
(re-pushed work is at most unmarked, and dōjō-side keyed writes converge).

#### Scenario: A crash mid-metric-day recomputes that day

- **WHEN** the daemon dies mid-metrics-compute and restarts
- **THEN** the missed day recomputes via the backfill path and the daily
  grain shows no gap; a sync boot pass terminal arrives with no error

### Requirement: Schedules need no new recovery

Schedule self-healing (stale `running_since` clear + next-tick pass) SHALL
be asserted as sufficient: no boot re-run, no new machinery.

#### Scenario: A crash mid-pass runs clean next tick

- **WHEN** the daemon dies mid-scheduled-pass and restarts
- **THEN** no stale running state shows, and the next scheduled tick runs
  the pass normally
