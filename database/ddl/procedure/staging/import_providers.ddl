set search_path to staging, extensions;

create or replace procedure import_providers()
language plpgsql
as $$
begin
  insert into inference.providers (name, display_name, description, website_url, is_open_source, is_active, sequence, modified_at)
  select
      stg.name
    , stg.display_name
    , stg.description
    , stg.website_url
    , coalesce(stg.is_open_source, false)
    , coalesce(stg.is_active, true)
    , coalesce(stg.sequence, 0)
    , coalesce(stg.modified_at, now())
  from staging.providers stg
  where stg.name is not null
  on conflict (name)
  do update set
      display_name = excluded.display_name
    , description  = excluded.description
    , website_url  = excluded.website_url
    , is_open_source = excluded.is_open_source
    , is_active    = excluded.is_active
    , sequence     = excluded.sequence
    , modified_at  = excluded.modified_at
  where excluded.modified_at >= inference.providers.modified_at;
end;
$$;
