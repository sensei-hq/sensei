-- Phase 1 Foundation: repos, symbols, call_edges, imports, scan_state

create extension if not exists "uuid-ossp";

create schema if not exists sensei;

-- Repos: one row per indexed repository
create table if not exists sensei.repos (
  id               uuid primary key default gen_random_uuid(),
  name             text not null,
  local_path       text not null unique,
  remote_url       text,
  stack            text[] not null default '{}',
  entry_points     jsonb not null default '[]',
  last_indexed_at  timestamptz,
  created_at       timestamptz not null default now()
);

-- Symbols: exported functions, classes, types, interfaces from parsed files
create table if not exists sensei.symbols (
  id          uuid primary key default gen_random_uuid(),
  repo_id     uuid not null references sensei.repos(id) on delete cascade,
  file_path   text not null,
  name        text not null,
  kind        text not null check(kind in ('function','class','type','interface','enum','const','method','component','hook','unknown')),
  signature   text,
  docstring   text,
  line_start  integer not null,
  line_end    integer not null,
  is_exported boolean not null default false,
  updated_at  timestamptz not null default now(),
  unique(repo_id, file_path, name, kind)
);

create index if not exists idx_symbols_repo_file on sensei.symbols(repo_id, file_path);
create index if not exists idx_symbols_name on sensei.symbols(name);

-- Call edges: which symbol calls which other symbol
create table if not exists sensei.call_edges (
  id            uuid primary key default gen_random_uuid(),
  repo_id       uuid not null references sensei.repos(id) on delete cascade,
  caller_id     uuid not null references sensei.symbols(id) on delete cascade,
  callee_name   text not null,
  callee_file   text,
  created_at    timestamptz not null default now()
);

create index if not exists idx_call_edges_caller on sensei.call_edges(caller_id);
create index if not exists idx_call_edges_repo on sensei.call_edges(repo_id);

-- Imports: module-level import relationships
create table if not exists sensei.imports (
  id           uuid primary key default gen_random_uuid(),
  repo_id      uuid not null references sensei.repos(id) on delete cascade,
  source_file  text not null,
  target_path  text not null,   -- resolved or raw specifier
  names        text[] not null, -- imported names (empty = namespace import)
  created_at   timestamptz not null default now(),
  unique(repo_id, source_file, target_path)  -- required for upsert ON CONFLICT
);

create index if not exists idx_imports_source on sensei.imports(repo_id, source_file);

-- Scan state: tracks file fingerprints for incremental indexing
create table if not exists sensei.scan_state (
  repo_id     uuid not null references sensei.repos(id) on delete cascade,
  file_path   text not null,
  mtime       bigint not null,   -- milliseconds epoch
  content_hash text not null,   -- sha256 hex
  indexed_at  timestamptz not null default now(),
  primary key (repo_id, file_path)
);

-- Events: telemetry from the collector daemon (collector already writes here)
create table if not exists sensei.events (
  id           bigserial primary key,
  user_uuid    text not null,
  session_id   text,
  repo_id      uuid references sensei.repos(id),
  phase        text not null check(phase in ('pre','post')),
  tool         text not null,
  project_path text not null default '',
  input        jsonb,
  ts           timestamptz not null,
  seq          integer,
  duration_ms  integer,
  success      boolean,
  error        text
);

create index if not exists idx_events_session on sensei.events(session_id);
create index if not exists idx_events_ts on sensei.events(ts desc);
create index if not exists idx_events_repo on sensei.events(repo_id);

-- NOTE: The `embeddings` table (pgvector) is intentionally deferred to Phase 2.
-- Phase 1 `search` uses ilike substring matching. Phase 2 adds semantic search via pgvector.

-- Grant PostgREST roles access
grant usage on schema sensei to anon, authenticated, service_role;
grant all on all tables in schema sensei to anon, authenticated, service_role;
grant all on all sequences in schema sensei to anon, authenticated, service_role;
alter default privileges in schema sensei grant all on tables to anon, authenticated, service_role;
alter default privileges in schema sensei grant all on sequences to anon, authenticated, service_role;
