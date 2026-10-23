-- Leaving the organization ends the membership org provisioning made (#309).
--
-- Repository roles follow the forge (D5), and so must the dōjō role that org
-- provisioning granted: someone the forge no longer lists as a member of the org
-- loses their membership — and with it any admin or lead power — on the next
-- complete read of their orgs. Members who joined BY INVITE (contractors,
-- clients) are not org members and are never touched; nor is a personal dōjō, or
-- a dōjō connected to another forge. Rejoining the org restores a membership
-- that ended by leaving.
--
-- WHAT BREAKS THIS TEST: an invited membership disabled; a kept org's membership
-- disabled; another forge's dōjō or a personal dōjō touched; a departed admin
-- still passing an admin check; a rejoin not restoring the membership; a
-- signed-in caller running it.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000361', 'a1000000-0000-4000-8000-000000000361', 'leaver');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000036a', 'organization/ztest-md-left', 'organization', 'ztest-md-left', 'Left', 'x'),
  ('b0000000-0000-4000-8000-00000000036b', 'organization/ztest-md-kept', 'organization', 'ztest-md-kept', 'Kept', 'x'),
  ('b0000000-0000-4000-8000-00000000036c', 'organization/ztest-md-inv', 'organization', 'ztest-md-inv', 'Invited', 'x'),
  ('b0000000-0000-4000-8000-00000000036d', 'organization/ztest-md-ado', 'organization', 'ztest-md-ado', 'Azure', 'x'),
  ('b0000000-0000-4000-8000-00000000036e', 'personal/ztest-md-me', 'personal', 'ztest-md-me', 'Me', 'x');
insert into dojo.tenant_connections (tenant_id, provider, external_slug, connected_by) values
  ('b0000000-0000-4000-8000-00000000036a', 'github', 'ztest-md-left', 'a1000000-0000-4000-8000-000000000361'),
  ('b0000000-0000-4000-8000-00000000036b', 'github', 'ztest-md-kept', 'a1000000-0000-4000-8000-000000000361'),
  ('b0000000-0000-4000-8000-00000000036c', 'github', 'ztest-md-inv', 'a1000000-0000-4000-8000-000000000361'),
  ('b0000000-0000-4000-8000-00000000036d', 'azure_devops', 'ztest-md-ado', 'a1000000-0000-4000-8000-000000000361');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via, source) values
  ('b0000000-0000-4000-8000-00000000036a', 'a0000000-0000-4000-8000-000000000361', 'admin',       'employer', 'github_oauth', 'provisioned'),
  ('b0000000-0000-4000-8000-00000000036b', 'a0000000-0000-4000-8000-000000000361', 'admin',       'employer', 'github_oauth', 'provisioned'),
  ('b0000000-0000-4000-8000-00000000036c', 'a0000000-0000-4000-8000-000000000361', 'contributor', 'client',   'sso',          'invited'),
  ('b0000000-0000-4000-8000-00000000036d', 'a0000000-0000-4000-8000-000000000361', 'maintainer',  'employer', 'github_oauth', 'provisioned'),
  ('b0000000-0000-4000-8000-00000000036e', 'a0000000-0000-4000-8000-000000000361', 'admin',       'personal', 'github_oauth', 'provisioned');

-- A dōjō known to GitHub AND Azure DevOps, where the person's membership was
-- provisioned by the AZURE import (#308 R5b review): a GitHub read that does not
-- list the dōjō is not the read that made the membership, and must not end it.
-- (Memberships above carry no source_provider: made before provenance, by GitHub.)
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000036f', 'organization/ztest-md-dual', 'organization', 'ztest-md-dual', 'Dual', 'x');
insert into dojo.tenant_connections (tenant_id, provider, external_id, external_slug, connected_by) values
  ('b0000000-0000-4000-8000-00000000036f', 'github',       'ztest-md-dual-gh', 'ztest-md-dual', 'a1000000-0000-4000-8000-000000000361'),
  ('b0000000-0000-4000-8000-00000000036f', 'azure_devops', 'ztest-md-dual-az', 'ztest-md-dual', 'a1000000-0000-4000-8000-000000000361');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via, source, source_provider, source_unit) values
  ('b0000000-0000-4000-8000-00000000036f', 'a0000000-0000-4000-8000-000000000361', 'admin', 'employer', 'azure_devops', 'provisioned', 'azure_devops', 'ztest-md-dual-az');

-- A signed-in caller cannot run it: it is provisioning's, after a forge read.
set local role authenticated;
do $$
begin
    perform dojo.reconcile_org_memberships('a0000000-0000-4000-8000-000000000361', 'github', '{}'::uuid[]);
    raise exception 'a signed-in caller reconciled org memberships';
exception when insufficient_privilege then null;
end $$;
reset role;

-- A complete GitHub read lists only the kept org.
set local role service_role;
select dojo.reconcile_org_memberships('a0000000-0000-4000-8000-000000000361', 'github',
       array['b0000000-0000-4000-8000-00000000036b']::uuid[]);
reset role;

do $$
declare
    ended text;
begin
    select string_agg(t.slug, ',' order by t.slug) into ended
      from dojo.memberships m join dojo.tenants t on t.id = m.tenant_id
     where m.user_id = 'a0000000-0000-4000-8000-000000000361' and m.disabled_at is not null;
    if ended is distinct from 'ztest-md-left' then
        raise exception 'leaving one org ended [%], expected [ztest-md-left] only', ended;
    end if;
    if (select disabled_reason::text from dojo.memberships
         where tenant_id = 'b0000000-0000-4000-8000-00000000036a') is distinct from 'left_org' then
        raise exception 'the ended membership does not say why';
    end if;
end $$;

-- The departed admin no longer passes an admin check there; still does where kept.
set local role authenticated;
do $$
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000361"}', true);
    if dojo.tenant_role_at_least('b0000000-0000-4000-8000-00000000036a', 'admin') then
        raise exception 'an admin who left the org is still an admin of its dōjō';
    end if;
    if not dojo.tenant_role_at_least('b0000000-0000-4000-8000-00000000036b', 'admin') then
        raise exception 'the kept org lost its admin';
    end if;
end $$;
reset role;

-- Rejoining the org restores the membership that ended by leaving.
set local role service_role;
select dojo.reconcile_org_memberships('a0000000-0000-4000-8000-000000000361', 'github',
       array['b0000000-0000-4000-8000-00000000036a', 'b0000000-0000-4000-8000-00000000036b']::uuid[]);
reset role;

do $$
begin
    if exists (select 1 from dojo.memberships
                where user_id = 'a0000000-0000-4000-8000-000000000361' and disabled_at is not null) then
        raise exception 'rejoining the org did not restore the membership';
    end if;
end $$;

rollback;
