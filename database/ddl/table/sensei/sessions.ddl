set search_path to sensei, extensions;

create table if not exists sessions (
  id             uuid        primary key default gen_random_uuid()
, repo_id        uuid        not null references sensei.repos(id) on delete cascade
, status         text        not null default 'active'
                             check (status in ('active', 'completed', 'crashed'))
, last_heartbeat timestamptz not null default now()
, created_at     timestamptz not null default now()
);

create index if not exists sessions_repo_id_status_idx on sessions(repo_id, status);
create index if not exists sessions_last_heartbeat_idx on sessions(last_heartbeat)
  where status = 'active';

comment on table sessions is
'One row per MCP server process (i.e. per agent session).
- Created when the agent calls get_session_context for the first time
- last_heartbeat is updated on every tool call; used for on-demand crash detection
- status transitions: active → completed (clean checkpoint) or active → crashed (detected lazily)
- Crash detection: on get_session_context, prior active sessions idle for >10 min are marked crashed';
