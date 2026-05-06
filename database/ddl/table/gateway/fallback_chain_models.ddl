set search_path to gateway, extensions;

create table if not exists fallback_chain_models (
  id                       uuid        primary key default gen_random_uuid()
, chain_id                 uuid        not null references gateway.fallback_chains(id) on delete cascade
, router_id                uuid        not null references gateway.routers(id) on delete cascade
, model_id                 uuid        not null references gateway.models(id) on delete cascade
, sequence_order           integer     not null
, max_retries              integer     not null default 1
, is_active                boolean     not null default true
, modified_at              timestamptz not null default now()
, unique(chain_id, sequence_order)
);

create index if not exists fallback_chain_models_chain_id_idx
    on fallback_chain_models(chain_id);

comment on table fallback_chain_models is
'Ordered model entries within a fallback chain.
sequence_order defines the try-order: 1 = first attempt, 2 = first fallback, etc.
Seed data loaded via staging.import_fallback_chain_models().';

comment on column fallback_chain_models.id
     is 'Surrogate primary key (UUID).';
comment on column fallback_chain_models.chain_id
     is 'Foreign key to fallback_chains — which chain this entry belongs to.';
comment on column fallback_chain_models.router_id
     is 'Foreign key to routers — which API endpoint to use for this attempt.';
comment on column fallback_chain_models.model_id
     is 'Foreign key to models — which model to try at this position.';
comment on column fallback_chain_models.sequence_order
     is 'Position in the chain: 1 = primary, 2 = first fallback, etc.';
comment on column fallback_chain_models.max_retries
     is 'How many times to retry this specific model before moving to next in chain.';
comment on column fallback_chain_models.is_active
     is 'Whether this chain entry is currently active.';
comment on column fallback_chain_models.modified_at
     is 'Timestamp of the last modification to this row.';
