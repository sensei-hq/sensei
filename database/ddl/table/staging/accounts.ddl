set search_path to staging;

create table if not exists accounts (
  slug         text
, display_name text
, account_type text
, is_platform  boolean
, modified_at  timestamptz default now()
, modified_by  text
);

create unique index if not exists accounts_slug_ukey on accounts(slug);

comment on column accounts.slug         is 'Natural key — must match the target core.accounts slug.';
comment on column accounts.display_name is 'Human-readable name to upsert into core.accounts.';
comment on column accounts.account_type is 'Account category (individual, team, enterprise); defaults to individual if null.';
comment on column accounts.is_platform  is 'Marks the platform account; at most one may be true.';
comment on column accounts.modified_at  is 'Source-side modification timestamp; used as freshness gate during import.';
comment on column accounts.modified_by  is 'Source-side modifier identity; passed through to core.accounts on upsert.';
