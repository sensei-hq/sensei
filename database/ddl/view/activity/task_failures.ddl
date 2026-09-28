set search_path to activity, sensei, extensions;

-- The CURRENTLY-broken set: one row per job whose most recent execution failed,
-- with what it was, where, why, and how many times it has tried. This is the
-- restart list — `(task_kind, folder_path, path)` is exactly the triple a task
-- is re-enqueued from.
--
-- ## Why "latest execution failed", not "has ever failed"
--
-- A job that failed twice and then succeeded is FIXED, and listing it would put
-- work already done on a restart list. So the view ranks each job's executions
-- by time and keeps it only when the newest one is a failure. `activity.task_health`
-- keeps the full history for anyone asking what happened rather than what is broken.
--
-- ## attempts vs retry_number — read both
--
-- `retry_number` is the attempt counter the runner stamped on the LAST execution.
-- `attempts` is how many failed executions this job has accumulated in the
-- retained window. They diverge when a job is re-enqueued fresh (counter resets,
-- attempts keeps climbing), and that divergence is the poison-pill signal:
-- a job with attempts in the hundreds and a low max_retry is not retrying, it is
-- being rediscovered and re-failed every pass.
--
-- That shape is not hypothetical. Measured 2026-09-23 before repair: 17,577,049
-- failures over 43,312 paths — ~400 apiece — because the first node write failed,
-- the folder fingerprint was never advanced, and the next scan re-enqueued the
-- identical file. A per-job count makes that visible as one number; a global
-- failure total hides it completely.
--
-- ## Grouping is the point
--
-- Per-job rows are for restarting. For triage, group them — `error_signature`
-- collapses the path out of the message so a thousand files failing one way read
-- as one cause:
--
--   select error_signature, count(*) as jobs, sum(attempts) as failures
--     from activity.task_failures group by 1 order by 3 desc;
create or replace view task_failures as
with ranked as (
  select te.id
       , te.task_id
       , te.task_kind
       , te.folder_path
       , te.path
       , te.status
       , te.error_message
       , te.retry_number
       , te.started_at
       , te.duration_ms
       , row_number() over (
           partition by te.task_kind, te.folder_path, coalesce(te.path, '')
           order by te.started_at desc, te.id desc)       as rn
    from task_executions te
),
history as (
  select task_kind
       , folder_path
       , coalesce(path, '')                               as path_key
       , count(*) filter (where status = 'failed')        as attempts
       , min(started_at) filter (where status = 'failed') as first_failed_at
       , max(started_at) filter (where status = 'failed') as last_failed_at
       , max(retry_number)                                as max_retry
    from task_executions
   group by 1, 2, 3
)
select l.task_kind::text                       as task_kind
     , l.folder_path
     , (l.folder_path like '/%')               as is_folder_scoped
     , f.id                                    as folder_id
     , f.name                                  as folder
     , f.repository_id
     , r.name                                  as repository
     , f.project_id
     , p.name                                  as project
     , l.path
     , l.error_message
     , sensei.error_signature(l.error_message) as error_signature
     , h.attempts
     , l.retry_number                          as last_retry_number
     , h.max_retry
     , h.first_failed_at
     , h.last_failed_at
     , l.duration_ms                           as last_duration_ms
     , l.task_id                               as last_task_id
     , l.id                                    as last_execution_id
  from ranked l
  join history h
    on h.task_kind  = l.task_kind
   and h.folder_path = l.folder_path
   and h.path_key   = coalesce(l.path, '')
  left join sensei.folders f
    on l.folder_path like '/%'
   and f.abs_path = l.folder_path
  left join sensei.repositories r
    on r.id = f.repository_id
  left join sensei.projects p
    on p.id = f.project_id
 where l.rn = 1
   and l.status = 'failed';

comment on view task_failures is
'Jobs whose LATEST execution failed — the restart list. One row per
(task_kind, folder_path, path), which is the triple a task is re-enqueued from.

A job that failed and later succeeded is absent: it is fixed, and restarting it
would redo finished work. Full history lives in activity.task_health.

attempts (failed executions accumulated) next to max_retry (the runner''s counter)
is the poison-pill signal — high attempts with a low max_retry means the job is
being rediscovered and re-failed each pass, not retried.

Common queries:
  -- triage: which causes, biggest first
  SELECT error_signature, count(*) AS jobs, sum(attempts) AS failures
    FROM activity.task_failures GROUP BY 1 ORDER BY 3 DESC;

  -- the poison pills
  SELECT repository, task_kind, path, attempts, max_retry
    FROM activity.task_failures WHERE attempts > 10 ORDER BY attempts DESC;

  -- what to restart for one repository
  SELECT task_kind, folder_path, path
    FROM activity.task_failures WHERE repository = ''sensei'';';

comment on column task_failures.attempts is 'Failed executions accumulated for this job in the retained window. Compare against max_retry: attempts >> max_retry means re-discovery each pass, not retry — the shape that produced 17.5M rows over 43,312 paths on 2026-09-23.';
comment on column task_failures.is_folder_scoped is 'Whether folder_path is a path (true) or a group UUID (false). A false row has no repository BY CONSTRUCTION, not through a failed lookup.';

grant select on task_failures to authenticated, service_role;
