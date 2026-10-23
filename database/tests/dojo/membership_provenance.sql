-- Which forge provisioned a membership is recorded, never guessed (#308 R5b
-- review 2). Only provisioning.ts ensureMembership writes source = 'provisioned':
-- in an ORGANIZATION dōjō it is GitHub org provisioning (authenticated_via
-- github_oauth) or the Azure DevOps import (azure_devops); `sso` reaches it only
-- for personal dōjōs, and `device_code` never does (admin- and invite-added rows
-- are 'invited'). So the backfill (apply/after/membership_source_backfill.sql)
-- attributes github_oauth and azure_devops org rows to their forge, leaves every
-- other provisioned row unattributed (and reports them), and the #309 relabel of
-- non-sso rows to 'provisioned' applies only to rows from before #309 shipped
-- (v0.16.0) — after it, provisioning writes the source itself, and relabelling a
-- newer admin-added row would hand it to a forge reconcile. A reconcile never
-- acts on a membership with no recorded forge.
--
-- WHAT BREAKS THIS TEST: an sso or device_code org row attributed to a forge; a
-- personal row attributed; a newer invited row relabelled provisioned; a
-- reconcile ending an unattributed row; the hook failing on a re-run.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000f01', 'a1000000-0000-4000-8000-000000000f01', 'ztest-prov-1');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-000000000f0a', 'organization/ztest-prov-gh', 'organization', 'ztest-prov-gh', 'GH', 'x'),
  ('b0000000-0000-4000-8000-000000000f0b', 'organization/ztest-prov-az', 'organization', 'ztest-prov-az', 'AZ', 'x'),
  ('b0000000-0000-4000-8000-000000000f0c', 'organization/ztest-prov-sso', 'organization', 'ztest-prov-sso', 'SSO', 'x'),
  ('b0000000-0000-4000-8000-000000000f0d', 'organization/ztest-prov-dev', 'organization', 'ztest-prov-dev', 'DEV', 'x'),
  ('b0000000-0000-4000-8000-000000000f0e', 'personal/ztest-prov-me', 'personal', 'ztest-prov-me', 'Me', 'x'),
  ('b0000000-0000-4000-8000-000000000f0f', 'organization/ztest-prov-new', 'organization', 'ztest-prov-new', 'NEW', 'x');
insert into dojo.tenant_connections (tenant_id, provider, external_id, external_slug, connected_by) values
  ('b0000000-0000-4000-8000-000000000f0a', 'github', 'ztest-prov-gh-1', 'ztest-prov-gh', 'a0000000-0000-4000-8000-000000000f01'),
  ('b0000000-0000-4000-8000-000000000f0b', 'azure_devops', 'ztest-prov-az-1', 'ztest-prov-az', 'a0000000-0000-4000-8000-000000000f01'),
  ('b0000000-0000-4000-8000-000000000f0c', 'github', 'ztest-prov-sso-1', 'ztest-prov-sso', 'a0000000-0000-4000-8000-000000000f01'),
  ('b0000000-0000-4000-8000-000000000f0d', 'github', 'ztest-prov-dev-1', 'ztest-prov-dev', 'a0000000-0000-4000-8000-000000000f01'),
  ('b0000000-0000-4000-8000-000000000f0f', 'github', 'ztest-prov-new-1', 'ztest-prov-new', 'a0000000-0000-4000-8000-000000000f01');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via, source, created_at) values
  ('b0000000-0000-4000-8000-000000000f0a', 'a0000000-0000-4000-8000-000000000f01', 'admin',       'employer', 'github_oauth', 'provisioned', '2026-10-01'),
  ('b0000000-0000-4000-8000-000000000f0b', 'a0000000-0000-4000-8000-000000000f01', 'admin',       'employer', 'azure_devops', 'provisioned', now()),
  ('b0000000-0000-4000-8000-000000000f0c', 'a0000000-0000-4000-8000-000000000f01', 'contributor', 'employer', 'sso',          'provisioned', '2026-10-01'),
  -- Pre-#309, admin-added over the device-code pairing: the #309 rule made it provisioned.
  ('b0000000-0000-4000-8000-000000000f0d', 'a0000000-0000-4000-8000-000000000f01', 'contributor', 'employer', 'device_code',  'invited',     '2026-10-01'),
  ('b0000000-0000-4000-8000-000000000f0e', 'a0000000-0000-4000-8000-000000000f01', 'admin',       'personal', 'github_oauth', 'provisioned', '2026-10-01'),
  -- Admin-added AFTER #309 shipped: provisioning would have said so; it stays invited.
  ('b0000000-0000-4000-8000-000000000f0f', 'a0000000-0000-4000-8000-000000000f01', 'contributor', 'employer', 'github_oauth', 'invited',     now());

\ir ../../apply/after/membership_source_backfill.sql
-- Idempotent: a second run changes nothing.
\ir ../../apply/after/membership_source_backfill.sql

do $$
declare
    r record;
begin
    for r in
        select t.slug, m.source::text as source, m.source_provider::text as provider, m.source_unit as unit
          from dojo.memberships m join dojo.tenants t on t.id = m.tenant_id
         where m.user_id = 'a0000000-0000-4000-8000-000000000f01'
    loop
        if r.slug = 'ztest-prov-gh' and (r.provider is distinct from 'github' or r.unit is distinct from 'ztest-prov-gh-1') then
            raise exception 'provenance: a GitHub-provisioned org row was not attributed to GitHub, got %', r;
        elsif r.slug = 'ztest-prov-az' and (r.provider is distinct from 'azure_devops' or r.unit is distinct from 'ztest-prov-az-1') then
            raise exception 'provenance: an Azure-provisioned org row was not attributed to Azure DevOps, got %', r;
        elsif r.slug in ('ztest-prov-sso', 'ztest-prov-dev', 'ztest-prov-me') and r.provider is not null then
            raise exception 'provenance: % was attributed to a forge nothing proves provisioned it, got %', r.slug, r;
        elsif r.slug = 'ztest-prov-dev' and r.source <> 'provisioned' then
            raise exception 'provenance: the pre-#309 relabel no longer applies to a pre-#309 row, got %', r;
        elsif r.slug = 'ztest-prov-new' and r.source <> 'invited' then
            raise exception 'provenance: an admin-added row from after #309 was relabelled provisioned, got %', r;
        end if;
    end loop;
end $$;

-- A complete GitHub read that lists none of these dōjōs ends only the row GitHub
-- provisioned — never the unattributed ones.
set local role service_role;
do $$
declare
    n integer;
begin
    select dojo.reconcile_org_memberships('a0000000-0000-4000-8000-000000000f01', 'github', '{}'::uuid[]) into n;
    if n <> 1 then
        raise exception 'provenance: expected the one GitHub-provisioned row ended, got %', n;
    end if;
end $$;
reset role;

do $$
begin
    if exists (select 1 from dojo.memberships m join dojo.tenants t on t.id = m.tenant_id
                where m.user_id = 'a0000000-0000-4000-8000-000000000f01' and m.disabled_at is not null
                  and t.slug <> 'ztest-prov-gh') then
        raise exception 'provenance: a reconcile ended a membership no forge was recorded as provisioning';
    end if;
end $$;

rollback;
