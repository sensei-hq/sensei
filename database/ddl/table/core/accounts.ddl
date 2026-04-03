set search_path to core, extensions;

create table if not exists accounts (
  id           uuid        primary key default gen_random_uuid()
, slug         text        not null unique
, display_name text        not null
, account_type text        not null default 'individual'
               check (account_type in ('individual','team','enterprise'))
, created_at   timestamptz not null default now()
);

comment on table accounts is
'One row per sensei account (individual, team, or enterprise).
- slug: short human-readable identifier, unique across all accounts
- account_type: determines partition and feature set';
