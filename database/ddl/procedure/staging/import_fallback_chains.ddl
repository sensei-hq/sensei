set search_path to staging, extensions;

create or replace procedure import_fallback_chains()
language plpgsql
as $$
begin
  insert into inference.fallback_chains (
      name, capability, description, max_fallback_attempts, is_active, sequence, modified_at
  )
  select
      stg.name
    , stg.capability::inference.model_capability
    , stg.description
    , coalesce(stg.max_fallback_attempts, 3)
    , coalesce(stg.is_active, true)
    , coalesce(stg.sequence, 0)
    , coalesce(stg.modified_at, now())
  from staging.fallback_chains stg
  where stg.name is not null
  on conflict (name)
  do update set
      capability            = excluded.capability
    , description           = excluded.description
    , max_fallback_attempts = excluded.max_fallback_attempts
    , is_active             = excluded.is_active
    , sequence              = excluded.sequence
    , modified_at           = excluded.modified_at
  where excluded.modified_at >= inference.fallback_chains.modified_at;
end;
$$;
