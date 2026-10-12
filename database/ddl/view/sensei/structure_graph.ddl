set search_path to sensei, extensions;

create or replace view structure_graph as
with sym as (
  select n.file_id
       , fp.project_id
       , fp.project
       , split_part(n.fqn, '·', 2)     as package
       , split_part(n.fqn, '·', 3)     as module
       , n.language
    from sensei.nodes            n
    join sensei.folder_projects  fp on fp.folder_id = n.folder_id
   where n.file_id is not null
     and n.fqn     is not null
)
select s.project
     , s.project_id
     , s.file_id
     , fi.file_path
     , mode() within group (order by s.package)  as package
     , mode() within group (order by s.module)   as module
     , mode() within group (order by s.language) as language
     , count(*)                                  as symbols
  from sym s
  join sensei.files fi on fi.id = s.file_id
 group by s.project, s.project_id, s.file_id, fi.file_path;

comment on view structure_graph is
'One row per FILE of a project, placed in the hierarchy the graph believes in —
the node set behind the Structure diagram.

THE HIERARCHY IS THE FQN, NOT THE FILESYSTEM, and that is the whole reason this
view exists rather than a GROUP BY over directory names. Segment 2 of the fqn is
the package and segment 3 the module, the same decomposition
`abstractions.component` uses. (`graph_placement` was named here too and does
not belong: it contains no `split_part`, no `·`, and no package or module
notion at all — it groups by project, edge_kind and verdict.) A workspace member sits at a
directory path that says nothing about the package it declares, so a
directory-derived tree disagrees with the call graph for every member in the
repository — it would group files that never call each other and separate files
that do.

KEYED ON `file_id`, NOT ON `file_path`. Paths in `files` are relative to their
OWN folder, so `src/lib.rs` exists once per crate; grouping by path would merge
unrelated files across packages into one node. The path is carried for display
only.

BUILT ON BASE TABLES, DELIBERATELY, AND THIS IS A MEASURED DEVIATION. The obvious
spelling is `FROM graph_nodes WHERE locality = ''internal''`, and it cannot be
written as it stands: `graph_nodes` does not expose `file_id`, the key this view
groups on, so `nodes` has to be joined back a SECOND time on top of the
`node_paths` (itself a view), `folders`, `projects`, `repositories` and parent
self-join that view already carries. Written that way the same per-file
aggregation reads 12.0M buffers against 325,746 on base tables — ~37x, and the
edge view needs that node set TWICE, over 3,178,411 edges. (Buffers, not seconds:
two runs a side on 2026-10-01 read 325,746 buffers BOTH times on base tables and
12,003,913 / 12,003,888 over `graph_nodes`, while the wall clock for the
base-table side alone varied by more than 3x between those same two runs — a full
pass over `nodes` does not stay in a 128MB shared_buffers, so seconds here are
not evidence of anything.)

`n.file_id IS NOT NULL` is EXACTLY `locality = ''internal''`, verified on the live
graph 2026-10-01: that CASE has THREE branches, and `file_id` is null in both of
the ones this view does not want — 66,749 nodes carry a `lib·` fqn (`external`)
and 9,175 carry neither a `lib·` fqn nor a `file_id` (`unknown`), against 668,999
`internal`, every one of which has a `file_id`. 744,923 nodes, no overlap. If that
CASE ever gains a fourth branch this view must be revisited with it.

`folder_projects` is the ONE exception to that rule, and it is a measured one: it
is two joins with no aggregate, so the planner inlines it entirely rather than
scanning it as a subquery — the plan for `WHERE project = ''sparsh''` names
`repositories_in_projects`, `folders` and `projects` directly, with no Subquery Scan
node. It does NOT reach them all by index, and at these sizes it should not: the
251-row `projects` and the 261-row `repositories_in_projects` are each a Seq Scan,
hash-joined, and THAT pair is what drives the Bitmap Index Scan on
`folders_repository_id_idx`, then `nodes_unique_identity`, then `files_pkey`. The
`folders.project_id` spelling it replaces ends identically — a Bitmap Index Scan
on `folders`, then `nodes_unique_identity`, then `files_pkey` — and differs only
at the head: no hash join, `folders_project_id_idx` driven from the `projects` Seq
Scan alone. Warm buffers 1,951 here against 1,945 there, all shared hits, the same
count on every one of four repetitions a side (measured 2026-10-01). Three of the
six are the `repositories_in_projects` Seq Scan; the other three are ONE EXTRA FOLDER
— 45 against 44, the old 44 a strict subset — whose `folders.project_id` says
`bridge` while its repository serves `sparsh` too. That extra folder IS the
disagreement this view was migrated to end, so half the extra cost is buying the
correct answer.
(Buffers, not milliseconds: across five warm repetitions a side the buffer counts
did not move at all while execution time varied by well over an order of
magnitude between repetitions on BOTH sides, so
milliseconds here are measuring the machine.) It is not `graph_nodes`; that one is
still far too heavy, for the reasons above.

MEMBERSHIP COMES FROM `folder_projects`, NEVER FROM `folders.project_id`. A
project owns REPOSITORIES, not folders, so a folder no longer carries its own
answer and the folders of one repository can no longer disagree. That
disagreement was not hypothetical here: before the switch `kavach` showed 285
files and `vite-multi-adapter` 215 — two disjoint halves of ONE repository''s
folders — while `bridge` showed NO files at all and `sparsh` showed its 155.
After it each pair sees the whole repository: 500 files for both kavach and
vite-multi-adapter, 155 for both bridge and sparsh (measured 2026-10-01).

ONE ROW PER (PROJECT, FILE), BECAUSE `folder_projects` IS MULTI-VALUED. Eight
repositories serve two projects each, so their folders yield two rows and their
files appear once under EACH project. That is the intended grain, not a double
count, and what keeps it so is that `project` and `project_id` are GROUP BY
keys: the two resolutions fall into DIFFERENT groups, so every node is still
counted exactly once per project and `symbols` cannot inflate. Verified by
diffing the view across the change on 2026-10-01 — 51,744 -> 57,926 rows, ZERO
rows lost, 6,182 gained, which is exactly the number of distinct files sitting in
those eight repositories, and for all 51,744 rows present on both sides
`symbols`, `package`, `module`, `language` and `file_path` were unchanged. A
caller that sums `symbols` ACROSS projects will therefore count those 6,182 files
twice; it must name one project or deduplicate on `file_id`.

A file whose repository belongs to no project returns NO row, rather than a row
under an invented project. Today that drops nothing: all 661,879 nodes this view
consumes sit in a folder carrying a `repository_id` that has at least one
`repositories_in_projects` row (measured 2026-10-01), which is why the diff above
lost nothing.

INTERNAL ONLY. External nodes are library surface, not this project''s structure;
the diagram offers them as an opt-in overlay sourced from `graph_boundary`.

`symbols` is the file''s weight — how much is declared in it — and needs no git
history, unlike churn.

Common queries:
  -- the node set for one project’s Structure diagram
  SELECT * FROM structure_graph WHERE project = ''sensei'' ORDER BY package, module, file_path
  -- module grain: collapse the rim
  SELECT package, module, count(*) files, sum(symbols) symbols FROM structure_graph WHERE project = ''sensei'' GROUP BY 1, 2
  -- the widest files, which are what a reader lands on first
  SELECT file_path, symbols FROM structure_graph WHERE project = ''sensei'' ORDER BY symbols DESC LIMIT 20
  -- the files a shared repository contributes to more than one project
  SELECT file_id, count(*) FROM structure_graph GROUP BY 1 HAVING count(*) > 1';

comment on column structure_graph.package is
'The fqn''s package segment — the crate / npm package that DECLARES the file, not its directory. For SQL nodes this is the schema, the same idea one level down.';
comment on column structure_graph.module is
'The fqn''s module segment, the second grouping ring. Empty for a file declared at package root.';
comment on column structure_graph.symbols is
'How many internal symbols the file declares. The file''s weight on the diagram.';
comment on column structure_graph.file_id is
'The node identity WITHIN one project. `file_path` is relative to its own folder and repeats across packages, so it cannot be the key; and across projects a file of a shared repository appears once per project, so the key of the whole view is (project_id, file_id).';
comment on column structure_graph.project_id is
'From `repositories_in_projects` via `folder_projects`, never from `folders.project_id`. A folder does not carry its own project; that is what let the folders of one repository drift apart.';
