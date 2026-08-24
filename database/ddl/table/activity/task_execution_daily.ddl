set search_path to activity, sensei, extensions;

-- Daily rollup of `activity.task_executions`, so retention can delete the raw
-- per-task rows without losing the shape of what the daemon did.
--
-- WHY. `task_executions` writes one row per task run and nothing pruned it: it
-- reached 4.8M rows / 1.5 GB in 69 days (~70k rows/day), the second-largest
-- table in the database. 83% of that is per-file index churn —
-- process_git_folder 1.38M, process_file 1.28M, process_folder 405k, and
-- ~220k each for embed_nodes / resolve_libs / build_connections / extract_deps.
--
-- Those rows have no individual value once they succeed. The QUESTIONS they
-- answer — how many ran, how long did they take, what failed — are all
-- aggregate, and survive perfectly in one row per (day, kind, status).
--
-- Failures are exempt from that reasoning and are retained raw for a longer
-- window: a failed task's `error_message`, `path` and `retry_number` are the
-- whole point of having a log, and there are few of them (32,664 of 4.8M).
create table if not exists task_execution_daily (
  day             date                not null
, task_kind       task_execution_kind not null
, status          text                not null
, runs            bigint              not null
, failures        bigint              not null default 0
, items_processed bigint
  -- Duration percentiles, not just an average: task duration is heavily skewed
  -- (a cold embedded model, a huge repo) and a mean hides exactly the tail that
  -- matters when the queue feels slow.
, p50_ms          integer
, p95_ms          integer
, max_ms          integer
, rolled_up_at    timestamptz         not null default now()
, primary key (day, task_kind, status)
);

create index if not exists task_execution_daily_day_idx
    on task_execution_daily(day desc);

comment on table task_execution_daily is
'Daily aggregate of activity.task_executions, written by the retention pass
before it deletes the raw rows. One row per (day, kind, status).

The raw table answers "what did task #4711 do"; this one answers "how much ran,
how slow, how often did it fail" — the questions that outlive an individual run.

Idempotent: the retention pass upserts on (day, task_kind, status), so
re-running over a day already rolled up produces the same row rather than
double-counting.';

comment on column task_execution_daily.runs
     is 'Rows collapsed into this bucket.';
comment on column task_execution_daily.failures
     is 'Of those runs, how many ended failed. Redundant with status but kept so a
single row answers "how bad was this day" without a second scan.';
comment on column task_execution_daily.items_processed
     is 'Sum of the handlers own return values (files queued, rows written …) —
NULL when no row in the bucket reported one.';
comment on column task_execution_daily.p50_ms
     is 'Median wall-clock duration. NULL when no row in the bucket completed.';
