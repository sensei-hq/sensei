-- api_requests
-- One row per LLM API call captured via OTLP telemetry (Claude Code only for now).
-- Non-OTLP coordinators do not populate this table; cost_usd shown as null in UI.
-- coordinator matches sessions.coordinator for join-free filtering.

create table if not exists api_requests (
  id                    text    not null primary key
, project_id            text    not null references projects(id) on delete cascade
, task_session_id       text    references task_sessions(id) on delete set null
, coordinator           text    not null default 'claude-code'
, prompt_id             text    not null
, input_tokens          integer not null
, output_tokens         integer not null
, cache_read_tokens     integer not null default 0
, cache_creation_tokens integer not null default 0
, cost_usd              real    not null
, duration_ms           integer
, model                 text
, recorded_at           text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_at           text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by           text    not null default 'system'
);

create index if not exists api_requests_project_idx       on api_requests(project_id, recorded_at desc);
create index if not exists api_requests_task_session_idx  on api_requests(task_session_id)
  where task_session_id is not null;
create index if not exists api_requests_prompt_id_idx     on api_requests(prompt_id);
