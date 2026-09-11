set search_path to sensei, extensions;

-- REPO-RELATIVE, reconstructed. `files.file_path` is FOLDER-relative and a
-- module folder's `folders.path` is repo-relative, so a node in
-- `crates/senseid/src/lib.rs` stores `src/lib.rs` against the `crates/senseid`
-- folder. Exposing that raw would rename the column's meaning without changing
-- its name — v1's `nodes.file_path` was repo-relative, and 17 of this repo's 18
-- folders are modules, so almost every path would have been silently truncated.
-- The repo-root folder is the exception: its `path` is ABSOLUTE, and its files
-- are already repo-relative, so it passes through.
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
     , case when f.kind = 'git' then fi.file_path
            else f.path || '/' || fi.file_path end as file_path
     , n.signature
     , n.description
     , n.docstring
     , n.line_start
     , n.line_end
     , n.is_exported
     , n.community_id
     , n.tags
     , n.props
     , n.modified_at
  from nodes n
  left join files fi on fi.id = n.file_id
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
