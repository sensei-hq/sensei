set search_path to staging, extensions, sensei;

drop table if exists routers cascade;
create table routers (
  name                     text
, display_name             text
, description              text
, router_type              text
, api_base_url             text
, api_key_env_var          text
, authentication_type      text        default 'api_key'
, default_headers          jsonb       default '{}'
, rate_limits              jsonb       default '{}'
, config                   jsonb       default '{}'
, is_active                boolean     default true
, sequence                 integer     default 0
, modified_at              timestamptz not null default now()
);
