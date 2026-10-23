-- Mapping a forge unit to a dōjō or to a workspace of one, and repositories
-- filing by that mapping (#308 R5a, D20).
--
-- Each GitHub org maps to ITS OWN dōjō (the default provisioning makes) or to a
-- WORKSPACE in a dōjō the mapper administers. One mapping per unit. Making a unit
-- its own dōjō needs proof the mapper administers the org; filing it as a
-- workspace does not — unless the org's dōjō is claimed, or an admin of the org
-- already placed it, in which case only an admin of the org may move it.
-- Registration follows the mapping: a unit mapped as a workspace files its
-- repositories into that workspace of the target dōjō, for members of that dōjō,
-- and nowhere else.
--
-- WHAT BREAKS THIS TEST: a non-admin of the target dōjō mapping into it; an
-- unproven mapping of a claimed org, or over a proven mapping; an own-dōjō
-- mapping without proof, or naming a dōjō the org is not connected to; two rows
-- for one unit; a mapping with no audit row; a signed-in caller mapping or
-- writing the table; a contributor reading the dōjō's mappings; registration
-- ignoring the mapping, filing into the org's own dōjō, or not filing the
-- workspace.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000601', 'a1000000-0000-4000-8000-000000000601', 'ada'),
  ('a0000000-0000-4000-8000-000000000602', 'a1000000-0000-4000-8000-000000000602', 'con'),
  ('a0000000-0000-4000-8000-000000000603', 'a1000000-0000-4000-8000-000000000603', 'cee');

-- A: the dōjō ada administers. B: org B's own dōjō, unclaimed. C: org C's, claimed.
insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-00000000060a', 'organization/ztest-map-a', 'organization', 'ztest-map-a', 'A', 'x', now()),
  ('b0000000-0000-4000-8000-00000000060b', 'organization/ztest-map-b', 'organization', 'ztest-map-b', 'B', 'x', null),
  ('b0000000-0000-4000-8000-00000000060c', 'organization/ztest-map-c', 'organization', 'ztest-map-c', 'C', 'x', now());
insert into dojo.tenant_connections (tenant_id, provider, external_id, external_slug, connected_by) values
  ('b0000000-0000-4000-8000-00000000060a', 'github', 'ztest-gh-60a', 'ztest-map-orga', 'a0000000-0000-4000-8000-000000000601'),
  ('b0000000-0000-4000-8000-00000000060b', 'github', 'ztest-gh-60b', 'ztest-map-orgb', 'a0000000-0000-4000-8000-000000000601'),
  ('b0000000-0000-4000-8000-00000000060c', 'github', 'ztest-gh-60c', 'ztest-map-orgc', 'a0000000-0000-4000-8000-000000000601');
-- ada: admin of A and of B. con: a contributor of A. cee: a contributor of C only.
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000060a', 'a0000000-0000-4000-8000-000000000601', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000060b', 'a0000000-0000-4000-8000-000000000601', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000060a', 'a0000000-0000-4000-8000-000000000602', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000060c', 'a0000000-0000-4000-8000-000000000603', 'contributor', 'employer', 'github_oauth');

-- A signed-in caller can neither map nor write the table: the forge proof is the
-- server's.
set local role authenticated;
do $$
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000601"}', true);
    begin
        perform dojo.map_forge_unit('a0000000-0000-4000-8000-000000000601', 'github', 'ztest-gh-60b', 'ztest-map-orgb',
                                    'b0000000-0000-4000-8000-00000000060a', 'orgb', 'Org B', true);
        raise exception 'map: a signed-in caller mapped a unit';
    exception when insufficient_privilege then null;
    end;
    begin
        insert into dojo.forge_unit_mappings (provider, external_id, external_slug, tenant_id, mapped_by)
        values ('github', 'ztest-gh-60b', 'ztest-map-orgb', 'b0000000-0000-4000-8000-00000000060a',
                'a0000000-0000-4000-8000-000000000601');
        raise exception 'map: a signed-in caller wrote a mapping directly';
    exception when insufficient_privilege then null;
    end;
end $$;
reset role;

set local role service_role;
do $$
declare
    r record;
    ws_b uuid;
begin
    -- Only an admin of the TARGET dōjō files a unit into it.
    begin
        perform dojo.map_forge_unit('a0000000-0000-4000-8000-000000000602', 'github', 'ztest-gh-60b', 'ztest-map-orgb',
                                    'b0000000-0000-4000-8000-00000000060a', 'orgb', 'Org B', true);
        raise exception 'map: a contributor of A mapped a unit into A';
    exception when insufficient_privilege then null;
    end;

    -- Unproven, into a workspace of A: allowed while org B's dōjō is unclaimed.
    select * into r from dojo.map_forge_unit('a0000000-0000-4000-8000-000000000601', 'github', 'ztest-gh-60b', 'ztest-map-orgb',
                                             'b0000000-0000-4000-8000-00000000060a', 'orgb', 'Org B', false);
    ws_b := r.workspace_id;
    if r.tenant_id <> 'b0000000-0000-4000-8000-00000000060a' or ws_b is null then
        raise exception 'map: org B should be a workspace of A, got %', r;
    end if;
    if (select tenant_id from dojo.workspaces where id = ws_b) <> 'b0000000-0000-4000-8000-00000000060a'
       or (select slug from dojo.workspaces where id = ws_b) <> 'orgb'
       or (select classification::text from dojo.workspaces where id = ws_b) <> 'company' then
        raise exception 'map: the workspace was not made in A as orgb (company)';
    end if;

    -- Unproven over a CLAIMED org: refused.
    begin
        perform dojo.map_forge_unit('a0000000-0000-4000-8000-000000000601', 'github', 'ztest-gh-60c', 'ztest-map-orgc',
                                    'b0000000-0000-4000-8000-00000000060a', 'orgc', 'Org C', false);
        raise exception 'map: a claimed org was moved without proof';
    exception when insufficient_privilege then null;
    end;
    -- Proven: an admin of org C may.
    select * into r from dojo.map_forge_unit('a0000000-0000-4000-8000-000000000601', 'github', 'ztest-gh-60c', 'ztest-map-orgc',
                                             'b0000000-0000-4000-8000-00000000060a', 'orgc', 'Org C', true);
    if r.workspace_id is null then raise exception 'map: a proven admin could not file org C as a workspace'; end if;
    -- …and once an admin placed it, nobody unproven moves it, claimed or not.
    begin
        perform dojo.map_forge_unit('a0000000-0000-4000-8000-000000000601', 'github', 'ztest-gh-60c', 'ztest-map-orgc',
                                    'b0000000-0000-4000-8000-00000000060a', 'other', 'Other', false);
        raise exception 'map: an unproven mapping replaced a proven one';
    exception when insufficient_privilege then null;
    end;

    -- Its own dōjō: needs proof, and must be the dōjō the org is connected to.
    begin
        perform dojo.map_forge_unit('a0000000-0000-4000-8000-000000000601', 'github', 'ztest-gh-60b', 'ztest-map-orgb',
                                    'b0000000-0000-4000-8000-00000000060b', null, null, false);
        raise exception 'map: an org became its own dōjō without proof';
    exception when insufficient_privilege then null;
    end;
    begin
        perform dojo.map_forge_unit('a0000000-0000-4000-8000-000000000601', 'github', 'ztest-gh-60b', 'ztest-map-orgb',
                                    'b0000000-0000-4000-8000-00000000060a', null, null, true);
        raise exception 'map: an org was made "its own dōjō" in a dōjō it is not connected to';
    exception when invalid_parameter_value then null;
    end;
end $$;
reset role;

-- One mapping per unit; each mapping audited in the dōjō it files into.
do $$
begin
    if (select count(*) from dojo.forge_unit_mappings where external_id in ('ztest-gh-60b', 'ztest-gh-60c')) <> 2 then
        raise exception 'mappings: expected one row per unit';
    end if;
    if (select proven_at from dojo.forge_unit_mappings where external_id = 'ztest-gh-60b') is not null
       or (select proven_at from dojo.forge_unit_mappings where external_id = 'ztest-gh-60c') is null then
        raise exception 'mappings: proven_at does not record which mapping was proven';
    end if;
    if (select count(*) from dojo.audit_events
         where tenant_id = 'b0000000-0000-4000-8000-00000000060a' and action = 'forge_unit_mapped') <> 2 then
        raise exception 'mappings: expected two forge_unit_mapped audit rows in A';
    end if;
end $$;

-- Reading: the dōjō's admins see its mappings; a contributor does not.
set local role authenticated;
do $$
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000601"}', true);
    if (select count(*) from dojo.forge_unit_mappings) <> 2 then
        raise exception 'mappings: ada (admin of A) should read A''s two mappings';
    end if;
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000602"}', true);
    if (select count(*) from dojo.forge_unit_mappings) <> 0 then
        raise exception 'mappings: a contributor read the dōjō''s mappings';
    end if;
end $$;

-- Registration follows the mapping.
do $$
declare
    r record;
    first_id uuid;
begin
    -- con, a member of A, registers an org C repository: it files into A, in C's workspace.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000602"}', true);
    select * into r from dojo.register_repository('github.com/ztest-map-orgc/api', 'github', 'ztest-map-orgc', 'api', null);
    if r.reason is not null or r.tenant_id <> 'b0000000-0000-4000-8000-00000000060a' or r.workspace_id is null then
        raise exception 'register: org C''s repository should file into A''s workspace, got %', r;
    end if;
    first_id := r.repository_id;
    select * into r from dojo.register_repository('github.com/ztest-map-orgc/api', 'github', 'ztest-map-orgc', 'api', null);
    if r.repository_id is distinct from first_id then raise exception 'register: a second registration made a second row'; end if;

    -- cee, in org C's own dōjō but not in A: the mapping decides, so not C.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000603"}', true);
    select * into r from dojo.register_repository('github.com/ztest-map-orgc/web', 'github', 'ztest-map-orgc', 'web', null);
    if r.reason is distinct from 'not_a_member' or r.tenant_id is not null then
        raise exception 'register: a unit mapped into A filed into its own dōjō for a non-member of A, got %', r;
    end if;

    -- An org with no mapping still files by its connection, as before.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000602"}', true);
    select * into r from dojo.register_repository('github.com/ztest-map-orga/x', 'github', 'ztest-map-orga', 'x', null);
    if r.reason is not null or r.tenant_id <> 'b0000000-0000-4000-8000-00000000060a' or r.workspace_id is not null then
        raise exception 'register: an unmapped org should file by its connection with no workspace, got %', r;
    end if;
end $$;
reset role;

do $$
begin
    if (select count(*)
          from dojo.repositories_in_workspaces riw
          join dojo.repositories rp on rp.id = riw.repository_id
          join dojo.forge_unit_mappings m on m.workspace_id = riw.workspace_id
         where rp.repo_key = 'github.com/ztest-map-orgc/api' and m.external_id = 'ztest-gh-60c'
           and riw.tenant_id = 'b0000000-0000-4000-8000-00000000060a') <> 1 then
        raise exception 'register: org C''s repository is not in the mapped workspace';
    end if;
    if exists (select 1 from dojo.repositories where repo_key like 'github.com/ztest-map-orgc/%'
                and tenant_id = 'b0000000-0000-4000-8000-00000000060c') then
        raise exception 'register: a repository of a unit mapped elsewhere filed into its own dōjō';
    end if;
end $$;

-- Back to its own dōjō: proven, connected — replaces the workspace mapping and
-- says so in the dōjō it left.
set local role service_role;
do $$
declare
    r record;
begin
    select * into r from dojo.map_forge_unit('a0000000-0000-4000-8000-000000000601', 'github', 'ztest-gh-60b', 'ztest-map-orgb',
                                             'b0000000-0000-4000-8000-00000000060b', null, null, true);
    if r.tenant_id <> 'b0000000-0000-4000-8000-00000000060b' or r.workspace_id is not null then
        raise exception 'map: org B should be its own dōjō again, got %', r;
    end if;

    -- Org B's dōjō is still unclaimed, but an admin of the org has now placed it:
    -- nobody unproven moves it back out.
    begin
        perform dojo.map_forge_unit('a0000000-0000-4000-8000-000000000601', 'github', 'ztest-gh-60b', 'ztest-map-orgb',
                                    'b0000000-0000-4000-8000-00000000060a', 'orgb', 'Org B', false);
        raise exception 'map: an unproven mapping replaced a proven one on an unclaimed org';
    exception when insufficient_privilege then null;
    end;
end $$;
reset role;

do $$
begin
    if (select count(*) from dojo.forge_unit_mappings where external_id = 'ztest-gh-60b') <> 1 then
        raise exception 'mappings: re-mapping made a second row';
    end if;
    if (select count(*) from dojo.audit_events
         where tenant_id = 'b0000000-0000-4000-8000-00000000060a' and action = 'forge_unit_unmapped'
           and detail ->> 'external_slug' = 'ztest-map-orgb') <> 1 then
        raise exception 'mappings: the dōjō a unit left has no forge_unit_unmapped row';
    end if;
end $$;

rollback;
