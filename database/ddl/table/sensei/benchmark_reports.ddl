set search_path to sensei, extensions;

create table if not exists benchmark_reports (
  id          uuid primary key default gen_random_uuid()
, repo_id     uuid references repos(id) on delete set null
, run_name    text not null
, strategy    text not null
, score       numeric
, tokens      integer
, elapsed_ms  integer
, payload     jsonb
, promoted    boolean not null default false
, created_at  timestamptz not null default now()
, modified_at timestamptz not null default now()
, modified_by text        not null default current_user
);

comment on column benchmark_reports.id is 'Surrogate primary key (UUID).';
comment on column benchmark_reports.repo_id is 'Foreign key to sensei.repos — scopes this report to a specific repository (nullable if repo is deleted).';
comment on column benchmark_reports.run_name is 'Human-readable label for this benchmark report.';
comment on column benchmark_reports.strategy is 'Context retrieval or configuration strategy evaluated in this report.';
comment on column benchmark_reports.score is 'Benchmark score achieved by this strategy.';
comment on column benchmark_reports.tokens is 'Total token count consumed during this benchmark run.';
comment on column benchmark_reports.elapsed_ms is 'Wall-clock duration of the benchmark run in milliseconds.';
comment on column benchmark_reports.payload is 'Arbitrary JSON payload with detailed benchmark results or metadata.';
comment on column benchmark_reports.promoted is 'Whether this report''s strategy has been promoted as the active configuration.';
comment on column benchmark_reports.created_at is 'Timestamp when the row was first created.';
comment on column benchmark_reports.modified_at is 'Timestamp of the last modification to this row.';
comment on column benchmark_reports.modified_by is 'Identity (user, role, or service) that last modified this row.';
