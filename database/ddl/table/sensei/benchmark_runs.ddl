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
, modified_at                timestamptz   not null default now()
, modified_by                text          not null default current_user
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

comment on column benchmark_runs.id is 'Surrogate primary key (UUID).';
comment on column benchmark_runs.repo_id is 'Foreign key to sensei.repos — scopes this row to a specific repository.';
comment on column benchmark_runs.task_description is 'Natural-language description of the coding task given to the AI during this run.';
comment on column benchmark_runs.branch is 'Git branch the benchmark was executed against.';
comment on column benchmark_runs.sensei_enabled is 'Whether sensei MCP tools were available during this run (false = baseline, true = sensei-assisted).';
comment on column benchmark_runs.started_at is 'Timestamp when the benchmark run began.';
comment on column benchmark_runs.ended_at is 'Timestamp when the benchmark run finished; null if still in progress.';
comment on column benchmark_runs.total_cost_usd is 'Total API cost in USD aggregated from all api_requests rows for this run.';
comment on column benchmark_runs.total_input_tokens is 'Total input tokens consumed across all API calls in this run.';
comment on column benchmark_runs.total_output_tokens is 'Total output tokens produced across all API calls in this run.';
comment on column benchmark_runs.total_cache_read_tokens is 'Total prompt-cache read tokens across all API calls in this run.';
comment on column benchmark_runs.total_cache_creation_tokens is 'Total prompt-cache creation tokens across all API calls in this run.';
comment on column benchmark_runs.worktree_path is 'Filesystem path of the temporary git worktree used during this run; deleted after completion.';
comment on column benchmark_runs.modified_at is 'Timestamp of the last modification to this row.';
comment on column benchmark_runs.modified_by is 'Identity (user, role, or service) that last modified this row.';
