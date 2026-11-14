-- member_count counts rows with an effective role and an active membership,
-- joined exactly as repo_role_at_least joins them (org-repositories r2,
-- test 3; L0 Q1 ruling 2026-11-06: effective role not null + active
-- membership; mirrored-without-scan counts, scanned-without-role doesn't).
--
-- WHAT BREAKS THIS TEST: a missing dojo.org_repositories(); the r1 mistake
-- (excluding every override_role IS NULL row — normal members carry NULL
-- there with overridden = false); a join that skips the membership, the
-- tenant scoping, or disabled_at; a zero-count row listed (the mockup lists
-- no zero-member rows).
--
-- Phase A: red — dojo.org_repositories(uuid) does not exist yet.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000121', 'a1000000-0000-4000-8000-000000000121', 'ada'),
  ('a0000000-0000-4000-8000-000000000122', 'a1000000-0000-4000-8000-000000000122', 'm1'),
  ('a0000000-0000-4000-8000-000000000123', 'a1000000-0000-4000-8000-000000000123', 'm2'),
  ('a0000000-0000-4000-8000-000000000124', 'a1000000-0000-4000-8000-000000000124', 'm3'),
  ('a0000000-0000-4000-8000-000000000125', 'a1000000-0000-4000-8000-000000000125', 'm4'),
  ('a0000000-0000-4000-8000-000000000126', 'a1000000-0000-4000-8000-000000000126', 'dis1'),
  ('a0000000-0000-4000-8000-000000000127', 'a1000000-0000-4000-8000-000000000127', 'dis2');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-000000000121', 'organization/ztest-orrepos-count', 'organization',
   'ztest-orrepos-count', 'count fixture', 'dojo.sensei-hq.org/organization/ztest-orrepos-count', now());

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via, disabled_at) values
  ('b0000000-0000-4000-8000-000000000121', 'a0000000-0000-4000-8000-000000000121', 'admin',       'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-000000000121', 'a0000000-0000-4000-8000-000000000122', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-000000000121', 'a0000000-0000-4000-8000-000000000123', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-000000000121', 'a0000000-0000-4000-8000-000000000124', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-000000000121', 'a0000000-0000-4000-8000-000000000125', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-000000000121', 'a0000000-0000-4000-8000-000000000126', 'contributor', 'employer', 'github_oauth', now()),
  ('b0000000-0000-4000-8000-000000000121', 'a0000000-0000-4000-8000-000000000127', 'contributor', 'employer', 'github_oauth', now());

insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b0000000-0000-4000-8000-000000000121', 'pro', 'active', current_date - 1, current_date + 30);

insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider) values
  ('90000000-0000-4000-8000-000000000131', 'b0000000-0000-4000-8000-000000000121',
   'github.com/ztest-orrepos-count/normal', 'normal', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000132', 'b0000000-0000-4000-8000-000000000121',
   'github.com/ztest-orrepos-count/revoked', 'revoked', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000133', 'b0000000-0000-4000-8000-000000000121',
   'github.com/ztest-orrepos-count/dropped', 'dropped', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000134', 'b0000000-0000-4000-8000-000000000121',
   'github.com/ztest-orrepos-count/offboarded', 'offboarded', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000135', 'b0000000-0000-4000-8000-000000000121',
   'github.com/ztest-orrepos-count/empty', 'empty', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000136', 'b0000000-0000-4000-8000-000000000121',
   'github.com/ztest-orrepos-count/ghost', 'ghost', 'private', now(), 'github');

-- normal: three ordinary members (overridden = false, override_role NULL).
insert into dojo.repository_roles (repository_id, principal_id, forge_role) values
  ('90000000-0000-4000-8000-000000000131', 'a0000000-0000-4000-8000-000000000122', 'contributor'),
  ('90000000-0000-4000-8000-000000000131', 'a0000000-0000-4000-8000-000000000123', 'maintainer'),
  ('90000000-0000-4000-8000-000000000131', 'a0000000-0000-4000-8000-000000000124', 'lead');
-- revoked: one ordinary member + one admin-revoked (the only
-- override_role IS NULL row that genuinely excludes).
insert into dojo.repository_roles (repository_id, principal_id, forge_role) values
  ('90000000-0000-4000-8000-000000000132', 'a0000000-0000-4000-8000-000000000122', 'contributor');
insert into dojo.repository_roles
       (repository_id, principal_id, forge_role, overridden, override_role, overridden_by, overridden_at) values
  ('90000000-0000-4000-8000-000000000132', 'a0000000-0000-4000-8000-000000000123', 'contributor',
   true, null, 'a0000000-0000-4000-8000-000000000121', now());
-- dropped: one ordinary member + one the forge no longer lists.
insert into dojo.repository_roles (repository_id, principal_id, forge_role) values
  ('90000000-0000-4000-8000-000000000133', 'a0000000-0000-4000-8000-000000000122', 'contributor'),
  ('90000000-0000-4000-8000-000000000133', 'a0000000-0000-4000-8000-000000000125', null);
-- offboarded: one ordinary member + one whose membership is disabled.
insert into dojo.repository_roles (repository_id, principal_id, forge_role) values
  ('90000000-0000-4000-8000-000000000134', 'a0000000-0000-4000-8000-000000000122', 'contributor'),
  ('90000000-0000-4000-8000-000000000134', 'a0000000-0000-4000-8000-000000000126', 'contributor');
-- empty: no roles. ghost: roles, but every member disabled.
insert into dojo.repository_roles (repository_id, principal_id, forge_role) values
  ('90000000-0000-4000-8000-000000000136', 'a0000000-0000-4000-8000-000000000126', 'contributor'),
  ('90000000-0000-4000-8000-000000000136', 'a0000000-0000-4000-8000-000000000127', 'maintainer');

set local role authenticated;

do $$
declare
    got int;
begin
    perform set_config('request.jwt.claims',
        json_build_object('sub', 'a1000000-0000-4000-8000-000000000121')::text, true);

    select member_count into got from dojo.org_repositories('b0000000-0000-4000-8000-000000000121')
     where name = 'normal';
    if got is distinct from 3 then
        raise exception 'member_count(normal): 3 ordinary members must count 3, got %', got;
    end if;

    select member_count into got from dojo.org_repositories('b0000000-0000-4000-8000-000000000121')
     where name = 'revoked';
    if got is distinct from 1 then
        raise exception 'member_count(revoked): overridden-to-null must exclude, got %', got;
    end if;

    select member_count into got from dojo.org_repositories('b0000000-0000-4000-8000-000000000121')
     where name = 'dropped';
    if got is distinct from 1 then
        raise exception 'member_count(dropped): forge-dropped (null forge_role) must exclude, got %', got;
    end if;

    select member_count into got from dojo.org_repositories('b0000000-0000-4000-8000-000000000121')
     where name = 'offboarded';
    if got is distinct from 1 then
        raise exception 'member_count(offboarded): disabled membership must exclude, got %', got;
    end if;

    -- Zero-standing repos are absent, never zero rows (mockup rule).
    perform 1 from dojo.org_repositories('b0000000-0000-4000-8000-000000000121')
     where name in ('empty', 'ghost');
    if found then
        raise exception 'member_count: repos with no standing (empty, ghost) must be absent from the list';
    end if;

    -- Roles added and removed move the count. Writes run as the session
    -- owner (fixtures bypass RLS); reads run back as the admin caller.
    reset role;
    insert into dojo.repository_roles (repository_id, principal_id, forge_role) values
      ('90000000-0000-4000-8000-000000000131', 'a0000000-0000-4000-8000-000000000125', 'contributor');
    set local role authenticated;
    perform set_config('request.jwt.claims',
        json_build_object('sub', 'a1000000-0000-4000-8000-000000000121')::text, true);
    select member_count into got from dojo.org_repositories('b0000000-0000-4000-8000-000000000121')
     where name = 'normal';
    if got is distinct from 4 then
        raise exception 'member_count(normal): adding a role must move 3 -> 4, got %', got;
    end if;
    reset role;
    delete from dojo.repository_roles
     where repository_id = '90000000-0000-4000-8000-000000000131'
       and principal_id = 'a0000000-0000-4000-8000-000000000125';
    set local role authenticated;
    perform set_config('request.jwt.claims',
        json_build_object('sub', 'a1000000-0000-4000-8000-000000000121')::text, true);
    select member_count into got from dojo.org_repositories('b0000000-0000-4000-8000-000000000121')
     where name = 'normal';
    if got is distinct from 3 then
        raise exception 'member_count(normal): removing a role must move 4 -> 3, got %', got;
    end if;
end $$;

rollback;
