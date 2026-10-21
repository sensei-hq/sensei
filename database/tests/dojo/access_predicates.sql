-- The access matrix's predicates (#236 step 2a, docs/spec/dojo/access-matrix.md).
--
-- Every dōjō policy reduces to a few relationships between the signed-in
-- principal and a row: is a member of its tenant, holds at least a tenant role,
-- holds at least a role on its repository (#307 D5 — access lives on
-- repositories, superseding team membership). These functions are those, and
-- nothing else decides them.
--
-- The fixture makes each assertion distinguishable from its neighbours:
--   alice  contributor in T, contributor on repository R
--   dave   maintainer  in T, maintainer on R
--   bob    lead (TENANT role) in T, no role on R — the tenant lead is not a repo maintainer
--   erin   admin in T, no role on R
--   frank  admin in T, DISABLED — a disabled membership is no membership
--   carol  contributor in tenant U only
--
-- WHAT BREAKS THIS TEST: ignoring `disabled_at`; comparing roles by text instead
-- of the enum's order (lead must outrank maintainer); treating the tenant role
-- `lead` as a repository role; letting any of these answer true with no JWT.

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

insert into dojo.projects (id, tenant_id, slug, name) values
  ('c0000000-0000-4000-8000-0000000000f1',
   'b0000000-0000-4000-8000-00000000000a', 'ztest-access-p', 'project P');

insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('90000000-0000-4000-8000-0000000000f1', 'b0000000-0000-4000-8000-00000000000a', 'github.com/ztest-access/r', 'r', 'github');
insert into dojo.repositories_in_projects (project_id, repository_id, tenant_id) values
  ('c0000000-0000-4000-8000-0000000000f1', '90000000-0000-4000-8000-0000000000f1', 'b0000000-0000-4000-8000-00000000000a');
insert into dojo.repository_roles (repository_id, principal_id, forge_permission, forge_role) values
  ('90000000-0000-4000-8000-0000000000f1', 'a0000000-0000-4000-8000-000000000001', 'write',    'contributor'),
  ('90000000-0000-4000-8000-0000000000f1', 'a0000000-0000-4000-8000-000000000002', 'maintain', 'maintainer'),
  ('90000000-0000-4000-8000-0000000000f1', 'a0000000-0000-4000-8000-000000000005', 'admin',    'admin');

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
  (null,    'repo>=contributor', false),
  ('alice', 'repo>=contributor', true),
  ('dave',  'repo>=contributor', true),
  ('bob',   'repo>=contributor', false),
  ('carol', 'repo>=contributor', false),
  ('frank', 'repo>=contributor', false),  -- a forge admin with a disabled membership is nothing
  ('alice', 'repo>=maintainer',  false),
  ('dave',  'repo>=maintainer',  true),
  ('bob',   'repo>=maintainer',  false),  -- the tenant lead is not a repository maintainer
  ('erin',  'repo>=maintainer',  false);

do $$
declare
    r record;
    sub text;
    got boolean;
    t constant uuid := 'b0000000-0000-4000-8000-00000000000a';
    repo constant uuid := '90000000-0000-4000-8000-0000000000f1';
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
            when 'repo>=contributor' then dojo.repo_role_at_least(repo, 'contributor')
            when 'repo>=maintainer'  then dojo.repo_role_at_least(repo, 'maintainer')
        end;
        if got is distinct from r.want then
            raise exception 'access predicate %: % should be %, got %',
                r.pred, coalesce(r.who, '(no JWT)'), r.want, got;
        end if;
    end loop;
end $$;

rollback;
