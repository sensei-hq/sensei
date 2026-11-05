-- Billing's hidden-repositories list reads the CALLER's repositories
-- (`/v1/you/repositories` → dojo.my_repositories()) and groups the org's
-- billing-hidden rows (Design billing follow-up; reviews/Design/L4-billing-followup-r1.md L3).
-- That is complete only because dojo.all_my_repositories has one row per
-- (repository, member): every member of an org, admin or not, gets every one of
-- the org's repositories, whether or not they hold a role on it.
--
-- WHAT BREAKS THIS TEST: D5-style repository access (dojo.repository_roles)
-- applied to all_my_repositories, which would make Billing's count shrink silently
-- for an admin with no role on a repository; a my_repositories() that drops a
-- refused (hidden) row instead of returning it with its verdict.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000201', 'a1000000-0000-4000-8000-000000000201', 'ada'),
  ('a0000000-0000-4000-8000-000000000202', 'a1000000-0000-4000-8000-000000000202', 'ola');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-00000000020a', 'organization/ztest-hidden-t', 'organization',
   'ztest-hidden-t', 'hidden fixture T', 'dojo.sensei-hq.org/organization/ztest-hidden-t', now());

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000020a', 'a0000000-0000-4000-8000-000000000201', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000020a', 'a0000000-0000-4000-8000-000000000202', 'contributor', 'employer', 'github_oauth');

-- T is past due: its private repositories are hidden for everyone (D88).
insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b0000000-0000-4000-8000-00000000020a', 'pro', 'past_due', current_date - 1, current_date + 30);
insert into dojo.tenant_share_policy (tenant_id, private_repos_shared) values
  ('b0000000-0000-4000-8000-00000000020a', true);
insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider) values
  ('90000000-0000-4000-8000-000000000201', 'b0000000-0000-4000-8000-00000000020a',
   'github.com/ztest-hidden-t/core', 'core', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000202', 'b0000000-0000-4000-8000-00000000020a',
   'github.com/ztest-hidden-t/portal', 'portal', 'private', now(), 'github');

do $$
declare n int;
begin
    -- The precondition: neither person holds a role on either repository.
    select count(*) into n from dojo.repository_roles
     where repository_id in ('90000000-0000-4000-8000-000000000201', '90000000-0000-4000-8000-000000000202');
    if n <> 0 then raise exception 'fixture: expected no repository_roles rows, found %', n; end if;
end $$;

set local role authenticated;

do $$
declare
    who text;
    got text;
    n int;
begin
    foreach who in array array['a1000000-0000-4000-8000-000000000201', 'a1000000-0000-4000-8000-000000000202'] loop
        perform set_config('request.jwt.claims', json_build_object('sub', who)::text, true);
        select coalesce(string_agg(repo_key, ',' order by repo_key), '') into got
          from dojo.my_repositories() where repo_key like 'github.com/ztest-hidden-t/%';
        if got <> 'github.com/ztest-hidden-t/core,github.com/ztest-hidden-t/portal' then
            raise exception 'my_repositories(%): a member with no repository role must get every org-private repository, got [%]', who, got;
        end if;
        -- Both rows come back refused for the plan, so the dōjō flags them hidden.
        select count(*) into n from dojo.my_repositories()
         where repo_key like 'github.com/ztest-hidden-t/%' and entitlement_refusal = 'not_subscribed' and not sync_enabled;
        if n <> 2 then
            raise exception 'my_repositories(%): both rows should be refused not_subscribed (past due), % were', who, n;
        end if;
    end loop;
end $$;

rollback;
