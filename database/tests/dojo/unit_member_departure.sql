-- Leaving an Azure DevOps organization ends the membership its import made
-- (#308 R5b, #309's rule from the organization's side).
--
-- GitHub provisioning reads ONE PERSON's orgs (reconcile_org_memberships). An
-- Azure organization is read from the other side: an admin's stored token lists
-- the organization's members. dojo.reconcile_unit_members takes such a COMPLETE
-- list (the members' forge subjects) and ends each PROVISIONED membership in the
-- dōjō whose person holds an identity of that forge no longer listed — and with
-- it any admin power; a membership that ended by leaving and is listed again is
-- restored. Invited members, people with no identity of that forge (the list
-- cannot name them either way), personal dōjōs and dōjōs not connected to that
-- forge are never touched. A missing or empty list is refused: only a complete
-- read may end anything, and an organization read with a member's token always
-- lists that member.
--
-- WHAT BREAKS THIS TEST: a listed member ended; an invited member or one with no
-- Azure identity ended; a departed member left active; a rejoin not restored; a
-- dōjō not connected to Azure DevOps touched; a NULL or empty list ending
-- everyone; a signed-in caller running it.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000d01', 'a1000000-0000-4000-8000-000000000d01', 'ztest-um-admin'),
  ('a0000000-0000-4000-8000-000000000d02', 'a1000000-0000-4000-8000-000000000d02', 'ztest-um-leaver'),
  ('a0000000-0000-4000-8000-000000000d03', 'a1000000-0000-4000-8000-000000000d03', 'ztest-um-unknown'),
  ('a0000000-0000-4000-8000-000000000d04', 'a1000000-0000-4000-8000-000000000d04', 'ztest-um-invited'),
  ('a0000000-0000-4000-8000-000000000d05', 'a1000000-0000-4000-8000-000000000d05', 'ztest-um-returner');
insert into dojo.identities (principal_id, provider, subject) values
  ('a0000000-0000-4000-8000-000000000d01', 'azure_devops_pat', 'ztest-az-s1'),
  ('a0000000-0000-4000-8000-000000000d02', 'azure_devops_pat', 'ztest-az-s2'),
  ('a0000000-0000-4000-8000-000000000d03', 'github_oauth',     'ztest-gh-s3'),
  ('a0000000-0000-4000-8000-000000000d04', 'azure_devops_pat', 'ztest-az-s4'),
  ('a0000000-0000-4000-8000-000000000d05', 'azure_devops_pat', 'ztest-az-s5');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-000000000d0a', 'organization/ztest-um-az', 'organization', 'ztest-um-az', 'Azure org', 'x'),
  ('b0000000-0000-4000-8000-000000000d0b', 'organization/ztest-um-gh', 'organization', 'ztest-um-gh', 'GitHub org', 'x');
insert into dojo.tenant_connections (tenant_id, provider, external_id, external_slug, connected_by) values
  ('b0000000-0000-4000-8000-000000000d0a', 'azure_devops', 'ztest-um-org', 'ztest-um-az', 'a0000000-0000-4000-8000-000000000d01'),
  ('b0000000-0000-4000-8000-000000000d0b', 'github',       'ztest-um-gh',  'ztest-um-gh', 'a0000000-0000-4000-8000-000000000d01');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via, source, disabled_at, disabled_reason) values
  ('b0000000-0000-4000-8000-000000000d0a', 'a0000000-0000-4000-8000-000000000d01', 'admin',       'employer', 'azure_devops_pat', 'provisioned', null, null),
  ('b0000000-0000-4000-8000-000000000d0a', 'a0000000-0000-4000-8000-000000000d02', 'admin',       'employer', 'azure_devops_pat', 'provisioned', null, null),
  ('b0000000-0000-4000-8000-000000000d0a', 'a0000000-0000-4000-8000-000000000d03', 'contributor', 'employer', 'github_oauth',     'provisioned', null, null),
  ('b0000000-0000-4000-8000-000000000d0a', 'a0000000-0000-4000-8000-000000000d04', 'contributor', 'client',   'sso',              'invited',     null, null),
  ('b0000000-0000-4000-8000-000000000d0a', 'a0000000-0000-4000-8000-000000000d05', 'contributor', 'employer', 'azure_devops_pat', 'provisioned', now(), 'left_org'),
  ('b0000000-0000-4000-8000-000000000d0b', 'a0000000-0000-4000-8000-000000000d02', 'contributor', 'employer', 'github_oauth',     'provisioned', null, null);

-- A signed-in caller cannot run it: it follows the server's forge read.
set local role authenticated;
do $$
begin
    perform dojo.reconcile_unit_members('b0000000-0000-4000-8000-000000000d0a', 'azure_devops', 'azure_devops_pat',
                                        array['ztest-az-s1']);
    raise exception 'a signed-in caller reconciled unit members';
exception when insufficient_privilege then null;
end $$;
reset role;

do $$
begin
    if has_function_privilege('anon', 'dojo.reconcile_unit_members(uuid, dojo.forge_provider, dojo.auth_method, text[])', 'execute') then
        raise exception 'anon may reconcile unit members';
    end if;
end $$;

set local role service_role;
do $$
declare
    n integer;
begin
    -- No list, or an empty one, is not a complete read: refused, nobody ended.
    begin
        perform dojo.reconcile_unit_members('b0000000-0000-4000-8000-000000000d0a', 'azure_devops', 'azure_devops_pat', null);
        raise exception 'unit members: a NULL list was accepted';
    exception when invalid_parameter_value then null;
    end;
    begin
        perform dojo.reconcile_unit_members('b0000000-0000-4000-8000-000000000d0a', 'azure_devops', 'azure_devops_pat', '{}'::text[]);
        raise exception 'unit members: an empty list was accepted';
    exception when invalid_parameter_value then null;
    end;

    -- A dōjō that is not connected to the forge is not this forge's to end.
    select dojo.reconcile_unit_members('b0000000-0000-4000-8000-000000000d0b', 'azure_devops', 'azure_devops_pat',
                                       array['ztest-az-s1']) into n;
    if n <> 0 then
        raise exception 'unit members: a dōjō not connected to Azure DevOps had % memberships ended', n;
    end if;

    -- The complete read lists the admin and the returner; the leaver is gone.
    select dojo.reconcile_unit_members('b0000000-0000-4000-8000-000000000d0a', 'azure_devops', 'azure_devops_pat',
                                       array['ztest-az-s1', 'ztest-az-s5']) into n;
    if n <> 1 then
        raise exception 'unit members: expected one membership ended, got %', n;
    end if;
end $$;
reset role;

do $$
declare
    r record;
begin
    for r in
        select p.display_name, m.disabled_at is not null as ended, m.disabled_reason::text as why
          from dojo.memberships m join dojo.principals p on p.id = m.user_id
         where m.tenant_id = 'b0000000-0000-4000-8000-000000000d0a'
    loop
        if r.display_name = 'ztest-um-leaver' and not (r.ended and r.why = 'left_org') then
            raise exception 'unit members: the departed member was not ended (%)', r;
        elsif r.display_name in ('ztest-um-admin', 'ztest-um-unknown', 'ztest-um-invited', 'ztest-um-returner') and r.ended then
            raise exception 'unit members: % was ended', r.display_name;
        end if;
    end loop;
    -- The leaver's membership of the GitHub dōjō is not this read's.
    if (select disabled_at from dojo.memberships
         where tenant_id = 'b0000000-0000-4000-8000-000000000d0b' and user_id = 'a0000000-0000-4000-8000-000000000d02') is not null then
        raise exception 'unit members: another forge''s dōjō was touched';
    end if;
    -- The departed admin no longer passes as an admin of the dōjō.
    if exists (select 1 from dojo.memberships m
                where m.tenant_id = 'b0000000-0000-4000-8000-000000000d0a'
                  and m.user_id = 'a0000000-0000-4000-8000-000000000d02'
                  and m.disabled_at is null and m.role = 'admin') then
        raise exception 'unit members: a departed admin is still an active admin';
    end if;
end $$;

rollback;
