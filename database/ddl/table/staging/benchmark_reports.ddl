set search_path to staging;

create table if not exists benchmark_reports (
  id          uuid
, repo_id     uuid
, run_name    text
, strategy    text
, score       numeric
, tokens      integer
, elapsed_ms  integer
, payload     jsonb
, promoted    boolean
, created_at  timestamptz default now()
);

create unique index if not exists benchmark_reports_id_ukey on benchmark_reports(id);
