set search_path to staging, extensions;

create or replace procedure import_models_in_router()
language plpgsql
as $$
begin
  insert into gateway.models_in_router (
      router_id, model_id, router_model_id
    , cost_per_input_token, cost_per_output_token, cost_per_request
    , is_active, is_default, props, modified_at
  )
  select
      r.id
    , m.id
    , stg.router_model_id
    , coalesce(stg.cost_per_input_token, 0)
    , coalesce(stg.cost_per_output_token, 0)
    , coalesce(stg.cost_per_request, 0)
    , coalesce(stg.is_active, true)
    , coalesce(stg.is_default, false)
    , coalesce(stg.props, '{}')
    , coalesce(stg.modified_at, now())
  from staging.models_in_router stg
  inner join gateway.routers r on r.name = stg.router_name
  inner join gateway.models  m on m.full_name = stg.model_full_name
  where stg.router_name is not null
    and stg.model_full_name is not null
  on conflict (router_id, model_id)
  do update set
      router_model_id      = excluded.router_model_id
    , cost_per_input_token  = excluded.cost_per_input_token
    , cost_per_output_token = excluded.cost_per_output_token
    , cost_per_request      = excluded.cost_per_request
    , is_active             = excluded.is_active
    , is_default            = excluded.is_default
    , props                 = excluded.props
    , modified_at           = excluded.modified_at
  where excluded.modified_at >= gateway.models_in_router.modified_at;
end;
$$;
