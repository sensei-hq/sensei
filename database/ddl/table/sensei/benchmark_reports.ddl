set search_path to sensei, extensions;

create table if not exists benchmark_reports (
  id                       uuid        primary key default gen_random_uuid()
, folder_id                uuid        references sensei.folders(id) on delete set null
, run_name                 text        not null
, strategy                 text        not null
, score                    numeric
, tokens                   integer
, elapsed_ms               integer
, payload                  jsonb
, promoted                 boolean     not null default false
, created_at               timestamptz not null default now()
, modified_at              timestamptz not null default now()
);

comment on table benchmark_reports is
'Benchmark reports — results from strategy comparison runs.';

comment on column benchmark_reports.id
     is 'Surrogate primary key (UUID).';
comment on column benchmark_reports.folder_id
     is 'Foreign key to folders — which repo this report covers. Nullable.';
comment on column benchmark_reports.run_name
     is 'Human-readable label for this benchmark report.';
comment on column benchmark_reports.strategy
     is 'Strategy evaluated in this report.';
comment on column benchmark_reports.score
     is 'Benchmark score achieved.';
comment on column benchmark_reports.tokens
     is 'Total token count consumed.';
comment on column benchmark_reports.elapsed_ms
     is 'Wall-clock duration in milliseconds.';
comment on column benchmark_reports.payload
     is 'Detailed results as JSON.';
comment on column benchmark_reports.promoted
     is 'Whether this strategy has been promoted as active.';
comment on column benchmark_reports.created_at
     is 'Timestamp when the row was first created.';
comment on column benchmark_reports.modified_at
     is 'Timestamp of the last modification to this row.';
