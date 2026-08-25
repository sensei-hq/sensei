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
, visibility  text        not null default 'private'
      check (visibility in ('private', 'public'))
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
