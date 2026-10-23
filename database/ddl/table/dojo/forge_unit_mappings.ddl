set search_path to dojo, extensions;

-- Where one forge unit's repositories file (#308 R5a, D20): a GitHub org (an
-- Azure DevOps organization or project in R5b) is either ITS OWN dōjō — the one
-- its tenant_connections row names — or a WORKSPACE inside a dōjō someone
-- administers. One mapping per unit. No row means the default provisioning
-- makes: the unit's own connected dōjō.
--
-- Not a column of tenant_connections: a connection says how a dōjō is known to a
-- forge; a mapping says where a unit's repositories go, which may be a different
-- dōjō entirely. Written only by dojo.map_forge_unit, after the server has read
-- the forge with the mapper's own token.
create table if not exists dojo.forge_unit_mappings (
    id             uuid                primary key default gen_random_uuid()
  , provider       dojo.forge_provider not null
  , external_id    text                not null
  , external_slug  text                not null
  , tenant_id      uuid                not null references dojo.tenants(id) on delete cascade
  , workspace_id   uuid                references dojo.workspaces(id) on delete cascade
  , proven_at      timestamptz
  , mapped_by      uuid                not null references dojo.principals(id)
  , mapped_at      timestamptz         not null default now()
  , constraint forge_unit_mappings_unit_unique unique (provider, external_id)
);

-- Registration resolves a repository's unit by NAME: a git remote carries the
-- org's name and never its stable id.
create index if not exists forge_unit_mappings_slug_idx
    on dojo.forge_unit_mappings (provider, lower(external_slug));
create index if not exists forge_unit_mappings_tenant_idx
    on dojo.forge_unit_mappings (tenant_id);

comment on table dojo.forge_unit_mappings is
'Where a forge unit''s repositories file (#308 R5a): its own dōjō (workspace_id
null) or a workspace in another dōjō. One per unit; no row = the unit''s own
connected dōjō. Written only by dojo.map_forge_unit; read by
dojo.register_repository.';
comment on column dojo.forge_unit_mappings.external_id
     is 'The forge''s STABLE id for the unit — the key, never the renameable name.';
comment on column dojo.forge_unit_mappings.external_slug
     is 'The forge''s name for the unit when it was last mapped — what registration matches a repository''s org against.';
comment on column dojo.forge_unit_mappings.tenant_id
     is 'The dōjō the unit''s repositories file into.';
comment on column dojo.forge_unit_mappings.workspace_id
     is 'The workspace in tenant_id they file into; null when the unit is its own dōjō.';
comment on column dojo.forge_unit_mappings.proven_at
     is 'When the mapper was proven an admin of the unit at the forge; null for a mapping made without that proof (allowed only as a workspace of an unclaimed unit). A proven mapping is replaced only by another proven one.';
comment on column dojo.forge_unit_mappings.mapped_by
     is 'The principal who made the mapping — an admin of tenant_id.';

-- RLS on; the read policy calls a function and lives in
-- policies/dojo/forge_unit_mappings.sql. No session writes.
alter table dojo.forge_unit_mappings enable row level security;
