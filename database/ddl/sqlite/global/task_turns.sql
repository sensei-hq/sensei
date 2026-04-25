-- task_turns
-- One row per tool call within a task session.
-- Written by the collector daemon from hook events.
-- project_id is denormalized to avoid joins in hot query paths.

create table if not exists task_turns (
  id              text    not null primary key
, task_session_id text    not null references task_sessions(id) on delete cascade
, project_id      text    not null references projects(id) on delete cascade
, tool            text    not null
, success         integer               -- null=unknown, 1=ok, 0=error
, duration_ms     integer               -- null when not available (non-OTLP coordinators)
, usage           text                   -- null=unknown, 'used', 'partial', 'ignored'
                       check (usage is null or usage in ('used','partial','ignored'))
, created_at      text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_at     text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by     text    not null default 'system'
);

create index if not exists task_turns_task_session_idx on task_turns(task_session_id, created_at desc);
create index if not exists task_turns_project_idx      on task_turns(project_id, created_at desc);
