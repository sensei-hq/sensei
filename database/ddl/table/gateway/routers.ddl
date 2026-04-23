set search_path to gateway, extensions;

create type if not exists router_type
    as enum ('direct', 'aggregator', 'local');

create table if not exists routers (
  id                       uuid        primary key default gen_random_uuid()
, name                     text        not null unique
, display_name             text        not null
, description              text
, router_type              router_type not null
, api_base_url             text
, api_key_env_var          text
, authentication_type      text        not null default 'api_key'
                                       check (authentication_type in ('api_key', 'none'))
, default_headers          jsonb       not null default '{}'
, rate_limits              jsonb       not null default '{}'
, config                   jsonb       not null default '{}'
, is_active                boolean     not null default true
, sequence                 integer     not null default 0
, modified_at              timestamptz not null default now()
);

comment on table routers is
'API access points for LLM gateway.
- direct: single-provider API (anthropic, openai)
- aggregator: multi-provider proxy (openrouter)
- local: on-device runtime (ollama)
- config: connection settings — {timeout_ms, retry_limit, retry_delay_ms, circuit_breaker_threshold, ...}
- rate_limits: {requests_per_minute, tokens_per_minute}
Seed data loaded via staging.import_routers().';

comment on column routers.id
     is 'Surrogate primary key (UUID).';
comment on column routers.name
     is 'Unique slug identifier (e.g. "ollama", "anthropic", "openai").';
comment on column routers.display_name
     is 'Human-readable name (e.g. "Ollama (local)", "Anthropic").';
comment on column routers.description
     is 'Brief description of this router.';
comment on column routers.router_type
     is 'Router category: direct (single provider), aggregator (multi-provider), local (on-device).';
comment on column routers.api_base_url
     is 'Base URL for API calls (e.g. "http://localhost:11434", "https://api.anthropic.com").';
comment on column routers.api_key_env_var
     is 'Environment variable name containing the API key (e.g. "ANTHROPIC_API_KEY"). Null for local routers.';
comment on column routers.authentication_type
     is 'How to authenticate: api_key (header-based) or none (local).';
comment on column routers.default_headers
     is 'Default HTTP headers for requests to this router.';
comment on column routers.rate_limits
     is 'Rate limits: {requests_per_minute, tokens_per_minute}.';
comment on column routers.config
     is 'Connection config: {timeout_ms, retry_limit, retry_delay_ms, circuit_breaker_threshold, circuit_breaker_window_minutes}.';
comment on column routers.is_active
     is 'Whether this router is available for use.';
comment on column routers.sequence
     is 'Display order — lower values shown first.';
comment on column routers.modified_at
     is 'Timestamp of the last modification to this row.';
