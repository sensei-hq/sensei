set search_path to dojo, extensions;

-- A repository, identified the way every install already identifies one.
--
-- `repo_key` is the normalized remote (`github.com/org/repo`) — identical for
-- every user of that repository on every machine, derivable with no central
-- registry. That is what lets two teammates converge on one row without
-- coordinating, and why the sync hangs off it rather than a local uuid.
--
-- ONE ROW PER (repo_key, tenant), not one globally. A consultant legitimately
-- has the same repository under two clients, and a fork under a personal account
-- alongside the org original. Scoping to the tenant keeps those separate without
-- forbidding the case.
create table if not exists repositories (
  id          uuid        primary key default gen_random_uuid()
, tenant_id   uuid        not null references dojo.tenants(id) on delete cascade
, repo_key    text        not null
, remote_url  text
, name        text        not null
  -- The FORGE's answer, captured at sign-in. NULLABLE and with NO DEFAULT, and
  -- both of those are load-bearing:
  --
  -- "not captured" must be its own state, because the two consumers of this
  -- column have OPPOSITE safe defaults. Entitlement wants to assume `private`
  -- (do not host unknown code free); authority wants to assume `public` (do not
  -- treat unknown code as org-mandated). No single default is safe, so an
  -- uncaptured repository has no authority, no election and no sync.
  --
  -- Verified before this was written: under the old `not null default 'private'`,
  -- `github.com/sensei-hq/dbd` — PUBLIC on GitHub — resolved to ORG-MANDATED, and
  -- would have been shared with no election by anyone.
, visibility  text
      check (visibility is null or visibility in ('private', 'public'))
  -- When that answer was captured. A capture that is too old is treated exactly
  -- as no capture: a repository turning private upstream would otherwise keep
  -- syncing FREE under a stale user election, because the "public is free" term
  -- fires before every billing check.
, visibility_captured_at timestamptz
  -- Which forge this repository lives on. Derived from `repo_key`'s host at
  -- registration and STORED, rather than re-derived in SQL: the host→provider
  -- mapping already exists once, in the registration path, and a second copy in
  -- a view would be two things to keep in step. Pulled forward from phase 2
  -- (spec §V.2) for exactly that reason; `external_id` stays in phase 2, since
  -- nothing yet knows the forge's id for a repository.
, provider    dojo.forge_provider not null
, created_at  timestamptz not null default now()
, updated_at  timestamptz not null default now()
, unique (tenant_id, repo_key)
);

comment on table repositories is
'The shared repository identity, keyed on the normalized remote so every user of
a repo lands on the same row. Scoped per tenant: the same repo may legitimately
appear under two clients.';

comment on column repositories.repo_key
     is 'Normalized remote — github.com/org/repo. Machine- and user-independent, which is what makes it a usable join key across installs.';

alter table repositories enable row level security;
drop policy if exists repositories_service_only on repositories;
create policy repositories_service_only on repositories
    for all to authenticated, anon using (false) with check (false);
