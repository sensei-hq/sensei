-- Who sees THAT a run is going on a project (#252; docs/spec/dojo/access-matrix.md
-- "Relay" — run status row).
--
-- | caller                                  | sees                                   |
-- |-----------------------------------------|----------------------------------------|
-- | the run's owner                         | their runs                             |
-- | anyone with a role on one of P's repos  | runs on P — status only (#307 D5)      |
-- | a tenant admin                          | every run in the tenant — status only  |
-- | anyone else (a tenant lead with no role,| nothing                                |
-- | member of another tenant)               |                                        |
--
-- Status only: `dojo.peer_run_status` returns no title, goal, phase or feature —
-- relay CONTENT stays the owner's (#249). A project is a SHARED item (#299): a
-- run may be filed under any project in a dōjō the runner is a member of, and
-- never under one in a dōjō they are not.
--
-- WHAT BREAKS THIS TEST: returning a content column; dropping the repository-role clause;
-- letting a tenant lead or another tenant's admin see runs; letting a session's
-- project_id name a project in a dōjō the runner is not a member of.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000031', 'a1000000-0000-4000-8000-000000000031', 'alice'),
  ('a0000000-0000-4000-8000-000000000032', 'a1000000-0000-4000-8000-000000000032', 'dave'),
  ('a0000000-0000-4000-8000-000000000033', 'a1000000-0000-4000-8000-000000000033', 'mia'),
  ('a0000000-0000-4000-8000-000000000034', 'a1000000-0000-4000-8000-000000000034', 'zed'),
  ('a0000000-0000-4000-8000-000000000035', 'a1000000-0000-4000-8000-000000000035', 'bob'),
  ('a0000000-0000-4000-8000-000000000036', 'a1000000-0000-4000-8000-000000000036', 'erin'),
  ('a0000000-0000-4000-8000-000000000037', 'a1000000-0000-4000-8000-000000000037', 'carol');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000003a', 'organization/ztest-peer-t', 'organization',
   'ztest-peer-t', 'peer fixture T', 'dojo.sensei-hq.org/organization/ztest-peer-t'),
  ('b0000000-0000-4000-8000-00000000003b', 'organization/ztest-peer-u', 'organization',
   'ztest-peer-u', 'peer fixture U', 'dojo.sensei-hq.org/organization/ztest-peer-u');

insert into dojo.memberships (id, tenant_id, user_id, role, kind, authenticated_via) values
  ('e0000000-0000-4000-8000-000000000031', 'b0000000-0000-4000-8000-00000000003a', 'a0000000-0000-4000-8000-000000000031', 'contributor', 'employer', 'github_oauth'),
  ('e0000000-0000-4000-8000-000000000032', 'b0000000-0000-4000-8000-00000000003a', 'a0000000-0000-4000-8000-000000000032', 'contributor', 'employer', 'github_oauth'),
  ('e0000000-0000-4000-8000-000000000033', 'b0000000-0000-4000-8000-00000000003a', 'a0000000-0000-4000-8000-000000000033', 'contributor', 'employer', 'github_oauth'),
  ('e0000000-0000-4000-8000-000000000034', 'b0000000-0000-4000-8000-00000000003a', 'a0000000-0000-4000-8000-000000000034', 'contributor', 'employer', 'github_oauth'),
  ('e0000000-0000-4000-8000-000000000035', 'b0000000-0000-4000-8000-00000000003a', 'a0000000-0000-4000-8000-000000000035', 'lead',        'employer', 'github_oauth'),
  ('e0000000-0000-4000-8000-000000000036', 'b0000000-0000-4000-8000-00000000003a', 'a0000000-0000-4000-8000-000000000036', 'admin',       'employer', 'github_oauth'),
  ('e0000000-0000-4000-8000-000000000037', 'b0000000-0000-4000-8000-00000000003b', 'a0000000-0000-4000-8000-000000000037', 'admin',       'employer', 'github_oauth');

-- Project P holds repository RP: dave maintains it, mia contributes. Nobody has a
-- role on Q's repositories (it has none).
-- R lives in U, where alice is not a member.
insert into dojo.projects (id, tenant_id, slug, name) values
  ('c0000000-0000-4000-8000-0000000000a1',
   'b0000000-0000-4000-8000-00000000003a', 'ztest-peer-p', 'project P'),
  ('c0000000-0000-4000-8000-0000000000a2',
   'b0000000-0000-4000-8000-00000000003a', 'ztest-peer-q', 'project Q'),
  ('c0000000-0000-4000-8000-0000000000a3',
   'b0000000-0000-4000-8000-00000000003b', 'ztest-peer-r', 'project R');
insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('90000000-0000-4000-8000-0000000000b1', 'b0000000-0000-4000-8000-00000000003a', 'github.com/ztest-peer/rp', 'rp', 'github');
insert into dojo.repositories_in_projects (project_id, repository_id, tenant_id) values
  ('c0000000-0000-4000-8000-0000000000a1', '90000000-0000-4000-8000-0000000000b1', 'b0000000-0000-4000-8000-00000000003a');
insert into dojo.repository_roles (repository_id, principal_id, forge_permission, forge_role) values
  ('90000000-0000-4000-8000-0000000000b1', 'a0000000-0000-4000-8000-000000000032', 'maintain', 'maintainer'),
  ('90000000-0000-4000-8000-0000000000b1', 'a0000000-0000-4000-8000-000000000033', 'write',    'contributor');

-- Runs, by last digit: 1 alice on P · 2 alice on no project · 3 zed on Q.
insert into dojo.relay_sessions (membership_id, run_id, title, goal, status, project_id) values
  ('e0000000-0000-4000-8000-000000000031', 'f0000000-0000-4000-8000-000000000001', 'secret title', 'secret goal', 'running', 'c0000000-0000-4000-8000-0000000000a1'),
  ('e0000000-0000-4000-8000-000000000031', 'f0000000-0000-4000-8000-000000000002', 'secret title', 'secret goal', 'running', null),
  ('e0000000-0000-4000-8000-000000000034', 'f0000000-0000-4000-8000-000000000003', 'secret title', 'secret goal', 'paused',  'c0000000-0000-4000-8000-0000000000a2');

-- ── the result carries status, never content ────────────────────────────────
do $$
declare
    result text := pg_get_function_result('dojo.peer_run_status(uuid)'::regprocedure);
begin
    if result ~* '(title|goal|phase|feature|pause_reason|membership)' then
        raise exception 'peer_run_status returns content: %', result;
    end if;
end $$;

set local role authenticated;

create temp table expect (who text, sub text, want text) on commit drop;
insert into expect values
  ('alice', 'a1000000-0000-4000-8000-000000000031', '1,2'),   -- own runs
  ('dave',  'a1000000-0000-4000-8000-000000000032', '1'),     -- maintains P's repository
  ('mia',   'a1000000-0000-4000-8000-000000000033', '1'),     -- contributes to P's repository
  ('zed',   'a1000000-0000-4000-8000-000000000034', '3'),     -- own run; no role on P's repositories
  ('bob',   'a1000000-0000-4000-8000-000000000035', ''),      -- tenant lead, no repository role: nothing
  ('erin',  'a1000000-0000-4000-8000-000000000036', '1,2,3'), -- tenant admin: every run
  ('carol', 'a1000000-0000-4000-8000-000000000037', '');      -- admin of ANOTHER tenant

do $$
declare
    r record;
    got text;
begin
    for r in select * from expect loop
        perform set_config('request.jwt.claims', json_build_object('sub', r.sub)::text, true);
        select coalesce(string_agg(right(run_id::text, 1), ',' order by run_id), '') into got
          from dojo.peer_run_status('b0000000-0000-4000-8000-00000000003a');
        if got is distinct from r.want then
            raise exception 'peer_run_status: % should see [%], saw [%]', r.who, r.want, got;
        end if;
    end loop;
end $$;

-- ── a session is filed under a project in a dōjō the runner belongs to ───────
do $$
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000031"}', true);
    begin
        update dojo.relay_sessions set project_id = 'c0000000-0000-4000-8000-0000000000a3'
         where run_id = 'f0000000-0000-4000-8000-000000000002';
        raise exception 'relay_sessions: alice filed her run under a project in a dōjō she is not in';
    exception when insufficient_privilege or check_violation then null;
    end;
    -- Any project of her own dōjō is fine — projects are shared, not anyone's.
    update dojo.relay_sessions set project_id = 'c0000000-0000-4000-8000-0000000000a2'
     where run_id = 'f0000000-0000-4000-8000-000000000002';
    update dojo.relay_sessions set project_id = 'c0000000-0000-4000-8000-0000000000a1'
     where run_id = 'f0000000-0000-4000-8000-000000000002';
end $$;

rollback;
