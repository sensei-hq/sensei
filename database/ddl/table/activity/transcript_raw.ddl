set search_path to activity, extensions;

-- Raw transcript backup: the file's bytes as ingested, gzipped, so a rotated
-- file stays recoverable for the retention window. Written during ingest
-- (`ingest_one`); pruned off `last_seen_at` by the activity prune pass.
-- Grain = one ingested unit. Identity = (source, file_path).
create table if not exists activity.transcript_raw (
    id             uuid primary key default gen_random_uuid()
  , source         text not null
  , file_path      text not null
  , session_id     text not null
  , sha256         text not null
  , raw_bytes      bigint not null
  , content_gzip   bytea not null
  , created_at     timestamptz not null default now()
  , last_seen_at   timestamptz not null default now()
  , unique (source, file_path)
);

create index if not exists transcript_raw_last_seen_idx
  on activity.transcript_raw (last_seen_at);
create index if not exists transcript_raw_session_idx
  on activity.transcript_raw (session_id);

comment on table activity.transcript_raw is
'Raw transcript bytes as ingested, gzipped — recovery for rotated files. Written during ingest with a sha-compare skip; pruned off last_seen_at. Grain = one ingested unit. Identity = (source, file_path).';
comment on column transcript_raw.sha256
     is 'Hex sha256 of the raw bytes: the skip-rewrite key — an upsert with the same sha advances last_seen_at without touching the blob.';
comment on column transcript_raw.content_gzip
     is 'Gzip of the raw unit bytes; gunzip to recover the file. Units too large to hold whole (load_content cap) have no row.';
comment on column transcript_raw.last_seen_at
     is 'The retention clock: advanced on every ingest of the present file (including fresh skips), never once the file is gone.';
