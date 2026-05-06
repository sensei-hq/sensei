set search_path to sensei, extensions;

create or replace view parent_folders as
select f.id
     , f.root_id
     , f.name
     , f.path
     , f.abs_path
     , f.icons
     , f.props
     , f.tags
     , f.modified_at
  from folders f
 where f.parent_id is null;

comment on view parent_folders is
'Convenience view: top-level folders directly under a watch root (parent_id is null).';
