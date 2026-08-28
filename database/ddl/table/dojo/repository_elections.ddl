set search_path to dojo, extensions;

-- WHO chose to share a repository, on whose authority.
--
-- Separate from `dojo.repositories.visibility`, which is the FORGE's answer.
-- Sharing is two questions — may it (entitlement) and did anyone choose it
-- (election) — and this table is the second. Conflating them produced a view
-- that reported every repository as shareable.
create table if not exists repository_elections (
  id            uuid                  primary key default gen_random_uuid()
      -- A surrogate key because the natural one cannot serve: `principal_id` is
      -- NULL by design for an organisation's election, and a primary key admits
      -- no NULLs.
, tenant_id     uuid                  not null references dojo.tenants(id)      on delete cascade
, repository_id uuid                  not null references dojo.repositories(id) on delete cascade
, authority     dojo.share_authority  not null
      -- The electing member. NULL when the ORGANISATION elected — an org speaks
      -- as itself, not as the admin who happened to click.
, principal_id  uuid                  references dojo.principals(id) on delete cascade
, elected       boolean               not null
, elected_at    timestamptz           not null default now()
, created_at    timestamptz           not null default now()
, modified_at   timestamptz           not null default now()
      -- A user's election and an org's mandate are DIFFERENT ROWS, so a
      -- repository going public (authority organization → user) does not silently
      -- convert one into the other. NULLS NOT DISTINCT so the org's single
      -- NULL-principal row collides with itself rather than duplicating.
, unique nulls not distinct (repository_id, authority, principal_id)
      -- An org election names no principal; a user election must.
, constraint repository_elections_principal_matches_authority
      check ((authority = 'organization' and principal_id is null)
          or (authority = 'user'         and principal_id is not null))
);

create index if not exists repository_elections_repo_idx
    on repository_elections (repository_id, authority);

comment on table repository_elections is
'Who chose to share a repository. One row per (repository, authority, principal),
so a member''s election and their organisation''s mandate coexist rather than
overwrite — which is what stops an election made under one authority from
silently surviving a change of authority.

Absent row = NOT elected. Never elected-by-default: the org''s default lives in
dojo.tenant_share_policy and is itself false.';

comment on column repository_elections.authority
     is 'Whose decision this row records. Must agree with the derived authority for the repository at the time it is read — a stale row for the other authority is kept, not applied.';
comment on column repository_elections.principal_id
     is 'The electing member, NULL for an organisation''s mandate. The CHECK keeps the two shapes from being mixed up.';
