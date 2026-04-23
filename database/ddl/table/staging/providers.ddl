set search_path to staging, extensions;

drop table if exists providers cascade;
create table providers (
  name                     text
, display_name             text
, description              text
, website_url              text
, is_open_source           boolean
, is_active                boolean     default true
, sequence                 integer     default 0
, modified_at              timestamptz not null default now()
);
