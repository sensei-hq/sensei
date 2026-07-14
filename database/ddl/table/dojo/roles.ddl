set search_path to dojo, extensions;

create table if not exists dojo.roles (
  id                 uuid             primary key default gen_random_uuid()
, tenant_id          uuid             references dojo.tenants(id)
, role               dojo.member_role not null
, git_provider_role  text
, description        text
, created_at         timestamptz      not null default now()
, updated_at         timestamptz      not null default now()
, constraint roles_tenant_git_role_unique unique (tenant_id, git_provider_role)
);

create index if not exists roles_tenant_idx on dojo.roles(tenant_id);

comment on table dojo.roles is
'Role definitions and the git-provider provisioning map. A row says "a member
whose git role is git_provider_role provisions as role". tenant_id null = a
global default rule; a non-null tenant_id = an admin override for that tenant.
Provisioning derives from the git provider by default (dojo-admin-console) and
admins may override.';

comment on column dojo.roles.tenant_id
     is 'Null for a global default mapping; set for a tenant-specific override.';
comment on column dojo.roles.role
     is 'The Dōjō role this maps to: contributor, maintainer, lead, or admin.';
comment on column dojo.roles.git_provider_role
     is 'The upstream git role that maps here (e.g. "write" -> contributor, "admin" -> admin). Null for a plain role definition row.';
