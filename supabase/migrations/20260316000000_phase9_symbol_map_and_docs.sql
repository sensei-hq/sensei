-- Phase 9: symbol_map (per-file L0/L1/L2 arrays) and docs (traceability) tables

-- symbol_map: one row per file, stores L0/L1/L2 symbol summaries
create table if not exists sensei.symbol_map (
  id          uuid primary key default gen_random_uuid(),
  repo_id     uuid not null references sensei.repos(id) on delete cascade,
  file_path   text not null,
  l0          text[] not null default '{}',
  l1          text not null default '',
  l2          text not null default '',
  updated_at  timestamptz not null default now(),
  unique(repo_id, file_path)
);

create index if not exists idx_symbol_map_repo on sensei.symbol_map(repo_id);

-- docs: traceability — which doc covers which symbols
create table if not exists sensei.docs (
  id             uuid primary key default gen_random_uuid(),
  repo_id        uuid not null references sensei.repos(id) on delete cascade,
  doc_path       text not null,
  covers         text[] not null default '{}',
  auto_detected  boolean not null default false,
  updated_at     timestamptz not null default now(),
  unique(repo_id, doc_path)
);

create index if not exists idx_docs_repo on sensei.docs(repo_id);
