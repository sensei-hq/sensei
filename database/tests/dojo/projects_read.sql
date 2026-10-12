-- Who reads which project (#236 step 3; docs/spec/dojo/access-matrix.md
-- "Projects and repositories").
--
-- | reader                        | sees                                            |
-- |-------------------------------|-------------------------------------------------|
-- | the owner                     | the project, wherever it is homed               |
-- | a member of an owning team    | the project, while it is homed in the team's tenant |
-- | a tenant admin                | every project homed in the tenant               |
--
-- A project's home follows its binding: the daemon files it under the one
-- membership it is bound to, and as personal (no tenant) everywhere else. A
-- team link left over from a tenant the project has LEFT must not keep showing
-- it there — the team belongs to the old tenant, the project no longer does.
--
-- WHAT BREAKS THIS TEST: a team predicate that ignores the project's tenant; a
-- read policy without the owner clause, the team clause or the admin clause;
-- an admin clause at a lower role.

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

-- 1 P: zed's, homed in T, owned by team X.
-- 2 Q: alice's, PERSONAL (no tenant), still linked to team X from before.
-- 3 R: zed's, re-homed to U, still linked to team X from before.
-- 4 S: carol's, homed in U.
insert into dojo.projects (id, user_id, tenant_id, slug, name, classification) values
  ('c0000000-0000-4000-8000-000000000061', 'a0000000-0000-4000-8000-000000000064',
   'b0000000-0000-4000-8000-00000000006a', 'ztest-p', 'P', 'company'),
  ('c0000000-0000-4000-8000-000000000062', 'a0000000-0000-4000-8000-000000000061',
   null, 'ztest-q', 'Q', 'personal'),
  ('c0000000-0000-4000-8000-000000000063', 'a0000000-0000-4000-8000-000000000064',
   'b0000000-0000-4000-8000-00000000006b', 'ztest-r', 'R', 'company'),
  ('c0000000-0000-4000-8000-000000000064', 'a0000000-0000-4000-8000-000000000065',
   'b0000000-0000-4000-8000-00000000006b', 'ztest-s', 'S', 'company');

insert into dojo.teams (id, tenant_id, name) values
  ('d0000000-0000-4000-8000-000000000061', 'b0000000-0000-4000-8000-00000000006a', 'ztest projects team X');
insert into dojo.team_members (team_id, principal_id, role) values
  ('d0000000-0000-4000-8000-000000000061', 'a0000000-0000-4000-8000-000000000061', 'contributor'),
  ('d0000000-0000-4000-8000-000000000061', 'a0000000-0000-4000-8000-000000000062', 'lead');
insert into dojo.team_projects (team_id, project_id) values
  ('d0000000-0000-4000-8000-000000000061', 'c0000000-0000-4000-8000-000000000061'),
  ('d0000000-0000-4000-8000-000000000061', 'c0000000-0000-4000-8000-000000000062'),
  ('d0000000-0000-4000-8000-000000000061', 'c0000000-0000-4000-8000-000000000063');

set local role authenticated;

create temp table expect (who text, sub text, want text, leads text) on commit drop;
insert into expect values
  -- who     login                                   sees       leads a team of
  ('alice', 'a1000000-0000-4000-8000-000000000061', '1,2',     ''),  -- team P + own Q; not R (left T)
  ('dave',  'a1000000-0000-4000-8000-000000000062', '1',       '1'), -- team lead: P only; Q is personal, R left
  ('erin',  'a1000000-0000-4000-8000-000000000063', '1',       ''),  -- admin of T: homed in T only
  ('zed',   'a1000000-0000-4000-8000-000000000064', '1,3',     ''),  -- own, wherever homed
  ('carol', 'a1000000-0000-4000-8000-000000000065', '3,4',     ''),  -- admin of U
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
                            'c0000000-0000-4000-8000-000000000063', 'c0000000-0000-4000-8000-000000000064']::uuid[]) p
         where dojo.leads_team_of(p);
        if led is distinct from r.leads then
            raise exception 'leads_team_of: % should lead a team of [%], got [%]', r.who, r.leads, led;
        end if;
    end loop;
end $$;

rollback;
