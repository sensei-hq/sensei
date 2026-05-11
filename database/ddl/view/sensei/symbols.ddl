set search_path to sensei, extensions;

create or replace view symbols as
select n.id
     , n.folder_id
     , f.name            as folder
     , f.project_id
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
  join folders f on f.id = n.folder_id
 where n.kind not in ('file', 'section', 'rationale');

comment on view symbols is
'Flattened code symbols (functions, classes, types, etc.) with folder and project context.
Excludes file nodes, doc sections, and rationale comments.
Use for: symbol search, callers/callees lookup, function/type listings.

Common queries:
  -- Find a function by name in a folder
  SELECT * FROM symbols WHERE folder = ''myrepo'' AND name ILIKE ''%auth%'' AND kind = ''function''
  -- All exported types in a project
  SELECT * FROM symbols WHERE project_id = $1 AND kind IN (''class'',''interface'',''type'') AND is_exported';
