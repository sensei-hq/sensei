-- supabase/migrations/20260313000003_phase4_analytics_ftr.sql

-- Task sessions: one per agent task (checkpoint boundary)
create table if not exists sensei.task_sessions (
  id               uuid        primary key default gen_random_uuid(),
  session_id       uuid        references sensei.sessions(id) on delete set null,
  repo_id          uuid        not null references sensei.repos(id) on delete cascade,
  task_description text,
  task_type        text        check (task_type in ('feat','fix','refactor','docs','test','chore','unknown')),
  status           text        not null default 'in_progress'
                               check (status in ('in_progress','completed','abandoned')),
  ftr_score        numeric(4,3),
  ftr_signals      jsonb,
  created_at       timestamptz not null default now(),
  completed_at     timestamptz
);

create index if not exists task_sessions_repo_id_idx    on sensei.task_sessions(repo_id, created_at desc);
create index if not exists task_sessions_session_id_idx on sensei.task_sessions(session_id)
  where session_id is not null;

-- Task turns: one per post-phase tool call
create table if not exists sensei.task_turns (
  id              uuid        primary key default gen_random_uuid(),
  task_session_id uuid        not null references sensei.task_sessions(id) on delete cascade,
  repo_id         uuid        not null references sensei.repos(id) on delete cascade,
  tool            text        not null,
  success         boolean,
  duration_ms     integer,
  created_at      timestamptz not null default now()
);

create index if not exists task_turns_task_session_id_idx on sensei.task_turns(task_session_id, created_at desc);
create index if not exists task_turns_repo_id_idx         on sensei.task_turns(repo_id, created_at desc);

-- Grants
grant all on all tables in schema sensei to anon, authenticated, service_role;
grant all on all sequences in schema sensei to anon, authenticated, service_role;
grant execute on all functions in schema sensei to anon, authenticated, service_role;
