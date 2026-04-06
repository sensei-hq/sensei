set search_path to core, extensions;

create table if not exists accounts (
  id           uuid        primary key default gen_random_uuid()
, slug         text        not null unique
, display_name text        not null
, account_type text        not null default 'individual'
               check (account_type in ('individual','team','enterprise'))
, is_platform  boolean     not null default false
, created_at   timestamptz not null default now()
, modified_at  timestamptz not null default now()
, modified_by  text        not null default current_user
);

-- At most one platform account at a time
create unique index if not exists accounts_platform_ukey
  on accounts(is_platform)
  where is_platform = true;

-- Idempotent column add for existing databases
do $$ begin
  alter table accounts add column if not exists is_platform boolean not null default false;
exception when others then null;
end $$;

comment on table accounts is
'One row per sensei account (individual, team, or enterprise).
- slug: short human-readable identifier, unique across all accounts
- account_type: determines partition and feature set
- is_platform: exactly one account may have this true — the sensei platform account.
  Users of this account can access aggregate platform analytics (/platform).';

comment on column accounts.id is 'Surrogate primary key (UUID).';
comment on column accounts.slug is 'URL-safe identifier, unique within the relevant scope.';
comment on column accounts.display_name is 'Human-readable display name shown in the UI.';
comment on column accounts.account_type is 'Account tier: individual, team, or enterprise — determines partition and feature set.';
comment on column accounts.is_platform is 'True for the single sensei platform account whose members can access aggregate analytics.';
comment on column accounts.created_at is 'Timestamp when the row was first created.';
comment on column accounts.modified_at is 'Timestamp of the last modification to this row.';
comment on column accounts.modified_by is 'Identity (user, role, or service) that last modified this row.';
