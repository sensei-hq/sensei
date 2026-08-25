set search_path to activity;

-- How far capture ingestion has got, per unit.
--
-- One of TWO watermark tables. The other is `sensei.metric_watermarks`
-- (repository × metric_group → sealed_through date). They answer the same
-- question — "how far have I got" — and deliberately remain SEPARATE tables
-- rather than one generic `pipeline_watermarks`:
--
--   * metric_watermarks keys on a real `repository_id uuid` with ON DELETE
--     CASCADE, verified by test. A generic `scope_key text` cannot reference
--     sensei.repositories, so collapsing them would trade a tested
--     referential-integrity guarantee for a shared name and leave orphan
--     watermarks behind every deleted repository.
--   * their cursors are different types with different meanings — a sealed-through
--     DATE plus a commit sha here, an mtime in nanoseconds plus an ingested count
--     there. A shared jsonb cursor would turn two column-typed values into two
--     runtime parse sites.
--   * they sit either side of the local/shared boundary: activity.* is raw
--     capture that never leaves the machine, sensei.* is partly shareable.
--
-- So the unification is in the VOCABULARY, not the storage: both are named
-- `*_watermarks`, both carry `updated_at`, and both mean "everything before this
-- point is done". Renamed from `transcript_cursor` when the pipeline itself
-- became IngestCaptures — the table tracks captures, not just transcripts.

create table if not exists activity.capture_watermarks (
    source         text not null
  , file_path      text not null
  , session_id     text
  , last_mtime_ns  bigint not null
  , turns_ingested integer not null default 0
  , updated_at     timestamptz not null default now()
  , primary key (source, file_path)
);

comment on table activity.capture_watermarks is
'Resumable cursor for transcript ingest (#73): per-file mtime so re-runs skip
unchanged transcripts. Shared by the backfill and forward-capture workers.';
