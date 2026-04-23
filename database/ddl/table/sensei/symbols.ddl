set search_path to sensei, extensions;

create type if not exists symbol_kind
    as enum ('function', 'class', 'interface', 'type', 'const', 'enum', 'module', 'package', 'rationale');

create table if not exists symbols (
  id                       uuid        primary key default gen_random_uuid()
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, file_path                text        not null
, name                     text        not null
, kind                     symbol_kind not null
, signature                text
, docstring                text
, line_start               integer
, line_end                 integer
, is_exported              boolean     not null default false
, community_id             integer
, tags                     text[]      not null default '{}'
, modified_at              timestamptz not null default now()
, unique(folder_id, file_path, name, kind)
);

create index if not exists symbols_folder_id_idx
    on symbols(folder_id);

create index if not exists symbols_name_idx
    on symbols(folder_id, name);

create index if not exists symbols_file_path_idx
    on symbols(folder_id, file_path);

create index if not exists symbols_exported_idx
    on symbols(folder_id, is_exported)
 where is_exported;

create index if not exists symbols_community_id_idx
    on symbols(community_id)
 where community_id is not null;

create index if not exists symbols_kind_idx
    on symbols(kind);

comment on table symbols is
'Per-symbol index written by the indexer.
One row per exported/top-level symbol in each file.
- kind: function, class, interface, type, const, enum, module, package, rationale
- rationale: extracted from NOTE/IMPORTANT/HACK/WHY/TODO comments
- community_id: Leiden community cluster (batch-computed, nullable)
- tags: quick-access labels for filtering';

comment on column symbols.id
     is 'Surrogate primary key (UUID).';
comment on column symbols.folder_id
     is 'Foreign key to folders — which repo this symbol belongs to.';
comment on column symbols.file_path
     is 'Repository-relative path of the source file that declares this symbol.';
comment on column symbols.name
     is 'Identifier name of the symbol as it appears in source code.';
comment on column symbols.kind
     is 'Symbol category: function, class, interface, type, const, enum, module, package, rationale.';
comment on column symbols.signature
     is 'Abbreviated type signature for display in context packs.';
comment on column symbols.docstring
     is 'JSDoc/TSDoc/rustdoc comment associated with this symbol, used for semantic search.';
comment on column symbols.line_start
     is 'One-based line number where this symbol declaration begins.';
comment on column symbols.line_end
     is 'One-based line number where this symbol declaration ends.';
comment on column symbols.is_exported
     is 'True when the symbol is exported from its file and visible to importers.';
comment on column symbols.community_id
     is 'Leiden community cluster ID. Batch-computed by workspace intelligence pipeline. Null until computed.';
comment on column symbols.tags
     is 'Array of tag strings for filtering. Vocabulary controlled by sensei.tags table.';
comment on column symbols.modified_at
     is 'Timestamp of the last modification to this row.';
