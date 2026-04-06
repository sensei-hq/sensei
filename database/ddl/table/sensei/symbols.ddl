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
, modified_at timestamptz not null default now()
, modified_by text        not null default current_user
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

comment on column symbols.id is 'Surrogate primary key (UUID).';
comment on column symbols.repo_id is 'Foreign key to sensei.repos — scopes this row to a specific repository.';
comment on column symbols.file_path is 'Repository-relative path of the source file that declares this symbol.';
comment on column symbols.name is 'Identifier name of the symbol as it appears in source code.';
comment on column symbols.kind is 'Symbol category: function, class, interface, type, const, or enum.';
comment on column symbols.signature is 'Abbreviated type signature for display in context packs.';
comment on column symbols.docstring is 'JSDoc/TSDoc comment associated with this symbol, used for semantic search.';
comment on column symbols.line_start is 'One-based line number where this symbol''s declaration begins.';
comment on column symbols.line_end is 'One-based line number where this symbol''s declaration ends.';
comment on column symbols.is_exported is 'True when the symbol is exported from its file and visible to importers.';
comment on column symbols.modified_at is 'Timestamp of the last modification to this row.';
comment on column symbols.modified_by is 'Identity (user, role, or service) that last modified this row.';
