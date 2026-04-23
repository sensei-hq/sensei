set search_path to staging, extensions;

drop table if exists models cascade;
create table models (
  provider_name            text
, name                     text
, version                  text
, variant                  text
, full_name                text
, display_name             text
, description              text
, capabilities             text[]
, context_window           integer
, max_output_tokens        integer
, parameters_count         bigint
, memory_gb                numeric(5,1)
, license_type             text
, released_on              date
, deprecated_on            date
, props                    jsonb       default '{}'
, is_active                boolean     default true
, created_at               timestamptz not null default now()
, modified_at              timestamptz not null default now()
, modified_by              text        not null default current_user
);
