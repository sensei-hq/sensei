set search_path to staging;

create table if not exists repos (
  id                  uuid
, name                text
, remote_url          text
, default_branch      text
, description         text
, stack               text[]
, entry_points        jsonb
, last_indexed_commit text
, last_indexed_at     timestamptz
, is_public           boolean
, modified_at         timestamptz default now()
, modified_by         text
);

create unique index if not exists repos_id_ukey on repos(id);

comment on table repos is 'Intermediate import buffer for bulk-loading rows into sensei.repos.';

comment on column repos.id                  is 'UUID that will become the surrogate primary key in sensei.repos.';
comment on column repos.name                is 'Human-readable repository name (e.g. "sensei" or "my-org/my-repo").';
comment on column repos.remote_url          is 'Git remote URL used to uniquely identify the repo in sensei.repos.';
comment on column repos.default_branch      is 'Default git branch for this repo (e.g. "main" or "master").';
comment on column repos.description         is 'Short human-readable description of the repo purpose.';
comment on column repos.stack               is 'Array of detected technology stack identifiers (e.g. ["typescript", "react"]).';
comment on column repos.entry_points        is 'JSON array of entry-point descriptors [{path, role, inferredRole}] matching IndexSummary shape.';
comment on column repos.last_indexed_commit is 'Git commit SHA of the most recent completed index run.';
comment on column repos.last_indexed_at     is 'Timestamp when the most recent index run completed.';
comment on column repos.is_public           is 'True if the repo is publicly accessible; used for multi-tenant RLS in sensei.repos.';
comment on column repos.modified_at         is 'Source-side modification timestamp; used as freshness gate during import.';
comment on column repos.modified_by         is 'Source-side modifier identity; passed through to sensei.repos on upsert.';
