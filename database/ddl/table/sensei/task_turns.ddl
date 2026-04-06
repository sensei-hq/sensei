set search_path to sensei, extensions;

create table if not exists task_turns (
  id              uuid        primary key default gen_random_uuid()
, task_session_id uuid        not null references sensei.task_sessions(id) on delete cascade
, repo_id         uuid        not null references sensei.repos(id) on delete cascade
, tool            text        not null
, success         boolean
, duration_ms     integer
, created_at      timestamptz not null default now()
, modified_at     timestamptz not null default now()
, modified_by     text        not null default current_user
);

create index if not exists task_turns_task_session_id_idx on task_turns(task_session_id, created_at desc);
create index if not exists task_turns_repo_id_idx         on task_turns(repo_id, created_at desc);

comment on table task_turns is
'One row per post-phase tool call within a task session.
- Written by the beat() wrapper in mcp-server.ts after each tool call
- repo_id is denormalized for direct indexed queries without join
- success: null if unknown (pre-phase only), false if tool threw
- Used to compute FTR toolErrorRate signal at checkpoint time';

comment on column task_turns.id is 'Surrogate primary key (UUID).';
comment on column task_turns.task_session_id is 'Foreign key to sensei.task_sessions — groups this turn under its parent task.';
comment on column task_turns.repo_id is 'Foreign key to sensei.repos — denormalized for direct indexed queries without a join.';
comment on column task_turns.tool is 'Name of the MCP tool invoked during this turn.';
comment on column task_turns.success is 'Whether the tool call succeeded; null if unknown (pre-phase), false if the tool threw.';
comment on column task_turns.duration_ms is 'Wall-clock duration of the tool call in milliseconds.';
comment on column task_turns.created_at is 'Timestamp when the row was first created.';
comment on column task_turns.modified_at is 'Timestamp of the last modification to this row.';
comment on column task_turns.modified_by is 'Identity (user, role, or service) that last modified this row.';
