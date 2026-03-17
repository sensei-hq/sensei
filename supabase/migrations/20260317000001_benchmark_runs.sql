-- benchmark_runs: one row per benchmark execution (with/without sensei)
create table if not exists sensei.benchmark_runs (
  id                          uuid          primary key default gen_random_uuid()
, repo_id                     uuid          not null references sensei.repos(id) on delete cascade
, task_description            text          not null
, branch                      text          not null
, sensei_enabled              boolean       not null
, started_at                  timestamptz   not null default now()
, ended_at                    timestamptz
, total_cost_usd              numeric(10,6)
, total_input_tokens          int
, total_output_tokens         int
, total_cache_read_tokens     int
, total_cache_creation_tokens int
, worktree_path               text
);

create index if not exists benchmark_runs_repo_idx
  on sensei.benchmark_runs(repo_id, task_description, branch);
