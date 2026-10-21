set search_path to dojo, extensions;

-- A named group of a dōjō's members (#307 D7): for "by team" reports and for
-- granting the workspace viewer to many people at once. A team grants NO access —
-- repository roles do (D5); this superseded the 2026-10-12 design in which team
-- membership was the access path. Written through dojo.create_team / rename_team /
-- delete_team (admins).
create table if not exists teams (
  id          uuid        primary key default gen_random_uuid()
, tenant_id   uuid        not null references dojo.tenants(id) on delete cascade
, name        text        not null
, is_default  boolean     not null default false
, created_at  timestamptz not null default now()
, unique (tenant_id, name)
);

create unique index if not exists teams_one_default_per_tenant
    on teams(tenant_id) where is_default;

comment on table teams is
'A named group of a dōjō''s members, for reports and bulk viewer grants (#307 D7).
Grants no access — repository roles do (D5). RLS in policies/dojo/teams.sql.';

comment on column teams.is_default
     is 'Reserved for a catch-all team; at most one per tenant. Unused since teams carry no access (#307 D7).';
