set search_path to sensei, extensions;

drop view if exists module_edges;

create or replace view module_edges as
select e.project
     , e.project_id
     , e.kind
     , module_of(e.source_package, e.source_module) as source_module
     , module_of(e.target_package, e.target_module) as target_module
     , e.source_package
     , e.target_package
     , count(distinct e.source_file_id)             as source_files
     , sum(e.occurrences)::bigint                   as occurrences
  from sensei.structure_edges e
 where e.source_package is not null
   and e.target_package is not null
 group by e.project, e.project_id, e.kind
        , module_of(e.source_package, e.source_module)
        , module_of(e.target_package, e.target_module)
        , e.source_package, e.target_package;

comment on view module_edges is
'Module-to-module dependencies of a project — the edge set behind the Layers and
Cycles diagrams, and the input to the dependency matrix.

BUILT ON `structure_edges` RATHER THAN ON `edges`, deliberately. That view already
owns the three decisions this one must not re-make: membership resolves through
`folder_projects` (so a repository serving two projects answers under both),
only PLACED edges count (`target_id IS NOT NULL`), and both ends are internal.
Re-deriving them here would mean the Structure diagram and the Layers diagram
could disagree about which edges exist, which is the exact drift the `span`
column was centralised to prevent.

Measured cost of that choice, stated rather than discovered: reading this view
for one project costs what `structure_edges` costs, because the aggregation on
top is free by comparison — about 300,000 buffers and 15-50 s for project
`sensei` depending on cache, against roughly half that for an equivalent query
written straight off `edges` with no `files` join. The ~2x is not worth two
definitions of a project''s edge set. A screen reading this needs the same
treatment Structure needs, and it is the same problem.

SELF-DEPENDENCIES ARE KEPT (`source_module = target_module` rows survive). At
FILE grain `structure_edges` already drops a file calling itself, so a row here
with equal ends means two DIFFERENT files of one module referring to each other —
internal cohesion, which is a fact about the module rather than noise. The
layering step removes them from the graph (a self-loop is a cycle no rank can
order, and it is what hangs a naive longest-path walk) but reports them
separately; measured 2026-10-05 on project `sensei`, 9 modules appear ONLY in
rows like these and would vanish from the diagram entirely if this view filtered
them out here.

THE GRAIN IS (PROJECT, KIND, MODULE PAIR, PACKAGE PAIR). The package pair is in
the key because `module_of` already folds the package into the identity, so
carrying the two halves costs nothing and lets a caller group by package without
re-splitting the id. `kind` is NOT folded: a caller asking only about `calls`
must be able to filter rather than subtract.

`occurrences` SUMS ACROSS THE FILE PAIRS BENEATH IT, which is what
`structure_edges` tells its own callers to do — one file-to-file relationship can
surface as several rows there when two symbols in one file sit in different
modules, and summing is the only reading that counts each once.

`source_files` IS A FAN-OUT MEASURE, not a row count: how many distinct files of
the source module take part in this dependency. One file reaching into another
module is a seam; twelve files reaching into it is a coupling.

FILTER BY PROJECT. `structure_edges` is driven from the folders of one project
through a LATERAL, so an UNFILTERED read of it is pathological by design and this
view inherits that exactly. Every consumer binds `project_id`.

NULL PACKAGES ARE EXCLUDED rather than coalesced. `module_of` propagates NULL,
and a node with no package has no module to belong to; grouping those under ''''
would invent a module that owns every unplaceable symbol in the project and sits
at the bottom of every layering.

Common queries:
  -- the dependency graph one Layers screen draws
  SELECT source_module, target_module, occurrences FROM module_edges WHERE project_id = $1 AND kind = ''calls''
  -- what a module leans on hardest
  SELECT target_module, sum(occurrences) FROM module_edges WHERE project_id = $1 AND source_module = $2 GROUP BY 1 ORDER BY 2 DESC
  -- internal cohesion: the modules that mostly talk to themselves
  SELECT source_module, occurrences FROM module_edges WHERE project_id = $1 AND source_module = target_module ORDER BY 2 DESC';

comment on column module_edges.source_module is
'From `module_of(source_package, source_module)` — the SAME expression the Structure diagram''s `module` rollup uses, so a node there and an edge here name one unit.';
comment on column module_edges.target_module is
'From `module_of(target_package, target_module)`. Equal to `source_module` on a self-dependency, which is kept; see the view comment.';
comment on column module_edges.source_files is
'Distinct files of the source module taking part. A fan-out measure of the dependency, not a row count.';
comment on column module_edges.occurrences is
'Summed over the file pairs beneath this module pair. Exact per project; a sum across projects counts a shared repository''s edges once per owning project, exactly as `structure_edges` does.';
