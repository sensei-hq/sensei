set search_path to sensei, extensions;

create or replace view project_library_version_conflicts as
with per_folder as (
  select f.project_id
       , rl.library_id
       , rl.version_used
       , f.id           as folder_id
       , f.name         as folder_name
    from sensei.referenced_libraries rl
    join sensei.folders f on f.id = rl.folder_id
   where f.project_id is not null
     and rl.version_used is not null
     and rl.version_used <> ''
     -- Exclude local-protocol deps (link:/workspace:/file:/path=) so only
     -- real registry-version drift surfaces.
     and coalesce(rl.props ? 'local_source', false) = false
),
conflicts as (
  select project_id, library_id
    from per_folder
   group by project_id, library_id
  having count(distinct version_used) > 1
)
select c.project_id
     , c.library_id
     , l.name                                                             as library_name
     , l.ecosystem::text                                                  as ecosystem
     , array_agg(distinct pf.version_used  order by pf.version_used)      as versions
     , array_agg(distinct pf.folder_name  order by pf.folder_name)        as folders
  from conflicts c
  join per_folder pf     on pf.project_id = c.project_id and pf.library_id = c.library_id
  join sensei.libraries l on l.id = c.library_id
 group by c.project_id, c.library_id, l.name, l.ecosystem;

comment on view project_library_version_conflicts is
'Per-project libraries pinned to different versions across folders.
Excludes local-protocol deps (link:/workspace:/file:/path=) so only registry
version drift surfaces. Powers the Track 3 Libraries screen "version conflicts"
signal.
- versions: array of distinct version_used values for the (project, library)
- folders: array of distinct folder names contributing conflicting pins';
