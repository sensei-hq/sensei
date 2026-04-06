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
, modified_by         text        not null default current_user
);

create unique index if not exists repos_remote_url_ukey on repos(remote_url)
  where remote_url is not null;

comment on table repos is
'Central registry of indexed repos.
- One row per repo, identified by remote_url (git remote)
- entry_points is [{path, role, inferredRole}] — matches IndexSummary shape
- is_public stub for future multi-tenant RLS';

comment on column repos.id is 'Surrogate primary key (UUID).';
comment on column repos.name is 'Human-readable repository name (e.g. "sensei" or "my-org/my-repo").';
comment on column repos.remote_url is 'Git remote URL used to uniquely identify the repo (unique, nullable).';
comment on column repos.default_branch is 'Default git branch for this repo (e.g. "main" or "master").';
comment on column repos.description is 'Short human-readable description of the repo purpose.';
comment on column repos.stack is 'Array of detected technology stack identifiers (e.g. ["typescript", "react"]).';
comment on column repos.entry_points is 'JSON array of entry-point descriptors [{path, role, inferredRole}] matching IndexSummary shape.';
comment on column repos.last_indexed_commit is 'Git commit SHA of the most recent completed index run.';
comment on column repos.last_indexed_at is 'Timestamp when the most recent index run completed.';
comment on column repos.is_public is 'Stub flag for future multi-tenant RLS — true if the repo is publicly accessible.';
comment on column repos.owner_id is 'UUID of the user or org that owns this repo (nullable; reserved for multi-tenant use).';
comment on column repos.created_at is 'Timestamp when the row was first created.';
comment on column repos.modified_at is 'Timestamp of the last modification to this row.';
comment on column repos.modified_by is 'Identity (user, role, or service) that last modified this row.';
