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

Filter/group dimensions: project, project_maturity, folder, edge_kind, confidence, source/target name.

Common queries:
  SELECT source_name, source_file FROM call_graph WHERE folder = ''myrepo'' AND target_name = ''handleAuth'' AND edge_kind = ''calls''
  SELECT edge_kind::text, count(*) FROM call_graph WHERE project = ''sensei'' GROUP BY edge_kind
  SELECT folder, count(*) FROM call_graph WHERE project_maturity = ''active'' AND edge_kind = ''calls'' GROUP BY folder';
