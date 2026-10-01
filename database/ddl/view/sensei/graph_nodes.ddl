set search_path to sensei, extensions;

-- Every node with its locality and its parent — the dimension `call_graph` lacks.
--
-- ## Why this exists
--
-- Until now the only way to ask "is this external?" was to ask "did the edge fail
-- to resolve?" — the proxy in the since-deleted `build_connections`. It was wrong in BOTH
-- directions. Measured 2026-09-01 on the live DB: of the 1,040 entries it wrote
-- into `folders.props.libs` for the `sensei` folder, **791 were this repo's own
-- code** — `crate::log_collector::LogCollector`, `./WizardRail.svelte`, `$lib/nav`.
-- rokkit read 807 of 890 (91%), OmniRoute 4,085 of 5,468. And it would have lost
-- every genuine dependency the moment resolution started working, because
-- `target_id` and `target_name` are mutually exclusive across all 715,757 edges —
-- resolving an edge ERASES the name the count was reading.
--
-- Java folders read 0 false positives purely by accident: Java has no relative
-- import form, so every Java import is an absolute FQN.
--
-- Locality is a property of the NODE, not of an edge's resolution state. Asking
-- the node makes both failure modes structurally impossible.
--
-- ## This is a projection, not a second copy of a rule
--
-- `languages::import_target::classify_import` parses a specifier string (`./x`,
-- `$lib/x`, `node:fs`, `java.util.List`) and exercises judgment; it stays in Rust
-- with ONE owner, because a SQL copy of a judgement rule is how the scan exclusion
-- resolver came to gate the watcher while pruning nothing (see
-- `import_target_counts`).
--
-- This view exercises no judgement. It reports the decision the WRITER already
-- recorded: an external symbol is minted with a `lib·` fqn, and a definition
-- gets a `file_id`. Reading those two cannot drift from the rule that set them,
-- because it IS the rule's output.
--
-- D12: externality was read off `kind in ('lib_symbol','lib_package')` until the
-- fqn prefix replaced it. Kind says WHAT a node is; the fqn prefix says WHERE it
-- came from, and collapsing the two destroyed the real kind on 18,240 rows.
-- Verified equivalent on the pre-wipe graph — internal 354,653 / external 21,928
-- / unknown 18,450 under BOTH rules, no off-diagonal cell.
--
-- ## Three-valued, deliberately
--
-- A boolean would bin every node with neither a `file_id` nor a `lib·` fqn as
-- external, reproducing exactly the false positives above.
-- Those are the unresolved reference stubs (84,396 of them `kind='function'`), and
-- `unknown` makes them countable — which is what turns the invariant "stub count
-- → 0" into a one-line query instead of a research project.
--
-- `nodes.resolved` is deliberately NOT the locality signal: 140,051 `section` rows
-- are `resolved=false` while sitting in real files, so that column answers "did
-- FQN enrichment run", not "where does this live". It is projected for filtering,
-- never for classification.
--
-- ## Hierarchy
--
-- `parent_id` already carries containment (296,744 of 430,977 rows), so a grouping
-- view needs no `contains` edge kind — parent for the bubbles, edges for the
-- lines. `parent_name`/`parent_kind` are surfaced so callers group without a
-- self-join, and are NULL for a top-level node rather than a placeholder.
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
-- `project_id`/`project` no longer read `folders.project_id`. That column was
-- settable per folder, so a repository's folders could name different projects,
-- and here they did: measured 2026-10-01 on the live DB, the `kavach` repository
-- has two folders holding nodes, one stamped `kavach` (1,604 nodes) and one
-- stamped `vite-multi-adapter` (844) — one repository, two answers, neither
-- wrong by any rule the column enforced. Membership now lives in
-- `project_repositories` and `sensei.folder_projects` resolves it.
--
-- `folder_projects` IS MULTI-VALUED, so joining it would change this view's
-- grain: measured 2026-10-01, a plain join yields 847,783 rows against 744,923
-- nodes — 102,860 duplicates. That is not a counting nuisance, it is a defect:
-- `get_callees` reads `LEFT JOIN sensei.graph_nodes gn ON gn.id = cg.target_id`
-- under `LIMIT 100`, so a second row per node both repeats callees and halves
-- the window. The grouping below collapses the set per folder BEFORE the join,
-- so one node stays one row — verified, 744,923 rows and 744,923 distinct ids,
-- the count unchanged.
--
-- THE SCALAR IS DEFINED ONLY WHERE THE SET HAS ONE MEMBER. `cardinality = 1`
-- then element 1 is not a pick — it is the only element. Where a repository
-- serves two projects the scalar is NULL and `projects`/`project_ids` carry
-- both, because choosing one of two true answers and presenting it as *the*
-- project is exactly the drift this migration removes. Measured 2026-10-01: of
-- the 253 repositories (of 475) that belong to any project, 8 serve two (bridge,
-- documentation, employee-portal, kavach, mycm.net, policy-management,
-- springboot-starter, zig); 6 of those hold nodes, so 102,860 of 744,923 nodes
-- (13.8%) read a NULL scalar and a two-element set. The set is the better
-- answer, not a consolation: `'client-h' = any(projects)` finds 383,881 nodes
-- where `project = 'client-h'` found 288,181, because the 95,700 in client-h's
-- policy-management, mycm.net and employee-portal repositories were stamped
-- with the other project name.
--
-- A GROUPED SUBQUERY, NOT A JOIN, AND THE REASON IS THE COLLAPSE LIMIT. As this
-- view stood before the change it flattened to exactly 8 base relations
-- (`nodes`, `node_paths`'s three, `folders`, `projects`, `repositories`, the
-- parent `nodes`), and `join_collapse_limit` is 8 (checked, not assumed:
-- `current_setting` reads 8 on this database). Joining `folder_projects`
-- adds three more; measured 2026-10-01, the planner then stops searching join
-- orders and hash-joins a Seq Scan of all 744,923 nodes —
-- `where project = 'sensei'` took 10.3 s. The same statement with
-- `join_collapse_limit = 20` planned at 1.99 s on the indexed path, which is
-- what proves the limit, not the predicate, is the obstacle. An aggregate
-- blocks sub-query pull-up, so the grouping below counts as ONE relation and
-- the view stays at 8.
--
-- Its price, and ONLY WHEN A PROJECT COLUMN IS SELECTED. `group by folder_id`
-- makes the sub-query provably unique on the join key, so Postgres drops the
-- join outright when nothing reads it — and the two shapes the daemon actually
-- runs read `locality`, never `project`.
--
-- `get_callees` is the one that has to be measured rather than reasoned about,
-- because it is the shape an earlier draft of this view broke. Running
-- `graph.rs`'s statement verbatim (folder b08ef286, source_name 'POST',
-- LIMIT 100): 866 ms / 716 ms against the `folders.project_id` form this
-- replaces, 964 ms against this one, with no sequential scan in either plan.
-- The same band.
--
-- WRITE THE SUB-QUERY INLINE IN THE FROM CLAUSE, NEVER AS A CTE. Spelled
-- `with folder_project_set as (...)` the identical SQL made that statement
-- 12,055 ms — the planner stopped parameterising this side by `cg.target_id`
-- and hashed the whole view against the 132-row `call_graph` side, adding three
-- sequential scans. See the comment on the join below.
--
-- Deliberately NOT measured into a number here: warm wall-clock on the
-- `where project = 'sensei'` shape. `sensei.folders` is 81 MB against a 128 MB
-- shared_buffers, so a scan of it uses the bulk-read ring and its pages are
-- never retained — EXPLAIN (BUFFERS) shows ~10.3k blocks re-read on every
-- repetition. There is no stable "warm" figure to quote, and an earlier draft
-- of this comment quoted three that could not be reproduced.
--
-- Resolving from `project_repositories` keyed by `repository_id` instead kept a
-- Bitmap Index Scan on `folders_repository_id_idx` — but it restates the
-- folder->project rule `folder_projects` owns, and a second copy of that rule
-- is how `folders.project_id` drifted in the first place.
--
-- ## Repository
--
-- `repository_id`/`repository` come from a DIRECT join on `folders.repository_id`
-- — no second resolver. `sensei.repo_anchor_for(path)` exists and is the
-- canonical resolver for an arbitrary PATH, but a node already carries a
-- `folder_id`, so walking a path back to an anchor here would be a second way to
-- answer a question the column already answers, and the two could disagree.
--
-- A node whose folder carries no `repository_id` reads NULL. That is honest-empty,
-- not a gap being masked: it means the folder genuinely has no repository, and the
-- NULL bucket is the query that finds folders needing attribution. Measured
-- 2026-09-23: 444,658 of 467,707 nodes (95.1%) resolve to a repository. The
-- 1.4% figure that folder-grain counting produces is the WRONG DENOMINATOR —
-- 12,371 of the 13,035 folders are plain `folder` kind holding no nodes at all,
-- while 183 of 193 `git` folders (which is where nodes live) are attributed.
--
-- Dropped before create: `repository_id`/`repository` sit BESIDE `project`, not
-- appended at the end, and `create or replace view` can only add columns to the
-- tail. Grouping the identity columns together is worth a drop — nothing depends
-- on this view (checked 2026-09-23 via pg_depend), so there is no cascade.
--
-- THAT DROP IS NOW GONE, because the premise expired. `abstractions` was built
-- on this view after it was written, so the bare `drop view if exists` fails:
-- run 2026-10-01 inside a transaction it raised `cannot drop view graph_nodes
-- because other objects depend on it — view abstractions depends on view
-- graph_nodes`, which would have failed the whole apply. Nothing here needs a
-- drop any more: every existing column keeps its name, type and position, and
-- `project_ids`/`projects` are APPENDED at the tail, which is the one thing
-- `create or replace view` does allow. The identity columns stay grouped where
-- the drop put them; the set-valued pair reads at the end instead of beside
-- them, and that is the price of not cascading into another view's definition.

create or replace view graph_nodes as
select n.id
     , n.folder_id
     , f.name         as folder
     , f.branch       as branch
     -- Element 1 of a ONE-element set is that element, not a choice among
     -- candidates; `cardinality <> 1` has no single answer, so it has none.
     , case when cardinality(ps.project_ids) = 1 then ps.project_ids[1] end as project_id
     , case when cardinality(ps.projects)    = 1 then ps.projects[1]    end as project
     , f.repository_id
     , r.name         as repository
     , n.kind::text   as kind
     , n.name
     , n.fqn
     , n.language
     , np.file_path
     , n.line_start
     , n.resolved
     , n.is_exported
     , n.is_test
     , n.community_id
     , case
         -- An external symbol is minted under a `lib·` fqn (R10.7d). This is
         -- the ONE discriminator — see D12.
         when n.fqn like 'lib·%'      then 'external'
         -- A local file is the definition of internal.
         when n.file_id is not null   then 'internal'
         -- Neither: an unresolved reference stub. Saying "external" here is the
         -- bug this view replaces.
         else                                              'unknown'
       end            as locality
     , n.parent_id
     , par.name       as parent_name
     , par.kind::text as parent_kind
     , ps.project_ids
     , ps.projects
  from nodes         n
  left join node_paths np on np.node_id = n.id
  join folders       f
    on f.id          = n.folder_id
  -- INLINE, NOT A CTE, and the spelling is the whole difference. One row per
  -- folder, grouped HERE — before anything joins a node — so the multi-valued
  -- source cannot change this view's grain.
  --
  -- Written as `with folder_project_set as (...)` this view made `get_callees`
  -- 20-30x slower (704 ms -> 12,055 ms, reproduced 5/5): the planner stopped
  -- parameterising the graph_nodes side by `cg.target_id` and hashed the ENTIRE
  -- view against the 132-row call_graph side, adding three sequential scans
  -- (nodes twice at 744,923 rows, folders at 13,722). Identical SQL in the FROM
  -- clause plans at 717 ms with none of them, and returns byte-identical output
  -- (744,923 rows, 744,923 distinct ids, 102,860 NULL scalars). The array
  -- columns are innocent — deleting them leaves the CTE form just as slow.
  left join (
    select fp.folder_id
         , array_agg(fp.project_id order by fp.project, fp.project_id) as project_ids
         , array_agg(fp.project    order by fp.project, fp.project_id) as projects
      from folder_projects fp
     group by fp.folder_id
  ) ps
    on ps.folder_id  = f.id
  left join repositories r
    on r.id          = f.repository_id
  left join nodes    par
    on par.id        = n.parent_id;

comment on view graph_nodes is
'Every node with its LOCALITY (internal | external | unknown) and its parent.

locality is read from what the writer recorded — a `lib·` fqn => external; a
non-null file_id => internal; neither => unknown (an unresolved reference stub). It is NOT derived from an edge failing to resolve, which is the
proxy that reported 791 of 1,040 of this repo''s own modules as dependencies.
It is NOT derived from nodes.resolved either: 140,051 section rows are
resolved=false while sitting in real files.

Three-valued on purpose: a boolean bins the 85,530 stubs as external. `unknown`
makes them countable, so "stub count -> 0" is a query.

parent_id/parent_name/parent_kind carry containment, so grouping needs no
`contains` edge kind: parent for the nesting, edges for the connections.

PROJECT MEMBERSHIP IS THE REPOSITORY''S, AND IT IS A SET. It comes from
`sensei.folder_projects`, never from `folders.project_id` — that column was
settable per folder, and on 2026-10-01 the `kavach` repository had one folder
stamped `kavach` (1,604 nodes) and another stamped `vite-multi-adapter` (844).
`projects`/`project_ids` carry every project; `project`/`project_id` are the
SOLE member of that set, and NULL when there is more than one, because naming
one of two true answers is the drift this replaces. Measured 2026-10-01: of the
253 repositories (of 475) that belong to any project, 8 serve two each and 6 of
those hold nodes, so 102,860 of 744,923 nodes (13.8%) read a NULL scalar and a
two-element set.

ONE ROW PER NODE, STILL. The set is collapsed per folder before any node joins
it. Verified 2026-10-01: 744,923 rows and 744,923 distinct ids, unchanged by the
migration, where a plain join to `folder_projects` would have returned 847,783.

Common queries:
  -- every node of a project — the one that reads a multi-project repo right
  SELECT count(*) FROM sensei.graph_nodes WHERE ''client-h'' = ANY(projects)
  -- 383,881, against the 288,181 that `project = ''client-h''` used to find

  -- dependency count, correct where folders.props.libs was not
  SELECT count(DISTINCT n.name) FROM sensei.edges e
    JOIN sensei.graph_nodes n ON n.id = e.target_id
   WHERE e.folder_id = ''...'' AND n.locality = ''external''

  -- the invariant the identity fix has to drive to zero
  SELECT folder, count(*) FROM sensei.graph_nodes
   WHERE locality = ''unknown'' GROUP BY folder ORDER BY 2 DESC

  -- patterns vs graph: pick the relation kinds, group by the hierarchy
  SELECT n.parent_name, n.kind, count(*) FROM sensei.graph_nodes n
   WHERE n.project = ''sensei'' AND n.locality = ''internal'' GROUP BY 1, 2';

comment on column graph_nodes.projects is 'Every project this node''s repository belongs to, from `sensei.folder_projects` — set-valued because a repository is keyed on its REMOTE and can serve more than one project. Filter with `''name'' = ANY(projects)`; that is the spelling that reads a multi-project repository correctly (measured 2026-10-01: 383,881 nodes for client-h, against 288,181 under the old per-folder column). NULL has TWO causes, not one: the folder belongs to no tracked repository, OR its repository belongs to no project. Measured 2026-10-01 both are currently empty — 0 folders carry a NULL `repository_id` — but 222 of 475 repositories sit in no project, so the second cause becomes real the moment one of them acquires a folder holding nodes. Either way the join finds nothing rather than inventing a project, and the LEFT JOIN is what keeps such a node visible instead of dropping it from the view.';
comment on column graph_nodes.project_ids is 'The `projects` set by id, in the same order — the join key for `sensei.projects` without a second name lookup.';
comment on column graph_nodes.project is 'The project, ONLY where the repository serves exactly one — the sole member of `projects`, not a choice among candidates. NULL means the set has more than one member (102,860 of 744,923 nodes on 2026-10-01), never that the node has no project: read `projects` for those. It is NOT `folders.project_id`, which let two folders of one repository name two different projects.';
comment on column graph_nodes.project_id is 'The id of `project`, under the same rule: the sole member of `project_ids`, or NULL where there is more than one.';
comment on column graph_nodes.repository is 'The repository this node belongs to, via folders.repository_id — a direct column read, never a second path-walking resolver that could disagree with it. NULL means the folder carries no repository (honest-empty, and the query that finds what needs attributing), never a failed lookup. 95.1% of nodes resolve; count at NODE grain, not folder grain — plain folders hold no nodes and drag a folder-grain percentage to a meaningless 1.4%.';
comment on column graph_nodes.branch is 'The checked-out branch of this node''s folder. A real partition key, not a label: the design is one folder per checkout (develop vs main = two folders, one repository), already live for fitness/strategos/website — so filtering by branch separates genuine graphs without branch appearing in node identity.';
comment on column graph_nodes.locality is 'internal (has a file_id) | external (a `lib·` fqn — the writer said so) | unknown (unresolved reference stub). Never inferred from an edge''s resolution state, and never collapsed to a boolean: that bins every stub as external.';
comment on column graph_nodes.parent_name is 'Containing node''s name — NULL at top level, not a placeholder. Lets a caller group by container without a self-join.';
comment on column graph_nodes.resolved is 'Whether FQN enrichment ran. Projected for filtering; NOT a locality signal — 140,051 section rows are resolved=false inside real files.';
