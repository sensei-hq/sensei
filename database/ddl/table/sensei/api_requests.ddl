set search_path to sensei, extensions;

create table if not exists api_requests (
  id                       uuid          primary key default gen_random_uuid()
, folder_id                uuid          not null references sensei.folders(id) on delete cascade
, task_session_id          uuid          references sensei.task_sessions(id) on delete set null
, benchmark_run_id         uuid          references sensei.benchmark_runs(id) on delete set null
, prompt_id                text          not null
, input_tokens             integer       not null
, output_tokens            integer       not null
, cache_read_tokens        integer       not null default 0
, cache_creation_tokens    integer       not null default 0
, cost_usd                 numeric(10,6) not null
, duration_ms              integer
, model                    text
, recorded_at              timestamptz   not null default now()
, modified_at              timestamptz   not null default now()
);

create index if not exists api_requests_folder_recorded_idx
    on api_requests(folder_id, recorded_at desc);

create index if not exists api_requests_task_session_idx
    on api_requests(task_session_id)
 where task_session_id is not null;

create index if not exists api_requests_benchmark_run_idx
    on api_requests(benchmark_run_id)
 where benchmark_run_id is not null;

comment on table api_requests is
'One row per API call captured via telemetry.
- prompt_id: prompt/task identifier from OTel attribute
- cost_usd: total cost for this API call
- cache_*_tokens: cache read vs creation token counts for cost breakdown';

comment on column api_requests.id
     is 'Surrogate primary key (UUID).';
comment on column api_requests.folder_id
     is 'Foreign key to folders — which repo this API call was made in.';
comment on column api_requests.task_session_id
     is 'Optional FK to task_sessions — the task active when this call was made.';
comment on column api_requests.benchmark_run_id
     is 'Optional FK to benchmark_runs — the benchmark run during which this call was made.';
comment on column api_requests.prompt_id
     is 'Prompt or task identifier extracted from telemetry.';
comment on column api_requests.input_tokens
     is 'Number of input tokens consumed.';
comment on column api_requests.output_tokens
     is 'Number of output tokens produced.';
comment on column api_requests.cache_read_tokens
     is 'Number of prompt-cache read tokens used.';
comment on column api_requests.cache_creation_tokens
     is 'Number of prompt-cache creation tokens used.';
comment on column api_requests.cost_usd
     is 'Total cost of this API call in USD.';
comment on column api_requests.duration_ms
     is 'Wall-clock duration in milliseconds.';
comment on column api_requests.model
     is 'Model identifier used for this call.';
comment on column api_requests.recorded_at
     is 'Timestamp when this call was recorded.';
comment on column api_requests.modified_at
     is 'Timestamp of the last modification to this row.';
