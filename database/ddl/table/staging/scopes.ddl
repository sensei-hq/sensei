set search_path to staging, extensions;

drop table if exists scopes cascade;
create table scopes (
  key                      text
, name                     text
, level                    integer
, shareable                boolean     default false
, description              text
, modified_at              timestamptz not null default now()
);
