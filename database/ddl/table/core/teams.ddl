set search_path to core, extensions;

create table if not exists teams (
  id           uuid        primary key default gen_random_uuid()
, account_id   uuid        not null references core.accounts(id) on delete cascade
, slug         text        not null
, display_name text        not null
, created_at   timestamptz not null default now()
, modified_at  timestamptz not null default now()
, modified_by  text        not null default current_user
, unique (account_id, slug)
);

create index if not exists teams_account_idx on teams(account_id);

comment on table teams is
'Teams within an account. An account may have multiple teams; a user can
belong to teams across different accounts.
- slug: URL-safe identifier, unique per account (e.g. engineering, product)
- Account owners and admins have implicit visibility across all teams in their account';

comment on column teams.id is 'Surrogate primary key (UUID).';
comment on column teams.account_id is 'Account this team belongs to.';
comment on column teams.slug is 'URL-safe identifier, unique within the relevant scope.';
comment on column teams.display_name is 'Human-readable display name shown in the UI.';
comment on column teams.created_at is 'Timestamp when the row was first created.';
comment on column teams.modified_at is 'Timestamp of the last modification to this row.';
comment on column teams.modified_by is 'Identity (user, role, or service) that last modified this row.';
