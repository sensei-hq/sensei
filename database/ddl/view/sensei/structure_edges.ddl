set search_path to sensei, extensions;

create or replace view structure_edges as
with e as (
  select fo.project_id
       , p.name                        as project
       , ed.kind::text                 as kind
       , sn.file_id                    as source_file_id
       , tn.file_id                    as target_file_id
       , split_part(sn.fqn, '·', 2)    as source_package
       , split_part(tn.fqn, '·', 2)    as target_package
       , split_part(sn.fqn, '·', 3)    as source_module
       , split_part(tn.fqn, '·', 3)    as target_module
    from sensei.edges        ed
    join sensei.folders      fo on fo.id = ed.folder_id
    left join sensei.projects p on p.id  = fo.project_id
    join sensei.nodes        sn on sn.id = ed.source_id
    join sensei.nodes        tn on tn.id = ed.target_id
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
not a small filter: measured on sensei 2026-09-30, `calls` was 44.7% placed, so
MOST call rows are excluded. A diagram built on this view is a FLOOR, never a
full picture, and any screen using it MUST show what was left out —
`graph_placement` holds that count. A sparse diagram with no coverage line reads
as a simple codebase rather than an unresolved one, and that misreading is the
entire risk.

SELF-FILE EDGES ARE DROPPED (`source_file_id <> target_file_id`). A file calling
itself draws nothing between two rim points, and at file grain it is the largest
population — keeping it would inflate `occurrences` for no visible effect. That
also makes this view the wrong place to ask how much a file does internally;
`structure_graph.symbols` answers that.

JOINED FROM `edges` VIA `folder_id`, WHICH IS THE PERFORMANCE CONTRACT. That
column is indexed (`edges_folder_id_idx`), so a `project` predicate narrows
folders first and only that project''s slice of 3.17M edges is touched. Selecting
from `graph_nodes` instead — which joins `node_paths`, `projects`, `repositories`
and `nodes` again — was cancelled after ten minutes on this same question. Keep
the project predicate on this view, not on a wrapper around it.

BOTH ENDS INTERNAL (`file_id IS NOT NULL` on both, which is exactly
`locality = ''internal''` — see `structure_graph` for the verification). An edge
into library surface is a boundary fact and `graph_boundary` owns it.

Common queries:
  -- the edge set for one project’s Structure diagram
  SELECT * FROM structure_edges WHERE project = ''sensei'' AND kind = ''calls''
  -- what cuts across the hierarchy, worst first — the finding the view exists for
  SELECT source_file, target_file, occurrences FROM structure_edges WHERE project = ''sensei'' AND span = ''cross_package'' ORDER BY occurrences DESC LIMIT 20
  -- module grain
  SELECT source_module, target_module, sum(occurrences) FROM structure_edges WHERE project = ''sensei'' AND span <> ''in_module'' GROUP BY 1, 2';

comment on column structure_edges.span is
'in_module | cross_module | cross_package. How far the edge reaches, and so whether it follows the hierarchy or cuts across it. Computed once here so every consumer agrees.';
comment on column structure_edges.occurrences is
'How many times this file-to-file relationship was observed, summed over the symbol pairs that produced it. Edge thickness.';
