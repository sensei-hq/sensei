set search_path to dojo, extensions;

-- A group within a tenant. ACCESS is granted here, not at the tenant.
--
-- A company runs several teams on different projects, so "everyone in the org
-- can see every repository's numbers" is exactly what this level exists to
-- prevent. Tenant membership is billing and identity; team membership is access.
--
-- Every tenant gets a DEFAULT team containing everyone on creation, so a small
-- org never encounters the concept — the schema is here from the start (adding
-- it later would be an access migration, which is the kind that goes wrong
-- quietly), while the management UI arrives when it is wanted.
create table if not exists teams (
  id          uuid        primary key default gen_random_uuid()
, tenant_id   uuid        not null references dojo.tenants(id) on delete cascade
, name        text        not null
  -- The auto-created catch-all. Exactly one per tenant, enforced below: two
  -- defaults would make "who is in the fallback team" ambiguous.
, is_default  boolean     not null default false
, created_at  timestamptz not null default now()
, unique (tenant_id, name)
);

create unique index if not exists teams_one_default_per_tenant
    on teams(tenant_id) where is_default;

comment on table teams is
'A group within a tenant, and the level at which repository access is granted.
Every tenant has exactly one default team so the concept can be ignored until an
admin needs it.';

alter table teams enable row level security;
drop policy if exists teams_service_only on teams;
create policy teams_service_only on teams
    for all to authenticated, anon
    using (false) with check (false);
