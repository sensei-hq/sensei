set search_path to activity;

-- Wave runs: every pipeline wave leaves a row. `WaveTotals` opens it,
-- the terminal closes it with settled counts (`done`); boot closes rows
-- left open by a dead session (`abandoned`). One table answers both "what
-- waves ran" (history for the center) and "what died mid-wave" (crash
-- recovery input for re-drive).
--
-- `key` is the wave key (dispatcher task id, sync pass uuid, or "manual");
-- `parent_task_id` the dispatcher when there is one. No prune: rows are
-- tiny and wave volume is low (tens per day worst case).
create table if not exists activity.wave_runs (
    id               uuid        primary key default gen_random_uuid()
  , wave             text        not null
  , key              text        not null
  , parent_task_id   bigint
  , total            integer     not null default 0
  , settled          integer     not null default 0
  , outcome          text        not null default 'open'
  , opened_at        timestamptz not null default now()
  , closed_at        timestamptz
);

-- No unique key on (wave, key): keys restart per session (dispatcher task
-- ids), so the same pair recurs across sessions. Closes match the latest
-- open row for the pair.
create index if not exists wave_runs_wave_key_idx
    on activity.wave_runs(wave, key, opened_at desc);

comment on table activity.wave_runs is
'Pipeline wave runs: opened by WaveTotals, closed done by the terminal or abandoned by boot reconcile. History for the operations center; re-drive input for crash recovery.';

comment on column activity.wave_runs.outcome is 'open, done, or abandoned. done = terminal emitted with settled counts; abandoned = owning session died first (boot closed it).';
