set search_path to sensei, extensions;

create table if not exists benchmark_runs (
  id                       uuid          primary key default gen_random_uuid()
, folder_id                uuid          not null references sensei.folders(id) on delete cascade
, task_description         text          not null
, branch                   text          not null
, sensei_enabled           boolean       not null
, started_at               timestamptz   not null default now()
, ended_at                 timestamptz
, total_cost_usd           numeric(10,6)
, total_input_tokens       integer
, total_output_tokens      integer
, total_cache_read_tokens  integer
, total_cache_creation_tokens integer
, worktree_path            text
, modified_at              timestamptz   not null default now()
);

create index if not exists benchmark_runs_folder_idx
    on benchmark_runs(folder_id, task_description, branch);

comment on table benchmark_runs is
'One row per benchmark execution (paired: same task + branch, sensei_enabled true/false).';

comment on column benchmark_runs.id
     is 'Surrogate primary key (UUID).';
comment on column benchmark_runs.folder_id
     is 'Foreign key to folders — which repo this benchmark ran against.';
comment on column benchmark_runs.task_description
     is 'Coding task given to the AI during this run.';
comment on column benchmark_runs.branch
     is 'Git branch the benchmark was executed against.';
comment on column benchmark_runs.sensei_enabled
     is 'Whether sensei MCP tools were available (false = baseline, true = assisted).';
comment on column benchmark_runs.started_at
     is 'Timestamp when the run began.';
comment on column benchmark_runs.ended_at
     is 'Timestamp when the run finished. Null if in progress.';
comment on column benchmark_runs.total_cost_usd
     is 'Total API cost in USD aggregated from api_requests.';
comment on column benchmark_runs.total_input_tokens
     is 'Total input tokens across all API calls.';
comment on column benchmark_runs.total_output_tokens
     is 'Total output tokens across all API calls.';
comment on column benchmark_runs.worktree_path
     is 'Path of the temporary git worktree used. Deleted after completion.';
comment on column benchmark_runs.modified_at
     is 'Timestamp of the last modification to this row.';
