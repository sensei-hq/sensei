# Tasks: add-relay-run-archive (#393)

Every numbered item is red-first: a failing test committed, then the code that turns it green, in
separate commits. Rust DB tests run serial (`--test-threads=1`). Before Rust commits:
`cargo fmt --all --check`, clippy `-D warnings`.

## Slice 1 — the archive schema (DDL only)

- [ ] 1.1 Red: `database/tests/relay/relay_archive_schema.sql` asserts the seven tables
  `relay_archive.{crews,lanes,leases,runs,run_items,tasks,actions}` exist with every column of their
  `relay` twin plus `archived_at`; `runs`/`crews` carry `delete_after not null`; `lanes.token_hash`
  is nullable; no FK leaves `relay_archive`; RLS on; `authenticated` holds no write grant.
- [ ] 1.2 Red: `database/tests/relay/relay_fk_loosened.sql` inserts a `relay.items` row whose
  `added_by` names no lane, and a `relay.actions` row whose `reply_to` names no action; both succeed.
- [ ] 1.3 Green: `database/ddl/table/relay_archive/*.ddl` (one file per table, `set search_path to
  relay_archive, relay, extensions;`, reusing `relay.*` enums), `database/policies/relay_archive/*.sql`
  (owner read, as `database/policies/relay/actions.sql`). Defaults use `current_timestamp`, never
  `now()` (`check-no-now-relay` scans only `relay` files, but the archive's search path includes
  `relay`). Default scope only: confirm the `dojo` scope's `includes` (design.yaml:52-134, which
  lists `relay` by name) does not pick up `relay_archive`.
- [ ] 1.4 Green: drop `items_adder_fk` (`relay/items.ddl:24`) and `actions_reply_fk`
  (`relay/actions.ddl:26`) in the DDL source; add `database/apply/before/relay_archive_fks_loosened.sql`
  (`alter table … drop constraint if exists`, no-op when absent) and register it in `design.yaml`
  `apply.before` with a one-line comment. Update the two column comments.
- [ ] 1.5 `dbd reconcile` (twice if dbd#121) then `dbd diff --exit-code` clean against local `sensei`
  and a fresh `sensei_test`; `make test-db-if-reachable` green; `make test-template` refreshed.
- [ ] 1.6 e2e proof: `make test-app-e2e` with `relay-run.spec.ts` green on the new DDL (the e2e
  globalSetup deploys the repo `database/` via `SENSEI_DDL_DIR`).

## Slice 2 — archive ended runs; reads span both schemas

- [ ] 2.1 Red (store, `relay.rs` tests): `archive_runs_moves_an_ended_run_whole` — a completed run with
  two tasks, run_items and five actions archives in one pass: zero rows for it in `relay.runs`,
  `relay.run_items`, `relay.tasks`, `relay.actions`; identical rows (column for column) in
  `relay_archive.*`; `archived_at` set; `delete_after = completed_at + 30 days`.
- [ ] 2.2 Red: `archive_runs_skips_open_runs` (draft, planning, active untouched; crew lanes untouched);
  `archive_runs_takes_cancelled_and_crewless_drafts`; `archive_runs_keeps_a_live_reply_to_a_moved_action`.
- [ ] 2.3 Red: `relay_runs_lists_archived_runs_with_counts` (total/done/members/delete_after equal to
  what `relay.run_list` served before archival); `run_detail_reads_an_archived_run` (tasks by state,
  members from live lanes while the crew is open, from archived lanes after);
  `task_view_reads_an_archived_task_read_only` (body served; claim/report/checkpoint on it → 404).
- [ ] 2.4 Red: `unmet_deps_reads_archived_deps` — a live task depending on an archived `done` task is
  satisfied; on an archived `cancelled` task it is unmet. Same assertion for `relay.task_queue`
  (`database/tests/relay/relay_task_queue.sql`), which gains an archive check, and for the two
  inline dep checks in `relay.rs` (`relay_claim_next` ~4153, the sweeper's broker-gate pick ~6852).
- [ ] 2.5 Red (sweep, `relay_lease_sweep.rs`): `pass` emits `run.archived` per archived run, after
  the `run.completed` events `sweep_at` sends (`sweep_events` is fed only by the sweep transaction's
  report, so the archive step's events go out from `pass` after it commits); the report `pass`
  returns gains `archived_runs`. Note `with_run_row` (`relay/rows.rs:27`) reads `relay_run_row`,
  which must find the run in the archive for the event to carry its row.
- [ ] 2.6 Green: `PgStore::relay_archive_runs(now)` — per root: lock run `for update`, re-check state,
  lock its tasks, `insert … select` children then root into `relay_archive`, delete in reverse order,
  commit. Called from `pass` after the sweep commits.
- [ ] 2.7 Green: run list, run detail, task view, task history and `unmet_deps` read the archive for
  ended work (one shared helper per shape, no copied SQL); `relay.task_queue` view treats an archived
  dep by its archived state. An action insert that loses the race to archival answers 404.
- [ ] 2.8 e2e proof: `app/e2e/tests/relay-run.spec.ts` done stage — after Past runs lists the run, open
  it: `[data-archived-run]` shows its tasks and members; psql on `sensei_e2e` reads zero
  `relay.tasks` rows for the run and the same count in `relay_archive.tasks`.

## Slice 3 — archive closed crews

- [ ] 3.1 Red: `archive_crews_moves_a_closed_crew_whole` — closed crew, two unregistered lanes, a
  released lease, crew-level messages, a runless task: zero rows in `relay.*` for the crew id
  (crews, lanes, leases, lane_waiters, actions, tasks); a full copy in `relay_archive.*` (lanes with
  `token_hash` null); `delete_after = archived_at + 30 days`.
- [ ] 3.2 Red: `archive_crews_waits_for_open_runs` (closed crew with an active run stays until the run
  ends and archives); `archive_crews_never_touches_an_active_crew`; a live backlog item added by one
  of its lanes keeps its `added_by`.
- [ ] 3.3 Red: sweep report `archived_crews`; no new event shape (the crew already left Now on close).
- [ ] 3.4 Green: `PgStore::relay_archive_crews(now)`, after `relay_archive_runs` in the same pass.
- [ ] 3.5 e2e proof: new `app/e2e/tests/relay-archive.spec.ts` "a closed crew leaves relay" — the
  RelayCrew driver registers a coordinator and a contributor, posts a message; the person closes the
  crew (`POST /relay/crews/{id}/close`); the Crew view drops it with no refresh; within one sweep pass
  psql reads zero `relay.*` rows for the crew id and the copy in `relay_archive.*`.

## Slice 4 — the daily purge

- [ ] 4.1 Red: `schedule.rs` `SCHEDULABLE` agreement test fails until `relay_purge` is listed, seeded
  and described.
- [ ] 4.2 Red (store): `purge_deletes_runs_past_delete_after` — fed `now`: a run 31 days past
  completion is gone from every `relay_archive` table; one 29 days past is intact (time-boxed);
  `purge_deletes_crews_30_days_after_archival`; `purge_batches` (250 due roots → three batches of at
  most 100, each its own transaction, all gone).
- [ ] 4.3 Red: the purge task emits `run.deleted` (action `remove`, `id` = run id) per purged run.
- [ ] 4.4 Green: `crates/senseid/src/tasks/relay_purge.rs` (`tick`/`pass(now)`/`spawn` on
  `run_scheduled("relay_purge")`, missing-schema warns like `relay_lease_sweep.rs:155`);
  `PgStore::relay_purge_archive(now, batch)`; `SCHEDULABLE` + `DESCRIPTIONS` entries;
  `{"name":"relay_purge","enabled":true,"interval_secs":86400,…}` in
  `database/import/staging/schedules.jsonl` (seeded by `dbd import`); spawn in `api/server.rs`.
- [ ] 4.5 e2e proof: `relay-archive.spec.ts` "the daily job deletes runs 30 days on" — psql seeds two
  archived runs (completed 31 and 29 days ago) in `sensei_e2e`; the Crew view's Past runs shows both;
  `PATCH /api/tasks/scheduled/relay_purge` sets `interval_secs: 1`; the 31-day run leaves Past runs
  with no refresh, the 29-day run stays and reads "Deleted … in 1 day"; the schedule is restored
  in `afterAll`.

## Close-out

- [ ] 5.1 `openspec validate add-relay-run-archive --strict`.
- [ ] 5.2 Full gates green (`make test`, clippy, fmt, `dbd diff --exit-code`); the e2e run restores the
  brew daemon on :7744.
- [ ] 5.3 Final commit says `Closes #393`; tick every task above.
