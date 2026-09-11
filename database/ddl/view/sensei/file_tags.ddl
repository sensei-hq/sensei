set search_path to sensei, extensions;

-- REPO-RELATIVE, reconstructed. `files.file_path` is FOLDER-relative and a
-- module folder's `folders.path` is repo-relative, so a node in
-- `crates/senseid/src/lib.rs` stores `src/lib.rs` against the `crates/senseid`
-- folder. Exposing that raw would rename the column's meaning without changing
-- its name — v1's `nodes.file_path` was repo-relative, and 17 of this repo's 18
-- folders are modules, so almost every path would have been silently truncated.
-- The repo-root folder is the exception: its `path` is ABSOLUTE, and its files
-- are already repo-relative, so it passes through.
create or replace view file_tags as
select n.id
     , n.folder_id
     , f.name            as folder
     , f.project_id
     , p.name            as project
     , p.maturity        as project_maturity
     , case when f.kind = 'git' then fi.file_path
            else f.path || '/' || fi.file_path end as file_path
     , n.tags
     , n.props
     , n.modified_at
  from nodes n
  left join files fi on fi.id = n.file_id
  join folders  f on f.id = n.folder_id
  left join projects p on p.id = f.project_id
 where n.kind = 'file';

comment on view file_tags is
'File nodes with classification tags, folder and project context.
Tags assigned during indexing: src, test, e2e, config.

Filter/group dimensions: project, project_maturity, folder, tags.

Common queries:
  SELECT file_path FROM file_tags WHERE folder = ''myrepo'' AND ''test'' = ANY(tags)
  SELECT unnest(tags) as tag, count(*) FROM file_tags WHERE project = ''sensei'' GROUP BY tag
  SELECT folder, count(*) FROM file_tags WHERE project_maturity = ''active'' AND ''test'' = ANY(tags) GROUP BY folder';
