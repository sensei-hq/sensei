-- imports
-- Import graph edges. source_file imports from target_path.
-- names: JSON array of imported identifiers ([] for side-effect-only imports).
-- Used by rank_bfs traversal for context pack assembly.

create table if not exists imports (
  id          text not null primary key
, source_file text not null
, target_path text not null   -- relative path or package name as written in source
, names       text not null default '[]'  -- JSON: ["useState","useEffect"]
, modified_at text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by text not null default 'system'
, unique(source_file, target_path)
);

create index if not exists imports_source_file_idx on imports(source_file);
create index if not exists imports_target_path_idx on imports(target_path);
