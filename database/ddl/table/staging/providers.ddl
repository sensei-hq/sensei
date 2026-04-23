set search_path to staging, extensions;

create table if not exists providers (
  name                     text
, display_name             text
, description              text
, website_url              text
, is_open_source           boolean
, is_active                boolean     default true
, sequence                 integer     default 0
, created_at               timestamptz not null default now()
, modified_at              timestamptz not null default now()
, modified_by              text        not null default current_user
);
