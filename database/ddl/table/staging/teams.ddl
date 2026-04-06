set search_path to staging;

create table if not exists teams (
  account_slug text
, slug         text
, display_name text
, modified_at  timestamptz default now()
, modified_by  text
);

create unique index if not exists teams_natural_ukey on teams(account_slug, slug);

comment on column teams.account_slug  is 'Natural key of the parent account — joined to core.accounts to resolve account_id.';
comment on column teams.slug          is 'Team slug, unique per account.';
comment on column teams.display_name  is 'Human-readable team name to upsert into core.teams.';
comment on column teams.modified_at   is 'Source-side modification timestamp; used as freshness gate during import.';
comment on column teams.modified_by   is 'Source-side modifier identity; passed through to core.teams on upsert.';
