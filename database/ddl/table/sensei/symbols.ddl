set search_path to sensei, extensions;

create table if not exists symbols (
  id          uuid        primary key default gen_random_uuid()
, repo_id     uuid        not null references sensei.repos(id) on delete cascade
, file_path   text        not null
, name        text        not null
, kind        text        not null
, signature   text
, docstring   text
, line_start  integer
, line_end    integer
, is_exported boolean     not null default false
, updated_at  timestamptz not null default now()
, unique(repo_id, file_path, name, kind)
);

create index if not exists symbols_repo_id_idx       on symbols(repo_id);
create index if not exists symbols_name_idx          on symbols(repo_id, name);
create index if not exists symbols_file_path_idx     on symbols(repo_id, file_path);
create index if not exists symbols_exported_idx      on symbols(repo_id, is_exported) where is_exported;

comment on table symbols is
'Per-symbol index written by the engine indexer (packages/engine/src/indexer.ts).
One row per exported/top-level symbol in each file.
- kind: function | class | interface | type | const | enum
- is_exported: true when symbol is exported from the file
- signature: abbreviated type signature for context display
- docstring: JSDoc/TSDoc description for semantic search';
