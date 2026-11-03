-- Reading a repository's metric series, and a workspace's rollup of them (#307
-- D6/D8; WP4).
--
-- dojo.repository_metric_series is the per-repository read: SECURITY INVOKER over
-- dojo.repository_metrics, so the table's own policy (can_read_repository_metric)
-- decides row by row — a role holder its repo rows, nobody another's user rows
-- below maintainer, a lead their teammates' on repositories the lead can read.
--
-- dojo.workspace_metric_series is the workspace rollup over
-- dojo.repositories_in_workspaces: the detail rows the caller may read, plus —
-- for a workspace viewer — the aggregate view D8 decides: repo-scoped rows
-- as-is, user-scoped readings pooled to one row per repository, metric and day
-- with NO principal. A viewer reads aggregates and per-repository splits,
-- never per-person. A pooled row carries scope 'pooled' — never 'user' — so a
-- screen summing scope = 'user' cannot count the same people twice. One
-- reading per person per day (a person's commit rows collapse to the latest by
-- authored_at desc nulls last, pushed_at desc, commit_sha desc; commit-less
-- rows add), and a pool with fewer than 2 contributors is withheld (F4, which
-- supersedes D10 for viewer pools). The pooling rides in props (pooled_rows,
-- contributors, and the rate numerator/denominator by team_member_metrics'
-- rule — null unless every reading has a positive denominator), so a reader
-- pools a rate across days by the same rule instead of summing rates; a screen
-- shows contributors (people), never pooled_rows (readings).
--
-- WHAT BREAKS THIS TEST: a contributor reading a peer's user rows; a maintainer
-- not reading them; a lead not reading a teammate's, or reading a non-teammate's;
-- a viewer seeing a per-person row, a pooled row under scope 'user', a pool of
-- one (withheld, F4), or a sum that includes a day outside the window; a
-- person's commit rows counted more than once, or a reading that is not their
-- latest commit's; a member's viewer aggregates duplicating detail rows the
-- member already reads; any row for a non-member; a series that guesses a
-- metric key instead of resolving it.
begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000041', 'a1000000-0000-4000-8000-000000000041', 'alice'),
  ('a0000000-0000-4000-8000-000000000042', 'a1000000-0000-4000-8000-000000000042', 'dave'),
  ('a0000000-0000-4000-8000-000000000043', 'a1000000-0000-4000-8000-000000000043', 'bob'),
  ('a0000000-0000-4000-8000-000000000044', 'a1000000-0000-4000-8000-000000000044', 'erin'),
  ('a0000000-0000-4000-8000-000000000045', 'a1000000-0000-4000-8000-000000000045', 'ivy'),
  ('a0000000-0000-4000-8000-000000000046', 'a1000000-0000-4000-8000-000000000046', 'carol'),
  ('a0000000-0000-4000-8000-000000000047', 'a1000000-0000-4000-8000-000000000047', 'zed'),
  ('a0000000-0000-4000-8000-000000000048', 'a1000000-0000-4000-8000-000000000048', 'client');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000004a', 'organization/ztest-wsmetrics-t', 'organization',
   'ztest-wsmetrics-t', 'workspace-metrics fixture T', 'x'),
  ('b0000000-0000-4000-8000-00000000004b', 'organization/ztest-wsmetrics-u', 'organization',
   'ztest-wsmetrics-u', 'workspace-metrics fixture U', 'x');

-- client has NO membership: a pure dōjō viewer holds no repository access.
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000004a', 'a0000000-0000-4000-8000-000000000041', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000004a', 'a0000000-0000-4000-8000-000000000042', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000004a', 'a0000000-0000-4000-8000-000000000043', 'lead',        'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000004a', 'a0000000-0000-4000-8000-000000000044', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000004a', 'a0000000-0000-4000-8000-000000000045', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000004a', 'a0000000-0000-4000-8000-000000000047', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000004b', 'a0000000-0000-4000-8000-000000000046', 'admin',       'employer', 'github_oauth');

-- Workspace P holds repository R (alice contributes, dave maintains, bob
-- contributes). Workspace Q holds repository S (nobody has a role on it).
insert into dojo.workspaces (id, tenant_id, slug, name) values
  ('c0000000-0000-4000-8000-0000000000f4',
   'b0000000-0000-4000-8000-00000000004a', 'ztest-wsmetrics-p', 'workspace P'),
  ('c0000000-0000-4000-8000-0000000000f5',
   'b0000000-0000-4000-8000-00000000004a', 'ztest-wsmetrics-q', 'workspace Q');

insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('90000000-0000-4000-8000-0000000000b1', 'b0000000-0000-4000-8000-00000000004a',
   'github.com/ztest-wsmetrics/r', 'r', 'github'),
  ('90000000-0000-4000-8000-0000000000b2', 'b0000000-0000-4000-8000-00000000004a',
   'github.com/ztest-wsmetrics/s', 's', 'github');
insert into dojo.repositories_in_workspaces (workspace_id, repository_id, tenant_id) values
  ('c0000000-0000-4000-8000-0000000000f4', '90000000-0000-4000-8000-0000000000b1', 'b0000000-0000-4000-8000-00000000004a'),
  ('c0000000-0000-4000-8000-0000000000f5', '90000000-0000-4000-8000-0000000000b2', 'b0000000-0000-4000-8000-00000000004a');
insert into dojo.repository_roles (repository_id, principal_id, forge_permission, forge_role) values
  ('90000000-0000-4000-8000-0000000000b1', 'a0000000-0000-4000-8000-000000000041', 'write',    'contributor'),
  ('90000000-0000-4000-8000-0000000000b1', 'a0000000-0000-4000-8000-000000000042', 'maintain', 'maintainer'),
  ('90000000-0000-4000-8000-0000000000b1', 'a0000000-0000-4000-8000-000000000043', 'write',    'contributor');

-- bob leads alice: his membership is lead and they share a team of this dōjō.
-- zed and dave are on no team, so a lead reads alice's numbers but not theirs.
insert into dojo.teams (id, tenant_id, name) values
  ('d0000000-0000-4000-8000-0000000000f1',
   'b0000000-0000-4000-8000-00000000004a', 'ztest-wsmetrics-team');
insert into dojo.team_members (team_id, principal_id) values
  ('d0000000-0000-4000-8000-0000000000f1', 'a0000000-0000-4000-8000-000000000043'),
  ('d0000000-0000-4000-8000-0000000000f1', 'a0000000-0000-4000-8000-000000000041');

-- ivy (a member), dave (a maintainer) and client (a pure viewer) may view
-- workspace P's metrics.
insert into dojo.workspace_viewers (workspace_id, principal_id) values
  ('c0000000-0000-4000-8000-0000000000f4', 'a0000000-0000-4000-8000-000000000045'),
  ('c0000000-0000-4000-8000-0000000000f4', 'a0000000-0000-4000-8000-000000000042'),
  ('c0000000-0000-4000-8000-0000000000f4', 'a0000000-0000-4000-8000-000000000048');

-- One metric, nine rows: R repo-scoped, R user-scoped for alice / dave / zed
-- (zed has no role on R), S repo-scoped, plus alice's and dave's yesterday rows
-- (no denominator — their pool must not invent a rate), alice's lone row two
-- days back (a pool of one — withheld, F4), and her stale row outside
-- the 30-day window (it must count nowhere).
insert into dojo.repository_metrics (id, tenant_id, repository_id, metric_id, scope, principal_id, computed_on, value, props)
select v.id::uuid, 'b0000000-0000-4000-8000-00000000004a', v.repo::uuid, m.id, v.scope, v.who::uuid, v.day::date, v.value, v.props::jsonb
  from (values
    ('81000000-0000-4000-8000-000000000001', '90000000-0000-4000-8000-0000000000b1', 'repo', null,                                       current_date,      10, '{}'),
    ('81000000-0000-4000-8000-000000000002', '90000000-0000-4000-8000-0000000000b1', 'user', 'a0000000-0000-4000-8000-000000000041', current_date,       1, '{"denominator": 1}'),
    ('81000000-0000-4000-8000-000000000003', '90000000-0000-4000-8000-0000000000b1', 'user', 'a0000000-0000-4000-8000-000000000042', current_date,       2, '{"denominator": 3}'),
    ('81000000-0000-4000-8000-000000000004', '90000000-0000-4000-8000-0000000000b1', 'user', 'a0000000-0000-4000-8000-000000000047', current_date,       4, '{"denominator": 2}'),
    ('81000000-0000-4000-8000-000000000005', '90000000-0000-4000-8000-0000000000b2', 'repo', null,                                       current_date,      20, '{}'),
    ('81000000-0000-4000-8000-000000000006', '90000000-0000-4000-8000-0000000000b1', 'user', 'a0000000-0000-4000-8000-000000000041', current_date - 1,   5, '{}'),
    ('81000000-0000-4000-8000-000000000007', '90000000-0000-4000-8000-0000000000b1', 'user', 'a0000000-0000-4000-8000-000000000041', current_date - 40, 100, '{"denominator": 1}'),
    ('81000000-0000-4000-8000-000000000008', '90000000-0000-4000-8000-0000000000b1', 'user', 'a0000000-0000-4000-8000-000000000042', current_date - 1,   3, '{}'),
    ('81000000-0000-4000-8000-000000000009', '90000000-0000-4000-8000-0000000000b1', 'user', 'a0000000-0000-4000-8000-000000000041', current_date - 2,   9, '{"denominator": 1}')
  ) as v(id, repo, scope, who, day, value, props)
 cross join (select id from sensei.metrics order by key limit 1) m;

set local role authenticated;

do $$
declare
    r_id  constant uuid := '90000000-0000-4000-8000-0000000000b1';
    s_id  constant uuid := '90000000-0000-4000-8000-0000000000b2';
    p_id  constant uuid := 'c0000000-0000-4000-8000-0000000000f4';
    q_id  constant uuid := 'c0000000-0000-4000-8000-0000000000f5';
    alice constant uuid := 'a0000000-0000-4000-8000-000000000041';
    dave  constant uuid := 'a0000000-0000-4000-8000-000000000042';
    since constant date := current_date - 30;
    n int;
    v numeric;
    k text;
    pn int;
    pnum numeric;
    pden numeric;
    contrib int;
begin
    select key into k from sensei.metrics order by key limit 1;

    -- ── role_holder_reads_repo_rows ──────────────────────────────────────────
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000041"}', true);
    select count(*), sum(value), min(metric_key) into n, v, k
      from dojo.repository_metric_series(r_id, since) where scope = 'repo';
    if n <> 1 or v <> 10 then
        raise exception 'role_holder_reads_repo_rows: alice should read R''s one repo row worth 10, got % rows worth %', n, v;
    end if;
    if k <> (select key from sensei.metrics order by key limit 1) then
        raise exception 'role_holder_reads_repo_rows: the series should resolve the metric key, got %', k;
    end if;

    -- ── contributor_does_not_read_a_peers_user_rows ──────────────────────────
    select count(*) into n from dojo.repository_metric_series(r_id, since)
     where scope = 'user' and principal_id is distinct from alice;
    if n <> 0 then
        raise exception 'contributor_does_not_read_a_peers_user_rows: alice read % of dave''s/zed''s rows', n;
    end if;
    select count(*) into n from dojo.repository_metric_series(r_id, since)
     where scope = 'user' and principal_id = alice;
    if n <> 3 then
        raise exception 'contributor_does_not_read_a_peers_user_rows: alice should read her own 3 in-window rows, got %', n;
    end if;

    -- ── maintainer_reads_peers_user_rows ─────────────────────────────────────
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000042"}', true);
    select count(*) into n from dojo.repository_metric_series(r_id, since) where scope = 'user';
    if n <> 6 then
        raise exception 'maintainer_reads_peers_user_rows: dave should read all 6 in-window user rows on R, got %', n;
    end if;
    if exists (select 1 from dojo.repository_metric_series(r_id, since) where value = 100) then
        raise exception 'maintainer_reads_peers_user_rows: the stale row is outside the window and must not be read';
    end if;

    -- ── lead_reads_teammates_user_rows ───────────────────────────────────────
    -- bob is a lead of the dōjō, holds a contributor role on R, and shares a
    -- team with alice: the leads_team_member arm of can_read_repository_metric.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000043"}', true);
    select count(*) into n from dojo.repository_metric_series(r_id, since)
     where scope = 'user' and principal_id = alice;
    if n <> 3 then
        raise exception 'lead_reads_teammates_user_rows: bob should read alice''s 3 in-window rows, got %', n;
    end if;
    select count(*) into n from dojo.repository_metric_series(r_id, since)
     where scope = 'user' and principal_id is distinct from alice;
    if n <> 0 then
        raise exception 'lead_reads_teammates_user_rows: bob read % rows of people on no team with him', n;
    end if;
    -- The rollup carries the same detail (bob holds no viewer grant, so no
    -- pooled rows): R's repo row plus alice's three.
    select count(*) into n from dojo.workspace_metric_series(p_id, since);
    if n <> 4 then
        raise exception 'lead_reads_teammates_user_rows: bob''s rollup should be 4 detail rows, got %', n;
    end if;
    select count(*) into n from dojo.workspace_metric_series(p_id, since) where scope = 'pooled';
    if n <> 0 then
        raise exception 'lead_reads_teammates_user_rows: bob holds no viewer grant and should see no pooled row, got %', n;
    end if;

    -- ── viewer_gets_aggregates_only ──────────────────────────────────────────
    -- client holds no role on R: the per-repository read shows them nothing —
    -- a viewer reads only through the workspace rollup, and only pooled.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000048"}', true);
    select count(*) into n from dojo.repository_metric_series(r_id, since);
    if n <> 0 then
        raise exception 'viewer_gets_aggregates_only: the per-repository read should show a pure viewer nothing, got % rows', n;
    end if;
    -- The rollup: R's repo row as-is, plus one pooled row per day with company.
    select count(*) into n from dojo.workspace_metric_series(p_id, since)
     where scope = 'user' and principal_id is not null;
    if n <> 0 then
        raise exception 'viewer_gets_aggregates_only: a viewer must never see a per-person row, got %', n;
    end if;
    -- No pooled row hides under scope 'user': pooled rows carry their own scope.
    select count(*) into n from dojo.workspace_metric_series(p_id, since)
     where scope = 'user' and principal_id is null;
    if n <> 0 then
        raise exception 'viewer_gets_aggregates_only: pooled rows must carry scope pooled, got % under user', n;
    end if;
    -- Today's pool: 1 + 2 + 4, with the rate pooled by its denominators
    -- (1×1 + 2×3 + 4×2) / (1 + 3 + 2) — and the stale 100 outside the window.
    select value, (props->>'pooled_rows')::int, (props->>'numerator')::numeric,
           (props->>'denominator')::numeric, (props->>'contributors')::int
      into v, pn, pnum, pden, contrib
      from dojo.workspace_metric_series(p_id, since)
     where scope = 'pooled' and computed_on = current_date;
    if v <> 7 or pn <> 3 or pnum <> 15 or pden <> 6 or contrib <> 3 then
        raise exception 'viewer_gets_aggregates_only: today''s pool should be 7 over 3 rows (15/6) from 3 contributors, got % over % rows (%/%) from %', v, pn, pnum, pden, contrib;
    end if;
    -- Yesterday's pool has no denominator anywhere, so it invents no rate.
    select value, (props->>'pooled_rows')::int, (props->>'numerator')::numeric,
           (props->>'denominator')::numeric, (props->>'contributors')::int
      into v, pn, pnum, pden, contrib
      from dojo.workspace_metric_series(p_id, since)
     where scope = 'pooled' and computed_on = current_date - 1;
    if v <> 8 or pn <> 2 or pnum is not null or pden is not null or contrib <> 2 then
        raise exception 'viewer_gets_aggregates_only: yesterday''s pool should be 8 over 2 rows with no rate from 2 contributors, got % over % rows (%/%) from %', v, pn, pnum, pden, contrib;
    end if;
    -- Two days back alice stands alone: a pool of one is withheld (F4) — one
    -- person's number never reaches a viewer.
    select count(*) into n from dojo.workspace_metric_series(p_id, since)
     where scope = 'pooled' and computed_on = current_date - 2;
    if n <> 0 then
        raise exception 'viewer_gets_aggregates_only: the lone day''s pool must be withheld from viewers, got % rows', n;
    end if;
    -- The repo row passes through as-is; S is not in P so it stays out.
    select count(*), sum(value) into n, v
      from dojo.workspace_metric_series(p_id, since) where scope = 'repo';
    if n <> 1 or v <> 10 then
        raise exception 'viewer_gets_aggregates_only: the rollup should carry R''s repo row worth 10, got % rows worth %', n, v;
    end if;
    select count(*) into n from dojo.workspace_metric_series(p_id, since)
     where repository_id = s_id;
    if n <> 0 then
        raise exception 'viewer_gets_aggregates_only: S is not in P and must not be in its rollup, got % rows', n;
    end if;

    -- ivy is a member AND a viewer: the viewer aggregates arrive, and the repo
    -- row arrives once — the grant must not duplicate what detail gave. (She
    -- reads no detail here — no role, no own rows — so hers matches client's.)
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000045"}', true);
    select count(*) into n from dojo.workspace_metric_series(p_id, since) where scope = 'repo';
    if n <> 1 then
        raise exception 'viewer_gets_aggregates_only: ivy should see R''s repo row exactly once, got %', n;
    end if;
    select count(*) into n from dojo.workspace_metric_series(p_id, since)
     where scope = 'user' and principal_id is not null;
    if n <> 0 then
        raise exception 'viewer_gets_aggregates_only: ivy must see no per-person row, got %', n;
    end if;
    select count(*) into n from dojo.workspace_metric_series(p_id, since) where scope = 'pooled';
    if n <> 2 then
        raise exception 'viewer_gets_aggregates_only: ivy should see 2 pooled rows (the lone day withheld, F4), got %', n;
    end if;

    -- dave maintains R AND views P: the repo row still arrives once — the
    -- viewer branch yields the repo rows detail already gave — while the pooled
    -- rows arrive beside his per-person detail.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000042"}', true);
    select count(*) into n from dojo.workspace_metric_series(p_id, since) where scope = 'repo';
    if n <> 1 then
        raise exception 'viewer_gets_aggregates_only: dave should see R''s repo row exactly once, got %', n;
    end if;
    select count(*) into n from dojo.workspace_metric_series(p_id, since)
     where scope = 'pooled';
    if n <> 2 then
        raise exception 'viewer_gets_aggregates_only: dave should see 2 pooled rows beside his detail, got %', n;
    end if;
    -- And the two halves do not double-count: detail user rows plus pooled rows
    -- of the same day are different scopes, never summed together.
    select count(*) into n from dojo.workspace_metric_series(p_id, since)
     where scope = 'user' and computed_on = current_date;
    if n <> 3 then
        raise exception 'viewer_gets_aggregates_only: dave should see 3 per-person user rows for today, got %', n;
    end if;

    -- alice is a member with NO viewer grant: detail only, no pooled rows.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000041"}', true);
    select count(*) into n from dojo.workspace_metric_series(p_id, since)
     where scope = 'pooled';
    if n <> 0 then
        raise exception 'viewer_gets_aggregates_only: alice holds no viewer grant and should see no pooled row, got %', n;
    end if;
    select count(*) into n from dojo.workspace_metric_series(p_id, since);
    if n <> 4 then
        raise exception 'viewer_gets_aggregates_only: alice''s rollup should be her 4 detail rows, got %', n;
    end if;

    -- The rollup is scoped to its workspace: Q holds only S.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000044"}', true);
    select count(*), sum(value) into n, v from dojo.workspace_metric_series(q_id, since);
    if n <> 1 or v <> 20 then
        raise exception 'viewer_gets_aggregates_only: Q''s rollup should be S''s repo row worth 20, got % rows worth %', n, v;
    end if;

    -- ── admin_reads_every_detail_row ─────────────────────────────────────────
    -- erin administers the dōjō: the rollup of P is every in-window detail row —
    -- R's repo row plus all six user rows, zed's included — and no pooled row
    -- (she holds no viewer grant).
    select count(*) into n from dojo.workspace_metric_series(p_id, since)
     where scope = 'user' and principal_id = 'a0000000-0000-4000-8000-000000000047';
    if n <> 1 then
        raise exception 'admin_reads_every_detail_row: erin should read zed''s user row, got %', n;
    end if;
    select count(*) into n from dojo.workspace_metric_series(p_id, since);
    if n <> 7 then
        raise exception 'admin_reads_every_detail_row: erin''s rollup of P should be 7 detail rows, got %', n;
    end if;
    select count(*) into n from dojo.workspace_metric_series(p_id, since) where scope = 'pooled';
    if n <> 0 then
        raise exception 'admin_reads_every_detail_row: erin holds no viewer grant and should see no pooled row, got %', n;
    end if;

    -- ── non_member_reads_nothing ─────────────────────────────────────────────
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000046"}', true);
    select count(*) into n from dojo.repository_metric_series(r_id, since);
    select count(*) + n into n from dojo.workspace_metric_series(p_id, since);
    if n <> 0 then
        raise exception 'non_member_reads_nothing: carol administers another dōjō and should read nothing here, got % rows', n;
    end if;
end $$;

-- ── F4: pools count people, and one person counts once ──────────────────────
-- Own fixture (amelia, boris, client-f4 in tenant ztest-wsmetrics-f4), so the
-- P-workspace counts above never move. Back to the owner for the inserts, then
-- authenticated again for the reads (the definer test's idiom).
reset role;

-- Five days — today two contributors
-- (shown), yesterday one commit-less row (withheld), day -2 one contributor
-- with two commit rows (withheld — F4 counts people), day -3 one contributor
-- with three commit rows plus a second contributor (shown, first person once),
-- day -4 one keyed and one keyless commit row plus a second contributor (the
-- keyed row wins). Per-commit rows carry props.authored_at (L0 ruling; the
-- daemon sends it from sensei.commits, unknown shas without the key).
insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000051', 'a1000000-0000-4000-8000-000000000051', 'amelia'),
  ('a0000000-0000-4000-8000-000000000052', 'a1000000-0000-4000-8000-000000000052', 'boris'),
  ('a0000000-0000-4000-8000-000000000053', 'a1000000-0000-4000-8000-000000000053', 'client-f4');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000005a', 'organization/ztest-wsmetrics-f4', 'organization',
   'ztest-wsmetrics-f4', 'f4 follow-up fixture', 'x');

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000005a', 'a0000000-0000-4000-8000-000000000051', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000005a', 'a0000000-0000-4000-8000-000000000052', 'contributor', 'employer', 'github_oauth');

insert into dojo.workspaces (id, tenant_id, slug, name) values
  ('c0000000-0000-4000-8000-00000000006a',
   'b0000000-0000-4000-8000-00000000005a', 'ztest-wsmetrics-f4-p', 'workspace F4P');

insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('90000000-0000-4000-8000-0000000000c1', 'b0000000-0000-4000-8000-00000000005a',
   'github.com/ztest-wsmetrics-f4/r', 'r', 'github');
insert into dojo.repositories_in_workspaces (workspace_id, repository_id, tenant_id) values
  ('c0000000-0000-4000-8000-00000000006a', '90000000-0000-4000-8000-0000000000c1', 'b0000000-0000-4000-8000-00000000005a');

insert into dojo.workspace_viewers (workspace_id, principal_id) values
  ('c0000000-0000-4000-8000-00000000006a', 'a0000000-0000-4000-8000-000000000053');

-- Day -3: time order f3 < 7c < 0a1, sha sort reversed, latest written first
-- (a last-write picker takes f3 and fails); latest middle by value and
-- numerator with its own denominator, so max, min and split-part pickers fail.
insert into dojo.repository_metrics (id, tenant_id, repository_id, metric_id, scope, principal_id, commit_sha, computed_on, value, props, pushed_at)
select v.id::uuid, 'b0000000-0000-4000-8000-00000000005a', '90000000-0000-4000-8000-0000000000c1', m.id, 'user', v.who::uuid, v.sha, v.day::date, v.value,
       v.props::jsonb || case when v.comm is null then '{}'::jsonb else jsonb_build_object('authored_at', v.comm) end,
       coalesce(v.pushed::timestamptz, now())
  from (values
    ('82000000-0000-4000-8000-000000000001', 'a0000000-0000-4000-8000-000000000051', null,                                       current_date,     1, '{"denominator": 1}', null, null),
    ('82000000-0000-4000-8000-000000000002', 'a0000000-0000-4000-8000-000000000052', null,                                       current_date,     2, '{"denominator": 3}', null, null),
    ('82000000-0000-4000-8000-000000000003', 'a0000000-0000-4000-8000-000000000051', null,                                       current_date - 1, 5, '{"denominator": 2}', null, null),
    ('82000000-0000-4000-8000-000000000004', 'a0000000-0000-4000-8000-000000000051', 'aa0000000000000000000000000000000000000001', current_date - 2, 4, '{"denominator": 2}', format('%sT09:00:00Z', (current_date - 2)::text), null),
    ('82000000-0000-4000-8000-000000000005', 'a0000000-0000-4000-8000-000000000051', 'bb0000000000000000000000000000000000000002', current_date - 2, 6, '{"denominator": 3}', format('%sT11:00:00Z', (current_date - 2)::text), null),
    ('82000000-0000-4000-8000-000000000006', 'a0000000-0000-4000-8000-000000000051', 'f3c81a000000000000000000000000000000000003', current_date - 3, 0.30, '{"denominator": 10}', format('%sT10:00:00Z', (current_date - 3)::text), ((current_date - 3) + time '08:02')::timestamptz),
    ('82000000-0000-4000-8000-000000000007', 'a0000000-0000-4000-8000-000000000051', '7c2d00000000000000000000000000000000000004', current_date - 3, 0.05, '{"denominator": 2}',  format('%sT12:00:00Z', (current_date - 3)::text), ((current_date - 3) + time '08:01')::timestamptz),
    ('82000000-0000-4000-8000-000000000008', 'a0000000-0000-4000-8000-000000000051', '0a1900000000000000000000000000000000000005', current_date - 3, 0.10, '{"denominator": 4}',  format('%sT14:00:00Z', (current_date - 3)::text), ((current_date - 3) + time '08:00')::timestamptz),
    ('82000000-0000-4000-8000-000000000009', 'a0000000-0000-4000-8000-000000000052', null,                                       current_date - 3, 0.50, '{"denominator": 6}', null, ((current_date - 3) + time '08:03')::timestamptz),
    ('82000000-0000-4000-8000-000000000010', 'a0000000-0000-4000-8000-000000000051', 'cc0000000000000000000000000000000000000006', current_date - 4, 7, '{"denominator": 2}', format('%sT10:00:00Z', (current_date - 4)::text), ((current_date - 4) + time '08:00')::timestamptz),
    ('82000000-0000-4000-8000-000000000011', 'a0000000-0000-4000-8000-000000000051', 'dd0000000000000000000000000000000000000007', current_date - 4, 100, '{"denominator": 100}', null, ((current_date - 4) + time '09:00')::timestamptz),
    ('82000000-0000-4000-8000-000000000012', 'a0000000-0000-4000-8000-000000000052', null,                                       current_date - 4, 1, '{"denominator": 1}', null, ((current_date - 4) + time '08:30')::timestamptz)
  ) as v(id, who, sha, day, value, props, comm, pushed)
 cross join (select id from sensei.metrics order by key limit 1) m;

set local role authenticated;

do $$
declare
    fp_id  constant uuid := 'c0000000-0000-4000-8000-00000000006a';
    fsince constant date := current_date - 30;
    n int;
    v numeric;
    pn int;
    pnum numeric;
    pden numeric;
    contrib int;
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000053"}', true);

    -- The two-contributor pool is shown, carrying its pooled rate and count.
    select value, (props->>'pooled_rows')::int, (props->>'numerator')::numeric,
           (props->>'denominator')::numeric, (props->>'contributors')::int
      into v, pn, pnum, pden, contrib
      from dojo.workspace_metric_series(fp_id, fsince)
     where scope = 'pooled' and computed_on = current_date;
    if v <> 3 or pn <> 2 or pnum <> 7 or pden <> 4 or contrib <> 2 then
        raise exception 'f4_two_pool_shown: today''s pool should be 3 over 2 rows (7/4) from 2 contributors, got % over % rows (%/%) from %', v, pn, pnum, pden, contrib;
    end if;

    -- The pool of one is withheld (F4).
    select count(*) into n from dojo.workspace_metric_series(fp_id, fsince)
     where scope = 'pooled' and computed_on = current_date - 1;
    if n <> 0 then
        raise exception 'f4_lone_pool_withheld: yesterday''s pool of one must be withheld from viewers, got % rows', n;
    end if;

    -- F1: one person with two commits is still one person — withheld.
    select count(*) into n from dojo.workspace_metric_series(fp_id, fsince)
     where scope = 'pooled' and computed_on = current_date - 2;
    if n <> 0 then
        raise exception 'f4_one_person_two_commits_withheld: day -2''s two rows are one person and must be withheld, got % pooled rows', n;
    end if;

    -- F2: one reading per person per day — amelia's latest (0a1: 0.10, num
    -- 0.4, den 4) once, plus boris's commit-less row.
    select value, (props->>'numerator')::numeric, (props->>'denominator')::numeric,
           (props->>'contributors')::int
      into v, pnum, pden, contrib
      from dojo.workspace_metric_series(fp_id, fsince)
     where scope = 'pooled' and computed_on = current_date - 3;
    if v <> 0.60 or pnum <> 3.4 or pden <> 10 then
        raise exception 'f4_one_person_many_commits_counts_once: day -3''s pool should be 0.60 (3.4/10) with amelia counted once, got % (%/%)', v, pnum, pden;
    end if;
    if contrib <> 2 then
        raise exception 'f4_one_person_many_commits_counts_once: day -3''s pool should cover 2 contributors, got %', contrib;
    end if;

    -- Keyless rows order after keyed: amelia's keyed row (7, num 14, den 2)
    -- is her reading even though the keyless row was written later; boris's
    -- commit-less row adds as now.
    select value, (props->>'numerator')::numeric, (props->>'denominator')::numeric,
           (props->>'contributors')::int
      into v, pnum, pden, contrib
      from dojo.workspace_metric_series(fp_id, fsince)
     where scope = 'pooled' and computed_on = current_date - 4;
    if v <> 8 or pnum <> 15 or pden <> 3 then
        raise exception 'f4_keyless_rows_order_after_keyed: day -4''s pool should be 8 (15/3) from the keyed row, got % (%/%)', v, pnum, pden;
    end if;
    if contrib <> 2 then
        raise exception 'f4_keyless_rows_order_after_keyed: day -4''s pool should cover 2 contributors, got %', contrib;
    end if;
end $$;

rollback;
