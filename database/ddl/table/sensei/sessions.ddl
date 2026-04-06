set search_path to sensei, extensions;

create table if not exists sessions (
  id             uuid        primary key default gen_random_uuid()
, repo_id        uuid        not null references sensei.repos(id) on delete cascade
, status         text        not null default 'active'
                             check (status in ('active', 'completed', 'crashed'))
, last_heartbeat timestamptz not null default now()
, created_at     timestamptz not null default now()
, modified_at    timestamptz not null default now()
, modified_by    text        not null default current_user
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

comment on column sessions.id is 'Surrogate primary key (UUID).';
comment on column sessions.repo_id is 'Foreign key to sensei.repos — scopes this row to a specific repository.';
comment on column sessions.status is 'Lifecycle state of the session: active, completed (clean checkpoint), or crashed (detected lazily).';
comment on column sessions.last_heartbeat is 'Timestamp of the most recent tool call; used to detect sessions that have gone idle or crashed.';
comment on column sessions.created_at is 'Timestamp when the row was first created.';
comment on column sessions.modified_at is 'Timestamp of the last modification to this row.';
comment on column sessions.modified_by is 'Identity (user, role, or service) that last modified this row.';
