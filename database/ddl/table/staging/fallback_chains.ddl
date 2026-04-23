set search_path to staging, extensions;

drop table if exists fallback_chains cascade;
create table fallback_chains (
  name                     text
, capability               text
, description              text
, max_fallback_attempts    integer     default 3
, is_active                boolean     default true
, sequence                 integer     default 0
, created_at               timestamptz not null default now()
, modified_at              timestamptz not null default now()
, modified_by              text        not null default current_user
);
