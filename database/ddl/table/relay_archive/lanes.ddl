set search_path to relay_archive, extensions;

-- Lanes of closed crews (#393): same columns as relay.lanes plus
-- archived_at; same primary key, row-local checks and per-crew code unique;
-- no foreign keys.
create table if not exists relay_archive.lanes (
  user_id         uuid            not null,
  id              uuid            not null,
  crew_id         uuid            not null,
  code            text            not null,
  title           text,
  suggested_name  text,
  role            relay.lane_role not null default 'contributor',
  family          text            not null,
  version         text,
  model           text,
  host            text,
  capabilities    text[]          not null default '{}',
  repo            text,
  worktree        text,
  token_hash      text            not null,
  checkpoint      jsonb,
  registered_at   timestamptz     not null default current_timestamp,
  unregistered_at timestamptz,
  limit_paused_at timestamptz,
  limit_scope     relay.limit_scope,
  limit_resumes_at timestamptz,
  limit_note      text,
  archived_at     timestamptz     not null default current_timestamp,
  constraint lanes_pkey primary key (user_id, id),
  constraint lanes_pause_coherent check (
    (limit_paused_at is null) = (limit_scope is null)),
  constraint lanes_id_v7 check (substring(id::text from 15 for 1) = '7'),
  constraint lanes_code_per_crew unique (user_id, crew_id, code)
);

create index if not exists lanes_archived_at_idx
  on relay_archive.lanes (archived_at);

alter table relay_archive.lanes enable row level security;

comment on table relay_archive.lanes is
'Lanes of closed crews. Moved here at crew close; purged past 30 days.';
comment on column relay_archive.lanes.archived_at is 'When the row moved here; the purge horizon counts from it.';
