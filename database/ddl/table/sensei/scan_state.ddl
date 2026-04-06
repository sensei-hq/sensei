set search_path to sensei, extensions;

create table if not exists scan_state (
  repo_id      uuid        not null references sensei.repos(id) on delete cascade
, file_path    text        not null
, mtime        bigint      not null
, content_hash text        not null
, indexed_at   timestamptz not null default now()
, modified_at  timestamptz not null default now()
, modified_by  text        not null default current_user
, primary key (repo_id, file_path)
);

create index if not exists scan_state_repo_id_idx    on scan_state(repo_id);
create index if not exists scan_state_indexed_at_idx on scan_state(repo_id, indexed_at desc);

comment on table scan_state is
'Per-file scan fingerprint used by the indexer to skip unchanged files.
- mtime: file modification time as unix ms
- content_hash: sha256 of file content for change detection
- indexed_at: when this file was last successfully indexed';

comment on column scan_state.repo_id is 'Foreign key to sensei.repos — scopes this row to a specific repository.';
comment on column scan_state.file_path is 'Repo-relative path of the source file being tracked.';
comment on column scan_state.mtime is 'File modification time as Unix epoch milliseconds, used for quick change detection.';
comment on column scan_state.content_hash is 'SHA-256 hash of the file''s content used to confirm whether it has changed since last index.';
comment on column scan_state.indexed_at is 'Timestamp when this file was last successfully indexed.';
comment on column scan_state.modified_at is 'Timestamp of the last modification to this row.';
comment on column scan_state.modified_by is 'Identity (user, role, or service) that last modified this row.';
