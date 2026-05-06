set search_path to sensei, extensions;

create or replace view repositories as
select id
     , root_id
     , parent_id
     , kind
     , name
     , path
     , abs_path
     , project_id
     , remote_urls
     , icons
     , props
     , tags
     , modified_at
  from folders
 where kind in ('git', 'subtree');

comment on view repositories is
'Convenience view over folders for git repos and subtrees.
Backward-compatible surface for code that queries repos.
All columns from folders — filter by kind in (git, subtree).';
