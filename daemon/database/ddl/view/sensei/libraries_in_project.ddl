set search_path to sensei, extensions;

create or replace view libraries_in_project as
select distinct
       f.project_id
     , rl.library_id
     , l.name
     , l.ecosystem
     , l.kind          as library_kind
     , l.description
     , l.page_count
     , l.icons
     , l.tags
  from referenced_libraries rl
  join folders              f on f.id = rl.folder_id
  join libraries            l on l.id = rl.library_id
 where f.project_id is not null;

comment on view libraries_in_project is
'Unique set of libraries used across all folders in each project.
Joins referenced_libraries through folders to projects, deduplicates by (project_id, library_id).';
