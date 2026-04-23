set search_path to staging, extensions;

drop table if exists benchmark_reports cascade;
create table benchmark_reports (
  id             uuid
, folder_id        uuid
, run_name       text
, strategy       text
, score          numeric
, tokens         integer
, elapsed_ms     integer
, payload        jsonb
, promoted       boolean
, created_at     timestamptz not null default now()
, modified_at    timestamptz not null default now()
, modified_by    text        not null default current_user
);

create unique index if not exists benchmark_reports_id_ukey
    on benchmark_reports(id);

comment on table benchmark_reports is
'Intermediate import buffer for bulk-loading rows into sensei.benchmark_reports.
Fields match sensei.benchmark_reports with staging-specific tracking columns.';

comment on column benchmark_reports.id is 'UUID that will become the surrogate primary key in sensei.benchmark_reports.';
comment on column benchmark_reports.folder_id is 'UUID of the folder this benchmark was run against; resolved against sensei.folders on import.';
comment on column benchmark_reports.run_name is 'Human-readable label identifying this benchmark run.';
comment on column benchmark_reports.strategy is 'Context-retrieval strategy evaluated in this benchmark run.';
comment on column benchmark_reports.score is 'Aggregate quality score produced by the benchmark.';
comment on column benchmark_reports.tokens is 'Total token count consumed during the benchmark run.';
comment on column benchmark_reports.elapsed_ms is 'Wall-clock duration of the benchmark run in milliseconds.';
comment on column benchmark_reports.payload is 'Full JSON detail of per-task results and metadata for this run.';
comment on column benchmark_reports.promoted is 'True if this run''s strategy was promoted to the active configuration.';
comment on column benchmark_reports.created_at is 'Timestamp when the staging row was inserted.';
comment on column benchmark_reports.modified_at is 'Source-side modification timestamp; used as freshness gate during import.';
comment on column benchmark_reports.modified_by is 'Source-side modifier identity; passed through to sensei.benchmark_reports on upsert.';