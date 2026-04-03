set search_path to core, extensions;

create table if not exists profile_accounts (
  user_id    uuid not null references auth.users(id) on delete cascade
, account_id uuid not null references core.accounts(id) on delete cascade
, role       text not null default 'member'
             check (role in ('owner','admin','member'))
, joined_at  timestamptz not null default now()
, primary key (user_id, account_id)
);

create index if not exists profile_accounts_account_idx on profile_accounts(account_id);

comment on table profile_accounts is
'Maps Supabase auth users to accounts with a role.
- A user may belong to multiple accounts
- role: owner (one per account), admin, or member';
