set search_path to staging, extensions;

drop table if exists assistant_catalogue cascade;
create table assistant_catalogue (
  id                       text
, name                     text
, kind                     text
, aliases                  text[]      default '{}'
, joins_crew               boolean     default true
, sequence                 integer
, modified_at              timestamptz not null default now()
);
