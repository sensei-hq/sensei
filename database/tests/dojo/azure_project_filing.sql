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
    -- A second unit that held the same name, filed to the SAME place: not ambiguous.
    perform dojo.map_forge_unit('a0000000-0000-4000-8000-000000000c01', 'azure_devops', 'ztest-proj-2-old',
        'ztest-az-org/platform', 'b0000000-0000-4000-8000-000000000c0a', 'platform', 'platform', true);
    perform dojo.map_forge_unit('a0000000-0000-4000-8000-000000000c01', 'azure_devops', 'ztest-proj-3',
        'ztest-az-org/R&D', 'b0000000-0000-4000-8000-000000000c0a', 'r-d', 'R&D', true);
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

    -- The project is read at ITS place in the key, not anywhere: a repository
    -- named after another project cannot move into that project's workspace.
    begin
        perform dojo.register_repository('dev.azure.com/ztest-az-org/sandbox/_git/platform', 'azure_devops',
            'ztest-az-org', 'platform', null, 'platform');
        raise exception 'azure: a repository named like a project filed into that project';
    exception when invalid_parameter_value then null;
    end;
    -- …and the organization at its place too.
    begin
        perform dojo.register_repository('dev.azure.com/other/ztest-az-org/_git/x', 'azure_devops',
            'ztest-az-org', 'x', null, null);
        raise exception 'azure: an org named as a project was read as the org';
    exception when invalid_parameter_value then null;
    end;
    -- A project name the key percent-encodes is still that project.
    select w.id into ws from dojo.workspaces w
     where w.tenant_id = 'b0000000-0000-4000-8000-000000000c0a' and w.slug = 'r-d';
    select * into r from dojo.register_repository('dev.azure.com/ztest-az-org/r%26d/_git/lab', 'azure_devops',
        'ztest-az-org', 'lab', null, 'r&d');
    if r.reason is not null or r.workspace_id is distinct from ws then
        raise exception 'azure: a percent-encoded project did not file by its mapping, got %', r;
    end if;

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

-- An import keeps a project an admin filed into ANOTHER dōjō where it is — the
-- database enforces it (p_keep_elsewhere), not only the server's pre-read
-- (#308 R5b review).
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-000000000c0b', 'organization/ztest-az-elsewhere', 'organization', 'ztest-az-elsewhere', 'Elsewhere', 'x');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-000000000c0b', 'a0000000-0000-4000-8000-000000000c01', 'admin', 'employer', 'sso');
set local role service_role;
do $$
declare
    r record;
    n bigint;
begin
    perform dojo.map_forge_unit('a0000000-0000-4000-8000-000000000c01', 'azure_devops', 'ztest-proj-away',
        'ztest-az-org/Away', 'b0000000-0000-4000-8000-000000000c0b', 'away', 'Away', true);
    select count(*) into n from dojo.audit_events where tenant_id = 'b0000000-0000-4000-8000-000000000c0a';

    select * into r from dojo.map_forge_unit('a0000000-0000-4000-8000-000000000c01', 'azure_devops', 'ztest-proj-away',
        'ztest-az-org/Away', 'b0000000-0000-4000-8000-000000000c0a', 'away', 'Away', true, true);
    if r.tenant_id <> 'b0000000-0000-4000-8000-000000000c0b' then
        raise exception 'keep elsewhere: the answer should name the dōjō it stays in, got %', r;
    end if;
    if (select tenant_id from dojo.forge_unit_mappings where external_id = 'ztest-proj-away')
       <> 'b0000000-0000-4000-8000-000000000c0b' then
        raise exception 'keep elsewhere: the project was moved';
    end if;
    if exists (select 1 from dojo.workspaces where tenant_id = 'b0000000-0000-4000-8000-000000000c0a' and slug = 'away') then
        raise exception 'keep elsewhere: a workspace was made for a project that stayed away';
    end if;
    if (select count(*) from dojo.audit_events where tenant_id = 'b0000000-0000-4000-8000-000000000c0a') <> n then
        raise exception 'keep elsewhere: a mapping that did not happen was audited';
    end if;

    -- Without the flag, a proven admin still moves it (R5a re-mapping is unchanged).
    select * into r from dojo.map_forge_unit('a0000000-0000-4000-8000-000000000c01', 'azure_devops', 'ztest-proj-away',
        'ztest-az-org/Away', 'b0000000-0000-4000-8000-000000000c0a', 'away', 'Away', true);
    if r.tenant_id <> 'b0000000-0000-4000-8000-000000000c0a' then
        raise exception 'keep elsewhere: an explicit re-map was refused, got %', r;
    end if;
end $$;
reset role;

-- Two maps of the SAME unit serialise (#308 R5b review 2): with no mapping row
-- yet there is nothing for FOR UPDATE to lock, so map_forge_unit takes a
-- transaction-scoped advisory lock on (provider, unit) before it reads.
set local role service_role;
do $$
declare
    k bigint := hashtext('azure_devops:ztest-proj-locked')::bigint;
begin
    perform dojo.map_forge_unit('a0000000-0000-4000-8000-000000000c01', 'azure_devops', 'ztest-proj-locked',
        'ztest-az-org/Locked', 'b0000000-0000-4000-8000-000000000c0a', 'locked', 'Locked', true, true);
    if not exists (
        select 1 from pg_locks l
         where l.locktype = 'advisory' and l.pid = pg_backend_pid() and l.granted
           and l.objid::bigint = (k & 4294967295) and l.classid::bigint = ((k >> 32) & 4294967295)
    ) then
        raise exception 'map: no advisory lock on (provider, unit) is held';
    end if;
end $$;
reset role;

rollback;
