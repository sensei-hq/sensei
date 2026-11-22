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
  clears count, behind `GET`/`PUT /api/ops/alerts/acks`; the alerts store
  hydrates on attach and writes through on ack/clear. Alert history itself
  stays derived from the feeds, which are already durable.
- Wave runs persist: `WaveTotals` opens a row, the terminal closes it, in a
  new `activity.wave_runs` table; `GET /api/tasks/waves` serves recent runs
  and the Pipelines/Sync sections render history beside live waves.
- Boot marks waves left open by a dead session `abandoned` and re-drives the
  missed work at dispatcher/wave grain: transcript backfill (idempotent
  per-file), metric compute for the missed day(s) via the `as_of` path,
  sync via its next scheduled pass (no boot pass — the push-ack race),
  index via the existing folder-state resume. Never a blind re-queue of
  orphaned task rows: session task IDs are dead and side effects must not
  double-run.
- Kill-restart e2e matrix over Tauri Playwright: index crash resumes without
  redo, wave crash abandons and re-drives, acks survive reload, schedules
  self-heal.

## Impact

- Affected specs: `alert-persistence`, `wave-history`, `crash-recovery`
  (all new capabilities).
- Affected code: new DDL tables (`activity.wave_runs`,
  `sensei.alert_acks`) + `dbd` deploy; `crates/senseid` wave emit paths +
  boot sequence + new endpoints; `packages/state` alerts store wiring;
  `app` Pipelines/Sync history + ack write-through; `app/e2e` kill-restart
  matrix via the existing daemon stop/start helpers.
- No breaking changes: new tables, new endpoints, additive UI. The boot
  sequence gains two steps (abandon open waves, re-drive) after the existing
  orphan reconcile.
