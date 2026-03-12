set search_path to sensei, extensions;

create table if not exists symbols (
  id          uuid primary key default gen_random_uuid()
, repo_id     uuid not null references sensei.repos(id) on delete cascade
, file_path   text not null
, l0          text[]
, l1          text
, l2          text
, modified_at timestamptz not null default now()
, unique(repo_id, file_path)
);

create index if not exists symbols_repo_id_idx on symbols(repo_id);

comment on table symbols is
'Per-file symbol index. Replaces .sensei/symbol-map.json.
- l0: exported names (for fast listing)
- l1: signatures (for context loading)
- l2: with docstrings (for deep context)
- l3 is NOT stored — served from local filesystem on demand';
