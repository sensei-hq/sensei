-- The org read on a personal tenant returns no rows: the org tab does not
-- exist there (org-repositories r2, test 9; the route 404s before SQL — the
-- function stays total and returns empty rather than raising).
--
-- WHAT BREAKS THIS TEST: a missing dojo.org_repositories(); personal rows
-- leaking into an org-grain read; a raise where empty is pinned.
--
-- Phase A: red — dojo.org_repositories(uuid) does not exist yet.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000191', 'a1000000-0000-4000-8000-000000000191', 'solo');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-000000000191', 'personal/ztest-orrepos-solo', 'personal',
   'ztest-orrepos-solo', 'solo fixture', 'dojo.sensei-hq.org/personal/ztest-orrepos-solo', now());

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-000000000191', 'a0000000-0000-4000-8000-000000000191', 'admin', 'employer', 'github_oauth');

insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider) values
  ('90000000-0000-4000-8000-000000000191', 'b0000000-0000-4000-8000-000000000191',
   'github.com/ztest-orrepos-solo/notes', 'notes', 'private', now(), 'github');

insert into dojo.repository_roles (repository_id, principal_id, forge_role) values
  ('90000000-0000-4000-8000-000000000191', 'a0000000-0000-4000-8000-000000000191', 'contributor');

set local role authenticated;

do $$
declare
    n int;
begin
    perform set_config('request.jwt.claims',
        json_build_object('sub', 'a1000000-0000-4000-8000-000000000191')::text, true);

    select count(*) into n from dojo.org_repositories('b0000000-0000-4000-8000-000000000191');
    if n <> 0 then
        raise exception 'org_repositories(personal): expected no rows, got %', n;
    end if;
end $$;

rollback;
