set search_path to core, extensions;

create table if not exists profile_accounts (
  user_id    uuid not null references auth.users(id) on delete cascade
, account_id uuid not null references core.accounts(id) on delete cascade
, role       text not null default 'member'
             check (role in ('owner','admin','member'))
, joined_at   timestamptz not null default now()
, modified_at  timestamptz not null default now()
, modified_by  text        not null default current_user
, primary key (user_id, account_id)
);

create index if not exists profile_accounts_account_idx on profile_accounts(account_id);

comment on table profile_accounts is
'Maps Supabase auth users to accounts with a role.
- A user may belong to multiple accounts
- role: owner (one per account), admin, or member';

comment on column profile_accounts.user_id is 'Foreign key to auth.users; identifies the user member of this account.';
comment on column profile_accounts.account_id is 'Foreign key to accounts; identifies the account the user belongs to.';
comment on column profile_accounts.role is 'Role within the account: owner (one per account), admin, or member.';
comment on column profile_accounts.joined_at is 'Timestamp when the user first joined the account.';
comment on column profile_accounts.modified_at is 'Timestamp of the last modification to this row.';
comment on column profile_accounts.modified_by is 'Identity (user, role, or service) that last modified this row.';
