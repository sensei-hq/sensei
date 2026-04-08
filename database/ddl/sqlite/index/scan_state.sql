-- scan_state
-- Per-file fingerprint used by the incremental indexer to skip unchanged files.
-- mtime: Unix epoch milliseconds (fast check).
-- content_hash: SHA-256 hex (authoritative check when mtime matches).

create table if not exists scan_state (
  file_path    text    not null primary key
, mtime        integer not null
, content_hash text    not null
, indexed_at   text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_at  text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by  text    not null default 'system'
);

create index if not exists scan_state_indexed_at_idx on scan_state(indexed_at desc);
