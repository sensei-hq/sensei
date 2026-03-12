set search_path to sensei, extensions;

create table if not exists repos (
  id                  uuid primary key default gen_random_uuid()
, name                text not null
, remote_url          text
, default_branch      text
, description         text
, stack               text[]
, entry_points        jsonb
, last_indexed_commit text
, last_indexed_at     timestamptz
, is_public           boolean not null default false
, owner_id            uuid
, created_at          timestamptz not null default now()
, modified_at         timestamptz not null default now()
);

create unique index if not exists repos_remote_url_ukey on repos(remote_url)
  where remote_url is not null;

comment on table repos is
'Central registry of indexed repos.
- One row per repo, identified by remote_url (git remote)
- entry_points is [{path, role, inferredRole}] — matches IndexSummary shape
- is_public stub for future multi-tenant RLS';
