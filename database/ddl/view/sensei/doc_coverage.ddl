set search_path to sensei, extensions;

-- Doc↔code traceability by filename-stem proximity, with drift detection.
--
-- ## Why this computes the pairing instead of reading stored edges
--
-- The pairing used to be built by the `build_connections` task, written into
-- `sensei.edges` as 601 `covers` rows, and then read back by THIS view — which
-- joined those rows to the very nodes they were derived from. The task's own
-- comment conceded what that made it: "covers becomes a pure function of the
-- current (docs, files) — idempotent".
--
-- A pure function of current stored state, recomputed wholesale on every index, is
-- a view that got stored. Storing it created a second writer of a fact the
-- database already held, and second writers drift — the same shape as
-- `folders.props.libs` (retired in 796a56a9), `nodes.degree` (56 of 430,988 rows
-- measurably stale) and `communities.god_node_ids`.
--
-- It also could never have been filled at insert time, which is the usual answer
-- to "don't store what the writer can compute": when `docs/api/auth.md` is
-- processed, `src/api/auth.ts` may not be indexed yet. The pairing is inherently
-- cross-file, so it is neither an insert-time fact nor barrier-worthy work.
--
-- ## The stem rule lives HERE now, not in two places
--
-- Matching by filename stem is a judgement heuristic, and this codebase's rule is
-- that judgement rules stay in Rust with one owner — a SQL *copy* of one is how
-- the scan exclusion resolver came to gate the watcher while pruning nothing. This
-- is a MOVE, not a copy: the `Path::file_stem()` call in `build_connections` is
-- gone, so exactly one implementation exists and there is nothing to drift from.
--
-- The repo-relative reconstruction is `sensei.repo_relative_path()`, the one
-- owner of that rule — this view held a hand-copy of it, as did three others.
--
-- The stem expression below reproduces `Path::file_stem()` exactly — verified against
-- all 45,186 distinct `file_path` values in the live DB, zero disagreements,
-- including the leading-dot case (`.gitignore` is entirely its own stem) and the
-- double-extension case (`appstate.svelte.ts` → `appstate.svelte`).
--
-- `sensei.edge_kind` keeps its `covers` value: nothing produces it now, but
-- dropping an enum member requires recreating the type, and the value remains
-- valid for a future writer.
--
-- Dropped rather than replaced because the old shape carried an `edge_id` column
-- that no longer has a source — and no consumer ever read it (`get_doc_drift`
-- selects doc_name, doc_file, code_name, code_file, drifted). No other view
-- depends on this one — re-checked via pg_depend 2026-10-02, still 0 rows, so
-- the drop still cascades into nothing.
--
-- ## Workspace comes from `folder_workspaces`, not from `folders.workspace_id` (#211)
--
-- Membership belongs to the REPOSITORY (`repositories_in_workspaces`); the per-folder
-- column is being retired. Measured 2026-10-02 over all 13,724 folders, the two
-- resolutions never disagree on a name: the old scalar is a member of the new
-- set for 13,724 of 13,724, and 9,595 folders resolve to exactly one workspace.
--
-- ONE ROW PER (doc, code) PAIR, STILL. `folder_workspaces` is multi-valued — 8
-- repositories serve two workspaces each, covering 4,129 of those 13,724 folders
-- — so a plain join would emit the same pair twice. That is not a counting
-- nuisance here: `get_doc_drift` reads this view under `WHERE folder = $1 ...
-- LIMIT 200` and never selects a workspace column, so a second row would repeat
-- every pair and halve its window. The sub-select below groups per folder
-- BEFORE the join, so the grain is untouched.
--
-- INLINE, NOT A CTE. Spelling the identical SQL as `with folder_workspace_set as
-- (...)` is what cost `get_callees` over `graph_nodes` an order of magnitude: the
-- planner stopped parameterising the view by the consumer's key and hashed the
-- whole thing. Same sub-select, same place in the FROM clause, same reason.
--
-- THE SCALAR IS DEFINED ONLY WHERE THE SET HAS ONE MEMBER, as in `graph_nodes`.
-- Element 1 of a one-element set is that element, not a pick; where a repository
-- serves two workspaces the scalar is NULL and `workspaces`/`workspace_ids` carry
-- both, because naming one of two true answers is the drift #211 removes.
-- `workspace_maturity` is gated on the same condition and read at the same index,
-- so it cannot describe a different workspace from the one `workspace` names.
--
-- The grouping makes the sub-select provably unique on `folder_id`, so Postgres
-- drops the left join outright when nothing reads a workspace column — which is
-- exactly the shape `get_doc_drift` runs. Verified 2026-10-02 at
-- `max_parallel_workers_per_gather = 0`: that plan is node-for-node identical to
-- the one this replaces, with no trace of the sub-select in it.
drop view if exists doc_coverage;

create view doc_coverage as
with stems as (
  select n.id
       , n.folder_id
       , n.kind
       , n.name
       , np.file_path
       , n.modified_at
         -- Path::file_stem(): the file name minus its final extension; a name that
         -- begins with `.` and has no other dot is entirely the stem.
       , case
           when position('.' in substring(regexp_replace(np.file_path, '^.*/', '') from 2)) = 0
           then regexp_replace(np.file_path, '^.*/', '')
           else substring(
                  regexp_replace(np.file_path, '^.*/', '') from 1
                  for length(regexp_replace(np.file_path, '^.*/', ''))
                    - position('.' in reverse(regexp_replace(np.file_path, '^.*/', '')))
                )
         end as stem
    from nodes n
    -- `node_paths` owns the files/folders join and the grain rule.
    join node_paths np on np.node_id = n.id
   where np.file_path is not null
     and n.kind in ('doc', 'file')
)
select d.folder_id
     , f.name            as folder
     , case when cardinality(ps.workspace_ids) = 1 then ps.workspace_ids[1] end as workspace_id
     , case when cardinality(ps.workspaces)    = 1 then ps.workspaces[1]    end as workspace
     , case when cardinality(ps.workspace_ids) = 1 then ps.maturities[1]  end as workspace_maturity
     , ps.workspace_ids
     , ps.workspaces
     , d.id              as doc_id
     , d.name            as doc_name
     , d.file_path       as doc_file
     , d.modified_at     as doc_modified
     , code.id           as code_id
     , code.name         as code_name
     , code.file_path    as code_file
     , code.modified_at  as code_modified
     , (code.modified_at > d.modified_at) as drifted
  from stems             d
  join stems             code
    on code.folder_id    = d.folder_id
   and code.kind         = 'file'
   and code.stem         = d.stem
   and code.file_path   <> d.file_path
  join folders           f
    on f.id              = d.folder_id
  -- One row per folder, grouped HERE — before the pair joins it — so the
  -- multi-valued source cannot change this view's grain. Inline, never a CTE.
  --
  -- `maturity` is aggregated HERE, in the same order as the other two, rather
  -- than from an outer `join workspaces on p.id = <the sole member>`. That outer
  -- form is correct but COSTS THE REMOVAL: measured 2026-10-02, its join clause
  -- keeps a reference to `ps.workspace_ids` alive, so Postgres drops `workspaces`
  -- and then cannot drop `ps`, and `get_doc_drift` — which selects no workspace
  -- column at all — acquires a Seq Scan on all 13,724 folders plus a
  -- GroupAggregate and a Merge Left Join that its old plan did not have.
  -- Aggregating the third column leaves ONE added relation, and it goes.
  left join (
    select fp.folder_id
         , array_agg(fp.workspace_id order by fp.workspace, fp.workspace_id) as workspace_ids
         , array_agg(fp.workspace  order by fp.workspace, fp.workspace_id) as workspaces
         , array_agg(pj.maturity   order by fp.workspace, fp.workspace_id) as maturities
      from folder_workspaces fp
      left join workspaces pj on pj.id = fp.workspace_id
     group by fp.folder_id
  ) ps
    on ps.folder_id      = d.folder_id
 where d.kind            = 'doc';

comment on view doc_coverage is
'Doc-to-code traceability by filename-stem proximity, with drift detection.
drifted = true when the code was modified more recently than the covering doc.

Computes the pairing rather than reading stored `covers` edges: it is a pure
function of the current (docs, files), so storing it meant a second writer of a
fact the DB already held. The stem expression reproduces Rust Path::file_stem()
exactly — verified against all 45,186 distinct file_path values in the live DB.

WORKSPACE MEMBERSHIP IS THE REPOSITORY''S, AND IT IS A SET. It comes from
`sensei.folder_workspaces`, never from `folders.workspace_id` (#211).
`workspaces`/`workspace_ids` carry every workspace; `workspace`/`workspace_id`/
`workspace_maturity` describe the SOLE member of that set and are NULL when there
is more than one, because naming one of two true answers is the drift this
replaces. Measured 2026-10-02: 8 repositories serve two workspaces each, covering
4,129 of 13,724 folders; the other 9,595 resolve to exactly one workspace, and the
old scalar is a member of the new set on all 13,724.

ONE ROW PER (doc, code) PAIR, STILL. The set is collapsed per folder before the
pair joins it, so the multi-valued source adds no row.

Filter/group dimensions: workspaces, workspace, workspace_maturity, folder, drifted.

Common queries:
  SELECT doc_file, code_file FROM doc_coverage WHERE folder = ''myrepo'' AND drifted
  -- `= ANY(workspaces)` is the form that reads a multi-workspace repository right;
  -- `workspace = ...` silently skips every folder whose repository serves two.
  SELECT folder, count(*) FILTER (WHERE drifted) as drifted FROM doc_coverage WHERE ''sensei'' = ANY(workspaces) GROUP BY folder
  SELECT p, count(*) as covered, count(*) FILTER (WHERE drifted) as drifted FROM doc_coverage, unnest(workspaces) p GROUP BY p';
comment on column doc_coverage.drifted is 'Code modified more recently than the doc that covers it.';
comment on column doc_coverage.workspaces is 'Every workspace this pair''s repository belongs to, from `sensei.folder_workspaces` — set-valued because a repository is keyed on its REMOTE and can serve more than one. Filter with `''name'' = ANY(workspaces)`. NULL means the folder''s repository belongs to no workspace; the LEFT JOIN is what keeps the pair visible instead of dropping it.';
comment on column doc_coverage.workspace_ids is 'The `workspaces` set by id, in the same order — the join key for `sensei.workspaces` without a second name lookup.';
comment on column doc_coverage.workspace is 'The workspace, ONLY where the repository serves exactly one — the sole member of `workspaces`, not a choice among candidates. NULL means the set has more than one member, never that the pair has no workspace: read `workspaces` for those.';
comment on column doc_coverage.workspace_id is 'The id of `workspace`, under the same rule: the sole member of `workspace_ids`, or NULL where there is more than one.';
comment on column doc_coverage.workspace_maturity is 'Maturity of `workspace`, read at the same index of the same per-folder aggregate — so it can never describe a workspace other than the one `workspace` names. NULL under the same rule.';
comment on column doc_coverage.code_file is 'A file sharing the doc''s filename stem — docs/api/auth.md pairs with src/api/auth.ts.';
