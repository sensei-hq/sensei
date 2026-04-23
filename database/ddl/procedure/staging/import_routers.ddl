set search_path to staging, sensei, gateway, inference, activity, extensions;

create or replace procedure import_routers()
language plpgsql
as $$
begin
  insert into gateway.routers (
      name, display_name, description, router_type
    , api_base_url, api_key_env_var, authentication_type
    , default_headers, rate_limits, config, is_active, sequence, modified_at
  )
  select
      stg.name
    , stg.display_name
    , stg.description
    , stg.router_type::sensei.router_type
    , stg.api_base_url
    , stg.api_key_env_var
    , coalesce(stg.authentication_type, 'api_key')
    , coalesce(stg.default_headers, '{}')
    , coalesce(stg.rate_limits, '{}')
    , coalesce(stg.config, '{}')
    , coalesce(stg.is_active, true)
    , coalesce(stg.sequence, 0)
    , coalesce(stg.modified_at, now())
  from staging.routers stg
  where stg.name is not null
  on conflict (name)
  do update set
      display_name        = excluded.display_name
    , description         = excluded.description
    , router_type         = excluded.router_type
    , api_base_url        = excluded.api_base_url
    , api_key_env_var     = excluded.api_key_env_var
    , authentication_type = excluded.authentication_type
    , default_headers     = excluded.default_headers
    , rate_limits         = excluded.rate_limits
    , config              = excluded.config
    , is_active           = excluded.is_active
    , sequence            = excluded.sequence
    , modified_at         = excluded.modified_at
  where excluded.modified_at >= gateway.routers.modified_at;
end;
$$;
