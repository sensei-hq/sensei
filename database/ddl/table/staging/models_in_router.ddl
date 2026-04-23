set search_path to staging, extensions;

create table if not exists models_in_router (
  router_name              text
, model_full_name          text
, router_model_id          text
, cost_per_input_token     numeric(12,10) default 0
, cost_per_output_token    numeric(12,10) default 0
, cost_per_request         numeric(10,6)  default 0
, is_active                boolean     default true
, is_default               boolean     default false
, props                    jsonb       default '{}'
, created_at               timestamptz not null default now()
, modified_at              timestamptz not null default now()
, modified_by              text        not null default current_user
);
