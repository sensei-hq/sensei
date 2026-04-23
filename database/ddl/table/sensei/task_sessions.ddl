set search_path to sensei, extensions;

create table if not exists task_sessions (
  id                       uuid        primary key default gen_random_uuid()
, session_id               uuid        references sensei.sessions(id) on delete set null
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, task_description         text
, task_type                text
                                       check (task_type in ('feat', 'fix', 'refactor', 'docs', 'test', 'chore', 'unknown'))
, status                   text        not null default 'in_progress'
                                       check (status in ('in_progress', 'completed', 'abandoned'))
, ftr_score                numeric(4,3)
, ftr_signals              jsonb
, created_at               timestamptz not null default now()
, modified_at              timestamptz not null default now()
, completed_at             timestamptz
);

create index if not exists task_sessions_folder_id_idx
    on task_sessions(folder_id, created_at desc);

create index if not exists task_sessions_session_id_idx
    on task_sessions(session_id)
 where session_id is not null;

comment on table task_sessions is
'One row per agent task (checkpoint boundary).
- ftr_score: 0.000–1.000, null until checkpoint is called
- ftr_signals: raw signals used to compute the score
- status: in_progress → completed (via checkpoint) or abandoned';

comment on column task_sessions.id
     is 'Surrogate primary key (UUID).';
comment on column task_sessions.session_id
     is 'Foreign key to sessions — the session this task belongs to.';
comment on column task_sessions.folder_id
     is 'Foreign key to folders — which folder this task ran in.';
comment on column task_sessions.task_description
     is 'Task description passed by the agent at get_session_context.';
comment on column task_sessions.task_type
     is 'Auto-detected category: feat, fix, refactor, docs, test, chore, unknown.';
comment on column task_sessions.status
     is 'Lifecycle: in_progress → completed or abandoned.';
comment on column task_sessions.ftr_score
     is 'First-Try-Right score 0.000–1.000. Null until checkpoint.';
comment on column task_sessions.ftr_signals
     is 'Raw signal values used to compute ftr_score.';
comment on column task_sessions.created_at
     is 'Timestamp when the row was first created.';
comment on column task_sessions.modified_at
     is 'Timestamp of the last modification to this row.';
comment on column task_sessions.completed_at
     is 'Timestamp when the task reached a terminal state.';
