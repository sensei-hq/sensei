-- symbols
-- Per-symbol index written by the engine indexer.
-- One row per exported/top-level symbol per file.
-- kind: function | class | interface | type | const | enum | module
--
-- resolution levels (L0-L3) stored as columns:
--   L0 = signature (always populated)
--   L1 = description (populated by local inference if available, else null)
--   L2 = logic_flow (populated on demand / deep index)
--   L3 = full source (not stored — read from file on demand)
--
-- degree: precomputed graph degree (in + out edges); updated after each index run.
-- community_id: Leiden community assignment; null until community detection runs.

create table if not exists symbols (
  id            text    not null primary key
, file_path     text    not null
, name          text    not null
, kind          text    not null
                        check (kind in ('function','class','interface','type',
                                        'const','enum','module'))
, signature     text                      -- L0: abbreviated type signature
, description   text                      -- L1: one-sentence summary (local inference)
, logic_flow    text                      -- L2: logic flow description (deep index)
, docstring     text                      -- extracted JSDoc/TSDoc comment
, line_start    integer
, line_end      integer
, is_exported   integer not null default 0
, degree        integer not null default 0  -- precomputed: in_edges + out_edges
, community_id  text                        -- Leiden community; null until detection runs
, modified_at   text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by   text    not null default 'system'
, unique(file_path, name, kind)
);

create index if not exists symbols_file_path_idx  on symbols(file_path);
create index if not exists symbols_name_idx       on symbols(name);
create index if not exists symbols_exported_idx   on symbols(is_exported) where is_exported = 1;
create index if not exists symbols_degree_idx     on symbols(degree desc);
create index if not exists symbols_community_idx  on symbols(community_id) where community_id is not null;

-- Full-text search over symbol names, signatures, docstrings, and descriptions
create virtual table if not exists symbols_fts using fts5(
  id unindexed
, name
, signature
, docstring
, description
, content='symbols'
, content_rowid='rowid'
);
