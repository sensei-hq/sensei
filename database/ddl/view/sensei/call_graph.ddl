set search_path to sensei, extensions;

-- ## Project — membership is the REPOSITORY's, and it is a SET
--
-- `project_id`/`project` no longer read `folders.project_id` (#211). Membership
-- lives in `sensei.project_repositories` and `sensei.folder_projects` resolves
-- it, exactly as in `graph_nodes`.
--
-- `folder_projects` IS MULTI-VALUED — a repository is keyed on its REMOTE, so
-- two checkouts collapse to one row and 8 repositories serve two projects each.
-- A plain join would therefore change this view's GRAIN, and that is a defect
-- here and not a counting nuisance: `get_callers`/`get_callees` read this view
-- under a LIMIT, so a second row per edge both repeats a caller and halves the
-- window. The sub-query below collapses the set per folder BEFORE anything
-- joins an edge, so one edge stays one row.
--
-- THE SCALAR IS DEFINED ONLY WHERE THE SET HAS ONE MEMBER. `count(*) = 1` then
-- element 1 is not a pick, it is the only element; where a repository serves two
-- projects the scalar is NULL and `project_ids`/`projects` carry both. Naming
-- one of two true answers as *the* project is the drift this migration removes.
--
-- Measured 2026-10-02 on the live DB, old against new inside ONE
-- `repeatable read` transaction — the daemon indexes while this runs, and a
-- `read committed` pass moved the baseline by 6 edges between the two snapshots.
-- Of 3,179,206 rows the symmetric difference is 478,965 each way, and it
-- decomposes into exactly two buckets with NO residue:
--
--   2,700,242 (64 folders)  identical — same project_id as before
--     436,683 (7 folders)   the multi-project population: scalar NULL, and
--                           `projects` carries both. `project_maturity` is NULL
--                           on these same rows and no others.
--      42,282 (5 folders)   `folders.project_id` and `project_repositories`
--                           DISAGREE, and the new answer is the right one.
--           0               no folder lacks a `project_repositories` row.
--
-- THAT THIRD BUCKET APPEARED WHILE THIS MIGRATION WAS BEING WRITTEN, which is
-- the argument for #211 making itself. A first pass at 09:12 measured it empty.
-- At 09:40 the daemon re-stamped five folders — `sanctioncheck.net`,
-- `disclosurereport`, `mcr.net`, `template-builder`, `WorkflowEngine` — with one
-- umbrella `folders.project_id` of `client-h`, while each of those folders carries
-- its OWN repository and `project_repositories` names each repository's own
-- project. One settable column overwrote five true answers with one in 28
-- minutes. The new resolution reports each repository's project; it does not
-- preserve the stamp, and preserving it is not the goal.
--
-- THE GRAIN IS UNCHANGED: 3,179,206 rows and 3,179,206 distinct `edge_id`, both
-- before and after. A snapshot for that comparison must be a TEMP TABLE, not a
-- temp view: `create or replace view` keeps the same OID, so a view-based
-- snapshot silently follows the new definition and the check passes vacuously.
--
-- WRITE THE SUB-QUERY INLINE IN THE FROM CLAUSE, NEVER AS A CTE. Spelled
-- `with folder_project_set as (...)` the identical SQL made `get_callees`
-- an order of magnitude in `graph_nodes`, because the planner stopped
-- parameterising the view by the consumer's key and hashed the whole thing,
-- adding three sequential scans.
--
-- An aggregate blocks sub-query pull-up, so the grouping counts as ONE relation
-- and this view stays at 8 base relations — `join_collapse_limit` reads 8 on
-- this database (checked, not assumed). Joining `folder_projects` raw would add
-- three and push it over, which is where `graph_nodes` measured the planner
-- abandoning its join-order search.
--
-- Its price, and ONLY WHEN A PROJECT COLUMN IS SELECTED. `group by folder_id`
-- makes the sub-query provably unique on the join key, so Postgres drops the
-- join outright when nothing reads it — and the shapes the daemon actually runs
-- read `folder_id`, `target_symbol` and `source_name`, never `project`. Verified
-- 2026-10-02 at `max_parallel_workers_per_gather = 0`: `get_callees`
-- (graph.rs:2630, the shape the CTE form broke) plans as nested loops over
-- `nodes_name_idx` -> `edges_source_id_idx` -> three pkey lookups, with the
-- grouped sub-query absent from the plan entirely and not one sequential scan.
--
-- `where project = 'sensei'` keeps the SAME downstream access path as the
-- `folders.project_id` form it replaces — `edges_folder_id_idx` driven once per
-- folder, then `nodes_pkey` — and returns the same 418 folders and 69,874 calls
-- edges. What it pays for is the folder set: the old form got there by a Bitmap
-- Index Scan on `folders_project_id_idx`, the new one aggregates a Seq Scan of
-- all 13,724 folders. Deliberately NOT quoted here: wall clock. `sensei.folders`
-- is 81 MB against a 128 MB shared_buffers, so that scan rides the bulk-read ring
-- and retains nothing; repeated identical runs vary by more than an order of
-- magnitude.

create or replace view call_graph as
select e.id              as edge_id
     , e.folder_id
     , f.name            as folder
     , ps.project_id
     , p.name            as project
     , p.maturity        as project_maturity
     , e.kind            as edge_kind
     , e.confidence
     , e.confidence_score
     , e.source_id
     , src.name          as source_name
     , src.kind          as source_kind
     , srcf.file_path    as source_file
     , src.line_start    as source_line
     , src.is_exported   as source_exported
     , e.target_id
     , tgt.name          as target_name
     , tgt.kind          as target_kind
     , tgtf.file_path    as target_file
     , tgt.line_start    as target_line
     , tgt.is_exported   as target_exported
     , e.target_name     as unresolved_target
     , e.props
     -- The target's symbol name HOWEVER it is recorded — the one column a
     -- "who calls X" filter should ever use. `target_name` above is `tgt.name`,
     -- which is NULL for an unresolved edge because `tgt` is a LEFT JOIN; the
     -- name is in `e.target_name` instead. Filtering the resolved-only column
     -- silently dropped 117,201 of 335,756 `calls` edges (34.9%) and made
     -- get_callers return [] for 8,680 symbol names that demonstrably have
     -- callers. Coalescing HERE means a caller cannot pick the wrong column.
     -- Appended last on purpose: `create or replace view` may add columns only
     -- at the end, so inserting this beside the other target_* columns would
     -- make the replace fail against an existing database.
     , coalesce(tgt.name, e.target_name) as target_symbol
     -- WHY an unresolved edge is unresolved, from the walk that could not
     -- place it. The vocabulary is sensei.reason_codes under domain
     -- `code_graph`, and Reason::as_label on the Rust side is the one producer
     -- of these strings. Written by the indexer PER USE — see the reduction
     -- below; this comment used to say `props.reason` and that was never a
     -- writer path.
     --
     -- A named column rather than every consumer spelling `props->>'reason'`
     -- for itself. Appended last for the reason target_symbol was: `create or
     -- replace view` may only add columns at the end.
     --
     -- READ FROM THE OCCURRENCES, which is where the indexer actually puts it.
     -- This column used to be `e.props->>'reason'` alone, and the comment above
     -- said the indexer writes `props.reason` — it does not.
     -- `persist::with_outcome` is called from `occurrence_prop`, so the verdict
     -- lands on each USE, nested under the file that made it:
     --   props.occurrences["src/a.rs"][0].reason
     -- There is no writer path that produces a top-level key. The result was
     -- this column reading NULL on all 2,428,016 unresolved edges, so not one
     -- production miss could be classified, in any language — while the tests
     -- stayed green because they seed the top-level shape by hand.
     --
     -- AN EDGE AGGREGATES MANY USES, so this is a REDUCTION and the rule is
     -- LOWEST PRECEDENCE WINS. `reason_codes.precedence` is severity order, so
     -- an edge whose uses are `plumbing` (a deliberate refusal, 90) and
     -- `receiver_type_unknown` (a fault, 30) reports the FAULT: a real gap must
     -- not be hidden by a filter that happens to share the edge.
     --
     -- THREE SOURCES, in this order, and each later one is a fallback for rows
     -- the earlier one does not cover:
     --
     --   1. `e.unresolved_reason` — the stored column, re-derived by
     --      merge_edge_occurrences / drop_edge_occurrences on every write.
     --   2. `props->>'reason'` — an explicit edge-level verdict, which is what
     --      the view tests seed by hand.
     --   3. `sensei.edge_verdict(...)` — the reduction over the per-use
     --      verdicts, for the 3.1M rows written before the column existed.
     --
     -- (3) is what keeps this view answering on today's data with no re-index;
     -- it can go once every folder has been re-indexed, and not before.
     , coalesce(
         e.unresolved_reason,
         e.props->>'reason',
         sensei.edge_verdict(e.props -> 'occurrences', 'code_graph', 'reason')
       )                 as unresolved_reason
     , src.language       as source_language
     -- WHICH RUNG of the resolution ladder placed a resolved edge. The
     -- sibling of unresolved_reason above: that one says why the ladder
     -- stopped, this one says how it got there. `declared_here` is a file
     -- pointing at its own declaration; `through_a_glob` is a name the source
     -- never wrote down. Same `target_id`, very different evidence.
     --
     -- Vocabulary: sensei.reason_codes under domain `code_graph_rung`, whose
     -- precedence is CLIMB ORDER. Written by the indexer into `props.rung`;
     -- Rung::as_label on the Rust side is the one producer of these strings. Same
     -- three-source chain as unresolved_reason above.
     --
     -- Same nesting and the same reduction as unresolved_reason above: the rung
     -- is written per USE, and precedence here is CLIMB ORDER, so the strongest
     -- proof sorts first. An edge placed `declared_here` by one use and
     -- `in_the_prelude` by another is reported on the stronger evidence.
     , coalesce(
         e.resolved_via,
         e.props->>'rung',
         sensei.edge_verdict(e.props -> 'occurrences', 'code_graph_rung', 'rung')
       )                 as resolved_via
     -- Appended at the tail because `create or replace view` may only add
     -- columns there. Filter a multi-project repository with
     -- `'name' = any(projects)`; the scalar above cannot see it.
     , ps.project_ids
     , ps.projects
  from edges         e
  join folders       f
    on f.id          = e.folder_id
  -- INLINE, NOT A CTE — see the note above; the spelling is the whole
  -- difference. One row per folder, grouped HERE, before any edge joins it.
  left join (
    select fp.folder_id
         , case when count(*) = 1
                then (array_agg(fp.project_id))[1]
           end                                                     as project_id
         , array_agg(fp.project_id order by fp.project, fp.project_id) as project_ids
         , array_agg(fp.project    order by fp.project, fp.project_id) as projects
      from folder_projects fp
     group by fp.folder_id
  ) ps
    on ps.folder_id  = f.id
  left join projects p
    on p.id          = ps.project_id
  join nodes         src
    on src.id        = e.source_id
  left join files    srcf
    on srcf.id       = src.file_id
  left join nodes    tgt
    on tgt.id        = e.target_id
  left join files    tgtf
    on tgtf.id       = tgt.file_id;

comment on view call_graph is
'Resolved and unresolved edges with source/target symbol details and project context.
LEFT JOIN on target — unresolved edges have target columns null but unresolved_target set.

FILTER A TARGET BY `target_symbol`, NEVER BY `target_name`. The source side is an
inner join so source_name is always present; the TARGET side is a left join, so
`target_name` (= tgt.name) is NULL for every unresolved edge and the name lives in
`unresolved_target`. `target_symbol` coalesces the two. This comment previously
recommended `target_name = ''handleAuth''` as the canonical example, and that query
is the bug: it silently dropped 117,201 of 335,756 calls edges (34.9%), so
get_callers returned an empty list for 8,680 symbol names that had callers. Use
target_name/unresolved_target only to ask WHICH of the two a row is — i.e. to report
resolution coverage — not to find a symbol.

PROJECT MEMBERSHIP IS THE REPOSITORY''S, AND IT IS A SET. It comes from
`sensei.folder_projects`, never from `folders.project_id` (#211). `projects`/
`project_ids` carry every project; `project`/`project_id` are the SOLE member of
that set, and NULL when there is more than one, because naming one of two true
answers is the drift this replaces. Measured 2026-10-02: 436,683 of 3,179,206
rows (13.7%) sit on a repository serving two projects, and a further 42,282 now
report a DIFFERENT project because `folders.project_id` had stamped five
single-repository folders with one umbrella name their repositories do not have.
ONE ROW PER EDGE, STILL — the set is collapsed per folder before any edge joins
it, verified 3,179,206 rows and 3,179,206 distinct edge_id, unchanged.

Filter/group dimensions: projects, project, project_maturity, folder, edge_kind,
confidence, source_name, target_symbol.

Common queries:
  -- who calls X (resolved AND unresolved, which is what a caller lookup means)
  SELECT source_name, source_file FROM call_graph WHERE folder = ''myrepo'' AND target_symbol = ''handleAuth'' AND edge_kind = ''calls''
  -- resolution coverage, the number that tells you how much to trust the above
  SELECT count(target_id) AS resolved, count(*) - count(target_id) AS unresolved FROM call_graph WHERE project = ''sensei'' AND edge_kind = ''calls''
  SELECT edge_kind::text, count(*) FROM call_graph WHERE project = ''sensei'' GROUP BY edge_kind
  SELECT folder, count(*) FROM call_graph WHERE project_maturity = ''active'' AND edge_kind = ''calls'' GROUP BY folder';

comment on column call_graph.projects is
'Every project this edge''s repository belongs to, from `sensei.folder_projects` — set-valued because a repository is keyed on its REMOTE and can serve more than one project. Filter with `''name'' = ANY(projects)`; that is the spelling that reads a multi-project repository correctly. NULL means the folder belongs to no tracked repository, or its repository belongs to no project — the join finds nothing rather than inventing a project. Measured 2026-10-02: neither cause currently fires, all 3,179,206 rows resolve to at least one project.';
comment on column call_graph.project_ids is
'The `projects` set by id, in the same order — the join key for `sensei.projects` without a second name lookup.';
comment on column call_graph.project is
'The project, ONLY where the repository serves exactly one — the sole member of `projects`, not a choice among candidates. NULL means the set has more than one member (436,683 of 3,179,206 rows on 2026-10-02), never that the edge has no project: read `projects` for those. It is NOT `folders.project_id`, which let two folders of one repository name two different projects.';
comment on column call_graph.project_id is
'The id of `project`, under the same rule: the sole member of `project_ids`, or NULL where there is more than one.';
comment on column call_graph.project_maturity is
'Maturity of `project`, and NULL under exactly the same rule — a repository serving two projects has no single maturity.';

comment on column call_graph.target_symbol is
'The target symbol name however recorded: tgt.name when the edge resolved, e.target_name when it did not. THE column to filter a target on — target_name is resolved-only and silently excludes 34.9% of calls edges.';
