set search_path to sensei, extensions;

create table if not exists imports (
  id          uuid   primary key default gen_random_uuid()
, repo_id     uuid   not null references sensei.repos(id) on delete cascade
, source_file text   not null
, target_path text   not null
, names       text[] not null default '{}'
, modified_at timestamptz not null default now()
, modified_by text        not null default current_user
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

comment on column imports.id is 'Surrogate primary key (UUID).';
comment on column imports.repo_id is 'Foreign key to sensei.repos — scopes this row to a specific repository.';
comment on column imports.source_file is 'Repository-relative path of the file that contains the import statement.';
comment on column imports.target_path is 'Import specifier as written in source: may be a relative path or a package name.';
comment on column imports.names is 'List of identifiers imported from target_path (empty array for side-effect-only imports).';
comment on column imports.modified_at is 'Timestamp of the last modification to this row.';
comment on column imports.modified_by is 'Identity (user, role, or service) that last modified this row.';
