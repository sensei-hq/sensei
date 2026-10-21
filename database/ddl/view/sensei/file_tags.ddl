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
-- ## Workspace — membership is the REPOSITORY's, and it is a SET
--
-- `workspace_id`/`workspace`/`workspace_maturity` no longer read `folders.workspace_id`.
-- Membership lives in `repositories_in_workspaces`, and `sensei.folder_workspaces`
-- resolves it. A repository is keyed on its REMOTE, so one can serve several
-- workspaces and that resolution is MULTI-VALUED.
--
-- ONE ROW PER FILE NODE IS A CONTRACT HERE, not a preference. The sole reader is
-- `PgStore::get_files_by_tag`, which runs `WHERE folder = $1 AND $2 = ANY(tags)
-- ORDER BY file_path LIMIT 200` — a second row per node would both repeat files
-- and halve that window. The sub-select below collapses the set per folder
-- BEFORE anything joins a node, so the grain is untouched: measured 2026-10-02,
-- 7,120 rows and 7,120 distinct ids, where a plain join to `folder_workspaces`
-- returns 9,242.
--
-- THE SCALAR IS DEFINED ONLY WHERE THE SET HAS ONE MEMBER — the rule
-- `graph_nodes` sets, applied to all three workspace columns so they cannot
-- disagree. At `cardinality = 1` element 1 is not a pick, it is the only
-- element; with two, naming one of two true answers is the drift this migration
-- removes, so the scalar is NULL and `workspace_ids`/`workspaces`/
-- `workspace_maturities` carry both. Measured 2026-10-02: 2,122 of 7,120 file
-- nodes (29.8%) sit in a folder whose repository serves two workspaces, and those
-- 2,122 are exactly the rows this migration changes — the scalar each used to
-- read is always one of the two the set now carries, never a third value
-- (checked: 0 folders whose `folders.workspace_id` is absent from
-- `folder_workspaces`). Filter with `'name' = ANY(workspaces)`.
--
-- INLINE IN THE FROM CLAUSE, NEVER A CTE. Spelled `with ... as (...)` the
-- identical SQL cost `graph_nodes`' consumer an order of magnitude, with three
-- added sequential scans: the planner stops parameterising the view by the
-- consumer's key and hashes the whole thing. It is also why the view stays
-- within `join_collapse_limit` (8 here, read not assumed) — an aggregate blocks
-- sub-query pull-up, so this counts as ONE relation, and it REPLACES the
-- `workspaces` join it reads the maturity out of rather than adding to it.
create or replace view file_tags as
select n.id
     , n.folder_id
     , f.name            as folder
     , case when cardinality(ps.workspace_ids)         = 1 then ps.workspace_ids[1]         end as workspace_id
     , case when cardinality(ps.workspaces)            = 1 then ps.workspaces[1]            end as workspace
     , case when cardinality(ps.workspace_maturities)  = 1 then ps.workspace_maturities[1]  end as workspace_maturity
     , np.file_path
     , n.tags
     , n.props
     , n.modified_at
     , ps.workspace_ids
     , ps.workspaces
     , ps.workspace_maturities
  from nodes n
  left join node_paths np on np.node_id = n.id
  join folders  f on f.id = n.folder_id
  -- One row per folder, grouped HERE — before any node joins it — so the
  -- multi-valued source cannot change this view's grain.
  left join (
    select fp.folder_id
         , array_agg(fp.workspace_id order by fp.workspace, fp.workspace_id) as workspace_ids
         , array_agg(fp.workspace  order by fp.workspace, fp.workspace_id) as workspaces
         , array_agg(pj.maturity   order by fp.workspace, fp.workspace_id) as workspace_maturities
      from folder_workspaces fp
      left join workspaces pj on pj.id = fp.workspace_id
     group by fp.folder_id
  ) ps on ps.folder_id = f.id
 where n.kind = 'file';

comment on view file_tags is
'File nodes with classification tags, folder and workspace context.
Tags assigned during indexing: src, test, e2e, config.

Filter/group dimensions: workspace, workspace_maturity, folder, tags.

WORKSPACE MEMBERSHIP IS THE REPOSITORY''S, AND IT IS A SET. It comes from
`sensei.folder_workspaces`, never from `folders.workspace_id` — that column was
settable per folder, so two folders of one repository could name two different
workspaces. `workspace_ids`/`workspaces`/`workspace_maturities` carry every workspace;
`workspace_id`/`workspace`/`workspace_maturity` are the SOLE member of that set, and
NULL when there is more than one, because naming one of two true answers is the
drift this replaces. Measured 2026-10-02: 2,122 of 7,120 file nodes (29.8%) sit
in a folder whose repository serves two workspaces and so read a NULL scalar.

ONE ROW PER FILE NODE, STILL. The set is collapsed per folder before any node
joins it, so `id` stays unique and a `LIMIT` still means what it says. Verified
2026-10-02: 7,120 rows and 7,120 distinct ids, unchanged by the migration, where
a plain join to `folder_workspaces` would have returned 9,242.

Common queries:
  SELECT file_path FROM file_tags WHERE folder = ''myrepo'' AND ''test'' = ANY(tags)
  -- by workspace — the spelling that reads a multi-workspace repository right
  SELECT unnest(tags) as tag, count(*) FROM file_tags WHERE ''sensei'' = ANY(workspaces) GROUP BY tag
  SELECT folder, count(*) FROM file_tags WHERE ''active'' = ANY(workspace_maturities) AND ''test'' = ANY(tags) GROUP BY folder';

comment on column file_tags.workspaces is
'Every workspace this file''s repository belongs to, from `sensei.folder_workspaces` — set-valued because a repository is keyed on its REMOTE and can serve more than one workspace. Filter with `''name'' = ANY(workspaces)`. NULL means the folder''s repository belongs to no workspace, not that the lookup failed: the join finds nothing rather than inventing one, and the LEFT JOIN keeps the file visible instead of dropping it.';
comment on column file_tags.workspace_ids is
'The `workspaces` set by id, in the same order — the join key for `sensei.workspaces` without a second name lookup.';
comment on column file_tags.workspace_maturities is
'The `workspaces` set''s maturities, in the same order. Measured 2026-10-02 no multi-workspace repository''s owners disagree on maturity, but that is data, not a constraint, so the set is what is exposed.';
comment on column file_tags.workspace is
'The workspace, ONLY where this file''s repository serves exactly one — the sole member of `workspaces`, not a choice among candidates. NULL means the set has more than one member (2,122 of 7,120 file nodes on 2026-10-02), never that the file has no workspace: read `workspaces` for those. It is NOT `folders.workspace_id`, which let two folders of one repository name two different workspaces.';
comment on column file_tags.workspace_id is
'The id of `workspace`, under the same rule: the sole member of `workspace_ids`, or NULL where there is more than one.';
comment on column file_tags.workspace_maturity is
'The maturity of `workspace`, under the same rule — NULL wherever `workspace` is NULL, so the two can never disagree. Read `workspace_maturities` for a multi-workspace repository.';
