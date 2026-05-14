set search_path to sensei, extensions;

create or replace view doc_coverage as
select e.id              as edge_id
     , e.folder_id
     , f.name            as folder
     , f.project_id
     , p.name            as project
     , p.maturity        as project_maturity
     , doc.id            as doc_id
     , doc.name          as doc_name
     , doc.file_path     as doc_file
     , doc.modified_at   as doc_modified
     , code.id           as code_id
     , code.name         as code_name
     , code.file_path    as code_file
     , code.modified_at  as code_modified
     , (code.modified_at > doc.modified_at) as drifted
  from edges e
  join folders  f  on f.id = e.folder_id
  left join projects p on p.id = f.project_id
  join nodes doc  on doc.id = e.source_id
  join nodes code on code.id = e.target_id
 where e.kind = 'covers';

comment on view doc_coverage is
'Doc-to-code traceability via covers edges, with drift detection and project context.
drifted = true when code was modified more recently than the covering doc.

Filter/group dimensions: project, project_maturity, folder, drifted.

Common queries:
  SELECT doc_file, code_file FROM doc_coverage WHERE folder = ''myrepo'' AND drifted
  SELECT folder, count(*) FILTER (WHERE drifted) as drifted FROM doc_coverage WHERE project = ''sensei'' GROUP BY folder
  SELECT project, count(*) as covered, count(*) FILTER (WHERE drifted) as drifted FROM doc_coverage GROUP BY project';
