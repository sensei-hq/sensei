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
-- the same column. Measured 2026-10-01: 1,070,392 of 7,784,735 rows are the
-- UUID form.
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
--
-- ## Project is a SET, and this view keeps one row per execution anyway
--
-- Project membership no longer lives on `folders.project_id` — it lives on
-- `sensei.repositories_in_projects` and resolves through `sensei.folder_projects`,
-- which is MULTI-VALUED BY DESIGN: a repository is keyed on its remote, so
-- measured 2026-10-01, 8 repositories serve two projects each and 4,129 of
-- 13,722 folders carry more than one project row.
--
-- 1,949,256 of this view's 7,784,735 executions sit in those folders. So a plain
-- `join sensei.folder_projects` would have taken the view from 7,784,735 rows to
-- 9,733,991 — both measured 2026-10-01 in one snapshot — and silently doubled
-- the `count(*)` in the first query this file's own comment advertises.
--
-- The grain is the contract (`docs/spec/indexer/19-observability.md`: "one
-- execution"), so it is kept, and the set is carried as ARRAYS instead:
-- `projects` / `project_ids`, aggregated per folder before the join.
--
-- A lateral `LIMIT 1` was the other way to hold the grain and is rejected here:
-- there is no basis for calling one of a repository's projects the real one, and
-- an arbitrary pick that reads as authoritative IS the defect this migration
-- removes. Measured 2026-10-01, the kavach repository's 52,209 executions were
-- split 32,563 to `kavach` and 19,646 to `vite-multi-adapter` by nothing but
-- which folder row got written when — one repository, two answers. Under the
-- arrays both projects see all 52,209, and no execution is counted twice.
--
-- ## The array filter is NOT indexed, and it costs 3.5-11x. Measured, not elided.
--
-- `'sparsh' = any(projects)` does not push down. EXPLAIN (ANALYZE, BUFFERS),
-- 2026-10-01: `sensei.folders` is Seq Scanned TWICE — once to build the
-- folder->project map, once to join it back — and the project predicate is
-- applied AFTER the aggregate, as `Filter: ('sparsh' = ANY (array_agg(...)))`,
-- Rows Removed by Filter 13,677. Only below that does `task_executions` get
-- reached by Index Only Scan on `task_executions_folder_path_idx`, 45 loops.
-- The index at the bottom is real; the folder lookup above it is a scan.
--
-- The `folders.project_id` form this replaces started from
-- `folders_project_id_idx` (Bitmap Index Scan, 44 folders) and never read
-- `folders` whole. Both return the SAME 38,203 rows. Buffers for `count(*)`
-- filtered to one project, this box, 2026-10-01:
--
--   folders.project_id    4,259   every rep, stable to the buffer
--   folder_projects      25,012   at 0 workers launched
--                        35,476   at 1
--                        45,940   at 2
--
-- Each worker adds exactly one more 10,464-buffer pass over `folders`, because
-- the hash side is re-executed per worker. Forced serial
-- (`max_parallel_workers_per_gather = 0`) the planner swaps the second scan for
-- `folders_pkey` and it is 14,762 — still 3.5x. No wall-clock is quoted for this
-- shape: `folders` is 81 MB (10,385 blocks) against a 128 MB shared_buffers, so
-- a seq scan takes the bulk-read ring and retains nothing; repeated timings here
-- carry no signal — the span across repetitions in one session exceeded an
-- order of magnitude with the plan and the buffer counts unchanged.
--
-- ## Why the migration was kept anyway
--
-- The regression is bounded on both ends, and the thing it buys is correctness.
--
-- Bound 1 — the join DISAPPEARS when the project columns are not referenced.
-- Verified by EXPLAIN: `select count(*) ... where status = 'failed'` plans as a
-- bare `Index Scan using task_executions_status_idx`, no folders, no aggregate.
-- `repository` alone also leaves the `folder_projects` sub-select out (it keeps
-- only the folders/repositories joins the old form had too). Three of the five
-- queries in this file's own comment therefore never pay a cent — Q1 and Q2,
-- verified by EXPLAIN. Q3 (`= any(projects)`), Q4 (`unnest(projects)`) and Q5
-- all reference a project column and all pay.
--
-- Bound 2 — on the WHOLE-VIEW shape the overhead is under 1%. `count(projects)`
-- across all 7,784,735 rows: 795,640 buffers, against 788,723 for the old
-- form's `count(project)` — +0.9%. The extra folders pass is noise beside the
-- ~789,000-buffer pass over `task_executions` that dominates both.
--
-- Escape hatch for the selective shape, verified 2026-10-01 — filter on
-- `folder_id` through the junction instead of on the array, and the predicate
-- reaches `folders_repository_id_idx`:
--
--   select count(*) from activity.task_health
--    where folder_id in (select folder_id from sensei.folder_projects
--                         where project = 'sparsh');
--
-- 4,401 buffers, Bitmap Index Scan, no folders seq scan, the same 38,203 rows —
-- parity with the column it replaced.
--
-- And what the scan buys: `folders.project_id` returns a WRONG answer here, not
-- merely a different one. It names one project for a repository that has two,
-- so the kavach split above is what a caller actually gets —
-- `where project = 'kavach'` returns 32,563 of the 52,209 executions that ran
-- against the kavach repository and silently omits 19,646 that are equally its
-- own, 38% of the truth. 1,949,256 of 7,784,735 rows sit in such folders. The
-- cheaper query is the one that is short, so the cost is paid. Attribution
-- itself did not move: measured at folder grain 2026-10-01, all 13,722 folders
-- are attributed under both forms, and the old scalar is an element of the new
-- array in every one — 0 lost, 0 gained, 0 disagreeing.
--
-- ## DEPLOY NOTE
--
-- `project_id uuid` / `project text` became `project_ids uuid[]` / `projects
-- text[]` — a rename AND a type change, and `create or replace view` can do
-- neither, so this file drops first. Nothing depends on this view (checked
-- 2026-10-01 via pg_depend: zero dependent rewrite rules, and no other view or
-- function in the database names it), so the bare drop cannot cascade. ACLs DO
-- NOT survive `drop view` — the `grant` at the foot is what restores them, and
-- it is not optional.
drop view if exists task_health;

create view task_health as
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
     , fps.project_ids
     , fps.projects
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
  -- Collapsed to one row per folder BEFORE the join, which is what keeps the
  -- execution grain. The `project_id` tiebreak is what makes element i of
  -- `project_ids` element i of `projects`: `sensei.projects.name` carries no
  -- unique constraint (only `projects_pkey` on id), so two projects may share a
  -- name, and `order by fp.project` alone would leave the two aggregates free to
  -- order the tied pair differently. No duplicate name exists today — measured
  -- 2026-10-01, 0 of 251 — which is exactly why the guarantee needs stating in
  -- the ORDER BY rather than in the data.
  left join (
         select fp.folder_id
              , array_agg(fp.project_id order by fp.project, fp.project_id) as project_ids
              , array_agg(fp.project    order by fp.project, fp.project_id) as projects
           from sensei.folder_projects fp
          group by fp.folder_id
       ) fps
    on fps.folder_id = f.id;

comment on view task_health is
'Every task execution with repository/project attribution and a groupable error
signature. The "what ran" grain; see activity.task_failures for "what is still
broken". ONE ROW PER EXECUTION — that grain is the contract, so count(*) here is
a count of executions and nothing in this view fans it out.

folder_path is overloaded — an absolute path for folder-scoped kinds, a UUID for
group-scoped ones. The folder join is guarded on `like ''/%''`, so a NULL
repository on a group-scoped row means "not folder-scoped", not "lookup failed".
Read is_folder_scoped before reading anything into a NULL repository. A NULL on a
row where is_folder_scoped IS true is the opposite — a path with no sensei.folders
row, a lookup that found nothing. Measured 2026-10-01: 4,052 rows over 69 paths.

project is a SET, carried as the arrays `projects` / `project_ids`, because a
repository keyed on its remote can serve more than one project. `= any(projects)`
is NOT indexed: it seq-scans sensei.folders twice and filters after the aggregate
(measured 2026-10-01 — 25,012 buffers vs 4,259 for the folders.project_id column
it replaced, on a one-project count). Touch neither project column and the join
is removed outright. If the selective shape matters, filter through the junction
on folder_id instead. That reaches `folders_repository_id_idx` at 4,401
buffers FOR A `count(*)`; selecting a project column (including `select *`)
re-introduces the folders scans and measures ~25,800, so the escape hatch is
the aggregate shape, not the row-returning one.

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
   GROUP BY 1 ORDER BY 1 DESC LIMIT 24;

  -- one project''s executions — the replacement for `WHERE project = $1`
  SELECT * FROM activity.task_health WHERE ''sparsh'' = any(projects);

  -- failures per project, with shared repositories counted under BOTH projects
  -- and no execution counted twice within one
  SELECT prj, count(*) FROM activity.task_health, unnest(projects) AS prj
   WHERE status = ''failed'' GROUP BY 1 ORDER BY 2 DESC;

  -- same 38,203 rows as `= any(projects)`. NOT index-reachable as written:
  -- `select *` includes `projects`, so the sub-select survives and folders is
  -- scanned twice (~25,800 buffers, measured 2026-10-01 forced-serial). Drop the
  -- project columns from the select list — or aggregate — and the same predicate
  -- plans as a Bitmap Index Scan at 4,401 buffers.
  SELECT * FROM activity.task_health
   WHERE folder_id IN (SELECT folder_id FROM sensei.folder_projects
                        WHERE project = ''sparsh'');';

comment on column task_health.is_folder_scoped is 'Whether folder_path holds a path (true) or a group UUID (false). A false row has no folder, repository or project BY CONSTRUCTION — never treat its NULL attribution as a gap.';
comment on column task_health.error_signature is 'error_message with paths collapsed, so failures group by cause instead of by file. NULL when error_message is NULL (a success).';
comment on column task_health.repository is 'Repository the task ran against, via folders.abs_path -> folders.repository_id. A direct column read, not repo_anchor_for: that resolver walks to an ancestor anchor, which would attribute a task to a folder it never touched.';
comment on column task_health.projects is 'EVERY project the execution''s repository serves, name-ordered — from sensei.folder_projects, never from folders.project_id. An array and not a scalar so the execution grain survives: 1,949,256 of 7,784,735 rows sit in a shared repository (measured 2026-10-01) and a flat join would have counted each of them twice. Filter with `''x'' = any(projects)` (not indexed — it seq-scans folders twice; see the view comment for the index-reachable form); expand with `unnest`. NULL IS THREE DIFFERENT THINGS, and one of them IS a failed lookup. Measured 2026-10-01 over 1,074,444 NULL rows: 1,070,392 are group-scoped (no folder by construction — read is_folder_scoped first); 4,052 are folder-scoped rows over 69 distinct paths that have NO sensei.folders row at all, which is a lookup that found nothing and is a real gap; 0 are a folder whose repository joins no project, which is possible by construction but does not occur today (all 253 repositories that folders reference are in some project). To separate the gap from the by-construction NULLs: is_folder_scoped and folder_id is null.';
comment on column task_health.project_ids is 'project_ids[i] is the id of projects[i]. Both aggregates share `order by project, project_id`; the project_id tiebreak is load-bearing because sensei.projects.name has no unique constraint, so two same-named projects would otherwise be free to order differently in the two arrays.';

grant select on task_health to authenticated, service_role;
