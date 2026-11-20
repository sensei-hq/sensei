# Archive ended relay runs and closed crews; purge them after 30 days (#393)

## Why
Ended runs and closed crews stay in the live `relay` schema forever. Every broker poll, sweep and
screen read scans them. Today a completed run keeps its `relay.runs` row, its tasks and every
action on them (`crates/senseid/src/db/pg_store/relay.rs:5008` only flips `state`); a closed crew
keeps its crew, lanes, leases and messages (`relay.rs:6024`, `relay.rs:6082` only set
`status = 'closed'`).

The app already promises the missing half. The past-run banner says "Deleted 30 days after it
completed, by a daily job" (`packages/state/relay/crew-selectors.ts:200-201`), and
`relay.run_list` computes `delete_after = completed_at + 30 days`
(`database/ddl/view/relay/run_list.ddl:20-21`). No job deletes anything. `RunChange::Archived` and
`RunChange::Deleted` exist (`crates/senseid/src/api/events.rs:218-219`) but nothing emits them.

## What Changes
- **New schema `relay_archive`** (dbd DDL files, default scope only — never the dōjō). One table per
  archived relay table, same columns plus `archived_at`; ended roots carry `delete_after`. No
  foreign keys into `relay`, so archived rows never pin or block live ones.
- **Two live FKs become plain ids**: `relay.items.added_by` (`items.ddl:24`) and
  `relay.actions.reply_to` (`actions.ddl:26`). A live row may point at a lane or action that has
  moved to the archive. The `tasks.deps` precedent: plain uuids, no FK (`tasks.ddl:29`, comment at
  `tasks.ddl:83`).
- **Archive on run end**: each sweeper pass moves every completed or cancelled run, its
  `run_items`, its tasks and the actions on those tasks into `relay_archive`, one run per
  transaction, and emits `run.archived`.
- **Archive on crew close**: a closed crew with no open run moves with its lanes, leases, crew-level
  actions and runless tasks; its waiter rows are dropped. Active crews are never touched.
- **Reads span both schemas**: `GET /relay/runs`, `GET /relay/runs/{id}`, the task view
  `GET /relay/tasks/{crew}/{task}`, and dependency checks read the archive for ended work. The
  past-run screen keeps working unchanged.
- **Purge**: a new daily `relay_purge` schedule hard-deletes archived runs past `delete_after`
  (`completed_at + 30 days`, the date the app already shows) and archived crews 30 days after
  archival, in bounded batches, and emits `run.deleted`.

## Impact
- Affected specs: new capability `relay-archive`.
- Affected code: `database/ddl/table/relay_archive/*` (new), `database/ddl/table/relay/{items,actions}.ddl`,
  `database/apply/before/` (one new idempotent hook), `database/import/staging/schedules.jsonl`,
  `database/tests/relay/{relay_archive_schema,relay_fk_loosened}.sql` (new),
  `database/tests/relay/relay_task_queue.sql`, `database/ddl/view/relay/task_queue.ddl`,
  `database/policies/relay_archive/*` (new), `crates/senseid/src/db/pg_store/relay.rs`,
  `crates/senseid/src/tasks/relay_lease_sweep.rs`, `crates/senseid/src/tasks/relay_purge.rs` (new),
  `crates/senseid/src/tasks/schedule.rs`, `crates/senseid/src/api/handlers/scheduled_tasks.rs`,
  `crates/senseid/src/api/server.rs`, `app/e2e/tests/relay-archive.spec.ts` (new),
  `app/e2e/tests/relay-run.spec.ts`.
- Related changes: none overlap. `add-relay-run-grouped-lanes` and `update-relay-task-view-attribution`
  read live runs and tasks; this change keeps their reads on `relay` for open work.
- No dōjō or production deployment: `relay_archive` is daemon-local. Released daemons converge to it
  at startup through the normal DDL bundle.
