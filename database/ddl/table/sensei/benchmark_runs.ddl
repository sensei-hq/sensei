set search_path to sensei, extensions;

create table if not exists benchmark_runs (
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
  on benchmark_runs(repo_id, task_description, branch);

comment on table benchmark_runs is
'One row per benchmark execution (paired: same task + branch, sensei_enabled true/false).
- sensei_enabled: false = no sensei MCP tools available; true = normal sensei config
- started_at / ended_at: time window used to correlate api_requests rows
- total_* columns: aggregated from api_requests rows after run completes
- worktree_path: path of the temporary benchmark worktree (deleted after run)
Pairs are matched by (repo_id, task_description, branch) with different sensei_enabled values.';
