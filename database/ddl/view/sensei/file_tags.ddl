set search_path to sensei, extensions;

create or replace view file_tags as
select n.id
     , n.folder_id
     , f.name            as folder
     , f.project_id
     , n.file_path
     , n.tags
     , n.props
     , n.modified_at
  from nodes n
  join folders f on f.id = n.folder_id
 where n.kind = 'file';

comment on view file_tags is
'File nodes with their classification tags and folder/project context.
Tags are assigned during indexing by classify_file_tag: src, test, e2e, config.

Common queries:
  -- All test files in a folder
  SELECT file_path FROM file_tags WHERE folder = ''myrepo'' AND ''test'' = ANY(tags)
  -- All config files in a project
  SELECT file_path, folder FROM file_tags WHERE project_id = $1 AND ''config'' = ANY(tags)';
