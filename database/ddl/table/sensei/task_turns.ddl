set search_path to sensei, extensions;

create table if not exists task_turns (
  id                       uuid        primary key default gen_random_uuid()
, task_session_id          uuid        not null references sensei.task_sessions(id) on delete cascade
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, tool                     text        not null
, success                  boolean
, duration_ms              integer
, created_at               timestamptz not null default now()
, modified_at              timestamptz not null default now()
);

create index if not exists task_turns_task_session_id_idx
    on task_turns(task_session_id, created_at desc);

create index if not exists task_turns_folder_id_idx
    on task_turns(folder_id, created_at desc);

comment on table task_turns is
'One row per tool call within a task session.
- folder_id denormalized for direct indexed queries without join
- success: null if unknown, false if tool threw';

comment on column task_turns.id
     is 'Surrogate primary key (UUID).';
comment on column task_turns.task_session_id
     is 'Foreign key to task_sessions — groups this turn under its parent task.';
comment on column task_turns.folder_id
     is 'Foreign key to folders — denormalized for direct indexed queries.';
comment on column task_turns.tool
     is 'Name of the MCP tool invoked during this turn.';
comment on column task_turns.success
     is 'Whether the tool call succeeded. Null if unknown, false if tool threw.';
comment on column task_turns.duration_ms
     is 'Wall-clock duration of the tool call in milliseconds.';
comment on column task_turns.created_at
     is 'Timestamp when the row was first created.';
comment on column task_turns.modified_at
     is 'Timestamp of the last modification to this row.';
