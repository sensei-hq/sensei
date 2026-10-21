-- Who reads which project (#299; docs/spec/dojo/access-matrix.md "Projects and
-- repositories").
--
-- A project is ONE SHARED item, the same in sensei and the dōjō, living in one
-- dōjō — an org's, or a personal dōjō for the owner's own GitHub account. It
-- has no owner: whoever may read one of its repositories may read the project.
--
-- | reader                          | sees                                     |
-- |---------------------------------|------------------------------------------|
-- | a tenant admin                  | every project in the tenant              |
-- | a member of a team owning it    | the project                              |
-- | the personal dōjō's owner       | their personal projects (they admin it)  |
-- | a member on no owning team      | nothing — membership alone reads nothing |
--
-- WHAT BREAKS THIS TEST: a read policy keyed on anything but repository access
-- (an owner column, plain membership); can_read_repository regaining an owner
-- clause; a team predicate that ignores the project's tenant.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000061', 'a1000000-0000-4000-8000-000000000061', 'alice'),
  ('a0000000-0000-4000-8000-000000000062', 'a1000000-0000-4000-8000-000000000062', 'dave'),
  ('a0000000-0000-4000-8000-000000000063', 'a1000000-0000-4000-8000-000000000063', 'erin'),
  ('a0000000-0000-4000-8000-000000000064', 'a1000000-0000-4000-8000-000000000064', 'zed'),
  ('a0000000-0000-4000-8000-000000000065', 'a1000000-0000-4000-8000-000000000065', 'carol'),
  ('a0000000-0000-4000-8000-000000000066', 'a1000000-0000-4000-8000-000000000066', 'bob');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000006a', 'organization/ztest-projects-t', 'organization',
   'ztest-projects-t', 'projects fixture T', 'dojo.sensei-hq.org/organization/ztest-projects-t'),
  ('b0000000-0000-4000-8000-00000000006b', 'organization/ztest-projects-u', 'organization',
   'ztest-projects-u', 'projects fixture U', 'dojo.sensei-hq.org/organization/ztest-projects-u');

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000006a', 'a0000000-0000-4000-8000-000000000061', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000006a', 'a0000000-0000-4000-8000-000000000062', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000006a', 'a0000000-0000-4000-8000-000000000063', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000006a', 'a0000000-0000-4000-8000-000000000064', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000006b', 'a0000000-0000-4000-8000-000000000064', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000006b', 'a0000000-0000-4000-8000-000000000065', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000006a', 'a0000000-0000-4000-8000-000000000066', 'lead',        'employer', 'github_oauth');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000006c', 'personal/ztest-projects-alice', 'personal',
   'ztest-projects-alice', 'alice', 'dojo.sensei-hq.org/personal/ztest-projects-alice');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000006c', 'a0000000-0000-4000-8000-000000000061', 'admin', 'personal', 'github_oauth');

-- 1 P: in T, owned by team X.   2 Q: alice's PERSONAL project, in her personal dōjō.
-- 4 S: in U.                     Each holds one repository; access is the repository's.
insert into dojo.projects (id, tenant_id, slug, name, classification) values
  ('c0000000-0000-4000-8000-000000000061', 'b0000000-0000-4000-8000-00000000006a', 'ztest-p', 'P', 'company'),
  ('c0000000-0000-4000-8000-000000000062', 'b0000000-0000-4000-8000-00000000006c', 'ztest-q', 'Q', 'personal'),
  ('c0000000-0000-4000-8000-000000000064', 'b0000000-0000-4000-8000-00000000006b', 'ztest-s', 'S', 'company');
insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('96000000-0000-4000-8000-000000000061', 'b0000000-0000-4000-8000-00000000006a', 'github.com/ztest-pr-t/p', 'p', 'github'),
  ('96000000-0000-4000-8000-000000000062', 'b0000000-0000-4000-8000-00000000006c', 'github.com/ztest-pr-alice/q', 'q', 'github'),
  ('96000000-0000-4000-8000-000000000064', 'b0000000-0000-4000-8000-00000000006b', 'github.com/ztest-pr-u/s', 's', 'github');
insert into dojo.repositories_in_projects (project_id, repository_id, tenant_id) values
  ('c0000000-0000-4000-8000-000000000061', '96000000-0000-4000-8000-000000000061', 'b0000000-0000-4000-8000-00000000006a'),
  ('c0000000-0000-4000-8000-000000000062', '96000000-0000-4000-8000-000000000062', 'b0000000-0000-4000-8000-00000000006c'),
  ('c0000000-0000-4000-8000-000000000064', '96000000-0000-4000-8000-000000000064', 'b0000000-0000-4000-8000-00000000006b');

insert into dojo.teams (id, tenant_id, name) values
  ('d0000000-0000-4000-8000-000000000061', 'b0000000-0000-4000-8000-00000000006a', 'ztest projects team X');
insert into dojo.team_members (team_id, principal_id, role) values
  ('d0000000-0000-4000-8000-000000000061', 'a0000000-0000-4000-8000-000000000061', 'contributor'),
  ('d0000000-0000-4000-8000-000000000061', 'a0000000-0000-4000-8000-000000000062', 'lead');
insert into dojo.team_projects (team_id, project_id) values
  ('d0000000-0000-4000-8000-000000000061', 'c0000000-0000-4000-8000-000000000061');

set local role authenticated;

create temp table expect (who text, sub text, want text, leads text) on commit drop;
insert into expect values
  -- who     login                                   sees       leads a team of
  ('alice', 'a1000000-0000-4000-8000-000000000061', '1,2',     ''),  -- team X → P; her personal dōjō → Q
  ('dave',  'a1000000-0000-4000-8000-000000000062', '1',       '1'), -- team lead: P
  ('erin',  'a1000000-0000-4000-8000-000000000063', '1',       ''),  -- admin of T: T's projects
  ('zed',   'a1000000-0000-4000-8000-000000000064', '',        ''),  -- member of T and U, on no team: nothing
  ('carol', 'a1000000-0000-4000-8000-000000000065', '4',       ''),  -- admin of U
  ('bob',   'a1000000-0000-4000-8000-000000000066', '',        '');  -- tenant lead, no team: nothing

do $$
declare
    r record;
    got text;
    led text;
begin
    for r in select * from expect loop
        perform set_config('request.jwt.claims', json_build_object('sub', r.sub)::text, true);
        select coalesce(string_agg(right(id::text, 1), ',' order by id), '') into got
          from dojo.projects where slug like 'ztest-_';
        if got is distinct from r.want then
            raise exception 'projects RLS: % should see [%], saw [%]', r.who, r.want, got;
        end if;
        select coalesce(string_agg(right(p::text, 1), ',' order by p), '') into led
          from unnest(array['c0000000-0000-4000-8000-000000000061', 'c0000000-0000-4000-8000-000000000062',
                            'c0000000-0000-4000-8000-000000000064']::uuid[]) p
         where dojo.leads_team_of(p);
        if led is distinct from r.leads then
            raise exception 'leads_team_of: % should lead a team of [%], got [%]', r.who, r.leads, led;
        end if;
    end loop;
end $$;

rollback;
