set search_path to activity, sensei;

create table if not exists task_executions (
  id                       uuid        primary key default gen_random_uuid()
, task_id                  bigint      not null
, parent_task_id           bigint
, task_kind                task_execution_kind not null
, folder_path              text        not null default ''
, path                     text        not null default ''
, status                   text        not null default 'running'
, items_processed          integer
, duration_ms              integer
, retry_number             integer     not null default 0
, error_message            text
, started_at               timestamptz not null default now()
, completed_at             timestamptz
, retried_from_execution_id uuid
, phases_ms              jsonb
);

create index if not exists task_executions_task_kind_idx
    on task_executions(task_kind);

create index if not exists task_executions_folder_path_idx
    on task_executions(folder_path);

create index if not exists task_executions_parent_task_id_idx
    on task_executions(parent_task_id)
 where parent_task_id is not null;

create index if not exists task_executions_started_at_idx
    on task_executions(started_at);

create index if not exists task_executions_status_idx
    on task_executions(status);

comment on table task_executions is
'Task execution history — one row per task run. Forms a tree via parent_task_id.

status: running → completed | failed
retry_number: 0 for first attempt, incremented on retry.
duration_ms and items_processed set on completion.
started_at vs completed_at shows wall-clock execution time.

Common queries:
  -- Execution tree for a scan
  WITH RECURSIVE tree AS (
    SELECT * FROM task_executions WHERE task_id = $1
    UNION ALL
    SELECT e.* FROM task_executions e JOIN tree t ON e.parent_task_id = t.task_id
  ) SELECT * FROM tree ORDER BY started_at
  -- Slowest tasks
  SELECT task_kind, avg(duration_ms), max(duration_ms) FROM task_executions WHERE status = ''completed'' GROUP BY task_kind
  -- Failed tasks in last hour
  SELECT * FROM task_executions WHERE status = ''failed'' AND started_at > now() - interval ''1h''';

comment on column task_executions.task_id
     is 'In-memory queue task ID. Unique per daemon session (resets on restart).';
comment on column task_executions.parent_task_id
     is 'Parent task ID for hierarchy. process_file → process_git_folder → scan_root.';
comment on column task_executions.task_kind
     is 'TaskKind::to_string() — scan_root, process_file, resolve_edges, etc.';
comment on column task_executions.folder_path
     is 'Git folder abs path — groups all tasks for a repo.';
comment on column task_executions.path
     is 'Specific path this task operates on (file, folder, URL).';
comment on column task_executions.status
     is 'running, completed, or failed.';
comment on column task_executions.items_processed
     is 'Count of items handled: symbols parsed, edges resolved, files queued, etc.';
comment on column task_executions.duration_ms
     is 'Wall-clock milliseconds from start to completion.';
comment on column task_executions.retry_number
     is '0 for first attempt. Incremented if the same task is retried.';
comment on column task_executions.error_message
     is 'Error detail if status = failed.';
comment on column task_executions.retried_from_execution_id
     is 'Crash/manual-retry lineage: the execution row id this attempt retries. A uuid, not a task_id — task IDs restart every session, so only the row id is unambiguous across restarts.';
comment on column task_executions.phases_ms
     is 'Per-phase wall milliseconds for handlers that record them (process_file: parse_ms, world_load_ms, compile_wait_ms, resolve_ms, write_ms). Absent phases read as missing keys, never zero — a phase the handler never reached is not a phase that took no time.';
