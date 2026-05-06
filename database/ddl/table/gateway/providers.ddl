set search_path to gateway, extensions;

create table if not exists providers (
  id                       uuid        primary key default gen_random_uuid()
, name                     text        not null unique
, display_name             text        not null
, description              text
, website_url              text
, is_open_source           boolean     not null default false
, is_active                boolean     not null default true
, sequence                 integer     not null default 0
, modified_at              timestamptz not null default now()
);

comment on table providers is
'AI model organizations — OpenAI, Anthropic, Meta, Google, Alibaba, etc.
Seed data loaded via staging.import_providers().';

comment on column providers.id
     is 'Surrogate primary key (UUID).';
comment on column providers.name
     is 'Unique slug identifier (e.g. "anthropic", "meta", "google").';
comment on column providers.display_name
     is 'Human-readable name (e.g. "Anthropic", "Meta").';
comment on column providers.description
     is 'Brief description of the provider.';
comment on column providers.website_url
     is 'Provider homepage URL.';
comment on column providers.is_open_source
     is 'Whether the provider primarily offers open-source models.';
comment on column providers.is_active
     is 'Whether models from this provider are available for selection.';
comment on column providers.sequence
     is 'Display order — lower values shown first.';
comment on column providers.modified_at
     is 'Timestamp of the last modification to this row.';
