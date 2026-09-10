set search_path to sensei, extensions;

-- THE FILE ENTITY. Renamed from `scan_state` (R13): the old name read as
-- transient bookkeeping, so "do we have a file entity?" stayed an open design
-- question for several rounds while the answer sat in this table. It is the
-- level between folders and symbols — `nodes.file_id` references it — and the
-- rename is what makes that obvious.
--
-- Two keys, serving two callers, neither redundant:
--   id                      `nodes.file_id` — 16 bytes, stable across a rename
--   (folder_id, file_path)  the walk, which only ever has a path
--
-- Keeping the composite as UNIQUE preserves the walk's once-per-file lookup at
-- O(log n) with no additional index.
create table if not exists files (
  id                       uuid        primary key default gen_random_uuid()
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, file_path                text        not null
, mtime                    bigint      not null
, content_hash             text        not null
, skip_reason              sensei.scan_skip_reason
  -- The parser's VERBATIM message with line and column (R10.9, A9). `skip_reason`
  -- carries the CODE; this carries the text. "parse_error" tells an agent a file
  -- is broken; "x.rs:142: expected `}`" tells it what to do. It lives here rather
  -- than on a node's props because splitting the code and the detail across two
  -- tables recreates the two-copies-of-one-fact problem R10.9 exists to avoid.
, skip_detail              text
  -- The file LIFECYCLE's missing bit (03 S4). The design names four states —
  -- discovered / parsed / unparseable / skipped — and `skip_reason` already
  -- separates the last two from the rest. What it cannot separate is
  -- `discovered` from `parsed`: both are skip_reason NULL, so a parse task
  -- that never ran looks exactly like one that succeeded and found nothing
  -- (a real state, R10.3). This column is that one bit and nothing more.
  --   parsed_at NULL, skip_reason NULL -> discovered  (stalled, if it lingers)
  --   parsed_at set,  skip_reason NULL -> parsed
  --   parsed_at set,  skip_reason set  -> unparseable | skipped, per the reason
  -- A four-value enum column was the other option and was rejected: it would
  -- restate what skip_reason already says, giving two writes of one fact that
  -- can disagree. See indexer-v2.md R13.
, parsed_at                timestamptz
, indexed_at               timestamptz not null default now()
, modified_at              timestamptz not null default now()
, unique (folder_id, file_path)
);

create index if not exists files_folder_id_idx
    on files(folder_id);

create index if not exists files_indexed_at_idx
    on files(folder_id, indexed_at desc);

comment on table files is
'THE FILE ENTITY — one row per file examined by the scan. Renamed from
scan_state (R13), which read as transient bookkeeping and hid the fact that
this IS the level between folders and symbols; nodes.file_id references it.
A row means the file was EXAMINED at this fingerprint — not necessarily indexed.
- id: surrogate key, referenced by nodes.file_id. Stable across a path change.
- (folder_id, file_path): UNIQUE — the walk only ever has a path.
- mtime: file modification time as unix ms
- content_hash: sha256 of file content for change detection
- skip_reason: null = indexed; non-null = examined and deliberately not indexed
- skip_detail: the parser''s verbatim message with location, when there is one
- parsed_at: null = discovered (no parse outcome yet); set = a parse ran
- indexed_at: when this file was last examined';

comment on column files.id
     is 'Surrogate primary key. Referenced by nodes.file_id — 16 bytes instead of a repeated path, and unchanged when the file moves.';
comment on column files.folder_id
     is 'Foreign key to folders — which folder this file belongs to.';
comment on column files.file_path
     is 'Folder-relative path of the source file being tracked. UNIQUE with folder_id: the walk resolves a file by path, having no id in hand.';
comment on column files.mtime
     is 'File modification time as Unix epoch milliseconds, used for quick change detection.';
comment on column files.content_hash
     is 'SHA-256 hash of the file content used to confirm whether it has changed since last index.';
comment on column files.skip_reason
     is 'Null when the file was indexed. Non-null records why it was examined but deliberately not indexed (unsupported format, binary content, invalid UTF-8, parse error, excluded by config). Recording the fingerprint alongside the reason is what stops a skipped file from being re-enqueued on every reconcile; because the skip is keyed to the fingerprint, fixing the file re-triggers indexing automatically.';
comment on column files.skip_detail
     is 'The parser''s VERBATIM error with file, line and column, when the grammar produced one (R10.9). skip_reason is the code; this is the text. An agent handed "x.rs:142: expected `}`" can fix the file; one handed "parse_error" can only shrug. Null when there is no detail to record.';
comment on column files.parsed_at
     is 'When a parse outcome was last recorded for this file. NULL means the walk created the row and no parse has run yet — the "discovered" state. Together with skip_reason this yields the full file lifecycle: (null, null) discovered; (set, null) parsed; (set, non-null) unparseable or skipped per the reason. It exists because those first two are otherwise indistinguishable, which would make a stalled parse task look identical to a file that parsed and legitimately declared nothing.';
comment on column files.indexed_at
     is 'Timestamp when this file was last examined (indexed, or skipped with a reason).';
comment on column files.modified_at
     is 'Timestamp of the last modification to this row.';
