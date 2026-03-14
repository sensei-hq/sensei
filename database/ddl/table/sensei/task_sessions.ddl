set search_path to sensei, extensions;

create table if not exists task_sessions (
  id               uuid        primary key default gen_random_uuid()
, session_id       uuid        references sensei.sessions(id) on delete set null
, repo_id          uuid        not null references sensei.repos(id) on delete cascade
, task_description text
, task_type        text
                   check (task_type in ('feat','fix','refactor','docs','test','chore','unknown'))
, status           text        not null default 'in_progress'
                   check (status in ('in_progress','completed','abandoned'))
, ftr_score        numeric(4,3)
, ftr_signals      jsonb
, created_at       timestamptz not null default now()
, completed_at     timestamptz
);

create index if not exists task_sessions_repo_id_idx    on task_sessions(repo_id, created_at desc);
create index if not exists task_sessions_session_id_idx on task_sessions(session_id)
  where session_id is not null;

comment on table task_sessions is
'One row per agent task (checkpoint boundary).
- session_id: FK to sessions; nullable on session delete
- task_description: optional task description passed by agent at get_session_context
- task_type: auto-detected from task_description keywords
- ftr_score: 0.000–1.000, null until checkpoint is called
- ftr_signals: raw signals used to compute the score (for auditability)
- status transitions: in_progress → completed (via checkpoint) or abandoned';
