set search_path to inference, extensions;

create table if not exists models_in_router (
  id                       uuid        primary key default gen_random_uuid()
, router_id                uuid        not null references inference.routers(id) on delete cascade
, model_id                 uuid        not null references inference.models(id) on delete cascade
, router_model_id          text        not null
, cost_per_input_token     numeric(12,10) not null default 0
, cost_per_output_token    numeric(12,10) not null default 0
, cost_per_request         numeric(10,6)  not null default 0
, is_active                boolean     not null default true
, is_default               boolean     not null default false
, props                    jsonb       not null default '{}'
, modified_at              timestamptz not null default now()
, created_at               timestamptz not null default now()
, unique(router_id, model_id)
);

create index if not exists models_in_router_router_id_idx
    on models_in_router(router_id);

create index if not exists models_in_router_model_id_idx
    on models_in_router(model_id);

comment on table models_in_router is
'Junction: which models are available on which router, with pricing.
- router_model_id: the identifier the router uses for this model (e.g. "gemma3:27b" for Ollama, "claude-haiku-4-5-20251001" for Anthropic)
- cost_per_input/output_token: 0 for local routers (Ollama), per-token pricing for external
- is_default: primary endpoint for this model (if available on multiple routers)
- props: extensible — {max_concurrent, supports_streaming, supports_tools, ...}
Seed data loaded via staging.import_models_in_router().';

comment on column models_in_router.id
     is 'Surrogate primary key (UUID).';
comment on column models_in_router.router_id
     is 'Foreign key to routers — which API endpoint serves this model.';
comment on column models_in_router.model_id
     is 'Foreign key to models — which model is available.';
comment on column models_in_router.router_model_id
     is 'The model identifier used by this router (e.g. "gemma3:27b", "claude-haiku-4-5-20251001").';
comment on column models_in_router.cost_per_input_token
     is 'Cost per input token in USD. 0 for local models.';
comment on column models_in_router.cost_per_output_token
     is 'Cost per output token in USD. 0 for local models.';
comment on column models_in_router.cost_per_request
     is 'Fixed cost per request in USD. 0 for most models.';
comment on column models_in_router.is_active
     is 'Whether this model-router combination is currently usable.';
comment on column models_in_router.is_default
     is 'Whether this is the primary endpoint for this model.';
comment on column models_in_router.props
     is 'Extensible: {max_concurrent, supports_streaming, supports_tools, ...}.';
comment on column models_in_router.modified_at
     is 'Timestamp of the last modification to this row.';
comment on column models_in_router.created_at
     is 'Timestamp when this model-router binding was first registered.';
