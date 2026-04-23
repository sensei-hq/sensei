set search_path to gateway, extensions;

create type if not exists model_capability
    as enum ('chat', 'reasoning', 'embed', 'classify', 'summarize', 'vision', 'audio');

create table if not exists models (
  id                       uuid               primary key default gen_random_uuid()
, provider_id              uuid               not null references gateway.providers(id) on delete cascade
, name                     text               not null
, version                  text
, variant                  text
, full_name                text               not null
, display_name             text               not null
, description              text
, capabilities             model_capability[] not null default '{}'
, context_window           integer
, max_output_tokens        integer
, parameters_count         bigint
, memory_gb                numeric(5,1)
, license_type             text
, released_on              date
, deprecated_on            date
, props                    jsonb              not null default '{}'
, is_active                boolean            not null default true
, modified_at              timestamptz        not null default now()
, unique(provider_id, full_name)
);

create index if not exists models_provider_id_idx
    on models(provider_id);

create index if not exists models_capabilities_idx
    on models using gin(capabilities);

comment on table models is
'AI model definitions with capabilities, sizing, and memory requirements.
- capabilities: array of model_capability enum — chat, reasoning, embed, classify, summarize, vision, audio
- memory_gb: RAM required to run locally via Ollama (null for external-only models)
- parameters_count: model parameter count in raw number (e.g. 27000000000 for 27B)
- props: extensible — {temperature_range, top_p_range, training_cutoff, ...}
Seed data loaded via staging.import_models().';

comment on column models.id
     is 'Surrogate primary key (UUID).';
comment on column models.provider_id
     is 'Foreign key to providers — who created this model.';
comment on column models.name
     is 'Model family name (e.g. "gemma3", "claude", "gpt").';
comment on column models.version
     is 'Version string (e.g. "3", "4o", "4.5").';
comment on column models.variant
     is 'Variant within version (e.g. "27b", "haiku", "mini"). Nullable.';
comment on column models.full_name
     is 'Complete model identifier (e.g. "gemma3:27b", "claude-haiku-4-5", "gpt-4o-mini").';
comment on column models.display_name
     is 'Human-readable display name.';
comment on column models.description
     is 'Brief description of model strengths and intended use.';
comment on column models.capabilities
     is 'Array of capabilities this model supports: chat, reasoning, embed, classify, summarize, vision, audio.';
comment on column models.context_window
     is 'Maximum input context in tokens.';
comment on column models.max_output_tokens
     is 'Maximum output tokens per request.';
comment on column models.parameters_count
     is 'Model parameter count (e.g. 27000000000 for 27B).';
comment on column models.memory_gb
     is 'RAM required to run locally via Ollama. Null for external-only models.';
comment on column models.license_type
     is 'License type: commercial, apache-2.0, llama-3, gemma, etc.';
comment on column models.released_on
     is 'Model release date.';
comment on column models.deprecated_on
     is 'Deprecation date. Null if active.';
comment on column models.props
     is 'Extensible config: {temperature_range, top_p_range, training_cutoff, ...}.';
comment on column models.is_active
     is 'Whether this model is available for selection.';
comment on column models.modified_at
     is 'Timestamp of the last modification to this row.';
