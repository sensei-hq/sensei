-- api_requests: one row per Claude Code API call captured via OTLP
create table if not exists sensei.api_requests (
  id                      uuid          primary key default gen_random_uuid()
, repo_id                 uuid          not null references sensei.repos(id) on delete cascade
, task_session_id         uuid          references sensei.task_sessions(id) on delete set null
, benchmark_run_id        uuid          references sensei.benchmark_runs(id) on delete set null
, prompt_id               text          not null
, input_tokens            int           not null
, output_tokens           int           not null
, cache_read_tokens       int           not null default 0
, cache_creation_tokens   int           not null default 0
, cost_usd                numeric(10,6) not null
, duration_ms             int
, model                   text
, recorded_at             timestamptz   not null default now()
);

create index if not exists api_requests_repo_recorded_idx
  on sensei.api_requests(repo_id, recorded_at desc);
create index if not exists api_requests_task_session_idx
  on sensei.api_requests(task_session_id)
  where task_session_id is not null;
create index if not exists api_requests_benchmark_run_idx
  on sensei.api_requests(benchmark_run_id)
  where benchmark_run_id is not null;
create index if not exists api_requests_prompt_id_idx
  on sensei.api_requests(prompt_id);
