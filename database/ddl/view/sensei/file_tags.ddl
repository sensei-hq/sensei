set search_path to sensei, extensions;

-- REPO-RELATIVE, reconstructed. `files.file_path` is FOLDER-relative and a
-- module folder's `folders.path` is repo-relative, so a node in
-- `crates/senseid/src/lib.rs` stores `src/lib.rs` against the `crates/senseid`
-- folder. Exposing that raw would rename the column's meaning without changing
-- its name — v1's `nodes.file_path` was repo-relative, and 17 of this repo's 18
-- folders are modules, so almost every path would have been silently truncated.
-- The repo-root folder is the exception: its `path` is ABSOLUTE, and its files
-- are already repo-relative, so it passes through.
--
-- ## Project — membership is the REPOSITORY's, and it is a SET
--
-- `project_id`/`project`/`project_maturity` no longer read `folders.project_id`.
-- Membership lives in `repositories_in_projects`, and `sensei.folder_projects`
-- resolves it. A repository is keyed on its REMOTE, so one can serve several
-- projects and that resolution is MULTI-VALUED.
--
-- ONE ROW PER FILE NODE IS A CONTRACT HERE, not a preference. The sole reader is
-- `PgStore::get_files_by_tag`, which runs `WHERE folder = $1 AND $2 = ANY(tags)
-- ORDER BY file_path LIMIT 200` — a second row per node would both repeat files
-- and halve that window. The sub-select below collapses the set per folder
-- BEFORE anything joins a node, so the grain is untouched: measured 2026-10-02,
-- 7,120 rows and 7,120 distinct ids, where a plain join to `folder_projects`
-- returns 9,242.
--
-- THE SCALAR IS DEFINED ONLY WHERE THE SET HAS ONE MEMBER — the rule
-- `graph_nodes` sets, applied to all three project columns so they cannot
-- disagree. At `cardinality = 1` element 1 is not a pick, it is the only
-- element; with two, naming one of two true answers is the drift this migration
-- removes, so the scalar is NULL and `project_ids`/`projects`/
-- `project_maturities` carry both. Measured 2026-10-02: 2,122 of 7,120 file
-- nodes (29.8%) sit in a folder whose repository serves two projects, and those
-- 2,122 are exactly the rows this migration changes — the scalar each used to
-- read is always one of the two the set now carries, never a third value
-- (checked: 0 folders whose `folders.project_id` is absent from
-- `folder_projects`). Filter with `'name' = ANY(projects)`.
--
-- INLINE IN THE FROM CLAUSE, NEVER A CTE. Spelled `with ... as (...)` the
-- identical SQL cost `graph_nodes`' consumer an order of magnitude, with three
-- added sequential scans: the planner stops parameterising the view by the
-- consumer's key and hashes the whole thing. It is also why the view stays
-- within `join_collapse_limit` (8 here, read not assumed) — an aggregate blocks
-- sub-query pull-up, so this counts as ONE relation, and it REPLACES the
-- `projects` join it reads the maturity out of rather than adding to it.
create or replace view file_tags as
select n.id
     , n.folder_id
     , f.name            as folder
     , case when cardinality(ps.project_ids)         = 1 then ps.project_ids[1]         end as project_id
     , case when cardinality(ps.projects)            = 1 then ps.projects[1]            end as project
     , case when cardinality(ps.project_maturities)  = 1 then ps.project_maturities[1]  end as project_maturity
     , np.file_path
     , n.tags
     , n.props
     , n.modified_at
     , ps.project_ids
     , ps.projects
     , ps.project_maturities
  from nodes n
  left join node_paths np on np.node_id = n.id
  join folders  f on f.id = n.folder_id
  -- One row per folder, grouped HERE — before any node joins it — so the
  -- multi-valued source cannot change this view's grain.
  left join (
    select fp.folder_id
         , array_agg(fp.project_id order by fp.project, fp.project_id) as project_ids
         , array_agg(fp.project    order by fp.project, fp.project_id) as projects
         , array_agg(pj.maturity   order by fp.project, fp.project_id) as project_maturities
      from folder_projects fp
      left join projects pj on pj.id = fp.project_id
     group by fp.folder_id
  ) ps on ps.folder_id = f.id
 where n.kind = 'file';

comment on view file_tags is
'File nodes with classification tags, folder and project context.
Tags assigned during indexing: src, test, e2e, config.

Filter/group dimensions: project, project_maturity, folder, tags.

PROJECT MEMBERSHIP IS THE REPOSITORY''S, AND IT IS A SET. It comes from
`sensei.folder_projects`, never from `folders.project_id` — that column was
settable per folder, so two folders of one repository could name two different
projects. `project_ids`/`projects`/`project_maturities` carry every project;
`project_id`/`project`/`project_maturity` are the SOLE member of that set, and
NULL when there is more than one, because naming one of two true answers is the
drift this replaces. Measured 2026-10-02: 2,122 of 7,120 file nodes (29.8%) sit
in a folder whose repository serves two projects and so read a NULL scalar.

ONE ROW PER FILE NODE, STILL. The set is collapsed per folder before any node
joins it, so `id` stays unique and a `LIMIT` still means what it says. Verified
2026-10-02: 7,120 rows and 7,120 distinct ids, unchanged by the migration, where
a plain join to `folder_projects` would have returned 9,242.

Common queries:
  SELECT file_path FROM file_tags WHERE folder = ''myrepo'' AND ''test'' = ANY(tags)
  -- by project — the spelling that reads a multi-project repository right
  SELECT unnest(tags) as tag, count(*) FROM file_tags WHERE ''sensei'' = ANY(projects) GROUP BY tag
  SELECT folder, count(*) FROM file_tags WHERE ''active'' = ANY(project_maturities) AND ''test'' = ANY(tags) GROUP BY folder';

comment on column file_tags.projects is
'Every project this file''s repository belongs to, from `sensei.folder_projects` — set-valued because a repository is keyed on its REMOTE and can serve more than one project. Filter with `''name'' = ANY(projects)`. NULL means the folder''s repository belongs to no project, not that the lookup failed: the join finds nothing rather than inventing one, and the LEFT JOIN keeps the file visible instead of dropping it.';
comment on column file_tags.project_ids is
'The `projects` set by id, in the same order — the join key for `sensei.projects` without a second name lookup.';
comment on column file_tags.project_maturities is
'The `projects` set''s maturities, in the same order. Measured 2026-10-02 no multi-project repository''s owners disagree on maturity, but that is data, not a constraint, so the set is what is exposed.';
comment on column file_tags.project is
'The project, ONLY where this file''s repository serves exactly one — the sole member of `projects`, not a choice among candidates. NULL means the set has more than one member (2,122 of 7,120 file nodes on 2026-10-02), never that the file has no project: read `projects` for those. It is NOT `folders.project_id`, which let two folders of one repository name two different projects.';
comment on column file_tags.project_id is
'The id of `project`, under the same rule: the sole member of `project_ids`, or NULL where there is more than one.';
comment on column file_tags.project_maturity is
'The maturity of `project`, under the same rule — NULL wherever `project` is NULL, so the two can never disagree. Read `project_maturities` for a multi-project repository.';
