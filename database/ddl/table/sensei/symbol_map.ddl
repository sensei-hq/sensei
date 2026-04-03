set search_path to sensei, extensions;

create table if not exists symbol_map (
  repo_id   uuid not null references sensei.repos(id) on delete cascade
, file_path text  not null
, l0        text[] not null default '{}'
, l1        text   not null default ''
, l2        text   not null default ''
, primary key (repo_id, file_path)
);

create index if not exists symbol_map_repo_id_idx on symbol_map(repo_id);

comment on table symbol_map is
'Per-file symbol summary written by the tools/reindex indexer.
- l0: exported symbol signatures (array)
- l1: L0 with JSDoc descriptions (newline-separated)
- l2: full body text (newline-separated, reserved for future use)';
