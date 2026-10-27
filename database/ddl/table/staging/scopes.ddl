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

-- Row level security (the Supabase advisor, 2026-10-27), with no client policy.
-- An import staging table: written and read by the import alone, as the owner; a client role reads nothing here. Inert on the daemon, which connects
-- as the owner.
alter table staging.scopes enable row level security;
