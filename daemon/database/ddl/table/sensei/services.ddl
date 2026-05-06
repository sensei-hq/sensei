set search_path to sensei, extensions;

create table if not exists services (
  id                       uuid         primary key default gen_random_uuid()
, name                     text         not null unique
, display_name             text         not null
, publisher                text
, protocol                 service_protocol not null default 'mcp'
, kind                     service_kind not null
, summary                  text
, trigger_stacks           text[]       not null default '{}'
, tools_count              integer      not null default 0
, verified                 boolean      not null default false
, installed                boolean      not null default false
, config                   jsonb        not null default '{}'
, icons                    jsonb        not null default '{}'
, tags                     text[]       not null default '{}'
, modified_at              timestamptz  not null default now()
);

create index if not exists services_kind_idx
    on services(kind);

create index if not exists services_installed_idx
    on services(installed)
 where installed = true;

create index if not exists services_tags_idx
    on services using gin(tags);

comment on table services is
'External service registry for MCP servers and inference providers.
- kind: data (databases), api (stripe, github), devtool (playwright, figma), service (sentry, etc.), inference (Ollama, external LLM providers)
- protocol: mcp (tools via http/stdio), ollama (local inference), anthropic/openai (external inference)
- trigger_stacks: which detected stacks recommend this service (e.g. ["PostgreSQL"])
- config: connection config, env vars, transport settings, model preferences';

comment on column services.id
     is 'Surrogate primary key (UUID).';
comment on column services.name
     is 'Unique identifier for the service (e.g. "postgres-mcp").';
comment on column services.display_name
     is 'Human-readable name (e.g. "PostgreSQL MCP").';
comment on column services.publisher
     is 'Who publishes this service (e.g. "supabase", "stripe").';
comment on column services.protocol
     is 'Communication protocol: mcp (tools via http/stdio), ollama (local inference), anthropic/openai (external LLM providers).';
comment on column services.kind
     is 'Service category: data (databases), api (external APIs), devtool (dev tools), service (monitoring etc.), inference (local/external LLM providers).';
comment on column services.summary
     is 'Short description of what this service provides.';
comment on column services.trigger_stacks
     is 'Stack names that trigger recommendation of this service (e.g. ["PostgreSQL","Redis"]).';
comment on column services.tools_count
     is 'Number of tools exposed by this service.';
comment on column services.verified
     is 'Whether the service is verified/official.';
comment on column services.installed
     is 'Whether the service is currently installed and configured.';
comment on column services.config
     is 'Connection config: MCP: {transport, command, args, url, env}. Ollama: {url, models, default_model}. External: {api_key_env, default_model}.';
comment on column services.icons
     is 'Display icons: {emoji, devicon, kanji, custom}.';
comment on column services.tags
     is 'Array of tag strings for filtering. Vocabulary controlled by sensei.tags table.';
comment on column services.modified_at
     is 'Timestamp of the last modification to this row.';
