-- Tracking and untracking a branch of a repository's shared code graph (#225;
-- docs/spec/dojo/graph-sync.md §9).
--
-- dojo.set_graph_branch decides from the caller's own verdict row of
-- dojo.all_my_repositories, as set_repository_election does:
--   no row for the caller          → no_data_found                    (404)
--   no authority yet (no capture)  → object_not_in_prerequisite_state (409)
--   not configurable_by_me         → insufficient_privilege           (403)
--   an empty branch name           → invalid_parameter_value          (400)
-- Tracking is idempotent (a re-track only updates the label); untracking deletes
-- the branch's row, and with it that branch's graph — and nothing else.
--
-- WHAT BREAKS THIS TEST: a contributor of an organisation changing what its
-- graph tracks; an untrack that leaves the branch's graph behind or takes
-- another branch's; a duplicate row on re-track; a nameless branch; any session
-- write grant on repository_graph_branches.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000151', 'a1000000-0000-4000-8000-000000000151', 'ada'),
  ('a0000000-0000-4000-8000-000000000152', 'a1000000-0000-4000-8000-000000000152', 'ola');

-- T: organisation (ada admin, ola contributor), shares private repos. P: ola's personal dōjō.
insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-00000000015a', 'organization/ztest-gb-t', 'organization', 'ztest-gb-t', 'T', 'x', now()),
  ('b0000000-0000-4000-8000-00000000015b', 'personal/ztest-gb-ola',   'personal',     'ztest-gb-ola', 'P', 'x', null);
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000015a', 'a0000000-0000-4000-8000-000000000151', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000015a', 'a0000000-0000-4000-8000-000000000152', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000015b', 'a0000000-0000-4000-8000-000000000152', 'admin',       'personal', 'github_oauth');
insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b0000000-0000-4000-8000-00000000015a', 'team', 'active', current_date - 1, current_date + 30);
insert into dojo.tenant_share_policy (tenant_id, private_repos_shared) values ('b0000000-0000-4000-8000-00000000015a', true);

-- 1 T private (the organisation's decision) · 2 P public (ola's own) · 3 P, visibility never captured
insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider, default_branch) values
  ('93000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-00000000015a', 'github.com/ztest-gb/priv', 'priv', 'private', now(), 'github', 'main'),
  ('93000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-00000000015b', 'github.com/ztest-ola/own', 'own',  'public',  now(), 'github', 'main'),
  ('93000000-0000-4000-8000-000000000003', 'b0000000-0000-4000-8000-00000000015b', 'github.com/ztest-ola/unk', 'unk',  null,      null,  'github', null);

-- U: a tenant ola is not in, with a tracked repository of its own.
insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-00000000015c', 'organization/ztest-gb-u', 'organization', 'ztest-gb-u', 'U', 'x', now());
insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider, default_branch) values
  ('93000000-0000-4000-8000-000000000004', 'b0000000-0000-4000-8000-00000000015c', 'github.com/ztest-gu/x', 'x', 'public', now(), 'github', 'main');
-- ada belongs to U and has elected its public repository, so it has a SYNCABLE
-- verdict row — hers, not ola's. Only the principal pin keeps it out of ola's.
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000015c', 'a0000000-0000-4000-8000-000000000151', 'admin', 'employer', 'github_oauth');
insert into dojo.repository_graph_branches (repository_id, tenant_id, branch, is_default) values
  ('93000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-00000000015a', 'main', true),
  ('93000000-0000-4000-8000-000000000004', 'b0000000-0000-4000-8000-00000000015c', 'main', true),
  -- ola's own repository whose visibility was never captured: hers, NOT syncable.
  ('93000000-0000-4000-8000-000000000003', 'b0000000-0000-4000-8000-00000000015b', 'main', true);
insert into dojo.repository_elections (tenant_id, repository_id, authority, principal_id, elected) values
  ('b0000000-0000-4000-8000-00000000015c', '93000000-0000-4000-8000-000000000004', 'user',
   'a0000000-0000-4000-8000-000000000151', true);

-- Repository 2 tracks main, and staging with a graph already filed.
insert into dojo.repository_graph_branches (repository_id, tenant_id, branch, is_default) values
  ('93000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-00000000015b', 'main',    true),
  ('93000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-00000000015b', 'staging', false);
insert into dojo.graph_files (tenant_id, repository_id, branch, path, content_hash, commit_sha, committed_at) values
  ('b0000000-0000-4000-8000-00000000015b', '93000000-0000-4000-8000-000000000002', 'main',    'a.ts', 'h', 'c', now()),
  ('b0000000-0000-4000-8000-00000000015b', '93000000-0000-4000-8000-000000000002', 'staging', 'a.ts', 'h', 'c', now());

set local role authenticated;

do $$
declare
    r record;
    n int;
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000152"}', true);

    -- ── track: ola's own repository ──────────────────────────────────────────
    select * into r from dojo.set_graph_branch('github.com/ztest-ola/own', 'production', true, 'prod');
    if r.tracked is distinct from true or r.branch <> 'production' then
        raise exception 'track: ola should track production on her own repository, got %', r;
    end if;
    select * into r from dojo.set_graph_branch('github.com/ztest-ola/own', 'production', true, 'production');
    reset role;
    select count(*) into n from dojo.repository_graph_branches
     where repository_id = '93000000-0000-4000-8000-000000000002' and branch = 'production';
    if n <> 1 then raise exception 'track: a re-track should leave one row, found %', n; end if;
    if (select label from dojo.repository_graph_branches
         where repository_id = '93000000-0000-4000-8000-000000000002' and branch = 'production') <> 'production' then
        raise exception 'track: a re-track should update the label';
    end if;
    if (select added_by from dojo.repository_graph_branches
         where repository_id = '93000000-0000-4000-8000-000000000002' and branch = 'production')
       is distinct from 'a0000000-0000-4000-8000-000000000152'::uuid then
        raise exception 'track: the row should name ola as who added it';
    end if;
    set local role authenticated;

    -- ── untrack: staging's graph goes, main's stays ──────────────────────────
    select * into r from dojo.set_graph_branch('github.com/ztest-ola/own', 'staging', false, null);
    if r.tracked is distinct from false then raise exception 'untrack: staging should be untracked, got %', r; end if;
    reset role;
    if exists (select 1 from dojo.graph_files where repository_id = '93000000-0000-4000-8000-000000000002' and branch = 'staging') then
        raise exception 'untrack: staging''s graph outlived its branch';
    end if;
    if not exists (select 1 from dojo.graph_files where repository_id = '93000000-0000-4000-8000-000000000002' and branch = 'main') then
        raise exception 'untrack: untracking staging took main''s graph with it';
    end if;
    if not exists (select 1 from dojo.repository_graph_branches
                    where repository_id = '93000000-0000-4000-8000-000000000002' and branch = 'production') then
        raise exception 'untrack: untracking staging untracked production too';
    end if;
    set local role authenticated;

    -- ── the plan's branches: every repository she may sync, none she may not ─
    -- ola is a contributor on no team: the table's read policy shows her nothing,
    -- yet her daemon must know which of her checkouts to push.
    if not exists (select 1 from dojo.my_graph_branches() b
                    where b.repository_id = '93000000-0000-4000-8000-000000000001' and b.branch = 'main' and b.is_default) then
        raise exception 'plan: ola may sync T''s repository, but its tracked main is not in her branches';
    end if;
    if exists (select 1 from dojo.my_graph_branches() b where b.repository_id = '93000000-0000-4000-8000-000000000004') then
        raise exception 'plan: another member''s repository leaked into ola''s branches';
    end if;
    if exists (select 1 from dojo.my_graph_branches() b where b.repository_id = '93000000-0000-4000-8000-000000000003') then
        raise exception 'plan: a repository of ola''s that may not sync is in her branches';
    end if;

    -- ── refusals ─────────────────────────────────────────────────────────────
    begin
        perform dojo.set_graph_branch('github.com/ztest-gb/priv', 'production', true, null);
        raise exception 'authority: a contributor changed what the organisation''s graph tracks';
    exception when insufficient_privilege then null;
    end;
    begin
        perform dojo.set_graph_branch('github.com/ztest-nobody/x', 'production', true, null);
        raise exception 'unknown: a repository the caller does not have was changed';
    exception when no_data_found then null;
    end;
    begin
        perform dojo.set_graph_branch('github.com/ztest-ola/unk', 'production', true, null);
        raise exception 'prerequisite: a repository with no authority yet was changed';
    exception when object_not_in_prerequisite_state then null;
    end;
    begin
        perform dojo.set_graph_branch('github.com/ztest-ola/own', '  ', true, null);
        raise exception 'name: a nameless branch was tracked';
    exception when invalid_parameter_value then null;
    end;
    begin
        insert into dojo.repository_graph_branches (repository_id, tenant_id, branch)
        values ('93000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-00000000015b', 'x');
        raise exception 'repository_graph_branches: a session wrote the table directly';
    exception when insufficient_privilege then null;
    end;

    -- ── the organisation's admin decides for it ──────────────────────────────
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000151"}', true);
    select * into r from dojo.set_graph_branch('github.com/ztest-gb/priv', 'production', true, null);
    if r.tracked is distinct from true then raise exception 'authority: the admin should track production, got %', r; end if;
end $$;

rollback;
