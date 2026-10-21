-- Who reads which workspace (#299; docs/spec/dojo/access-matrix.md "Workspaces and
-- repositories").
--
-- A workspace is ONE SHARED item, the same in sensei and the dōjō, living in one
-- dōjō — an org's, or a personal dōjō for the owner's own GitHub account. It
-- has no owner: whoever may read one of its repositories may read the workspace.
--
-- | reader                          | sees                                     |
-- |---------------------------------|------------------------------------------|
-- | a tenant admin                  | every workspace in the tenant              |
-- | a role on one of its repos      | the workspace (#307 D5)                    |
-- | a workspace viewer                | that workspace, members or dōjō viewers (D8)|
-- | the personal dōjō's owner       | their personal workspaces (they admin it)  |
-- | a member with no repo role      | nothing — membership alone reads nothing |
--
-- WHAT BREAKS THIS TEST: a read policy keyed on anything but repository access, a
-- viewer grant or the dōjō's admin (an owner column, plain membership, a team);
-- can_read_repository regaining an owner clause; a viewer grant reading another
-- workspace; an admin unable to see their dōjō's workspace before it holds a repository.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000061', 'a1000000-0000-4000-8000-000000000061', 'alice'),
  ('a0000000-0000-4000-8000-000000000062', 'a1000000-0000-4000-8000-000000000062', 'dave'),
  ('a0000000-0000-4000-8000-000000000063', 'a1000000-0000-4000-8000-000000000063', 'erin'),
  ('a0000000-0000-4000-8000-000000000064', 'a1000000-0000-4000-8000-000000000064', 'zed'),
  ('a0000000-0000-4000-8000-000000000065', 'a1000000-0000-4000-8000-000000000065', 'carol'),
  ('a0000000-0000-4000-8000-000000000066', 'a1000000-0000-4000-8000-000000000066', 'bob'),
  ('a0000000-0000-4000-8000-000000000067', 'a1000000-0000-4000-8000-000000000067', 'vic');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000006a', 'organization/ztest-workspaces-t', 'organization',
   'ztest-workspaces-t', 'workspaces fixture T', 'dojo.sensei-hq.org/organization/ztest-workspaces-t'),
  ('b0000000-0000-4000-8000-00000000006b', 'organization/ztest-workspaces-u', 'organization',
   'ztest-workspaces-u', 'workspaces fixture U', 'dojo.sensei-hq.org/organization/ztest-workspaces-u');

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000006a', 'a0000000-0000-4000-8000-000000000061', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000006a', 'a0000000-0000-4000-8000-000000000062', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000006a', 'a0000000-0000-4000-8000-000000000063', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000006a', 'a0000000-0000-4000-8000-000000000064', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000006b', 'a0000000-0000-4000-8000-000000000064', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000006b', 'a0000000-0000-4000-8000-000000000065', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000006a', 'a0000000-0000-4000-8000-000000000066', 'lead',        'employer', 'github_oauth');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000006c', 'personal/ztest-workspaces-alice', 'personal',
   'ztest-workspaces-alice', 'alice', 'dojo.sensei-hq.org/personal/ztest-workspaces-alice');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000006c', 'a0000000-0000-4000-8000-000000000061', 'admin', 'personal', 'github_oauth');

-- 1 P: in T (alice contributes, dave maintains; vic views).   2 Q: alice's PERSONAL workspace, in her personal dōjō.
-- 4 S: in U.                     Each holds one repository; access is the repository's.
-- 5 R: in T, NOT YET holding any repository — its admins still see it, to fill it
--      and grant viewers on it; nobody else does.
insert into dojo.workspaces (id, tenant_id, slug, name, classification) values
  ('c0000000-0000-4000-8000-000000000061', 'b0000000-0000-4000-8000-00000000006a', 'ztest-p', 'P', 'company'),
  ('c0000000-0000-4000-8000-000000000062', 'b0000000-0000-4000-8000-00000000006c', 'ztest-q', 'Q', 'personal'),
  ('c0000000-0000-4000-8000-000000000064', 'b0000000-0000-4000-8000-00000000006b', 'ztest-s', 'S', 'company'),
  ('c0000000-0000-4000-8000-000000000065', 'b0000000-0000-4000-8000-00000000006a', 'ztest-r', 'R', 'company');
insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('96000000-0000-4000-8000-000000000061', 'b0000000-0000-4000-8000-00000000006a', 'github.com/ztest-pr-t/p', 'p', 'github'),
  ('96000000-0000-4000-8000-000000000062', 'b0000000-0000-4000-8000-00000000006c', 'github.com/ztest-pr-alice/q', 'q', 'github'),
  ('96000000-0000-4000-8000-000000000064', 'b0000000-0000-4000-8000-00000000006b', 'github.com/ztest-pr-u/s', 's', 'github');
insert into dojo.repositories_in_workspaces (workspace_id, repository_id, tenant_id) values
  ('c0000000-0000-4000-8000-000000000061', '96000000-0000-4000-8000-000000000061', 'b0000000-0000-4000-8000-00000000006a'),
  ('c0000000-0000-4000-8000-000000000062', '96000000-0000-4000-8000-000000000062', 'b0000000-0000-4000-8000-00000000006c'),
  ('c0000000-0000-4000-8000-000000000064', '96000000-0000-4000-8000-000000000064', 'b0000000-0000-4000-8000-00000000006b');

insert into dojo.repository_roles (repository_id, principal_id, forge_permission, forge_role) values
  ('96000000-0000-4000-8000-000000000061', 'a0000000-0000-4000-8000-000000000061', 'write',    'contributor'),
  ('96000000-0000-4000-8000-000000000061', 'a0000000-0000-4000-8000-000000000062', 'maintain', 'maintainer');
-- vic is a client invited into T as a viewer (not a member), granted the view of P.
insert into dojo.dojo_viewers (tenant_id, principal_id) values
  ('b0000000-0000-4000-8000-00000000006a', 'a0000000-0000-4000-8000-000000000067');
insert into dojo.workspace_viewers (workspace_id, principal_id) values
  ('c0000000-0000-4000-8000-000000000061', 'a0000000-0000-4000-8000-000000000067');

set local role authenticated;

create temp table expect (who text, sub text, want text) on commit drop;
insert into expect values
  -- who     login                                   sees
  ('alice', 'a1000000-0000-4000-8000-000000000061', '1,2'),  -- a role on P's repository; her personal dōjō → Q
  ('dave',  'a1000000-0000-4000-8000-000000000062', '1'),    -- maintains P's repository
  ('erin',  'a1000000-0000-4000-8000-000000000063', '1,5'),  -- admin of T: T's workspaces, the empty one too
  ('zed',   'a1000000-0000-4000-8000-000000000064', ''),     -- member of T and U, no repository role: nothing
  ('carol', 'a1000000-0000-4000-8000-000000000065', '4'),    -- admin of U
  ('bob',   'a1000000-0000-4000-8000-000000000066', ''),     -- tenant lead, no repository role: nothing
  ('vic',   'a1000000-0000-4000-8000-000000000067', '1');    -- dōjō viewer granted P, not a member

do $$
declare
    r record;
    got text;
begin
    for r in select * from expect loop
        perform set_config('request.jwt.claims', json_build_object('sub', r.sub)::text, true);
        select coalesce(string_agg(right(id::text, 1), ',' order by id), '') into got
          from dojo.workspaces where slug like 'ztest-_';
        if got is distinct from r.want then
            raise exception 'workspaces RLS: % should see [%], saw [%]', r.who, r.want, got;
        end if;
    end loop;
end $$;

rollback;
