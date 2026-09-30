set search_path to sensei, extensions;

create or replace view structure_graph as
with sym as (
  select n.file_id
       , fo.project_id
       , p.name                        as project
       , split_part(n.fqn, '·', 2)     as package
       , split_part(n.fqn, '·', 3)     as module
       , n.language
    from sensei.nodes        n
    join sensei.folders      fo on fo.id = n.folder_id
    left join sensei.projects p on p.id  = fo.project_id
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
`abstractions.component` and `graph_placement` use. A workspace member sits at a
directory path that says nothing about the package it declares, so a
directory-derived tree disagrees with the call graph for every member in the
repository — it would group files that never call each other and separate files
that do.

KEYED ON `file_id`, NOT ON `file_path`. Paths in `files` are relative to their
OWN folder, so `src/lib.rs` exists once per crate; grouping by path would merge
unrelated files across packages into one node. The path is carried for display
only.

BUILT ON BASE TABLES, DELIBERATELY, AND THIS IS A MEASURED DEVIATION. The obvious
spelling is `FROM graph_nodes WHERE locality = ''internal''`, and it is
unusably slow here: `graph_nodes` joins `node_paths` (itself a view), `folders`,
`projects`, `repositories` and `nodes` again for the parent, and the edge view
needs it TWICE over 3.17M edges — the first attempt was cancelled after ten
minutes. `n.file_id IS NOT NULL` is EXACTLY `locality = ''internal''`, verified
on the live graph 2026-09-30: of 744,869 nodes, 66,741 carry a `lib·` fqn and
NONE of them has a `file_id`, so the two arms cannot overlap. If the locality
rule ever gains a third case this view must be revisited with it.

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
  SELECT file_path, symbols FROM structure_graph WHERE project = ''sensei'' ORDER BY symbols DESC LIMIT 20';

comment on column structure_graph.package is
'The fqn''s package segment — the crate / npm package that DECLARES the file, not its directory. For SQL nodes this is the schema, the same idea one level down.';
comment on column structure_graph.module is
'The fqn''s module segment, the second grouping ring. Empty for a file declared at package root.';
comment on column structure_graph.symbols is
'How many internal symbols the file declares. The file''s weight on the diagram.';
comment on column structure_graph.file_id is
'The node identity. `file_path` is relative to its own folder and repeats across packages, so it cannot be the key.';
