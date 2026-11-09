-- The org repositories read lists the org's repositories for its admin, one
-- row each, ordered by name — and nothing from any other tenant
-- (org-repositories r2, test 1; L0 Q1/Q2 ruling 2026-11-06).
--
-- WHAT BREAKS THIS TEST: a missing dojo.org_repositories(); rows from
-- another tenant leaking in; an ordering other than name; a renamed or
-- missing output column (the select list pins the phase-B contract).
--
-- Phase A: red — dojo.org_repositories(uuid) does not exist yet.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000101', 'a1000000-0000-4000-8000-000000000101', 'ada'),
  ('a0000000-0000-4000-8000-000000000102', 'a1000000-0000-4000-8000-000000000102', 'ola'),
  ('a0000000-0000-4000-8000-000000000103', 'a1000000-0000-4000-8000-000000000103', 'kim');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-000000000101', 'organization/ztest-orrepos-list', 'organization',
   'ztest-orrepos-list', 'list fixture', 'dojo.sensei-hq.org/organization/ztest-orrepos-list', now()),
  ('b0000000-0000-4000-8000-000000000102', 'organization/ztest-orrepos-out', 'organization',
   'ztest-orrepos-out', 'outsider fixture', 'dojo.sensei-hq.org/organization/ztest-orrepos-out', now());

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-000000000101', 'a0000000-0000-4000-8000-000000000101', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-000000000101', 'a0000000-0000-4000-8000-000000000102', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-000000000102', 'a0000000-0000-4000-8000-000000000103', 'contributor', 'employer', 'github_oauth');

insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b0000000-0000-4000-8000-000000000101', 'pro', 'active', current_date - 1, current_date + 30),
  ('b0000000-0000-4000-8000-000000000102', 'pro', 'active', current_date - 1, current_date + 30);

-- Three repos whose names sort differently from insertion order: the pinned
-- order is name, and the mockup's display order is arbitrary.
insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider) values
  ('90000000-0000-4000-8000-000000000101', 'b0000000-0000-4000-8000-000000000101',
   'github.com/ztest-orrepos-list/zeta', 'zeta', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000102', 'b0000000-0000-4000-8000-000000000101',
   'github.com/ztest-orrepos-list/alpha', 'alpha', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000103', 'b0000000-0000-4000-8000-000000000101',
   'github.com/ztest-orrepos-list/mid', 'mid', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000104', 'b0000000-0000-4000-8000-000000000102',
   'github.com/ztest-orrepos-out/other', 'other', 'private', now(), 'github');

-- Standing roles (test 3's rule: no standing, no listing). The outsider
-- repo is deliberately listable too, so the isolation check below proves
-- tenant scoping rather than passing on role-absence.
insert into dojo.repository_roles (repository_id, principal_id, forge_role) values
  ('90000000-0000-4000-8000-000000000101', 'a0000000-0000-4000-8000-000000000102', 'contributor'),
  ('90000000-0000-4000-8000-000000000102', 'a0000000-0000-4000-8000-000000000102', 'contributor'),
  ('90000000-0000-4000-8000-000000000103', 'a0000000-0000-4000-8000-000000000102', 'contributor'),
  ('90000000-0000-4000-8000-000000000104', 'a0000000-0000-4000-8000-000000000103', 'contributor');

set local role authenticated;

do $$
declare
    got text[];
    n int;
begin
    perform set_config('request.jwt.claims',
        json_build_object('sub', 'a1000000-0000-4000-8000-000000000101')::text, true);

    -- One row per repo of the admin's org, ordered by name.
    select array_agg(name) into got
      from dojo.org_repositories('b0000000-0000-4000-8000-000000000101');
    if got is distinct from array['alpha', 'mid', 'zeta'] then
        raise exception 'org_repositories(admin): expected [alpha,mid,zeta] in name order, got [%]',
            array_to_string(got, ',');
    end if;

    -- No row from the outsider org, which is org-kind: this pins
    -- cross-ORG isolation, not just cross-kind.
    select count(*) into n from dojo.org_repositories('b0000000-0000-4000-8000-000000000101')
     where repo_key like 'github.com/ztest-orrepos-out/%';
    if n <> 0 then
        raise exception 'org_repositories(admin): % outsider-org rows leaked in', n;
    end if;

    -- Identity columns ride along for the tab's rows.
    select count(*) into n from dojo.org_repositories('b0000000-0000-4000-8000-000000000101')
     where repository_id is null or repo_key is null or name is null or provider is null;
    if n <> 0 then
        raise exception 'org_repositories(admin): % rows with null identity columns', n;
    end if;
end $$;

rollback;
