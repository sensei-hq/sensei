set search_path to sensei, extensions;

create or replace view parent_folders as
select id
     , root_id
     , name
     , path
     , abs_path
     , icons
     , props
     , tags
     , modified_at
     , created_at
  from folders
 where kind = 'parent';

comment on view parent_folders is
'Convenience view over folders for organisational parent directories (depth 1 from root).';
