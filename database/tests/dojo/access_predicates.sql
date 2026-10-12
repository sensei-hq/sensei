-- The access matrix's predicates (#236 step 2a, docs/spec/dojo/access-matrix.md).
--
-- Every dōjō policy reduces to four relationships between the signed-in
-- principal and a row: is a member of its tenant, holds at least a tenant role,
-- is on a team that owns its project, leads such a team. These functions are
-- those four, and nothing else decides them.
--
-- The fixture makes each assertion distinguishable from its neighbours:
--   alice  contributor in T, on team X (which owns project P)
--   dave   maintainer  in T, LEAD of team X
--   bob    lead (TENANT role) in T, on no team — the tenant lead is not a team lead
--   erin   admin in T, on no team
--   frank  admin in T, DISABLED — a disabled membership is no membership
--   carol  contributor in tenant U only
--
-- WHAT BREAKS THIS TEST: ignoring `disabled_at`; comparing roles by text instead
-- of the enum's order (lead must outrank maintainer); treating the tenant role
-- `lead` as a team lead; letting any of these answer true with no JWT.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000001', 'a1000000-0000-4000-8000-000000000001', 'alice'),
  ('a0000000-0000-4000-8000-000000000002', 'a1000000-0000-4000-8000-000000000002', 'dave'),
  ('a0000000-0000-4000-8000-000000000003', 'a1000000-0000-4000-8000-000000000003', 'bob'),
  ('a0000000-0000-4000-8000-000000000004', 'a1000000-0000-4000-8000-000000000004', 'erin'),
  ('a0000000-0000-4000-8000-000000000005', 'a1000000-0000-4000-8000-000000000005', 'frank'),
  ('a0000000-0000-4000-8000-000000000006', 'a1000000-0000-4000-8000-000000000006', 'carol');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000000a', 'organization/ztest-access-t', 'organization',
   'ztest-access-t', 'access fixture T', 'dojo.sensei-hq.org/organization/ztest-access-t'),
  ('b0000000-0000-4000-8000-00000000000b', 'organization/ztest-access-u', 'organization',
   'ztest-access-u', 'access fixture U', 'dojo.sensei-hq.org/organization/ztest-access-u');

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via, disabled_at) values
  ('b0000000-0000-4000-8000-00000000000a', 'a0000000-0000-4000-8000-000000000001', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000000a', 'a0000000-0000-4000-8000-000000000002', 'maintainer',  'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000000a', 'a0000000-0000-4000-8000-000000000003', 'lead',        'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000000a', 'a0000000-0000-4000-8000-000000000004', 'admin',       'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000000a', 'a0000000-0000-4000-8000-000000000005', 'admin',       'employer', 'github_oauth', now()),
  ('b0000000-0000-4000-8000-00000000000b', 'a0000000-0000-4000-8000-000000000006', 'contributor', 'employer', 'github_oauth', null);

insert into dojo.projects (id, user_id, tenant_id, slug, name) values
  ('c0000000-0000-4000-8000-0000000000f1', 'a0000000-0000-4000-8000-000000000001',
   'b0000000-0000-4000-8000-00000000000a', 'ztest-access-p', 'project P');

insert into dojo.teams (id, tenant_id, name) values
  ('d0000000-0000-4000-8000-0000000000e1', 'b0000000-0000-4000-8000-00000000000a', 'ztest team X');
insert into dojo.team_members (team_id, principal_id, role) values
  ('d0000000-0000-4000-8000-0000000000e1', 'a0000000-0000-4000-8000-000000000001', 'contributor'),
  ('d0000000-0000-4000-8000-0000000000e1', 'a0000000-0000-4000-8000-000000000002', 'lead');
insert into dojo.team_projects (team_id, project_id) values
  ('d0000000-0000-4000-8000-0000000000e1', 'c0000000-0000-4000-8000-0000000000f1');

set local role authenticated;

-- One row per (who, predicate, expected). `who` null = no JWT at all.
create temp table expect (who text, pred text, want boolean) on commit drop;
insert into expect values
  (null,    'is_member',        false),
  ('alice', 'is_member',        true),
  ('frank', 'is_member',        false),
  ('carol', 'is_member',        false),
  ('alice', 'role>=maintainer', false),
  ('dave',  'role>=maintainer', true),
  ('bob',   'role>=maintainer', true),   -- lead outranks maintainer
  ('dave',  'role>=lead',       false),
  ('bob',   'role>=lead',       true),
  ('erin',  'role>=lead',       true),
  ('frank', 'role>=lead',       false),  -- disabled admin is nothing
  ('carol', 'role>=lead',       false),
  (null,    'on_team',          false),
  ('alice', 'on_team',          true),
  ('dave',  'on_team',          true),
  ('bob',   'on_team',          false),
  ('carol', 'on_team',          false),
  ('alice', 'leads_team',       false),
  ('dave',  'leads_team',       true),
  ('bob',   'leads_team',       false),  -- the tenant lead is not a team lead
  ('erin',  'leads_team',       false);

do $$
declare
    r record;
    sub text;
    got boolean;
    t constant uuid := 'b0000000-0000-4000-8000-00000000000a';
    p constant uuid := 'c0000000-0000-4000-8000-0000000000f1';
begin
    for r in select * from expect loop
        sub := case r.who
            when 'alice' then 'a1000000-0000-4000-8000-000000000001'
            when 'dave'  then 'a1000000-0000-4000-8000-000000000002'
            when 'bob'   then 'a1000000-0000-4000-8000-000000000003'
            when 'erin'  then 'a1000000-0000-4000-8000-000000000004'
            when 'frank' then 'a1000000-0000-4000-8000-000000000005'
            when 'carol' then 'a1000000-0000-4000-8000-000000000006'
        end;
        perform set_config('request.jwt.claims',
                           case when sub is null then '' else json_build_object('sub', sub)::text end,
                           true);
        got := case r.pred
            when 'is_member'        then dojo.is_member(t)
            when 'role>=maintainer' then dojo.tenant_role_at_least(t, 'maintainer')
            when 'role>=lead'       then dojo.tenant_role_at_least(t, 'lead')
            when 'on_team'          then dojo.on_team_of(p)
            when 'leads_team'       then dojo.leads_team_of(p)
        end;
        if got is distinct from r.want then
            raise exception 'access predicate %: % should be %, got %',
                r.pred, coalesce(r.who, '(no JWT)'), r.want, got;
        end if;
    end loop;
end $$;

rollback;
