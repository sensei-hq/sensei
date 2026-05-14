set search_path to sensei, extensions;

create or replace view symbols as
select n.id
     , n.folder_id
     , f.name            as folder
     , f.project_id
     , p.name            as project
     , p.maturity        as project_maturity
     , f.status          as folder_status
     , f.kind            as folder_kind
     , n.parent_id
     , n.kind
     , n.name
     , n.file_path
     , n.signature
     , n.description
     , n.docstring
     , n.line_start
     , n.line_end
     , n.is_exported
     , n.community_id
     , n.degree
     , n.tags
     , n.props
     , n.modified_at
  from nodes n
  join folders  f on f.id = n.folder_id
  left join projects p on p.id = f.project_id
 where n.kind not in ('file', 'section', 'rationale');

comment on view symbols is
'Flattened code symbols (functions, classes, types, etc.) with folder and project context.
Excludes file nodes, doc sections, and rationale comments.

Filter/group dimensions: project, project_maturity, folder, folder_status, kind, is_exported, tags.

Common queries:
  SELECT * FROM symbols WHERE folder = ''myrepo'' AND name ILIKE ''%auth%'' AND kind = ''function''
  SELECT kind::text, count(*) FROM symbols WHERE project = ''sensei'' GROUP BY kind
  SELECT folder, count(*) FROM symbols WHERE project_maturity = ''active'' GROUP BY folder';
