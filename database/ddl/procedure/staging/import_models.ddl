set search_path to staging, sensei, gateway, inference, activity, extensions;

create or replace procedure import_models()
language plpgsql
as $$
begin
  insert into gateway.models (
      provider_id, name, version, variant, full_name, display_name
    , description, capabilities, context_window, max_output_tokens
    , parameters_count, memory_gb, license_type, released_on, deprecated_on
    , props, is_active, modified_at
  )
  select
      p.id
    , stg.name
    , stg.version
    , stg.variant
    , stg.full_name
    , stg.display_name
    , stg.description
    , stg.capabilities::gateway.model_capability[]
    , stg.context_window
    , stg.max_output_tokens
    , stg.parameters_count
    , stg.memory_gb
    , stg.license_type
    , stg.released_on
    , stg.deprecated_on
    , coalesce(stg.props, '{}')
    , coalesce(stg.is_active, true)
    , coalesce(stg.modified_at, now())
  from staging.models stg
  inner join gateway.providers p on p.name = stg.provider_name
  where stg.full_name is not null
  on conflict (provider_id, full_name)
  do update set
      name             = excluded.name
    , version          = excluded.version
    , variant          = excluded.variant
    , display_name     = excluded.display_name
    , description      = excluded.description
    , capabilities     = excluded.capabilities
    , context_window   = excluded.context_window
    , max_output_tokens = excluded.max_output_tokens
    , parameters_count = excluded.parameters_count
    , memory_gb        = excluded.memory_gb
    , license_type     = excluded.license_type
    , released_on      = excluded.released_on
    , deprecated_on    = excluded.deprecated_on
    , props            = excluded.props
    , is_active        = excluded.is_active
    , modified_at      = excluded.modified_at
  where excluded.modified_at >= gateway.models.modified_at;
end;
$$;
