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
--
-- ## Workspace is an ARRAY, because one row per job outranks one workspace per row
--
-- Workspace membership is no longer a column on `folders`. It lives in
-- `sensei.repositories_in_workspaces` and resolves through `sensei.folder_workspaces`,
-- which is MULTI-VALUED BY DESIGN: a repository is keyed on its remote, so two
-- checkouts collapse to one repository row and that row can serve more than one
-- workspace. Measured 2026-10-01: 8 repositories carry two workspace links each
-- (`bridge` in `bridge` + `sparsh`, `kavach` in `kavach` + `vite-multi-adapter`,
-- `documentation` in `documentation` + `client-q`, and five more), which fans
-- 13,722 folders out to 17,851 `folder_workspaces` rows — 4,129 folders return two.
--
-- A plain join to that view would therefore emit the SAME broken job twice, and
-- this is a restart list: a duplicated row is a job re-enqueued twice. The fan-out
-- is live here, not theoretical — measured 2026-10-01: 24 of the 297 folder paths
-- carrying a failed execution sit on a two-workspace repository, and 2 of this
-- view's 622 rows land on one, so a plain join would have returned 624.
--
-- So the workspaces are collapsed per folder BEFORE the join — `folder_workspace_sets`
-- groups by `folder_id`, which makes it unique by construction, and the join to it
-- cannot change the row count. `workspaces` / `workspace_ids` are arrays; a folder
-- under no tracked repository gets NULL, which means "no tracked repository", not
-- "lookup failed" (same shape as the NULL `repository` beside it).
--
-- The swap did not move attribution. Measured 2026-10-01 over the same 622 rows,
-- the old `folders.workspace_id` path and the new one attributed the SAME 447 rows:
-- 0 lost, 0 gained, 0 disagreeing. The other 175 carry no workspace because 39 are
-- group-scoped (no folder at all) and 136 name a path with no `folders` row —
-- every folder that DOES exist here has a tracked repository.
--
-- `LIMIT 1` on an ordered pick was rejected. It would have kept a scalar `workspace`
-- column, and it is precisely the defect this migration exists to remove: an
-- arbitrary choice between two true answers, rendered in a column name that reads
-- as authoritative. Callers that need one workspace must name which:
--
--   -- what to restart for one workspace
--   select task_kind, folder_path, path
--     from activity.task_failures where 'sensei' = any(workspaces);
--
-- Dropped before create: `workspace_id`/`workspace` become `workspace_ids`/`workspaces` —
-- a rename AND a type change, and `create or replace view` can do neither. Nothing
-- depends on this view (checked 2026-10-01 via pg_depend: zero dependents), so
-- there is no cascade. ACLs do not survive a drop; the grant at the foot restores
-- them.
drop view if exists task_failures;

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
),
folder_workspace_sets as (
  -- One row per folder — `group by folder_id` is what keeps the join below from
  -- multiplying the restart list by a folder's workspace count.
  select fp.folder_id
       , array_agg(fp.workspace_id order by fp.workspace, fp.workspace_id) as workspace_ids
       , array_agg(fp.workspace  order by fp.workspace, fp.workspace_id) as workspaces
    from sensei.folder_workspaces fp
   group by fp.folder_id
)
select l.task_kind::text                       as task_kind
     , l.folder_path
     , (l.folder_path like '/%')               as is_folder_scoped
     , f.id                                    as folder_id
     , f.name                                  as folder
     , f.repository_id
     , r.name                                  as repository
     , fps.workspace_ids
     , fps.workspaces
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
  left join folder_workspace_sets fps
    on fps.folder_id = f.id
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

workspaces is an ARRAY and not a scalar. Membership resolves through
sensei.folder_workspaces, which is multi-valued by design — a repository keyed on
its remote can serve more than one workspace — and one row per job matters more
here than one workspace per row, because a duplicated row is a job restarted twice.

Common queries:
  -- triage: which causes, biggest first
  SELECT error_signature, count(*) AS jobs, sum(attempts) AS failures
    FROM activity.task_failures GROUP BY 1 ORDER BY 3 DESC;

  -- the poison pills
  SELECT repository, task_kind, path, attempts, max_retry
    FROM activity.task_failures WHERE attempts > 10 ORDER BY attempts DESC;

  -- what to restart for one repository
  SELECT task_kind, folder_path, path
    FROM activity.task_failures WHERE repository = ''sensei'';

  -- what to restart for one workspace
  SELECT task_kind, folder_path, path
    FROM activity.task_failures WHERE ''sensei'' = any(workspaces);';

comment on column task_failures.attempts is 'Failed executions accumulated for this job in the retained window. Compare against max_retry: attempts >> max_retry means re-discovery each pass, not retry — the shape that produced 17.5M rows over 43,312 paths on 2026-09-23.';
comment on column task_failures.is_folder_scoped is 'Whether folder_path is a path (true) or a group UUID (false). A false row has no repository and no workspaces BY CONSTRUCTION, not through a failed lookup.';
comment on column task_failures.workspaces is 'EVERY workspace this job''s folder belongs to, ordered by name — an array because sensei.folder_workspaces is multi-valued (a repository keyed on its remote can serve two workspaces). Collapsed per folder before the join so the restart list keeps one row per job; test membership with ''name'' = any(workspaces). NULL means the folder is under no tracked repository, not that a lookup failed.';
comment on column task_failures.workspace_ids is 'Workspace UUIDs matching workspaces element-for-element (same ORDER BY). From sensei.repositories_in_workspaces, never from folders.workspace_id — a folder does not carry its own workspace.';

grant select on task_failures to authenticated, service_role;
