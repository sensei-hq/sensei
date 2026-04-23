set search_path to staging, extensions;

create or replace procedure import_fallback_chain_models()
language plpgsql
as $$
begin
  insert into gateway.fallback_chain_models (
      chain_id, router_id, model_id, sequence_order, max_retries, is_active, modified_at
  )
  select
      fc.id
    , r.id
    , m.id
    , stg.sequence_order
    , coalesce(stg.max_retries, 1)
    , coalesce(stg.is_active, true)
    , coalesce(stg.modified_at, now())
  from staging.fallback_chain_models stg
  inner join gateway.fallback_chains fc on fc.name = stg.chain_name
  inner join gateway.routers         r  on r.name  = stg.router_name
  inner join gateway.models          m  on m.full_name = stg.model_full_name
  where stg.chain_name is not null
    and stg.router_name is not null
    and stg.model_full_name is not null
  on conflict (chain_id, sequence_order)
  do update set
      router_id    = excluded.router_id
    , model_id     = excluded.model_id
    , max_retries  = excluded.max_retries
    , is_active    = excluded.is_active
    , modified_at  = excluded.modified_at
  where excluded.modified_at >= gateway.fallback_chain_models.modified_at;
end;
$$;
