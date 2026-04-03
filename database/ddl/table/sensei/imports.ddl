set search_path to sensei, extensions;

create table if not exists imports (
  id          uuid   primary key default gen_random_uuid()
, repo_id     uuid   not null references sensei.repos(id) on delete cascade
, source_file text   not null
, target_path text   not null
, names       text[] not null default '{}'
, unique(repo_id, source_file, target_path)
);

create index if not exists imports_repo_id_idx     on imports(repo_id);
create index if not exists imports_source_file_idx on imports(repo_id, source_file);
create index if not exists imports_target_path_idx on imports(repo_id, target_path);

comment on table imports is
'Import graph edges written by the engine indexer.
- source_file: file that imports from target_path
- target_path: import specifier (may be relative or package name)
- names: list of imported identifiers';
