-- supabase/migrations/20260313000002_phase3_session_continuity.sql

-- Sessions: one per MCP server process
create table if not exists sensei.sessions (
  id             uuid        primary key default gen_random_uuid(),
  repo_id        uuid        not null references sensei.repos(id) on delete cascade,
  status         text        not null default 'active'
                             check (status in ('active', 'completed', 'crashed')),
  last_heartbeat timestamptz not null default now(),
  created_at     timestamptz not null default now()
);
create index if not exists sessions_repo_id_status_idx on sensei.sessions(repo_id, status);
create index if not exists sessions_last_heartbeat_idx on sensei.sessions(last_heartbeat)
  where status = 'active';

-- Snapshots: multiple per session
create table if not exists sensei.snapshots (
  id               uuid        primary key default gen_random_uuid(),
  session_id       uuid        not null references sensei.sessions(id) on delete cascade,
  repo_id          uuid        not null references sensei.repos(id) on delete cascade,
  kind             text        not null check (kind in ('manual', 'checkpoint')),
  progress_summary text        not null,
  next_step_hint   text,
  completed_steps  text[]      not null default '{}',
  in_flight_files  text[]      not null default '{}',
  worktree_refs    jsonb       not null default '[]',
  diff_stat_summary text,
  created_at       timestamptz not null default now()
);
create index if not exists snapshots_session_id_idx on sensei.snapshots(session_id, created_at desc);
create index if not exists snapshots_repo_id_idx    on sensei.snapshots(repo_id, created_at desc);

-- Memory items: project-scoped, survive session deletion
create table if not exists sensei.memory_items (
  id          uuid        primary key default gen_random_uuid(),
  repo_id     uuid        not null references sensei.repos(id) on delete cascade,
  session_id  uuid        references sensei.sessions(id) on delete set null,
  type        text        not null check (type in ('decision', 'pattern', 'question')),
  title       text        not null,
  content     text        not null,
  status      text        not null default 'open' check (status in ('open', 'closed')),
  resolution  text,
  closed_at   timestamptz,
  created_at  timestamptz not null default now()
);
create index if not exists memory_items_repo_id_idx    on sensei.memory_items(repo_id, type, status);
-- Additional index for session-scoped queries (present in DDL source files):
create index if not exists memory_items_session_id_idx on sensei.memory_items(session_id)
  where session_id is not null;

-- Grants (same pattern as Phase 1 and 2)
grant all on all tables in schema sensei to anon, authenticated, service_role;
grant all on all sequences in schema sensei to anon, authenticated, service_role;
grant execute on all functions in schema sensei to anon, authenticated, service_role;
