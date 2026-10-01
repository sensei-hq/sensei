set search_path to sensei, extensions;

create or replace view structure_edges as
with e as (
  select fp.project_id
       , fp.project                    as project
       , ed.kind::text                 as kind
       , sn.file_id                    as source_file_id
       , tn.file_id                    as target_file_id
       , split_part(sn.fqn, '·', 2)    as source_package
       , split_part(tn.fqn, '·', 2)    as target_package
       , split_part(sn.fqn, '·', 3)    as source_module
       , split_part(tn.fqn, '·', 3)    as target_module
    from sensei.edges           ed
    join sensei.folder_projects fp on fp.folder_id = ed.folder_id
    join sensei.nodes           sn on sn.id = ed.source_id
    join sensei.nodes           tn on tn.id = ed.target_id
   where ed.target_id  is not null
     and sn.file_id    is not null
     and tn.file_id    is not null
     and sn.fqn        is not null
     and tn.fqn        is not null
     and sn.file_id   <> tn.file_id
)
select e.project
     , e.project_id
     , e.kind
     , e.source_file_id
     , e.target_file_id
     , sf.file_path                    as source_file
     , tf.file_path                    as target_file
     , e.source_package
     , e.target_package
     , e.source_module
     , e.target_module
     , case
         when e.source_package <> e.target_package then 'cross_package'
         when e.source_module  <> e.target_module  then 'cross_module'
         else                                           'in_module'
       end                             as span
     , count(*)                        as occurrences
  from e
  join sensei.files sf on sf.id = e.source_file_id
  join sensei.files tf on tf.id = e.target_file_id
 group by e.project, e.project_id, e.kind, e.source_file_id, e.target_file_id
        , sf.file_path, tf.file_path
        , e.source_package, e.target_package, e.source_module, e.target_module;

comment on view structure_edges is
'File-to-file edges of a project, each labelled with how far it reaches — the
edge set behind the Structure diagram.

`span` IS WHAT THE DIAGRAM IS ABOUT, which is why it is computed here and not in
the client. An edge inside one module follows the hierarchy and bundles tightly;
one that crosses packages cuts across it and is drawn through the centre. Three
consumers — the diagram, the coverage line, and any later DSM or layering work —
would otherwise each re-derive the same CASE, and the first to change it would
silently disagree with the others.

ONLY PLACED EDGES. `target_id IS NULL` means the resolver reached no conclusion,
and drawing those would assert a relationship the index does not have. This is
not a small filter: measured 2026-10-01 on project `sensei`, 31,166 of 69,619
`calls` edges carry a target — 44.8% — so MOST call rows are excluded, and
across every project at once it is 368,423 of 1,280,605, or 28.8%. A diagram
built on this view is a FLOOR, never a full picture, and any screen using it
MUST show what was left out —
`graph_placement` holds that count. A sparse diagram with no coverage line reads
as a simple codebase rather than an unresolved one, and that misreading is the
entire risk.

SELF-FILE EDGES ARE DROPPED (`source_file_id <> target_file_id`). A file calling
itself draws nothing between two rim points, and at file grain it is the largest
population — keeping it would inflate `occurrences` for no visible effect. That
also makes this view the wrong place to ask how much a file does internally;
`structure_graph.symbols` answers that.

THE GRAIN IS (PROJECT, KIND, FILE PAIR, PACKAGE PAIR, MODULE PAIR) — WHICH IS
NOT ONE ROW PER FILE PAIR. The package and module labels are split out of the
symbols'' FQNs, and two symbols in the same file can sit in different modules
(`dojo::client` beside `dojo::client::tests`), so one file-to-file relationship
can surface as SEVERAL rows carrying different `source_module`/`target_module`
and sometimes a different `span`. Measured 2026-10-01: 1,464 (project, kind,
source file, target file) keys carry more than one row. This is not something
the `folder_projects` change introduced — the GROUP BY is untouched, and the
same count against the old `folders.project_id` definition is 1,236 — but
anything drawing ONE line per file pair must `sum(occurrences)` across those
rows rather than read one of them.

PROJECT IS RESOLVED THROUGH `folder_projects`, NEVER FROM `folders.project_id`.
Membership belongs to the REPOSITORY (`project_repositories`), so every folder of
a checkout now answers the same way. The old per-folder column could be set
independently on each folder, and a repository''s folders did disagree: measured
2026-10-01, ONE checkout had 147 of its folders filed under one project and 99
under another — 173 occurrences surfaced under the first name, 85 under the
second — and neither project could see the other part. Both now report the whole
258. Four repositories carry that split today, of 246, 45, 3 and 2 folders.

THAT RESOLUTION IS MULTI-VALUED, AND HERE IT MAKES A SECOND ROW, NOT A DOUBLED
COUNT. A repository is keyed on its REMOTE, so one repository can serve several
projects — measured 2026-10-01, 8 of the 253 repositories in
`project_repositories` serve two projects each, none more, covering 436,683 edge
rows. Each such edge produces one CTE row per project, and `project` AND
`project_id` are BOTH in the GROUP BY above, so those rows fall into SEPARATE
groups: the edge is reported once under each project carrying the SAME
`occurrences`, never once carrying doubled `occurrences`. Two facts make that
structural rather than lucky. A folder carries ONE `repository_id` and
`project_repositories` is keyed (project_id, repository_id), so no folder can
emit two rows with the same project_id and the fan-out can never collapse back
into one group. And it holds against an independent ground truth: re-run
2026-10-01, for each of the 10 projects that own a shared repository and have
any qualifying edge, `sum(occurrences)` equals a fan-out-free count taken
straight off `edges` through `folders.repository_id`, with no project join at
all — 15,080 / 5,346 / 720 / 488 / 258 / 258 / 125 / 33 / 33 / 11, ten matches
out of ten.

WHAT A CALLER MUST HOLD: an aggregate spanning MORE THAN ONE project counts a
shared repository''s edges once per project that owns it, so a cross-project
total needs to say what it is distinct on. Per project — which is how every query
below reads it — the numbers are exact. Switching to `folder_projects` moved the
row count 18,784 -> 21,278 for exactly this reason: every added row is a
(project, edge) pair that the other owner of a shared repository could not see
before. Re-measured 2026-10-01 by materialising both definitions and joining
them on the full group key: 0 of the 18,784 pre-existing rows went missing and 0
changed their `occurrences`, and all 2,494 added rows land in the five projects
that share a repository (2,340 / 62 / 48 / 33 / 11).

THE JOIN IS INNER. An edge in a folder under no tracked repository is dropped
rather than surfaced with a NULL project, because a NULL-project row is unusable
on a view every consumer filters by project, and `folder_projects` deliberately
returns nothing rather than inventing one. Measured 2026-10-01: 0 folders and 0
edges are in that state, so this changed no row today.

JOINED FROM `edges` VIA `folder_id`, WHICH IS THE PERFORMANCE CONTRACT. Two
indexes lead on that column — `edges_folder_id_idx` and the partial unique
`edges_unique_resolved (folder_id, source_id, target_id, kind) WHERE target_id
IS NOT NULL` — and EXPLAIN picks the second, so a `project` predicate narrows
folders first and only that project''s slice of 3,178,411 edges is touched.
Selecting from `graph_nodes` instead would drag in `node_paths`, `projects`,
`repositories` and a second pass over `nodes` to answer the same question. Keep
the project predicate on this view, not on a wrapper around it.

`folder_projects` KEEPS THAT CONTRACT BECAUSE IT IS TWO PLAIN JOINS, NOT A
FUNCTION. The first attempt at this migration found a folder''s repository by
walking ancestors through `repo_anchor_for`, a set-returning function. A
predicate cannot be pushed into a function scan, and EXPLAIN still shows it:
rebuild that shape and the plan reaches `edges` by SEQ SCAN of all 3,178,411
rows and `nodes` by seq scan as well, with the `project` filter stranded up on
`projects`, above a `Function Scan on repo_anchor_for` the planner cannot see
through. The shape above instead reaches `edges` by `Index Only Scan using
edges_unique_resolved` under `Index Cond: (folder_id = ...)` — 45 loops for a
45-folder project, not 3.18M rows.

THE TWO DEFINITIONS COST THE SAME, AND BUFFER COUNTS WILL NOT SEPARATE THEM.
Measured 2026-10-01, six repetitions each on a 45-folder project: the old
`folders.project_id` join read 12,310..12,511 blocks, `folder_projects` read
12,316..12,545. The ranges overlap, so no ordering can be read off them, and an
earlier revision of this comment that quoted one draw from each and inferred a
one-block saving was reading noise. Quote the SHAPE instead. Both
definitions read `folders` whole — a Parallel Seq Scan of 10,385 blocks, 81 MB
for 13,722 rows, most of either total — so that cost was already being paid.
`folder_projects` adds exactly one hash join whose build side is a Seq Scan of
`project_repositories`: 261 rows in 3 blocks, which is the right plan for a
3-block table, and 3 blocks against ~12,400 is why the totals do not move.

BOTH ENDS INTERNAL (`file_id IS NOT NULL` on both, which is exactly
`locality = ''internal''` — see `structure_graph` for the verification). An edge
into library surface is a boundary fact and `graph_boundary` owns it.

Common queries:
  -- the edge set for one project’s Structure diagram
  SELECT * FROM structure_edges WHERE project = ''sensei'' AND kind = ''calls''
  -- what cuts across the hierarchy, worst first — the finding the view exists for
  SELECT source_file, target_file, sum(occurrences) FROM structure_edges WHERE project = ''sensei'' AND span = ''cross_package'' GROUP BY 1, 2 ORDER BY 3 DESC LIMIT 20
  -- module grain
  SELECT source_module, target_module, sum(occurrences) FROM structure_edges WHERE project = ''sensei'' AND span <> ''in_module'' GROUP BY 1, 2
  -- a shared repository answers under BOTH its projects, with the same numbers
  SELECT project, sum(occurrences) FROM structure_edges WHERE project IN (''kavach'', ''vite-multi-adapter'') GROUP BY 1';

comment on column structure_edges.project is
'From `folder_projects`, which resolves through `project_repositories` — never from `folders.project_id`. A repository serving two projects reports its edges under BOTH, with identical `occurrences` under each.';
comment on column structure_edges.span is
'in_module | cross_module | cross_package. How far the edge reaches, and so whether it follows the hierarchy or cuts across it. Computed once here so every consumer agrees.';
comment on column structure_edges.occurrences is
'How many times this file-to-file relationship was observed, summed over the symbol pairs that produced it AND SHARE ITS MODULE PAIR — symbol pairs in different modules land on separate rows, so one file pair can carry several (measured 2026-10-01: 1,464 file pairs do). For edge thickness, sum over the file pair. Exact PER PROJECT; a sum across projects counts a shared repository''s edges once per owning project.';
