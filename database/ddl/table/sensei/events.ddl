set search_path to sensei, extensions;

create table if not exists events (
  id           uuid primary key default gen_random_uuid()
, user_uuid    text not null
, session_id   text
, repo_id      uuid references repos(id) on delete set null
, phase        text not null check (phase in ('pre', 'post'))
, tool         text not null
, project_path text
, input        jsonb
, ts           timestamptz not null
, created_at   timestamptz not null default now()
);

create index if not exists idx_events_user_uuid on events(user_uuid);
create index if not exists idx_events_ts on events(ts desc);
create index if not exists idx_events_tool on events(tool);
