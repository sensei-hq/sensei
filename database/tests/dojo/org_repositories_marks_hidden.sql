-- billing_hidden marks exactly the hidden set, and hidden rows are still
-- returned (org-repositories r2, test 8; mark-don't-drop: the count is
-- impossible if the read drops the rows it counts — same rule as
-- listMyRepositories).
--
-- WHAT BREAKS THIS TEST: a missing dojo.org_repositories(); hidden rows
-- dropped instead of marked; the mark set drifting from the hide predicate
-- (unclaimed / not_subscribed / subscription_expired on org-private).
--
-- Phase A: red — dojo.org_repositories(uuid) does not exist yet.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000181', 'a1000000-0000-4000-8000-000000000181', 'ada'),
  ('a0000000-0000-4000-8000-000000000182', 'a1000000-0000-4000-8000-000000000182', 'ola');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-000000000181', 'organization/ztest-orrepos-mark', 'organization',
   'ztest-orrepos-mark', 'mark fixture', 'dojo.sensei-hq.org/organization/ztest-orrepos-mark', now());

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-000000000181', 'a0000000-0000-4000-8000-000000000181', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-000000000181', 'a0000000-0000-4000-8000-000000000182', 'contributor', 'employer', 'github_oauth');

-- Past due: private rows hide, and stay listed.
insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b0000000-0000-4000-8000-000000000181', 'pro', 'past_due', current_date - 1, current_date + 30);

insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider) values
  ('90000000-0000-4000-8000-000000000181', 'b0000000-0000-4000-8000-000000000181',
   'github.com/ztest-orrepos-mark/core', 'core', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000182', 'b0000000-0000-4000-8000-000000000181',
   'github.com/ztest-orrepos-mark/open', 'open', 'public', now(), 'github');

insert into dojo.repository_roles (repository_id, principal_id, forge_role) values
  ('90000000-0000-4000-8000-000000000181', 'a0000000-0000-4000-8000-000000000182', 'contributor'),
  ('90000000-0000-4000-8000-000000000182', 'a0000000-0000-4000-8000-000000000182', 'contributor');

set local role authenticated;

do $$
declare
    n int;
begin
    perform set_config('request.jwt.claims',
        json_build_object('sub', 'a1000000-0000-4000-8000-000000000181')::text, true);

    -- Both rows listed (nothing dropped) …
    select count(*) into n from dojo.org_repositories('b0000000-0000-4000-8000-000000000181');
    if n <> 2 then
        raise exception 'billing_hidden: past-due org must list both repos, listed %', n;
    end if;

    -- … the private one marked, the public one not.
    perform 1 from dojo.org_repositories('b0000000-0000-4000-8000-000000000181')
     where name = 'core' and billing_hidden is true;
    if not found then
        raise exception 'billing_hidden: past-due private repo must mark billing_hidden true';
    end if;
    perform 1 from dojo.org_repositories('b0000000-0000-4000-8000-000000000181')
     where name = 'open' and billing_hidden is false;
    if not found then
        raise exception 'billing_hidden: public repo must mark billing_hidden false';
    end if;
end $$;

rollback;
