-- `dojo.workspace_activity(uuid[])` — per-workspace latest activity behind a
-- per-id gate: the newest of any reading landed (`repository_metrics.pushed_at`)
-- or graph sync filed (`graph_heads.pushed_at`) across the workspace's
-- repositories, for the ids passed in that the caller may read. Backs the
-- workspaces sort (most recently active first, quiet last, never invented).
--
-- SECURITY DEFINER, gated by `dojo.can_read_workspace(id)` per id, called and
-- never copied: the links admit no direct reads, so an invoker view 500s for
-- every caller (r1 N2). Activity counts at the WORKSPACE level — the latest
-- landing of any of its repositories, even ones the caller may not read — and
-- a timestamp exposes no metric value.
--
-- WHAT BREAKS THIS TEST: the max reading min or first-wins, computed_on used
-- instead of pushed_at, graph syncs ignored, a quiet workspace dated or
-- dropped, a tenant-B row answered to tenant A, an outsider answered
-- anything, a viewer refused or short-counted, the function losing
-- security-definer or its `authenticated` execute, the `p_ids` arg renamed.
--
-- Run with the function's DDL applied (`run.sh --apply dojo
-- ../ddl/function/dojo/workspace_activity.ddl dojo/workspace_activity.sql`), or
-- against a database the DDL has deployed to: opens its own transaction,
-- asserts, rolls back.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a1111111-1111-1111-1111-111111111111', 'b1111111-1111-1111-1111-111111111111', 'alice'),
  ('a2222222-2222-2222-2222-222222222222', 'b2222222-2222-2222-2222-222222222222', 'dave'),
  ('a3333333-3333-3333-3333-333333333333', 'b3333333-3333-3333-3333-333333333333', 'bob'),
  ('a4444444-4444-4444-4444-444444444444', 'b4444444-4444-4444-4444-444444444444', 'carol');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('c1111111-1111-1111-1111-111111111111', 'organization/ztest-act', 'organization', 'ztest-act',
   'Act', 'dojo.sensei-hq.org/organization/ztest-act'),
  ('d1111111-1111-1111-1111-111111111111', 'organization/ztest-bee', 'organization', 'ztest-bee',
   'Bee', 'dojo.sensei-hq.org/organization/ztest-bee');

-- Alice admins A; dave is A's plain member with a role on r1 only; bob is a
-- member of nothing; carol views A (and w-act below) without membership.
insert into dojo.memberships (id, tenant_id, user_id, role, kind, authenticated_via) values
  ('e1111111-1111-1111-1111-111111111111', 'c1111111-1111-1111-1111-111111111111',
   'a1111111-1111-1111-1111-111111111111', 'admin', 'employer', 'github_oauth'),
  ('e2222222-2222-2222-2222-222222222222', 'c1111111-1111-1111-1111-111111111111',
   'a2222222-2222-2222-2222-222222222222', 'contributor', 'employer', 'github_oauth');

insert into dojo.workspaces (id, tenant_id, slug, name) values
  ('c2222222-2222-2222-2222-222222222222', 'c1111111-1111-1111-1111-111111111111', 'w-act', 'Act'),
  ('c3333333-3333-3333-3333-333333333333', 'c1111111-1111-1111-1111-111111111111', 'w-graph', 'Graph'),
  ('c4444444-4444-4444-4444-444444444444', 'c1111111-1111-1111-1111-111111111111', 'w-quiet', 'Quiet'),
  ('c5555555-5555-5555-5555-555555555555', 'c1111111-1111-1111-1111-111111111111', 'w-empty', 'Empty'),
  ('d2222222-2222-2222-2222-222222222222', 'd1111111-1111-1111-1111-111111111111', 'w-bee', 'Bee');

insert into dojo.repositories (id, tenant_id, repo_key, remote_url, name, provider) values
  ('c6666666-6666-6666-6666-666666666666', 'c1111111-1111-1111-1111-111111111111', 'github.com/act/r1',
   'git@github.com:act/r1.git', 'r1', 'github'),
  ('c7777777-7777-7777-7777-777777777777', 'c1111111-1111-1111-1111-111111111111', 'github.com/act/r2',
   'git@github.com:act/r2.git', 'r2', 'github'),
  ('c8888888-8888-8888-8888-888888888888', 'c1111111-1111-1111-1111-111111111111', 'github.com/act/r3',
   'git@github.com:act/r3.git', 'r3', 'github'),
  ('c9999999-9999-9999-9999-999999999999', 'c1111111-1111-1111-1111-111111111111', 'github.com/act/r4',
   'git@github.com:act/r4.git', 'r4', 'github'),
  ('d9999999-9999-9999-9999-999999999999', 'd1111111-1111-1111-1111-111111111111', 'github.com/bee/r5',
   'git@github.com:bee/r5.git', 'r5', 'github');

insert into dojo.repositories_in_workspaces (workspace_id, repository_id, tenant_id) values
  ('c2222222-2222-2222-2222-222222222222', 'c6666666-6666-6666-6666-666666666666', 'c1111111-1111-1111-1111-111111111111'),
  ('c2222222-2222-2222-2222-222222222222', 'c7777777-7777-7777-7777-777777777777', 'c1111111-1111-1111-1111-111111111111'),
  ('c4444444-4444-4444-4444-444444444444', 'c8888888-8888-8888-8888-888888888888', 'c1111111-1111-1111-1111-111111111111'),
  ('c3333333-3333-3333-3333-333333333333', 'c9999999-9999-9999-9999-999999999999', 'c1111111-1111-1111-1111-111111111111'),
  ('d2222222-2222-2222-2222-222222222222', 'd9999999-9999-9999-9999-999999999999', 'd1111111-1111-1111-1111-111111111111');

insert into dojo.repository_roles (repository_id, principal_id, forge_permission, forge_role) values
  ('c6666666-6666-6666-6666-666666666666', 'a2222222-2222-2222-2222-222222222222', 'write', 'contributor');

insert into dojo.dojo_viewers (tenant_id, principal_id) values
  ('c1111111-1111-1111-1111-111111111111', 'a4444444-4444-4444-4444-444444444444');
insert into dojo.workspace_viewers (workspace_id, principal_id) values
  ('c2222222-2222-2222-2222-222222222222', 'a4444444-4444-4444-4444-444444444444');

-- r1 carries two readings; the workspace's newest landing sits on r1's SECOND
-- reading, so per-repo min, latest-computed_on and computed_on all die (F1).
-- r2's single reading sits in between.
insert into dojo.repository_metrics (tenant_id, repository_id, metric_id, scope, computed_on, value, pushed_at)
select 'c1111111-1111-1111-1111-111111111111', 'c6666666-6666-6666-6666-666666666666', m.id, 'repo',
       d.computed_on, 1, d.pushed_at
  from (values ('2026-11-05'::date, '2026-11-02T10:00:00Z'::timestamptz),
               ('2026-11-01'::date, '2026-11-06T10:00:00Z'::timestamptz)) as d(computed_on, pushed_at)
 cross join (select id from sensei.metrics order by key limit 1) m;

insert into dojo.repository_metrics (tenant_id, repository_id, metric_id, scope, computed_on, value, pushed_at)
select 'c1111111-1111-1111-1111-111111111111', 'c7777777-7777-7777-7777-777777777777', m.id, 'repo',
       '2026-10-01', 1, '2026-11-04T10:00:00Z'
  from (select id from sensei.metrics order by key limit 1) m;

-- r5 (tenant B) lands newer than anything in A: if B's data ever leaks into
-- A's read, the value gives it away.
insert into dojo.repository_metrics (tenant_id, repository_id, metric_id, scope, computed_on, value, pushed_at)
select 'd1111111-1111-1111-1111-111111111111', 'd9999999-9999-9999-9999-999999999999', m.id, 'repo',
       '2026-11-01', 1, '2026-11-07T10:00:00Z'
  from (select id from sensei.metrics order by key limit 1) m;

-- r4 syncs graphs but pushes no readings.
insert into dojo.repository_graph_branches (repository_id, tenant_id, branch) values
  ('c9999999-9999-9999-9999-999999999999', 'c1111111-1111-1111-1111-111111111111', 'main');
insert into dojo.graph_heads (repository_id, branch, tenant_id, commit_sha, committed_at, pushed_at) values
  ('c9999999-9999-9999-9999-999999999999', 'main', 'c1111111-1111-1111-1111-111111111111',
   'abc123', '2026-11-01T00:00:00Z', '2026-11-05T10:00:00Z');

-- ── Everything below runs as the caller ────────────────────────────────────
-- The gate answers for the caller, and the owner session has no JWT identity,
-- so even the aggregation checks run authenticated (as Alice, who admins A).
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub":"b1111111-1111-1111-1111-111111111111","role":"authenticated"}', true);

-- ── Aggregation: the math, as Alice ────────────────────────────────────────
do $$
declare
  v_act timestamptz;
  v_tenant uuid;
  v_graph timestamptz;
  v_quiet timestamptz;
  v_empty timestamptz;
  n int;
begin
  -- 1. Max across readings and repositories: r1's second landing wins.
  select count(*), max(last_activity_at) into n, v_act
    from dojo.workspace_activity(array['c2222222-2222-2222-2222-222222222222']::uuid[]);
  if n <> 1 then
    raise exception 'w-act must read exactly one grouped row, saw %.', n;
  end if;
  if v_act is distinct from '2026-11-06T10:00:00Z'::timestamptz then
    raise exception 'w-act activity should be r1''s 11-06 landing, saw %.', v_act;
  end if;

  -- 1b. The row carries its tenant (F4).
  select tenant_id into v_tenant
    from dojo.workspace_activity(array['c2222222-2222-2222-2222-222222222222']::uuid[]);
  if v_tenant is distinct from 'c1111111-1111-1111-1111-111111111111'::uuid then
    raise exception 'w-act tenant_id is wrong, saw %.', v_tenant;
  end if;

  -- 2. A graph sync counts as activity even with no readings (brief: "sync or reading").
  select count(*), max(last_activity_at) into n, v_graph
    from dojo.workspace_activity(array['c3333333-3333-3333-3333-333333333333']::uuid[]);
  if n <> 1 then
    raise exception 'w-graph must read exactly one grouped row, saw %.', n;
  end if;
  if v_graph is distinct from '2026-11-05T10:00:00Z'::timestamptz then
    raise exception 'w-graph activity should be the 11-05 graph sync, saw %.', v_graph;
  end if;

  -- 3. Quiet workspaces read one NULL row each: a repo with no readings, and
  -- no repos at all. Never invented, never absent.
  select count(*), max(last_activity_at) into n, v_quiet
    from dojo.workspace_activity(array['c4444444-4444-4444-4444-444444444444']::uuid[]);
  if n <> 1 then
    raise exception 'w-quiet must read exactly one row, saw %.', n;
  end if;
  if v_quiet is not null then
    raise exception 'w-quiet must read NULL, saw %.', v_quiet;
  end if;
  select count(*), max(last_activity_at) into n, v_empty
    from dojo.workspace_activity(array['c5555555-5555-5555-5555-555555555555']::uuid[]);
  if n <> 1 then
    raise exception 'w-empty must read exactly one row, saw %.', n;
  end if;
  if v_empty is not null then
    raise exception 'w-empty must read NULL, saw %.', v_empty;
  end if;

  -- 3b. Unknown ids read no row (never a NULL row for a workspace that is not there).
  select count(*) into n
    from dojo.workspace_activity(array['c2222222-2222-2222-2222-222222222222',
                                       '00000000-0000-0000-0000-000000000000']::uuid[]);
  if n <> 1 then
    raise exception 'an unknown id must read no row, saw % row(s).', n;
  end if;
end $$;

-- ── Shape: one arg named p_ids, three OUT columns, definer, callable ────────
do $$
declare
  n int;
begin
  -- 4. The rpc contract: a single uuid[] arg named `p_ids` (PostgREST matches
  -- the TS `{ p_ids }` by name), and the three OUT columns the selects above
  -- already name. A rename breaks the rpc and dies here first.
  select count(*) into n
    from pg_proc p
    join pg_namespace s on s.oid = p.pronamespace
   where s.nspname = 'dojo' and p.proname = 'workspace_activity'
     and p.pronargs = 1
     and p.proargnames[1] = 'p_ids'
     and p.proargnames @> array['workspace_id', 'tenant_id', 'last_activity_at'];
  if n <> 1 then
    raise exception 'workspace_activity(p_ids) must exist exactly once with its OUT columns.';
  end if;

  -- 5. Definer, and granted to the caller: the read path only works when both
  -- hold (an invoker body 500s on the links; a missing grant 403s the rpc).
  select count(*) into n
    from pg_proc p
    join pg_namespace s on s.oid = p.pronamespace
   where s.nspname = 'dojo' and p.proname = 'workspace_activity'
     and p.prosecdef
     and has_function_privilege('authenticated', p.oid, 'execute');
  if n <> 1 then
    raise exception 'workspace_activity must be security definer, executable by authenticated.';
  end if;
end $$;

-- ── Access: one principal at a time ────────────────────────────────────────
do $$
declare
  n int;
  v_act timestamptz;
begin
  -- 6a. Alice (admin of A) reads A's activity, and B's workspace is absent —
  -- the gate runs per id, not per call.
  perform set_config('request.jwt.claims',
    '{"sub":"b1111111-1111-1111-1111-111111111111","role":"authenticated"}', true);
  select count(*), max(last_activity_at) into n, v_act
    from dojo.workspace_activity(array['c2222222-2222-2222-2222-222222222222',
                                       'd2222222-2222-2222-2222-222222222222']::uuid[]);
  if n <> 1 then
    raise exception 'alice must read exactly w-act, saw % row(s).', n;
  end if;
  if v_act is distinct from '2026-11-06T10:00:00Z'::timestamptz then
    raise exception 'alice''s w-act activity should be the 11-06 landing, saw %.', v_act;
  end if;

  -- 6b. Dave (member of A, a role on r1 only) reads w-act but not w-graph:
  -- per-id gating for a non-admin.
  perform set_config('request.jwt.claims',
    '{"sub":"b2222222-2222-2222-2222-222222222222","role":"authenticated"}', true);
  select count(*) into n
    from dojo.workspace_activity(array['c2222222-2222-2222-2222-222222222222',
                                       'c3333333-3333-3333-3333-333333333333']::uuid[]);
  if n <> 1 then
    raise exception 'dave must read exactly w-act, saw % row(s).', n;
  end if;

  -- 6c. Bob (a member of nothing) reads nothing — and no error.
  perform set_config('request.jwt.claims',
    '{"sub":"b3333333-3333-3333-3333-333333333333","role":"authenticated"}', true);
  select count(*) into n
    from dojo.workspace_activity(array['c2222222-2222-2222-2222-222222222222']::uuid[]);
  if n <> 0 then
    raise exception 'bob must read nothing, saw % row(s).', n;
  end if;

  -- 6d. Carol (viewer of w-act, no membership, no repo role) reads w-act's
  -- FULL activity: workspace level — she reads none of its repositories, but
  -- the sort is a workspace attribute. w-graph (not granted) stays absent.
  perform set_config('request.jwt.claims',
    '{"sub":"b4444444-4444-4444-4444-444444444444","role":"authenticated"}', true);
  select count(*), max(last_activity_at) into n, v_act
    from dojo.workspace_activity(array['c2222222-2222-2222-2222-222222222222',
                                       'c3333333-3333-3333-3333-333333333333']::uuid[]);
  if n <> 1 then
    raise exception 'carol must read exactly w-act, saw % row(s).', n;
  end if;
  if v_act is distinct from '2026-11-06T10:00:00Z'::timestamptz then
    raise exception 'carol''s w-act activity should be the 11-06 landing, saw %.', v_act;
  end if;
end $$;

rollback;
