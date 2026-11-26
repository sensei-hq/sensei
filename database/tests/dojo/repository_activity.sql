-- `dojo.repository_activity(uuid[])` (#531) — per-repository latest activity
-- behind a per-id gate: the newest of any reading landed
-- (`repository_metrics.pushed_at`) or graph sync filed
-- (`graph_heads.pushed_at`). Backs the repository sort (most recently active
-- first, quiet last, never invented) and the timestamp the lists show, so the
-- two can never disagree.
--
-- SECURITY DEFINER, gated per id by `dojo.can_read_repository` — the caller
-- learns activity only for repositories they may read. A timestamp exposes no
-- metric value.
--
-- WHAT BREAKS THIS TEST: the max reading min or first-wins, computed_on used
-- instead of pushed_at, graph syncs ignored, a quiet repository dated or
-- dropped, a tenant-B row answered to tenant A, an outsider answered
-- anything, the function losing security-definer or its `authenticated`
-- execute, the `p_ids` arg renamed.
--
-- Run with the function's DDL applied (`run.sh --apply dojo
-- ../ddl/function/dojo/repository_activity.ddl dojo/repository_activity.sql`),
-- or against a database the DDL has deployed to: opens its own transaction,
-- asserts, rolls back.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('f1111111-1111-4111-8111-111111111111', 'f2111111-1111-4111-8111-111111111111', 'amy'),
  ('f1111111-2222-4111-8111-111111111111', 'f2111111-2222-4111-8111-111111111111', 'max'),
  ('f1111111-3333-4111-8111-111111111111', 'f2111111-3333-4111-8111-111111111111', 'otto');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('f3111111-1111-4111-8111-111111111111', 'organization/ztest-ract', 'organization', 'ztest-ract',
   'Ract', 'dojo.sensei-hq.org/organization/ztest-ract'),
  ('f3111111-2222-4111-8111-111111111111', 'organization/ztest-rbee', 'organization', 'ztest-rbee',
   'Rbee', 'dojo.sensei-hq.org/organization/ztest-rbee');

-- Amy admins A; max is A's plain member with a role on r1 only; otto is a
-- member of nothing.
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('f3111111-1111-4111-8111-111111111111', 'f1111111-1111-4111-8111-111111111111', 'admin', 'employer', 'github_oauth'),
  ('f3111111-1111-4111-8111-111111111111', 'f1111111-2222-4111-8111-111111111111', 'contributor', 'employer', 'github_oauth');

insert into dojo.repositories (id, tenant_id, repo_key, remote_url, name, provider) values
  ('f4111111-1111-4111-8111-111111111111', 'f3111111-1111-4111-8111-111111111111', 'github.com/ract/r1',
   'git@github.com:ract/r1.git', 'r1', 'github'),
  ('f4111111-2222-4111-8111-111111111111', 'f3111111-1111-4111-8111-111111111111', 'github.com/ract/r2',
   'git@github.com:ract/r2.git', 'r2', 'github'),
  ('f4111111-3333-4111-8111-111111111111', 'f3111111-1111-4111-8111-111111111111', 'github.com/ract/r3',
   'git@github.com:ract/r3.git', 'r3', 'github'),
  ('f4111111-4444-4111-8111-111111111111', 'f3111111-2222-4111-8111-111111111111', 'github.com/rbee/r4',
   'git@github.com:rbee/r4.git', 'r4', 'github');

insert into dojo.repository_roles (repository_id, principal_id, forge_permission, forge_role) values
  ('f4111111-1111-4111-8111-111111111111', 'f1111111-2222-4111-8111-111111111111', 'write', 'contributor');

-- r1: two readings (newest pushed_at wins, not newest computed_on) plus a
-- graph sync NEWER than both — the graph landing is the activity.
insert into dojo.repository_metrics (tenant_id, repository_id, metric_id, scope, computed_on, value, pushed_at)
select 'f3111111-1111-4111-8111-111111111111', 'f4111111-1111-4111-8111-111111111111', m.id, 'repo',
       d.computed_on, 1, d.pushed_at
  from (values ('2026-11-05'::date, '2026-11-02T10:00:00Z'::timestamptz),
               ('2026-11-01'::date, '2026-11-03T10:00:00Z'::timestamptz)) as d(computed_on, pushed_at)
 cross join (select id from sensei.metrics order by key limit 1) m;
insert into dojo.repository_graph_branches (repository_id, tenant_id, branch) values
  ('f4111111-1111-4111-8111-111111111111', 'f3111111-1111-4111-8111-111111111111', 'main');
insert into dojo.graph_heads (repository_id, branch, tenant_id, commit_sha, committed_at, pushed_at) values
  ('f4111111-1111-4111-8111-111111111111', 'main', 'f3111111-1111-4111-8111-111111111111',
   'abc123', '2026-11-01T00:00:00Z', '2026-11-06T10:00:00Z');

-- r2: one reading, no graph.
insert into dojo.repository_metrics (tenant_id, repository_id, metric_id, scope, computed_on, value, pushed_at)
select 'f3111111-1111-4111-8111-111111111111', 'f4111111-2222-4111-8111-111111111111', m.id, 'repo',
       '2026-10-01', 1, '2026-11-04T10:00:00Z'
  from (select id from sensei.metrics order by key limit 1) m;

-- r3: quiet — no readings, no graph.
-- r4 (tenant B) lands newer than anything in A: a leak would show here.
insert into dojo.repository_metrics (tenant_id, repository_id, metric_id, scope, computed_on, value, pushed_at)
select 'f3111111-2222-4111-8111-111111111111', 'f4111111-4444-4111-8111-111111111111', m.id, 'repo',
       '2026-11-01', 1, '2026-11-07T10:00:00Z'
  from (select id from sensei.metrics order by key limit 1) m;

-- ── Everything below runs as the caller ────────────────────────────────────
set local role authenticated;
select set_config('request.jwt.claims',
  '{"sub":"f2111111-1111-4111-8111-111111111111","role":"authenticated"}', true);

do $$
declare
  v_act timestamptz;
  n int;
begin
  -- 1. Max across readings and graph: r1's graph landing wins.
  select last_activity_at into v_act
    from dojo.repository_activity(array['f4111111-1111-4111-8111-111111111111']::uuid[]);
  if v_act <> '2026-11-06T10:00:00Z'::timestamptz then
    raise exception 'r1 should read its graph landing 2026-11-06, read %', v_act;
  end if;

  -- 2. Metrics-only repo reads its reading.
  select last_activity_at into v_act
    from dojo.repository_activity(array['f4111111-2222-4111-8111-111111111111']::uuid[]);
  if v_act <> '2026-11-04T10:00:00Z'::timestamptz then
    raise exception 'r2 should read 2026-11-04, read %', v_act;
  end if;

  -- 3. Quiet repo reads one NULL row — present, never dated, never dropped.
  select count(*), max(last_activity_at) into n, v_act
    from dojo.repository_activity(array['f4111111-3333-4111-8111-111111111111']::uuid[]);
  if n <> 1 or v_act is not null then
    raise exception 'r3 should read one null row, saw % rows %', n, v_act;
  end if;

  -- 4. Unknown ids read no row.
  select count(*) into n
    from dojo.repository_activity(array['00000000-0000-4000-8000-000000000000']::uuid[]);
  if n <> 0 then raise exception 'unknown ids must read no row'; end if;

  -- 5. Tenant B's data never answers tenant A's admin.
  select count(*) into n
    from dojo.repository_activity(array['f4111111-4444-4111-8111-111111111111']::uuid[]);
  if n <> 0 then raise exception 'tenant B row answered to tenant A'; end if;
end $$;

-- Max (role on r1 only) reads r1, and nothing else in A.
select set_config('request.jwt.claims',
  '{"sub":"f2111111-2222-4111-8111-111111111111","role":"authenticated"}', true);
do $$
declare
  n int;
begin
  select count(*) into n
    from dojo.repository_activity(array['f4111111-1111-4111-8111-111111111111',
                                         'f4111111-2222-4111-8111-111111111111']::uuid[]);
  if n <> 1 then raise exception 'max should read r1 only, saw % rows', n; end if;
end $$;

-- Otto (member of nothing) reads nothing.
select set_config('request.jwt.claims',
  '{"sub":"f2111111-3333-4111-8111-111111111111","role":"authenticated"}', true);
do $$
declare
  n int;
begin
  select count(*) into n
    from dojo.repository_activity(array['f4111111-1111-4111-8111-111111111111']::uuid[]);
  if n <> 0 then raise exception 'outsider answered % rows', n; end if;
end $$;

rollback;
