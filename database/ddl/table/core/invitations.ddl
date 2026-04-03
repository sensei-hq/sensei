set search_path to core, extensions;

create table if not exists invitations (
  id          uuid        primary key default gen_random_uuid()
, account_id  uuid        not null references core.accounts(id) on delete cascade
, email       text        not null
, role        text        not null default 'member'
              check (role in ('admin','member'))
, expires_at  timestamptz
, accepted_at timestamptz
, created_at  timestamptz not null default now()
);

create index if not exists invitations_account_idx on invitations(account_id);

comment on table invitations is
'Pending and accepted invitations to join an account.
- accepted_at: null while pending, set when the invite is claimed
- expires_at: set to 7 days from creation; expired invites are ignored at claim time';
