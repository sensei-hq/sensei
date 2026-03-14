set search_path to sensei, extensions;

create table if not exists task_turns (
  id              uuid        primary key default gen_random_uuid()
, task_session_id uuid        not null references sensei.task_sessions(id) on delete cascade
, repo_id         uuid        not null references sensei.repos(id) on delete cascade
, tool            text        not null
, success         boolean
, duration_ms     integer
, created_at      timestamptz not null default now()
);

create index if not exists task_turns_task_session_id_idx on task_turns(task_session_id, created_at desc);
create index if not exists task_turns_repo_id_idx         on task_turns(repo_id, created_at desc);

comment on table task_turns is
'One row per post-phase tool call within a task session.
- Written by the beat() wrapper in mcp-server.ts after each tool call
- repo_id is denormalized for direct indexed queries without join
- success: null if unknown (pre-phase only), false if tool threw
- Used to compute FTR toolErrorRate signal at checkpoint time';
