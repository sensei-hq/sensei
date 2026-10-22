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
-- WORKSPACE MEMBERSHIP IS THE REPOSITORY'S, AND IT IS A SET. `workspace_id`/`workspace`
-- no longer read `folders.workspace_id`; membership lives in
-- `repositories_in_workspaces` and `sensei.folder_workspaces` resolves it.
--
-- `folder_workspaces` IS MULTI-VALUED — a repository is keyed on its REMOTE, so
-- two checkouts collapse to one repository row and eight repositories serve two
-- workspaces each. A PLAIN JOIN WOULD CHANGE THIS VIEW'S GRAIN: measured
-- 2026-10-02 on the live DB in ONE snapshot it yields 838,718 rows against
-- 737,980 symbols, +100,738 — exactly the symbol count of the 7
-- shared-repository folders that hold nodes (46,985 policy-management + 32,539
-- mycm.net + 14,218 employee-portal + 4,146 client-q-reference-schema + 1,602
-- kavach + 844 vite-multi-adapter + 404 sparsh). This view is a symbol
-- CATALOGUE read one
-- row per symbol — the comment below advertises `SELECT * ... WHERE folder = x
-- AND name ILIKE y` and `SELECT folder, count(*) ... GROUP BY folder`, and both
-- are wrong under duplication. The sub-select below collapses the set per
-- folder BEFORE any node joins it, so one symbol stays one row.
--
-- WRITE THAT SUB-SELECT INLINE IN THE FROM CLAUSE, NEVER AS A CTE. The same SQL
-- spelled `with ... as` cost `graph_nodes` a consumer query an order of magnitude
-- with three sequential scans: the planner stops parameterising this side by the
-- consumer's key and hashes the whole view. Same rule, same spelling, here.
--
-- THE SCALAR IS DEFINED ONLY WHERE THE SET HAS ONE MEMBER, applied column by
-- column. `cardinality = 1` then element 1 is not a pick — it is the only
-- element. Where a repository serves two workspaces `workspace_id`/`workspace` are
-- NULL and `workspace_ids`/`workspaces` carry both, because naming one of two true
-- answers is the drift this migration removes. `workspace_maturity` is read off
-- the DISTINCT maturity set, so it survives wherever the two workspaces agree:
-- measured 2026-10-02 all 7 shared folders holding symbols resolve to a
-- single maturity (`discovery`), so `workspace_maturity IS NULL` counts 0 rows
-- after the migration against 0 before — the advertised
-- `WHERE workspace_maturity = x` query loses nothing. It goes NULL only when two
-- workspaces of one repository genuinely differ, which has no single answer.
create or replace view symbols as
select n.id
     , n.folder_id
     , f.name            as folder
     , case when cardinality(ps.workspace_ids) = 1 then ps.workspace_ids[1] end as workspace_id
     , case when cardinality(ps.workspaces)    = 1 then ps.workspaces[1]    end as workspace
     , case when cardinality(ps.maturities)  = 1 then ps.maturities[1]  end as workspace_maturity
     , f.status          as folder_status
     , f.kind            as folder_kind
     , n.parent_id
     , n.kind
     , n.name
     , np.file_path
     , n.signature
     , n.description
     , n.docstring
     , n.line_start
     , n.line_end
     , n.is_exported
     , n.community_id
     , n.tags
     , n.props
     , n.modified_at
     , ps.workspace_ids
     , ps.workspaces
  from nodes n
  left join node_paths np on np.node_id = n.id
  join folders  f on f.id = n.folder_id
  -- INLINE, NOT A CTE — see the note above. One row per folder, grouped HERE,
  -- before anything joins a node, so the multi-valued source cannot change the
  -- grain. `workspace_ids` and `workspaces` share an ORDER BY so they zip; the
  -- maturity set is DISTINCT because two workspaces that agree on maturity give
  -- the column one true answer, not two.
  --
  -- JOINED ON `f.id`, NOT ON `n.folder_id`, AND THE TWO PLAN DIFFERENTLY. They
  -- are the same predicate — `f.id = n.folder_id` is an inner join above — and
  -- the output is identical, but on `n.folder_id` the planner can only apply a
  -- workspace predicate AFTER the node scan: `where workspace_maturity = x` planned
  -- as a Seq Scan of all 737,980 nodes feeding a HashAggregate. On `f.id` the
  -- predicate lands on the folders side first and the small folder set drives
  -- `nodes_folder_id_idx` — the same index-driven shape the `folders.workspace_id`
  -- spelling had (both measured at max_parallel_workers_per_gather = 0,
  -- 2026-10-02). The folder+name+kind query plans identically either way
  -- (BitmapAnd of `nodes_folder_id_idx` and `nodes_kind_idx`).
  left join (
    select fp.folder_id
         , array_agg(fp.workspace_id order by fp.workspace, fp.workspace_id) as workspace_ids
         , array_agg(fp.workspace  order by fp.workspace, fp.workspace_id) as workspaces
         , array_agg(distinct pj.maturity)                             as maturities
      from folder_workspaces fp
      left join workspaces pj on pj.id = fp.workspace_id
     group by fp.folder_id
  ) ps on ps.folder_id = f.id
 where n.kind not in ('file', 'section', 'rationale');

comment on view symbols is
'Flattened code symbols (functions, classes, types, etc.) with folder and workspace context.
Excludes file nodes, doc sections, and rationale comments.

WORKSPACE MEMBERSHIP IS THE REPOSITORY''S, AND IT IS A SET. It comes from
`sensei.folder_workspaces`, never from `folders.workspace_id` — that column was
settable per folder, so the folders of one repository could name different
workspaces. `workspaces`/`workspace_ids` carry every workspace; `workspace`/`workspace_id`
are the SOLE member of that set and NULL when it has more than one, because
naming one of two true answers is the drift this replaces. Measured 2026-10-02:
7 of the 76 folders holding symbols sit in a repository serving two workspaces, so
100,738 of 737,980 rows (13.65%) read a NULL scalar and a two-element set. Filter
those with `''name'' = ANY(workspaces)`.

ONE ROW PER SYMBOL, STILL. The set is collapsed per folder before any node joins
it. Verified 2026-10-02: 737,980 rows and 737,980 distinct ids, unchanged by the
migration, where a plain join to `folder_workspaces` would have returned 838,718.
Diffed row by row under one snapshot: 0 ids gained, 0 lost, and of the 22
pre-existing columns only `workspace_id` and `workspace` moved, on exactly those
109,674 rows, and they split TWO ways rather than one — the earlier claim that
the diff was "exactly those 100,738" was wrong:

- 100,738 move from a scalar to NULL because the row''s repository serves more
  than one workspace. The old value is still a member of `workspace_ids`; nothing
  was lost, it stopped being singular.
- 3,375 are RELABELLED — the scalar changes to a different workspace. These are
  not noise. They are rows whose `folders.workspace_id` disagreed with what
  `repositories_in_workspaces` maps their repository to, and the junction is
  authoritative (#210). Measured in one REPEATABLE READ snapshot on
  2026-10-02, 601 folders were in that state. An hour earlier the same query
  returned 0: the column and the junction are written at different points in a
  scan, so while indexing is in flight they disagree. That is the second source
  of truth this migration exists to remove, observed live.

0 rows end with no workspace set at all.

Filter/group dimensions: workspaces, workspace_maturity, folder, folder_status, kind, is_exported, tags.

Common queries:
  SELECT * FROM symbols WHERE folder = ''myrepo'' AND name ILIKE ''%auth%'' AND kind = ''function''
  SELECT kind::text, count(*) FROM symbols WHERE ''sensei'' = ANY(workspaces) GROUP BY kind
  SELECT folder, count(*) FROM symbols WHERE workspace_maturity = ''active'' GROUP BY folder';

comment on column symbols.workspaces is
'Every workspace this symbol''s repository belongs to, from `sensei.folder_workspaces` — set-valued because a repository is keyed on its REMOTE and can serve more than one workspace. Filter with `''name'' = ANY(workspaces)`; that is the spelling that reads a shared repository completely. NULL has two causes, both honest-empty: the folder belongs to no tracked repository, or its repository belongs to no workspace. Measured 2026-10-02 neither occurs — all 13,724 folders resolve to at least one workspace — and the LEFT JOIN is what keeps such a symbol in the catalogue instead of dropping it.';
comment on column symbols.workspace_ids is
'The `workspaces` set by id, in the same order — the join key for `sensei.workspaces` without a second name lookup.';
comment on column symbols.workspace is
'The workspace, ONLY where the repository serves exactly one — the sole member of `workspaces`, not a choice among candidates. NULL means the set has more than one member (100,738 of 737,980 rows on 2026-10-02), never that the symbol has no workspace: read `workspaces` for those. It is NOT `folders.workspace_id`, which let two folders of one repository name two different workspaces.';
comment on column symbols.workspace_id is
'The id of `workspace`, under the same rule: the sole member of `workspace_ids`, or NULL where there is more than one.';
comment on column symbols.workspace_maturity is
'The maturity of the symbol''s workspace, under the same cardinality-1 rule applied to the DISTINCT maturity set: where a repository''s two workspaces agree on maturity the column has one true answer and keeps it, so it does not go NULL alongside `workspace`. Measured 2026-10-02 all 7 shared folders holding symbols agree (`discovery`), and a row-by-row diff across the migration found this column changed on 0 of 737,980 rows.';
