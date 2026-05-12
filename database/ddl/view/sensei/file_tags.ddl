set search_path to sensei, extensions;

create or replace view file_tags as
select n.id
     , n.folder_id
     , f.name            as folder
     , f.project_id
     , p.name            as project
     , p.maturity        as project_maturity
     , n.file_path
     , n.tags
     , n.props
     , n.modified_at
  from nodes n
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
