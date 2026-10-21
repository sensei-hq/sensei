-- Who reads which repository metric (#236 step 2c; docs/spec/dojo/access-matrix.md
-- "Metrics").
--
-- | row                 | sees it                                                      |
-- |---------------------|--------------------------------------------------------------|
-- | scope = repo        | anyone with a role on the repository (#307 D5); a tenant      |
-- |                     | admin. Workspaces have no owner (#299): shared items.          |
-- | scope = user, own   | its own person, always                                       |
-- | scope = user, other | a MAINTAINER or above on the repository (#307 D6); an admin  |
--
-- Access lives on repositories (#307 D5, superseding team membership).
--
-- WHAT BREAKS THIS TEST: requiring a role for own rows; letting a contributor
-- read another's user-scoped rows; treating the TENANT lead as a repository
-- maintainer; dropping the admin clause; reintroducing a workspace-owner or team
-- clause.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000021', 'a1000000-0000-4000-8000-000000000021', 'alice'),
  ('a0000000-0000-4000-8000-000000000022', 'a1000000-0000-4000-8000-000000000022', 'dave'),
  ('a0000000-0000-4000-8000-000000000023', 'a1000000-0000-4000-8000-000000000023', 'bob'),
  ('a0000000-0000-4000-8000-000000000024', 'a1000000-0000-4000-8000-000000000024', 'erin'),
  ('a0000000-0000-4000-8000-000000000025', 'a1000000-0000-4000-8000-000000000025', 'ivy'),
  ('a0000000-0000-4000-8000-000000000026', 'a1000000-0000-4000-8000-000000000026', 'carol'),
  ('a0000000-0000-4000-8000-000000000027', 'a1000000-0000-4000-8000-000000000027', 'zed');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000002a', 'organization/ztest-metrics-t', 'organization',
   'ztest-metrics-t', 'metrics fixture T', 'dojo.sensei-hq.org/organization/ztest-metrics-t'),
  ('b0000000-0000-4000-8000-00000000002b', 'organization/ztest-metrics-u', 'organization',
   'ztest-metrics-u', 'metrics fixture U', 'dojo.sensei-hq.org/organization/ztest-metrics-u');

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000002a', 'a0000000-0000-4000-8000-000000000021', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000002a', 'a0000000-0000-4000-8000-000000000022', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000002a', 'a0000000-0000-4000-8000-000000000023', 'lead',        'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000002a', 'a0000000-0000-4000-8000-000000000024', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000002a', 'a0000000-0000-4000-8000-000000000025', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000002b', 'a0000000-0000-4000-8000-000000000026', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000002a', 'a0000000-0000-4000-8000-000000000027', 'contributor', 'employer', 'github_oauth');

-- Workspace P holds repository R (alice contributes, dave maintains).
-- Workspace Q holds repository S (nobody has a role on it).
insert into dojo.workspaces (id, tenant_id, slug, name) values
  ('c0000000-0000-4000-8000-0000000000f2',
   'b0000000-0000-4000-8000-00000000002a', 'ztest-metrics-p', 'workspace P'),
  ('c0000000-0000-4000-8000-0000000000f3',
   'b0000000-0000-4000-8000-00000000002a', 'ztest-metrics-q', 'workspace Q');

insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('90000000-0000-4000-8000-0000000000a1', 'b0000000-0000-4000-8000-00000000002a',
   'github.com/ztest/r', 'r', 'github'),
  ('90000000-0000-4000-8000-0000000000a2', 'b0000000-0000-4000-8000-00000000002a',
   'github.com/ztest/s', 's', 'github');
insert into dojo.repositories_in_workspaces (workspace_id, repository_id, tenant_id) values
  ('c0000000-0000-4000-8000-0000000000f2', '90000000-0000-4000-8000-0000000000a1', 'b0000000-0000-4000-8000-00000000002a'),
  ('c0000000-0000-4000-8000-0000000000f3', '90000000-0000-4000-8000-0000000000a2', 'b0000000-0000-4000-8000-00000000002a');
-- Repository roles, as the forge mirror leaves them.
insert into dojo.repository_roles (repository_id, principal_id, forge_permission, forge_role) values
  ('90000000-0000-4000-8000-0000000000a1', 'a0000000-0000-4000-8000-000000000021', 'write',    'contributor'),
  ('90000000-0000-4000-8000-0000000000a1', 'a0000000-0000-4000-8000-000000000022', 'maintain', 'maintainer');

-- One metric, five rows: R repo-scoped, R user-scoped for alice / dave / zed
-- (zed has no role on R), S repo-scoped.
insert into dojo.repository_metrics (id, tenant_id, repository_id, metric_id, scope, principal_id, computed_on, value)
select v.id::uuid, 'b0000000-0000-4000-8000-00000000002a', v.repo::uuid, m.id, v.scope, v.who::uuid, current_date, 1
  from (values
    ('80000000-0000-4000-8000-000000000001', '90000000-0000-4000-8000-0000000000a1', 'repo', null),
    ('80000000-0000-4000-8000-000000000002', '90000000-0000-4000-8000-0000000000a1', 'user', 'a0000000-0000-4000-8000-000000000021'),
    ('80000000-0000-4000-8000-000000000003', '90000000-0000-4000-8000-0000000000a1', 'user', 'a0000000-0000-4000-8000-000000000022'),
    ('80000000-0000-4000-8000-000000000004', '90000000-0000-4000-8000-0000000000a1', 'user', 'a0000000-0000-4000-8000-000000000027'),
    ('80000000-0000-4000-8000-000000000005', '90000000-0000-4000-8000-0000000000a2', 'repo', null)
  ) as v(id, repo, scope, who)
 cross join (select id from sensei.metrics order by key limit 1) m;

set local role authenticated;

-- Which of the five rows each person sees, by the row's last digit.
create temp table expect (who text, sub text, want text) on commit drop;
insert into expect values
  ('alice', 'a1000000-0000-4000-8000-000000000021', '1,2'),     -- contributor on R: repo row + own; not dave's
  ('dave',  'a1000000-0000-4000-8000-000000000022', '1,2,3,4'), -- maintainer on R: every user row on R
  ('zed',   'a1000000-0000-4000-8000-000000000027', '4'),       -- own row, no role on R
  ('bob',   'a1000000-0000-4000-8000-000000000023', ''),        -- tenant lead, no repository role: nothing
  ('erin',  'a1000000-0000-4000-8000-000000000024', '1,2,3,4,5'),-- tenant admin: all
  ('ivy',   'a1000000-0000-4000-8000-000000000025', ''),        -- member with no repository role: nothing (no owners, #299)
  ('carol', 'a1000000-0000-4000-8000-000000000026', '');        -- admin of ANOTHER tenant

do $$
declare
    r record;
    got text;
begin
    for r in select * from expect loop
        perform set_config('request.jwt.claims', json_build_object('sub', r.sub)::text, true);
        select coalesce(string_agg(right(id::text, 1), ',' order by id), '') into got
          from dojo.repository_metrics
         where id::text like '80000000-0000-4000-8000-00000000000%';
        if got is distinct from r.want then
            raise exception 'repository_metrics RLS: % should see [%], saw [%]', r.who, r.want, got;
        end if;
    end loop;
end $$;

rollback;
