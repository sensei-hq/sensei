set search_path to sensei, extensions;

create table if not exists symbol_map (
  repo_id   uuid not null references sensei.repos(id) on delete cascade
, file_path text  not null
, l0        text[] not null default '{}'
, l1        text   not null default ''
, l2          text        not null default ''
, modified_at  timestamptz not null default now()
, modified_by  text        not null default current_user
, primary key (repo_id, file_path)
);

create index if not exists symbol_map_repo_id_idx on symbol_map(repo_id);

comment on table symbol_map is
'Per-file symbol summary written by the tools/reindex indexer.
- l0: exported symbol signatures (array)
- l1: L0 with JSDoc descriptions (newline-separated)
- l2: full body text (newline-separated, reserved for future use)';

comment on column symbol_map.repo_id is 'Foreign key to sensei.repos — scopes this row to a specific repository.';
comment on column symbol_map.file_path is 'Repo-relative path of the source file this symbol summary covers.';
comment on column symbol_map.l0 is 'Array of exported symbol signatures extracted from the file (L0 detail level).';
comment on column symbol_map.l1 is 'L0 signatures augmented with JSDoc descriptions, newline-separated (L1 detail level).';
comment on column symbol_map.l2 is 'Full body text of the file, newline-separated; reserved for future deep-context use (L2 detail level).';
comment on column symbol_map.modified_at is 'Timestamp of the last modification to this row.';
comment on column symbol_map.modified_by is 'Identity (user, role, or service) that last modified this row.';
