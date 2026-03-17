-- Phase 10: lib indexing status + query log

-- Add indexing status to shared_libs
alter table sensei.shared_libs
  add column if not exists index_status text not null default 'pending'
    check (index_status in ('pending', 'indexing', 'ready', 'error')),
  add column if not exists index_error text;

-- Query log: every simulate/MCP query against a lib
create table if not exists sensei.lib_queries (
  id            uuid primary key default gen_random_uuid(),
  shared_lib_id uuid not null references sensei.shared_libs(id) on delete cascade,
  query_text    text not null,
  source        text not null default 'simulate'
                  check (source in ('simulate', 'mcp', 'api')),
  sections_hit  int not null default 0,
  created_at    timestamptz not null default now()
);

create index if not exists idx_lib_queries_lib on sensei.lib_queries(shared_lib_id, created_at desc);
