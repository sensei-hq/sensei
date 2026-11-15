-- The org repositories read refuses every non-admin at the SQL grain, and
-- strangers entirely (org-repositories r2, test 2; L1: the route's 403 comes
-- from ACCESS.admin before SQL — the raise below is the definer's own verdict,
-- pinned here so direct RPC and the demote-mid-request race stay closed).
--
-- WHAT BREAKS THIS TEST: a missing dojo.org_repositories(); a lead,
-- maintainer or contributor reading rows (the tenant assert compares by enum
-- order, never text); a cross-tenant member or a membership-less principal
-- reading rows; a refusal that does not name the admin rule.
--
-- Phase A: red — dojo.org_repositories(uuid) does not exist yet. The red
-- reads as "wrong refusal [function … does not exist]", which is the honest
-- phase-A shape: fixtures run, the call cannot.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000111', 'a1000000-0000-4000-8000-000000000111', 'ada'),
  ('a0000000-0000-4000-8000-000000000112', 'a1000000-0000-4000-8000-000000000112', 'leo'),
  ('a0000000-0000-4000-8000-000000000113', 'a1000000-0000-4000-8000-000000000113', 'max'),
  ('a0000000-0000-4000-8000-000000000114', 'a1000000-0000-4000-8000-000000000114', 'con'),
  ('a0000000-0000-4000-8000-000000000115', 'a1000000-0000-4000-8000-000000000115', 'out'),
  ('a0000000-0000-4000-8000-000000000116', 'a1000000-0000-4000-8000-000000000116', 'str');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-000000000111', 'organization/ztest-orrepos-ref', 'organization',
   'ztest-orrepos-ref', 'refusal fixture', 'dojo.sensei-hq.org/organization/ztest-orrepos-ref', now()),
  ('b0000000-0000-4000-8000-000000000112', 'organization/ztest-orrepos-refout', 'organization',
   'ztest-orrepos-refout', 'refusal outsider', 'dojo.sensei-hq.org/organization/ztest-orrepos-refout', now());

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-000000000111', 'a0000000-0000-4000-8000-000000000111', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-000000000111', 'a0000000-0000-4000-8000-000000000112', 'lead',        'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-000000000111', 'a0000000-0000-4000-8000-000000000113', 'maintainer',  'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-000000000111', 'a0000000-0000-4000-8000-000000000114', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-000000000112', 'a0000000-0000-4000-8000-000000000115', 'contributor', 'employer', 'github_oauth');

insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b0000000-0000-4000-8000-000000000111', 'pro', 'active', current_date - 1, current_date + 30);

insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider) values
  ('90000000-0000-4000-8000-000000000111', 'b0000000-0000-4000-8000-000000000111',
   'github.com/ztest-orrepos-ref/core', 'core', 'private', now(), 'github');

set local role authenticated;

do $$
declare
    pair text[];
    who text;
    label text;
begin
    -- Every non-admin role of the org, a cross-tenant member, and a
    -- principal with no membership anywhere: all refused, all naming admin.
    foreach pair slice 1 in array
        array[['a1000000-0000-4000-8000-000000000112', 'lead'],
              ['a1000000-0000-4000-8000-000000000113', 'maintainer'],
              ['a1000000-0000-4000-8000-000000000114', 'contributor'],
              ['a1000000-0000-4000-8000-000000000115', 'cross-tenant member'],
              ['a1000000-0000-4000-8000-000000000116', 'membership-less stranger']] loop
        who := pair[1];
        label := pair[2];
        perform set_config('request.jwt.claims', json_build_object('sub', who)::text, true);
        begin
            perform * from dojo.org_repositories('b0000000-0000-4000-8000-000000000111');
            raise exception 'org_repositories(%): a % must be refused, rows came back', who, label;
        exception when others then
            if sqlerrm not like '%admin%' then
                raise exception 'org_repositories(%): wrong refusal for a % [%]', who, label, sqlerrm;
            end if;
        end;
    end loop;
end $$;

rollback;
