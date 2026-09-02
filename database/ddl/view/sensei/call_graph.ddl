set search_path to sensei, extensions;

create or replace view call_graph as
select e.id              as edge_id
     , e.folder_id
     , f.name            as folder
     , f.project_id
     , p.name            as project
     , p.maturity        as project_maturity
     , e.kind            as edge_kind
     , e.confidence
     , e.confidence_score
     , e.source_id
     , src.name          as source_name
     , src.kind          as source_kind
     , src.file_path     as source_file
     , src.line_start    as source_line
     , src.is_exported   as source_exported
     , e.target_id
     , tgt.name          as target_name
     , tgt.kind          as target_kind
     , tgt.file_path     as target_file
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
  from edges         e
  join folders       f
    on f.id          = e.folder_id
  left join projects p
    on p.id          = f.project_id
  join nodes         src
    on src.id        = e.source_id
  left join nodes    tgt
    on tgt.id        = e.target_id;

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

Filter/group dimensions: project, project_maturity, folder, edge_kind, confidence,
source_name, target_symbol.

Common queries:
  -- who calls X (resolved AND unresolved, which is what a caller lookup means)
  SELECT source_name, source_file FROM call_graph WHERE folder = ''myrepo'' AND target_symbol = ''handleAuth'' AND edge_kind = ''calls''
  -- resolution coverage, the number that tells you how much to trust the above
  SELECT count(target_id) AS resolved, count(*) - count(target_id) AS unresolved FROM call_graph WHERE project = ''sensei'' AND edge_kind = ''calls''
  SELECT edge_kind::text, count(*) FROM call_graph WHERE project = ''sensei'' GROUP BY edge_kind
  SELECT folder, count(*) FROM call_graph WHERE project_maturity = ''active'' AND edge_kind = ''calls'' GROUP BY folder';

comment on column call_graph.target_symbol is
'The target symbol name however recorded: tgt.name when the edge resolved, e.target_name when it did not. THE column to filter a target on — target_name is resolved-only and silently excludes 34.9% of calls edges.';
