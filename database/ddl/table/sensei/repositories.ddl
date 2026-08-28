set search_path to sensei, extensions;

-- Canonical, GLOBAL repository registry — the metric grain. A repository is
-- identified by its NORMALIZED REMOTE (`repo_key`), never by a local path, so two
-- clones of one remote, a rename, or a re-checkout all resolve to ONE repository
-- and its metric history survives a folder prune/move (D10). project_metrics key
-- on repository_id; a project is a GROUP of repositories (M:N via the folders
-- junction) and a project metric is an aggregation view over its repositories.
--
-- NOT owned by any project (no project_id) — a repository may belong to several
-- projects. folders.repository_id points HERE, and only the repo-root/checkout
-- folder carries it (I16); subfolders resolve via nearest ancestor (repo_anchor_for).
create table if not exists repositories (
  id           uuid        primary key default gen_random_uuid()
, repo_key     text        unique
, remote_url   text
, name         text        not null
  -- The dōjō TENANT this repository is enrolled with. Named `dojo_id` until the
  -- sync slice needed to store one: `projects.dojo_id` holds a MEMBERSHIP id, so
  -- one name meant two things and the plan consumer could not say which it had.
  -- Plain uuid, no FK — the referent lives in another database.
, tenant_id    uuid
, created_at   timestamptz not null default now()
, modified_at  timestamptz not null default now()
  -- Whether this repository participates in sync. Private by default: sync is
  -- gated on authentication, but signing in should not silently start sharing a
  -- repo the user never chose to share.
, visibility   repo_visibility not null default 'private'
, synced_at    timestamptz
);

comment on table repositories is
'Canonical global repository registry — the metric grain. Identified by repo_key
(the normalized remote: git@host:Org/Repo.git and https://host/Org/Repo both →
host/org/repo; scheme/creds/port/.git stripped, host lowercased). One row per real
repository regardless of how many times or where it is checked out. A UNIQUE repo_key
that is NULL means a local-only repo with no remote — never federated, and multiple
such rows coexist (nulls distinct). No owning project: folders.repository_id
references this and a project aggregates repositories via the folders junction
(D1/D2/D10).';

comment on column repositories.repo_key
     is 'Normalized remote identity (host/org/repo, lowercased). Unique. NULL = local-only (no remote) — never federated; multiple NULLs coexist.';
comment on column repositories.remote_url
     is 'A representative raw remote URL for display / re-derivation; repo_key is the identity.';
comment on column repositories.name
     is 'Display name — typically the repository basename.';
comment on column repositories.tenant_id
     is 'The dojo.tenants.id this repository is enrolled with when federated. NULL = not federated. Distinct from projects.dojo_id, which holds a MEMBERSHIP id — the ambiguity that forced this rename.';
comment on column repositories.created_at
     is 'When the repository was first registered.';
comment on column repositories.modified_at
     is 'Timestamp of the last modification to this row.';
