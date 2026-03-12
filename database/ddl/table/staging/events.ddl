set search_path to staging;

create table if not exists events (
  id           uuid
, user_uuid    text
, session_id   text
, repo_id      uuid
, phase        text
, tool         text
, project_path text
, input        jsonb
, ts           timestamptz
, created_at   timestamptz default now()
);

create unique index if not exists events_id_ukey on events(id);
