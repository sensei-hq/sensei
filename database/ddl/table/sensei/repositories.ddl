set search_path to sensei, extensions;

-- Canonical, GLOBAL repository registry — the metric grain. A repository is
-- identified by its NORMALIZED REMOTE (`repo_key`), never by a local path, so two
-- clones of one remote, a rename, or a re-checkout all resolve to ONE repository
-- and its metric history survives a folder prune/move (D10). project_metrics key
-- on repository_id; a project is a GROUP of repositories (M:N via the folders
-- junction) and a project metric is an aggregation view over its repositories.
--
-- NOT owned by any project (no project_id) — a CANONICAL repository may be
-- checked out more than once and each checkout serve a different project.
-- Measured 2026-09-30: 4 repositories do exactly that (`kavach` serves projects
-- `kavach` and `vite-multi-adapter`; `bridge` serves `bridge` and `sparsh`), so a
-- project_id here would force those to pick one and silently drop the other.
--
-- DECISION CHANGED 2026-09-30 (supersedes the folder-grain junction in D1/D2/D10):
-- the project is owned by the repo-ANCHOR FOLDER — the checkout — and every folder
-- beneath it DERIVES its project from that anchor. The junction is therefore
-- per-checkout, not per-folder. Before this, project_id was settable independently
-- on all 13,697 folders and one repository had already drifted: the client-q
-- documentation checkout's root sat in project `client-q` while its 362 subfolders
-- sat in `documentation`.
--
-- folders.repository_id points HERE, and only the repo-root/checkout folder carries
-- it (I16); subfolders resolve via nearest ancestor (repo_anchor_for).
create table if not exists repositories (
  id           uuid        primary key default gen_random_uuid()
, repo_key     text        unique
, remote_url   text
, name         text        not null
, tenant_id    uuid
, created_at   timestamptz not null default now()
, modified_at  timestamptz not null default now()
, visibility   repo_visibility not null default 'private'
, synced_at    timestamptz
);

comment on table repositories is
'Canonical global repository registry — the metric grain. Identified by repo_key
(the normalized remote: git@host:Org/Repo.git and https://host/Org/Repo both →
host/org/repo; scheme/creds/port/.git stripped, host lowercased). One row per real
repository regardless of how many times or where it is checked out. A UNIQUE repo_key
that is NULL means a local-only repo with no remote — never federated, and multiple
such rows coexist (nulls distinct).

NO OWNING PROJECT, and that is about CHECKOUTS rather than ownership: one canonical
repository can be checked out twice and each checkout belong to a different project.
Measured 2026-09-30, 4 repositories do — `kavach` serves `kavach` and
`vite-multi-adapter`, `bridge` serves `bridge` and `sparsh` — so a project_id on
this table would collapse them.

The project is owned by the repo-ANCHOR FOLDER (the checkout); every folder beneath
derives from it. One authoritative row per checkout, not per folder. That supersedes
the folder-grain junction described in D1/D2/D10 — see the header for why.';

comment on column repositories.repo_key
     is 'Normalized remote identity (host/org/repo, lowercased). Unique. NULL = local-only (no remote) — never federated; multiple NULLs coexist.';
comment on column repositories.remote_url
     is 'A representative raw remote URL for display / re-derivation; repo_key is the identity.';
comment on column repositories.name
     is 'Display name — typically the repository basename.';
comment on column repositories.tenant_id
     is 'The dojo.tenants.id this repository is enrolled with when federated. NULL = not federated. Distinct from projects.dojo_id, which holds a MEMBERSHIP id — the ambiguity that forced this rename.

The dōjō TENANT this repository is enrolled with. Named `dojo_id` until the sync slice needed to store one: `projects.dojo_id` holds a MEMBERSHIP id, so one name meant two things and the plan consumer could not say which it had. Plain uuid, no FK — the referent lives in another database.';
comment on column repositories.created_at
     is 'When the repository was first registered.';
comment on column repositories.modified_at
     is 'Timestamp of the last modification to this row.';
comment on column repositories.visibility
     is 'Whether this repository participates in sync. Private by default: sync is gated on authentication, but signing in should not silently start sharing a repo the user never chose to share.';
