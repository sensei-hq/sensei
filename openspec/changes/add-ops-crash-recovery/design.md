# Design: ops crash recovery

## What already exists (verified, not assumed)

- Boot reconcile (`reconcile_orphaned_task_executions`) fails `running` rows
  from dead sessions. Task IDs reset per session, so orphans can never
  complete — failing them is correct.
- Index resume (`resume_pending_scans`): folders in
  `discovered`/`queued`/`indexing`/`failed` re-enqueue `ProcessGitFolder` at
  boot, deduped by `enqueue_unique`.
- File-grain resume point: the resumed folder fans out only
  `list_unparsed_files` — fingerprints advance on completion, writers upsert
  by path, so parsed files are skipped and a mid-parse crash redoes one file.
- Schedules self-heal: stale `running_since` clears at boot, the next tick
  runs the pass. No boot re-run needed.
- Alert history inputs are durable: executions, index errors, schedules,
  sync rows all persist. Only dispositions (acks) and recurrence memory are
  in-memory.

## D1. Recovery grain: re-drive plus universal orphan re-queue

Every crash-orphaned task returns as a NEW tracked task rather than
resuming in place: resuming the dead session's IDs would overwrite status
and error
info, losing the record of what died. New rows preserve history — the
orphan stays failed-with-marker, the retry carries `retried_from_execution_id`
to the exact prior attempt (uuid PK, unambiguous across sessions where
task IDs restart). Re-drive covers fan-out at dispatcher grain (below);
the orphan sweep catches whatever the fan-outs did not, with each layer
deduping via `enqueue_unique`:

- Index: existing folder-state resume (no change).
- Transcript: enqueue a backfill; per-file ingest overwrites, so re-covered
  sessions converge instead of duplicating.
- Metric: enqueue compute for each missed day via the `as_of` backfill path;
  per-day upserts make re-compute safe.
- Sync: no boot pass. Entity marks make the next scheduled pass skip pushed
  work, but a crash between push and mark-ack would double-push on an
  immediate retry; the bounded cadence delay is the safety margin.
- Orphan sweep: every crash-orphaned task returns as a NEW tracked task
  (new session ID, `retry_number` carried, `retried_from_execution_id`
  lineage), bounded to the orphan marker + retryable kinds +
  `retry_number < MAX`. History is never mutated — recovery only adds rows.

Layered boot order — each layer dedups against the last via
`enqueue_unique`, so nothing runs twice: orphan reconcile → abandon open
waves → wave re-drive → folder resume → orphan re-queue sweep as the
catch-all.

Explicit boundary: tasks queued-but-never-started leave no row (no
write-ahead enqueue), so a crash erases them silently. Dispatcher re-drive
covers fan-out children; un-dispatched one-offs are unrecoverable by design.
A write-ahead queue is a separate, much larger change and is out of scope.

## D2. Open-wave tracking doubles as history

`activity.wave_runs(wave, key, parent_task_id, total, settled, opened_at,
closed_at, outcome)`: `WaveTotals` inserts the open row, the terminal closes
it (`done`), boot closes leftovers (`abandoned`). One table answers "what
waves ran" and "what died mid-wave". No prune: rows are tiny and pass volume
is low (tens per day worst case).

## D3. Ack dispositions, not an alert event log

`activity.alert_acks(class, key, acked, seen_clears, updated_at)`: ack sets
`acked`, a client-observed clear sets `acked=false, seen_clears+1`.
Escalation reads `seen_clears > 0` exactly as today's in-memory history
does. Stale rows (`acked=false`, 90+ days untouched) delete in the PUT path
— no new worker. The feeds remain the single history of record; this table
holds only what derivation cannot recompute.

Schema placement: `activity`, not `sensei`. Per `docs/architecture/data.md`,
`sensei` is the knowledge core plus daemon config; `activity` is runtime
operations data — executions, runs, and upserted state like
`capture_watermarks`. Dispositions hang off execution incidents and are
meaningless without them, so they colocate with `task_executions`, and both
new tables (`alert_acks`, `wave_runs`) join it locally.

## D4. Client derivation stays authoritative for standing

The alerts store keeps deriving standing alerts from the domain states; the
acks table only annotates (acked flag, escalation memory). A derivation bug
therefore degrades to unacked alerts, never to phantom ones — the failure
mode points at the live condition, not at stored state.

## D5. Kill-restart proof via existing helpers

`app/e2e/helpers.ts` already stops/starts the daemon. The matrix kills the
daemon mid-scan, mid-wave, and mid-ack-visibility, restarts, and asserts:
no file re-parsed (fingerprint counts), abandoned waves marked and
re-driven, acks intact. No new harness needed.
