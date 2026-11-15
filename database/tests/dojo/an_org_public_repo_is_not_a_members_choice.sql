-- WP8: an org public repository is not a member's choice (D46 ELECTION).
-- A member's own opt-out is not read under org authority, and only an admin
-- may change it — a lead is not an admin.
--
-- WHAT BREAKS THIS TEST (today): the member's election is read and any member
-- (contributor, lead) may flip it. Red until WP8.
--
-- Both halves: the VIEW columns (`authority`, `configurable_by_me`,
-- `elected`), read as postgres, and the 403/200 the election route maps —
-- `set_repository_election` raises `insufficient_privilege` for the
-- contributor and the lead (each under their own `authenticated` JWT) and
-- succeeds for the admin, filing an organisation election with no principal.
--
-- Run through --apply (the view DDL is passed in, not \ir'd): the file seeds,
-- asserts, rolls back. Nothing persists.
begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a4444444-4444-4444-4444-444444444444', 'aa444444-4444-4444-4444-444444444444', 'Alice'),
  ('b4444444-4444-4444-4444-444444444444', 'bb444444-4444-4444-4444-444444444444', 'Bob'),
  ('c4444444-4444-4444-4444-444444444444', 'cc444444-4444-4444-4444-444444444444', 'Carol');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('d4444444-4444-4444-4444-444444444444', 'organization/ztest-wp8-acme', 'organization', 'ztest-wp8-acme',
   'Acme', 'dojo.sensei-hq.org/organization/ztest-wp8-acme', now());

insert into dojo.memberships (tenant_id, user_id, kind, authenticated_via, role, disabled_at) values
  ('d4444444-4444-4444-4444-444444444444', 'a4444444-4444-4444-4444-444444444444',
   'employer', 'github_oauth', 'admin', null),
  ('d4444444-4444-4444-4444-444444444444', 'b4444444-4444-4444-4444-444444444444',
   'employer', 'github_oauth', 'contributor', null),
  ('d4444444-4444-4444-4444-444444444444', 'c4444444-4444-4444-4444-444444444444',
   'employer', 'github_oauth', 'lead', null);

insert into dojo.repositories (tenant_id, repo_key, remote_url, name, provider, visibility, visibility_captured_at) values
  ('d4444444-4444-4444-4444-444444444444', 'github.com/ztest-wp8-acme/web',
   'git@github.com:ztest-wp8-acme/web.git', 'web', 'github', 'public', now());

-- Bob's own opt-out: unread under org authority, kept as history.
insert into dojo.repository_elections (tenant_id, repository_id, authority, principal_id, elected)
  select r.tenant_id, r.id, 'user', 'b4444444-4444-4444-4444-444444444444', false
    from dojo.repositories r
   where r.repo_key = 'github.com/ztest-wp8-acme/web';

do $$
declare
    b_elected boolean;
    b_config boolean;
    c_config boolean;
    a_elected boolean;
    a_config boolean;
begin
    select elected, configurable_by_me into b_elected, b_config
      from dojo.all_my_repositories
     where principal_id = 'b4444444-4444-4444-4444-444444444444'
       and repo_key = 'github.com/ztest-wp8-acme/web';
    if b_elected is not true then
        raise exception 'a member''s opt-out switched an org public repo off.';
    end if;
    if b_config is not false then
        raise exception 'a contributor may change an org public repo.';
    end if;

    select configurable_by_me into c_config
      from dojo.all_my_repositories
     where principal_id = 'c4444444-4444-4444-4444-444444444444'
       and repo_key = 'github.com/ztest-wp8-acme/web';
    if c_config is not false then
        raise exception 'a lead may change an org public repo (admin-only).';
    end if;

    select elected, configurable_by_me into a_elected, a_config
      from dojo.all_my_repositories
     where principal_id = 'a4444444-4444-4444-4444-444444444444'
       and repo_key = 'github.com/ztest-wp8-acme/web';
    if a_elected is not true then
        raise exception 'org public repo with no org decision is not elected.';
    end if;
    if a_config is not true then
        raise exception 'an admin may not change an org public repo.';
    end if;
end $$;

-- ── the 403/200 halves, as each role ─────────────────────────────────────────
set local role authenticated;
do $$
declare
    r record;
begin
    -- Bob (contributor): refused.
    perform set_config('request.jwt.claims', '{"sub":"bb444444-4444-4444-4444-444444444444"}', true);
    begin
        perform dojo.set_repository_election('github.com/ztest-wp8-acme/web', false);
        raise exception 'a contributor changed an org public repo (403).';
    exception when insufficient_privilege then null;
    end;

    -- Carol (lead): refused — a lead is not an admin.
    perform set_config('request.jwt.claims', '{"sub":"cc444444-4444-4444-4444-444444444444"}', true);
    begin
        perform dojo.set_repository_election('github.com/ztest-wp8-acme/web', false);
        raise exception 'a lead changed an org public repo (403).';
    exception when insufficient_privilege then null;
    end;

    -- Alice (admin): allowed, under organisation authority.
    perform set_config('request.jwt.claims', '{"sub":"aa444444-4444-4444-4444-444444444444"}', true);
    select * into r from dojo.set_repository_election('github.com/ztest-wp8-acme/web', false);
    if r.authority is distinct from 'organization' then
        raise exception 'admin election on org public returned authority %, want organization.', r.authority;
    end if;
    if r.elected is not false then
        raise exception 'admin election on org public did not switch it off.';
    end if;
end $$;
reset role;

-- The stored row: organisation, no principal. Bob's user opt-out stays as
-- history, unread.
do $$
declare
    rows text;
begin
    -- `authority::text`: the enum's order is {user,organization}, which would
    -- list the member's history row first whatever the implementation does.
    select string_agg(authority || ':' || coalesce(principal_id::text, 'none') || ':' || elected, ', '
                      order by authority::text, principal_id nulls first)
      into rows
      from dojo.repository_elections e
      join dojo.repositories r on r.id = e.repository_id
     where r.repo_key = 'github.com/ztest-wp8-acme/web';
    if rows is distinct from 'organization:none:false, user:b4444444-4444-4444-4444-444444444444:false' then
        raise exception 'stored elections for the org public repo: %.', coalesce(rows, 'NONE');
    end if;
end $$;

rollback;
