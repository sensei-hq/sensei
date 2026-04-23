set search_path to activity, sensei, extensions;

create type if not exists task_type_kind
    as enum ('feat', 'fix', 'refactor', 'docs', 'test', 'chore', 'unknown');

create type if not exists task_status
    as enum ('in_progress', 'completed', 'abandoned');


create table if not exists task_sessions (
  id                       uuid        primary key default gen_random_uuid()
, session_id               uuid        references activity.sessions(id) on delete set null
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, task_description         text
, task_type                task_type_kind
, status                   task_status    not null default 'in_progress'
, ftr_score                numeric(4,3)
, ftr_signals              jsonb
, modified_at              timestamptz not null default now()
, completed_at             timestamptz
);

create index if not exists task_sessions_folder_id_idx
    on task_sessions(folder_id, modified_at desc);

create index if not exists task_sessions_session_id_idx
    on task_sessions(session_id)
 where session_id is not null;

comment on table task_sessions is
'Task boundaries within sessions. One row per checkpoint boundary.
- ftr_score: 0.000–1.000, null until checkpoint
- status: in_progress → completed or abandoned
Task-level events (task_start, task_end) are in the events table.';

comment on column task_sessions.id
     is 'Surrogate primary key (UUID).';
comment on column task_sessions.session_id
     is 'Foreign key to sessions.';
comment on column task_sessions.folder_id
     is 'Foreign key to folders.';
comment on column task_sessions.task_description
     is 'Task description passed at get_session_context.';
comment on column task_sessions.task_type
     is 'Auto-detected: feat, fix, refactor, docs, test, chore, unknown.';
comment on column task_sessions.status
     is 'Lifecycle: in_progress → completed or abandoned.';
comment on column task_sessions.ftr_score
     is 'First-Try-Right score 0.000–1.000. Null until checkpoint.';
comment on column task_sessions.ftr_signals
     is 'Raw signal values used to compute ftr_score.';
comment on column task_sessions.modified_at
     is 'Timestamp of last modification.';
comment on column task_sessions.completed_at
     is 'Timestamp when task reached terminal state.';
