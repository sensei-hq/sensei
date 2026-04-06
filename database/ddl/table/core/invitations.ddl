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
, modified_at timestamptz not null default now()
, modified_by text        not null default current_user
);

create index if not exists invitations_account_idx on invitations(account_id);

comment on table invitations is
'Pending and accepted invitations to join an account.
- accepted_at: null while pending, set when the invite is claimed
- expires_at: set to 7 days from creation; expired invites are ignored at claim time';

comment on column invitations.id is 'Surrogate primary key (UUID).';
comment on column invitations.account_id is 'Account the invitee is being invited to join.';
comment on column invitations.email is 'Email address of the person being invited.';
comment on column invitations.role is 'Role the invitee will receive upon accepting: admin or member.';
comment on column invitations.expires_at is 'Timestamp after which the invitation is no longer claimable (typically 7 days from creation).';
comment on column invitations.accepted_at is 'Timestamp when the invitation was claimed; null while still pending.';
comment on column invitations.created_at is 'Timestamp when the row was first created.';
comment on column invitations.modified_at is 'Timestamp of the last modification to this row.';
comment on column invitations.modified_by is 'Identity (user, role, or service) that last modified this row.';
