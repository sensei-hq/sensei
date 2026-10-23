-- Azure DevOps repositories file by PROJECT (#308 R5b, D36): an organization is
-- a dōjō and each of its projects a workspace in it. An import maps each project
-- (dojo.map_forge_unit, keyed by the project's stable id, named `{org}/{project}`)
-- to a workspace of the organization's dōjō. dojo.register_repository resolves an
-- Azure repository to its PROJECT's mapping first, then its organization's, then
-- the organization's connection — and unmapped stays unmapped.
--
-- WHAT BREAKS THIS TEST: an Azure repository ignoring its project's mapping; a
-- project in one organization catching a same-named project of another; a
-- project the import did not map filing anywhere but the organization's dōjō; a
-- caller naming a project the key does not carry; GitHub registration changing.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000c01', 'a1000000-0000-4000-8000-000000000c01', 'ztest-az-admin'),
  ('a0000000-0000-4000-8000-000000000c02', 'a1000000-0000-4000-8000-000000000c02', 'ztest-az-outsider');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-000000000c0a', 'organization/ztest-az-org', 'organization', 'ztest-az-org', 'Org', 'x');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-000000000c0a', 'a0000000-0000-4000-8000-000000000c01', 'admin', 'employer', 'sso');
insert into dojo.tenant_connections (tenant_id, provider, external_id, external_slug, connected_by, verified_at) values
  ('b0000000-0000-4000-8000-000000000c0a', 'azure_devops', 'ztest-az-org-id', 'ztest-az-org',
   'a0000000-0000-4000-8000-000000000c01', now());

-- The import: two projects, each a workspace of the organization's dōjō.
set local role service_role;
do $$
begin
    perform dojo.map_forge_unit('a0000000-0000-4000-8000-000000000c01', 'azure_devops', 'ztest-proj-1',
        'ztest-az-org/Mobile Apps', 'b0000000-0000-4000-8000-000000000c0a', 'mobile-apps', 'Mobile Apps', true);
    perform dojo.map_forge_unit('a0000000-0000-4000-8000-000000000c01', 'azure_devops', 'ztest-proj-2',
        'ztest-az-org/platform', 'b0000000-0000-4000-8000-000000000c0a', 'platform', 'platform', true);
end $$;
reset role;

set local role authenticated;
do $$
declare
    r  record;
    ws uuid;
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000c01"}', true);

    -- The https form, a project name with a space.
    select w.id into ws from dojo.workspaces w
     where w.tenant_id = 'b0000000-0000-4000-8000-000000000c0a' and w.slug = 'mobile-apps';
    select * into r from dojo.register_repository('dev.azure.com/ztest-az-org/mobile%20apps/_git/ios', 'azure_devops',
        'ztest-az-org', 'ios', null, 'mobile apps');
    if r.reason is not null or r.tenant_id <> 'b0000000-0000-4000-8000-000000000c0a' or r.workspace_id is distinct from ws then
        raise exception 'azure: a repository did not file into its project''s workspace, got %', r;
    end if;

    -- The SSH form of the other project.
    select w.id into ws from dojo.workspaces w
     where w.tenant_id = 'b0000000-0000-4000-8000-000000000c0a' and w.slug = 'platform';
    select * into r from dojo.register_repository('ssh.dev.azure.com/v3/ztest-az-org/platform/api', 'azure_devops',
        'ztest-az-org', 'api', null, 'platform');
    if r.reason is not null or r.workspace_id is distinct from ws then
        raise exception 'azure: the SSH form did not file by project, got %', r;
    end if;

    -- A project the import did not map files into the organization's dōjō, no workspace.
    select * into r from dojo.register_repository('dev.azure.com/ztest-az-org/sandbox/_git/play', 'azure_devops',
        'ztest-az-org', 'play', null, 'sandbox');
    if r.reason is not null or r.tenant_id <> 'b0000000-0000-4000-8000-000000000c0a' or r.workspace_id is not null then
        raise exception 'azure: an unmapped project did not fall back to the organization''s dōjō, got %', r;
    end if;

    -- A same-named project in ANOTHER organization is not this one's.
    select * into r from dojo.register_repository('dev.azure.com/ztest-az-other/platform/_git/api', 'azure_devops',
        'ztest-az-other', 'api', null, 'platform');
    if r.reason is distinct from 'no_connection' then
        raise exception 'azure: another organization''s project filed here, got %', r;
    end if;

    -- The caller cannot name a project the key does not carry.
    begin
        perform dojo.register_repository('dev.azure.com/ztest-az-org/sandbox/_git/play', 'azure_devops',
            'ztest-az-org', 'play', null, 'mobile apps');
        raise exception 'azure: a caller filed a repository under a project its key does not name';
    exception when invalid_parameter_value then null;
    end;

    -- Someone outside the dōjō is told so, and nothing is filed for them.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000c02"}', true);
    select * into r from dojo.register_repository('dev.azure.com/ztest-az-org/platform/_git/web', 'azure_devops',
        'ztest-az-org', 'web', null, 'platform');
    if r.reason is distinct from 'not_a_member' then
        raise exception 'azure: an outsider was not refused, got %', r;
    end if;
end $$;
reset role;

do $$
begin
    if exists (select 1 from dojo.repositories where repo_key = 'dev.azure.com/ztest-az-org/platform/_git/web') then
        raise exception 'azure: an outsider''s repository was filed';
    end if;
end $$;

rollback;
