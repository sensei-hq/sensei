set search_path to staging, sensei, extensions;

create or replace procedure import_scopes()
language plpgsql
set search_path = staging, sensei, extensions
as $$
begin
  insert into sensei.scopes (
      key, name, level, shareable, description, modified_at
  )
  select
      stg.key
    , stg.name
    , stg.level
    , coalesce(stg.shareable, false)
    , stg.description
    , coalesce(stg.modified_at, now())
  from staging.scopes stg
  where stg.key is not null
  on conflict (key)
  do update set
      name        = excluded.name
    , level       = excluded.level
    , shareable   = excluded.shareable
    , description  = excluded.description
    , modified_at = excluded.modified_at
  where excluded.modified_at >= sensei.scopes.modified_at;
end;
$$;
