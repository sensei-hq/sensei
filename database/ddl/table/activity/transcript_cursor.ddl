set search_path to activity, extensions;

-- Resumable ingest cursor for the transcript backfill / forward workers (#73).
-- One row per transcript file so a re-run skips files unchanged since the last
-- ingest (by mtime) — backfill and ongoing forward capture share one
-- incremental path. Keyed per (source, file_path).
create table if not exists activity.transcript_cursor (
    source         text not null
  , file_path      text not null
  , session_id     text
  , last_mtime_ns  bigint not null
  , turns_ingested integer not null default 0
  , updated_at     timestamptz not null default now()
  , primary key (source, file_path)
);

comment on table activity.transcript_cursor is
'Resumable cursor for transcript ingest (#73): per-file mtime so re-runs skip
unchanged transcripts. Shared by the backfill and forward-capture workers.';
