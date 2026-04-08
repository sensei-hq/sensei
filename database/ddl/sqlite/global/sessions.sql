-- sessions
-- One row per MCP server process (one per agent session).
-- Created when the agent calls get_session_context for the first time.
-- status transitions: active → completed (clean checkpoint) | crashed (idle >10 min)
--
-- coordinator: which coding assistant is running this session
-- (claude-code | opencode | copilot | kiro | codex | unknown)

create table if not exists sessions (
  id              text not null primary key
, project_id      text not null references projects(id) on delete cascade
, coordinator     text not null default 'claude-code'
, status          text not null default 'active'
                       check (status in ('active','completed','crashed'))
, last_heartbeat  text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, created_at      text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_at     text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by     text not null default 'system'
);

create index if not exists sessions_project_status_idx  on sessions(project_id, status);
create index if not exists sessions_heartbeat_idx       on sessions(last_heartbeat)
  where status = 'active';
