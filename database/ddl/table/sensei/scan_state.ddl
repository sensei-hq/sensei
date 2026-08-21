set search_path to sensei, extensions;

create table if not exists scan_state (
  folder_id                uuid        not null references sensei.folders(id) on delete cascade
, file_path                text        not null
, mtime                    bigint      not null
, content_hash             text        not null
, skip_reason              sensei.scan_skip_reason
, indexed_at               timestamptz not null default now()
, modified_at              timestamptz not null default now()
, primary key (folder_id, file_path)
);

create index if not exists scan_state_folder_id_idx
    on scan_state(folder_id);

create index if not exists scan_state_indexed_at_idx
    on scan_state(folder_id, indexed_at desc);

comment on table scan_state is
'Per-file scan fingerprint used by the indexer to skip unchanged files.
A row means the file was EXAMINED at this fingerprint — not necessarily indexed.
- mtime: file modification time as unix ms
- content_hash: sha256 of file content for change detection
- skip_reason: null = indexed; non-null = examined and deliberately not indexed
- indexed_at: when this file was last examined';

comment on column scan_state.folder_id
     is 'Foreign key to folders — which folder this file belongs to.';
comment on column scan_state.file_path
     is 'Folder-relative path of the source file being tracked.';
comment on column scan_state.mtime
     is 'File modification time as Unix epoch milliseconds, used for quick change detection.';
comment on column scan_state.content_hash
     is 'SHA-256 hash of the file content used to confirm whether it has changed since last index.';
comment on column scan_state.skip_reason
     is 'Null when the file was indexed. Non-null records why it was examined but deliberately not indexed (unsupported format, binary content, invalid UTF-8, parse error, excluded by config). Recording the fingerprint alongside the reason is what stops a skipped file from being re-enqueued on every reconcile; because the skip is keyed to the fingerprint, fixing the file re-triggers indexing automatically.';
comment on column scan_state.indexed_at
     is 'Timestamp when this file was last examined (indexed, or skipped with a reason).';
comment on column scan_state.modified_at
     is 'Timestamp of the last modification to this row.';
