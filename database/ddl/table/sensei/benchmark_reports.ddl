set search_path to sensei, extensions;

create table if not exists benchmark_reports (
  id          uuid primary key default gen_random_uuid()
, repo_id     uuid references repos(id)
, run_name    text not null
, strategy    text not null
, score       numeric
, tokens      integer
, elapsed_ms  integer
, payload     jsonb
, promoted    boolean not null default false
, created_at  timestamptz not null default now()
);
