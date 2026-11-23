set search_path to relay_archive, extensions;

-- Closed crews (#393): same columns as relay.crews plus archived_at; same
-- primary key and row-local check; no foreign keys.
create table if not exists relay_archive.crews (
  user_id       uuid        not null,
  id            uuid        not null,
  workspace_id  uuid,
  repo_key      text,
  goal          text,
  status        text        not null,
  heartbeat_at  timestamptz,
  started_at    timestamptz not null default current_timestamp,
  completed_at  timestamptz,
  archived_at   timestamptz not null default current_timestamp,
  constraint crews_pkey primary key (user_id, id),
  constraint crews_id_v7 check (substring(id::text from 15 for 1) = '7')
);

create index if not exists crews_archived_at_idx
  on relay_archive.crews (archived_at);

alter table relay_archive.crews enable row level security;

comment on table relay_archive.crews is
'Closed crews. Moved here at crew close; purged past 30 days.';
comment on column relay_archive.crews.archived_at is 'When the row moved here; the purge horizon counts from it.';
