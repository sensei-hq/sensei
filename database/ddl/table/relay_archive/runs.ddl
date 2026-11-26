set search_path to relay_archive, extensions;

-- Ended sprint runs, moved here at completion (#393): the run's own row.
-- Same columns as relay.runs plus archived_at; same primary key; no foreign
-- keys, so archived rows never block live deletes. Purged past 30 days.
create table if not exists relay_archive.runs (
  user_id      uuid            not null,
  id           uuid            not null,
  crew_id      uuid,
  title        text            not null,
  goal         text            not null,
  state        relay.run_state not null default 'draft',
  repo_key     text,
  workspace_id uuid,
  config       jsonb           not null default '{}',
  created_at   timestamptz     not null default current_timestamp,
  started_at   timestamptz,
  completed_at timestamptz,
  archived_at  timestamptz     not null default current_timestamp,
  constraint runs_pkey primary key (user_id, id),
  constraint runs_id_v7 check (substring(id::text from 15 for 1) = '7')
);

create index if not exists runs_archived_at_idx
  on relay_archive.runs (archived_at);

alter table relay_archive.runs enable row level security;

comment on table relay_archive.runs is
'Ended sprint runs. Moved here at completion; purged past 30 days.';
comment on column relay_archive.runs.archived_at is 'When the row moved here; the purge horizon counts from it.';
