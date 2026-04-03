set search_path to sensei, extensions;

create table if not exists scan_state (
  repo_id      uuid        not null references sensei.repos(id) on delete cascade
, file_path    text        not null
, mtime        bigint      not null
, content_hash text        not null
, indexed_at   timestamptz not null default now()
, primary key (repo_id, file_path)
);

create index if not exists scan_state_repo_id_idx    on scan_state(repo_id);
create index if not exists scan_state_indexed_at_idx on scan_state(repo_id, indexed_at desc);

comment on table scan_state is
'Per-file scan fingerprint used by the indexer to skip unchanged files.
- mtime: file modification time as unix ms
- content_hash: sha256 of file content for change detection
- indexed_at: when this file was last successfully indexed';
