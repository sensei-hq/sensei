## ADDED Requirements

### Requirement: Archive schema
The database SHALL hold a `relay_archive` schema, defined as dbd DDL files and deployed only in the
daemon's default scope, with one table for each of `relay.crews`, `relay.lanes`, `relay.leases`,
`relay.runs`, `relay.run_items`, `relay.tasks` and `relay.actions`. Each archive table SHALL carry
every column of its live twin plus `archived_at`; archived runs and crews SHALL carry `delete_after`.
No archive table SHALL hold a foreign key, and no live table SHALL reference an archive table.

#### Scenario: Schema deployed and diff-clean
- **GIVEN** a database deployed from `database/` with dbd
- **WHEN** `database/tests/relay/relay_archive_schema.sql` runs and `dbd diff --exit-code` runs
- **THEN** the seven archive tables exist with their live columns plus `archived_at`
- **AND** `dbd diff` reports no difference

#### Scenario: The dōjō never gets the archive
- **WHEN** the `dojo` scope deploys
- **THEN** no `relay_archive` object is created

### Requirement: Live references survive archival
`relay.items.added_by` and `relay.actions.reply_to` SHALL be plain ids with no foreign key, so a
live backlog item or action may name a lane or action that has moved to the archive.

#### Scenario: A reply to an archived action
- **GIVEN** a crew-level message that replies to an action on a task of a run
- **WHEN** that run archives
- **THEN** the message stays in `relay.actions` with its `reply_to` unchanged

#### Scenario: A backlog item added by an archived lane
- **GIVEN** a backlog item added by a lane of a crew that closes
- **WHEN** the crew archives
- **THEN** the item stays in `relay.items` with its `added_by` unchanged

### Requirement: Archive an ended run
On each relay sweep pass the daemon SHALL move every `completed` or `cancelled` run, its
`run_items`, its tasks and every action on those tasks from `relay` to `relay_archive`, one run per
transaction, re-checking the run's state under a row lock, and SHALL emit `run.archived` for it. A
`draft`, `planning` or `active` run and its crew's lanes SHALL NOT be touched.

#### Scenario: A completed run archives whole
- **GIVEN** a completed run with two tasks, two run items and five task actions
- **WHEN** a sweep pass runs
- **THEN** `relay.runs`, `relay.run_items`, `relay.tasks` and `relay.actions` hold no row of it
- **AND** `relay_archive` holds each row column for column, with `archived_at` set
- **AND** the archived run's `delete_after` equals its `completed_at` plus 30 days

#### Scenario: Open runs are untouched
- **GIVEN** a draft, a planning and an active run in a live crew
- **WHEN** a sweep pass runs
- **THEN** all three runs, their tasks and the crew's lanes remain in `relay`

#### Scenario: The run journey ends in the archive
- **GIVEN** the run journey e2e (`app/e2e/tests/relay-run.spec.ts`) reaches its done stage
- **WHEN** Past runs lists the run and the person opens it
- **THEN** the read-only run screen shows its tasks and members with no refresh
- **AND** `relay.tasks` holds none of its tasks while `relay_archive.tasks` holds all of them

### Requirement: Archive a closed crew
On each relay sweep pass the daemon SHALL move every `closed` crew that has no `draft`, `planning`
or `active` run bound to it, with its lanes, leases, crew-level actions and runless tasks (and their
actions), to `relay_archive`, and SHALL delete its `lane_waiters` rows. An archived lane SHALL NOT
keep its `token_hash`. The archived crew's `delete_after` SHALL be its `archived_at` plus 30 days.

#### Scenario: Closing a crew empties it from relay
- **GIVEN** a crew with two lanes, a lease, crew-level messages and a runless task
- **WHEN** the person closes the crew and a sweep pass runs
- **THEN** no `relay.*` table holds a row with that crew's id
- **AND** `relay_archive` holds a complete copy, its lanes with `token_hash` null
- **AND** `app/e2e/tests/relay-archive.spec.ts` sees the crew leave the Crew view with no refresh

#### Scenario: A closed crew with an open run waits
- **GIVEN** a closed crew whose run is still active
- **WHEN** a sweep pass runs
- **THEN** the crew and its lanes stay in `relay` until the run ends and archives

#### Scenario: An active crew is never archived
- **GIVEN** a crew whose status is not `closed`
- **WHEN** a sweep pass runs
- **THEN** none of its rows move

### Requirement: Ended work stays readable
`GET /relay/runs`, `GET /relay/runs/{id}` and the task view `GET /relay/tasks/{crew}/{task}` SHALL
serve archived runs and tasks with the same shapes as before archival. An archived task SHALL be
read-only: every lane write naming it answers 404. A dependency on an archived task SHALL be
satisfied only when the archived task is `done`.

#### Scenario: Past runs after archival
- **GIVEN** a run archived after it completed
- **WHEN** the person reads `GET /relay/runs`
- **THEN** the run is listed under `archived` with the same `total_tasks`, `done_tasks`, `members`
  and `delete_after` it had before archival

#### Scenario: An archived task view
- **GIVEN** an archived task
- **WHEN** the person opens its task view
- **THEN** its header, steps and history are served
- **AND** a lane's claim, checkpoint or report on it answers 404

#### Scenario: A dependency on archived work
- **GIVEN** a live task depending on one archived `done` task and one archived `cancelled` task
- **WHEN** its unmet dependencies are computed
- **THEN** only the cancelled task is unmet

### Requirement: Daily purge after 30 days
A `relay_purge` schedule, seeded through the staging import with a daily interval, SHALL hard-delete
every archived run and archived crew whose `delete_after` has passed, with all their archived
children, in batches of at most 100 roots per transaction, and SHALL emit `run.deleted` for each
purged run.

#### Scenario: Time-boxed purge
- **GIVEN** an archived run completed 31 days ago and another completed 29 days ago
- **WHEN** the purge runs at now
- **THEN** the first run and all its archived rows are gone
- **AND** the second run is intact

#### Scenario: Batched under load
- **GIVEN** 250 archived runs past `delete_after`
- **WHEN** the purge runs
- **THEN** it commits at most 100 runs per transaction and leaves none due

#### Scenario: Past runs drops a purged run live
- **GIVEN** the Crew view open with Past runs listing a 31-day-old and a 29-day-old run
- **WHEN** the purge schedule runs
- **THEN** the 31-day-old run leaves Past runs with no refresh and the 29-day-old run stays
- **AND** `app/e2e/tests/relay-archive.spec.ts` asserts both
