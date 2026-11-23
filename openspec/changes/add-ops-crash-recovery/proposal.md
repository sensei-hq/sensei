# Change: Operations crash recovery (ack persistence, wave history, boot re-drive)

## Why

The operations center answers "what is running, queued, failed and why" —
but only for this process lifetime. A daemon crash loses the in-memory queue
(boot reconcile fails the orphaned rows and stops), waves that were open die
without a terminal and leave no trace, and an app reload wipes every ack plus
the recurrence memory that drives escalation. Without persistence, recovery
after a crash is partial by construction: index scans resume (folder states +
file fingerprints), everything else restarts from zero with no record of what
died.

## What Changes

- Alert dispositions persist: one row per (class, key) with acked flag and
  clears count in `activity.alert_acks`, behind `GET`/`PUT /api/ops/alerts/acks`;
  the alerts store hydrates on attach and writes through on ack/clear. Alert
  history itself stays derived from the feeds, which are already durable.
- Wave runs persist: `WaveTotals` opens a row, the terminal closes it, in a
  new `activity.wave_runs` table; `GET /api/tasks/waves` serves recent runs
  and the Pipelines/Sync sections render history beside live waves.
- Boot re-queues every crash-orphaned task as a NEW tracked task: new session
  ID, `retry_number` carried forward, lineage via `retried_from_execution_id`
  (the prior attempt's uuid PK — unambiguous across sessions, unlike task
  IDs). History is never mutated: recovery only adds rows. Bounded to rows
  carrying the orphan marker, retryable kinds, and `retry_number < MAX`, so
  poison work terminates instead of resurrecting every boot.
- Layered boot order with `enqueue_unique` dedup so layers never double-run:
  orphan reconcile → abandon open waves → wave re-drive (transcript
  backfill, metric `as_of`, sync boot pass — mark-after-ack makes it safe)
  → folder resume → orphan re-queue sweep as the catch-all.
- Kill-restart e2e matrix over Tauri Playwright: index crash resumes without
  redo, wave crash abandons and re-drives, orphaned tasks return with
  lineage, acks survive reload, schedules self-heal.

## Impact

- Affected specs: `alert-persistence`, `wave-history`, `crash-recovery`
  (all new capabilities).
- Affected code: new DDL tables (`activity.wave_runs`,
  `activity.alert_acks`) + `retried_from_execution_id` column on
  `activity.task_executions` + `dbd` deploy; `crates/senseid` wave emit
  paths + boot sequence + new endpoints; `packages/state` alerts store
  wiring; `app` Pipelines/Sync history + ack write-through + lineage in
  drill-down; `app/e2e` kill-restart matrix via the existing daemon
  stop/start helpers.
- No breaking changes: new tables, one nullable column, new endpoints,
  additive UI. The boot sequence gains layered recovery after the existing
  orphan reconcile; history rows are never mutated, only transitioned
  (running→failed) or added.
