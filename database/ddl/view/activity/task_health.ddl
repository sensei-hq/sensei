set search_path to activity, sensei, extensions;

-- Every task execution, attributed to the repository and project it ran against
-- — the "what ran" grain. Its sibling `activity.task_failures` is the "what is
-- still broken, and what would I restart" grain.
--
-- ## Why this exists
--
-- `activity.task_executions` records the receipts but names nothing a human
-- groups by: `folder_path` is a path, not a repository, and `error_message`
-- embeds the file it is about, so grouping on it yields one group per file.
-- Answering "which repository is failing, and why" meant a bespoke query with a
-- join and a regexp every time.
--
-- It is also how a failure lake stays invisible. Measured 2026-09-23: the tree
-- held 19,807,261 executions of which 17,634,822 were failures — 89% — and a
-- LIFETIME count read as an ongoing incident when an hourly one showed it had
-- stopped dead the hour a fix deployed. Grouping is not a convenience here; a
-- raw total with no time axis and no cause axis is actively misleading.
--
-- ## `folder_path` is overloaded, and this view says so
--
-- For folder-scoped kinds it is an absolute path. For group-scoped kinds
-- (`compute_group_metrics` and friends) it is a UUID — a different key space in
-- the same column. 414,809 of 2,230,464 rows are the UUID form.
--
-- So the folder join is GUARDED on `folder_path like '/%'` rather than left to
-- miss. The difference matters: an unguarded join makes a group-scoped task look
-- like a folder-scoped task whose folder is missing, which is a bug report about
-- nothing. `is_folder_scoped` makes the two populations separable, and a NULL
-- repository on a group-scoped row is CORRECT rather than unattributed.
--
-- Attribution is a direct join on `folders.abs_path` -> `folders.repository_id`,
-- never `repo_anchor_for`. That resolver walks to the nearest ANCESTOR anchor,
-- which is the right answer for an arbitrary cwd and the wrong answer here: a
-- task ran against one specific folder, and rolling it up to an ancestor would
-- report it against a folder it never touched.
create or replace view task_health as
select te.id
     , te.task_id
     , te.parent_task_id
     , te.task_kind::text                    as task_kind
     , te.status
     , te.folder_path
     , (te.folder_path like '/%')            as is_folder_scoped
     , f.id                                  as folder_id
     , f.name                                as folder
     , f.branch
     , f.repository_id
     , r.name                                as repository
     , f.project_id
     , p.name                                as project
     , te.path
     , te.error_message
     , sensei.error_signature(te.error_message) as error_signature
     , te.retry_number
     , te.items_processed
     , te.duration_ms
     , te.started_at
     , te.completed_at
  from task_executions te
  left join sensei.folders f
    on te.folder_path like '/%'
   and f.abs_path = te.folder_path
  left join sensei.repositories r
    on r.id = f.repository_id
  left join sensei.projects p
    on p.id = f.project_id;

comment on view task_health is
'Every task execution with repository/project attribution and a groupable error
signature. The "what ran" grain; see activity.task_failures for "what is still
broken".

folder_path is overloaded — an absolute path for folder-scoped kinds, a UUID for
group-scoped ones. The folder join is guarded on `like ''/%''`, so a NULL
repository on a group-scoped row means "not folder-scoped", not "lookup failed".
Read is_folder_scoped before reading anything into a NULL repository.

Common queries:
  -- which repository is failing, and why
  SELECT repository, error_signature, count(*)
    FROM activity.task_health WHERE status = ''failed''
   GROUP BY 1, 2 ORDER BY 3 DESC;

  -- is it ongoing, or a historical lake? ALWAYS ask this before reporting a total
  SELECT date_trunc(''hour'', started_at) AS hr,
         count(*) FILTER (WHERE status = ''failed'')    AS failed,
         count(*) FILTER (WHERE status = ''completed'') AS completed
    FROM activity.task_health WHERE task_kind = ''process_file''
   GROUP BY 1 ORDER BY 1 DESC LIMIT 24;';

comment on column task_health.is_folder_scoped is 'Whether folder_path holds a path (true) or a group UUID (false). A false row has no folder, repository or project BY CONSTRUCTION — never treat its NULL attribution as a gap.';
comment on column task_health.error_signature is 'error_message with paths collapsed, so failures group by cause instead of by file. NULL when error_message is NULL (a success).';
comment on column task_health.repository is 'Repository the task ran against, via folders.abs_path -> folders.repository_id. A direct column read, not repo_anchor_for: that resolver walks to an ancestor anchor, which would attribute a task to a folder it never touched.';

grant select on task_health to authenticated, service_role;
