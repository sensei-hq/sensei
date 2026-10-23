-- Team totals for every member, never per person (#307 R8, D38 follow-up, 2026-10-22).
--
-- dojo.team_metric_totals answers, for each team the caller is ON (every team of
-- the dōjō for an admin), each metric's team total over a window:
--   * only rows of the team's ACTIVE members, on repositories the CALLER can read
--     (access lives on repositories, D5);
--   * a count sums; a rate is pooled by its own denominator — Σ(value × den) /
--     Σ den, i.e. Σ numerator / Σ denominator, the daemon's own pooling — and is
--     null when a row lacks its denominator (never an unweighted mean);
--   * a metric is returned only when at least dojo.k_anonymity_min() (3) people
--     sent it — the dōjō's k-anonymity floor (K_ANONYMITY_MIN_CONTRIBUTORS) — so a
--     total over one or two people, from which a member could recover a
--     colleague's number, is never answered.
-- A workspace viewer is not a member and gets nothing.
--
-- dojo.team_member_metrics (the per-person read) gains the same pooled rate.
--
-- WHAT BREAKS THIS TEST: an unweighted mean; a floor below 3; counting a
-- departed member, a repository the caller cannot read, or a team the caller is
-- not on; a viewer getting a row.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000901', 'a1000000-0000-4000-8000-000000000901', 'cat (contributor)'),
  ('a0000000-0000-4000-8000-000000000902', 'a1000000-0000-4000-8000-000000000902', 'p2'),
  ('a0000000-0000-4000-8000-000000000903', 'a1000000-0000-4000-8000-000000000903', 'p3'),
  ('a0000000-0000-4000-8000-000000000904', 'a1000000-0000-4000-8000-000000000904', 'p4 (on R2 only)'),
  ('a0000000-0000-4000-8000-000000000905', 'a1000000-0000-4000-8000-000000000905', 'q1 (team B)'),
  ('a0000000-0000-4000-8000-000000000906', 'a1000000-0000-4000-8000-000000000906', 'gone (left)'),
  ('a0000000-0000-4000-8000-000000000907', 'a1000000-0000-4000-8000-000000000907', 'ada (admin)'),
  ('a0000000-0000-4000-8000-000000000908', 'a1000000-0000-4000-8000-000000000908', 'vic (viewer)'),
  ('a0000000-0000-4000-8000-000000000909', 'a1000000-0000-4000-8000-000000000909', 'p5 (team C)'),
  ('a0000000-0000-4000-8000-000000000910', 'a1000000-0000-4000-8000-000000000910', 'p6 (team C)');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000090a', 'organization/ztest-r8-totals', 'organization', 'ztest-r8-totals', 'R8 totals', 'x');

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via, disabled_at)
select 'b0000000-0000-4000-8000-00000000090a', v.who::uuid, v.role::dojo.member_role, 'employer', 'github_oauth', v.gone
  from (values
    ('a0000000-0000-4000-8000-000000000901', 'contributor', null::timestamptz),
    ('a0000000-0000-4000-8000-000000000902', 'contributor', null),
    ('a0000000-0000-4000-8000-000000000903', 'contributor', null),
    ('a0000000-0000-4000-8000-000000000904', 'contributor', null),
    ('a0000000-0000-4000-8000-000000000905', 'contributor', null),
    ('a0000000-0000-4000-8000-000000000906', 'contributor', now()),
    ('a0000000-0000-4000-8000-000000000907', 'admin',       null),
    ('a0000000-0000-4000-8000-000000000909', 'contributor', null),
    ('a0000000-0000-4000-8000-000000000910', 'contributor', null)
  ) as v(who, role, gone);
insert into dojo.workspaces (id, tenant_id, slug, name, classification) values
  ('c0000000-0000-4000-8000-000000000901', 'b0000000-0000-4000-8000-00000000090a', 'ztest-r8-totals-w', 'w', 'company');
select dojo.add_dojo_viewer('b0000000-0000-4000-8000-00000000090a', 'a0000000-0000-4000-8000-000000000908',
                            'a0000000-0000-4000-8000-000000000907', 'vic@example.com');
insert into dojo.workspace_viewers (workspace_id, principal_id) values
  ('c0000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000908');

-- R1: cat reads it. R2: cat does not.
insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('90000000-0000-4000-8000-000000000901', 'b0000000-0000-4000-8000-00000000090a', 'github.com/ztest-r8t/r1', 'r1', 'github'),
  ('90000000-0000-4000-8000-000000000902', 'b0000000-0000-4000-8000-00000000090a', 'github.com/ztest-r8t/r2', 'r2', 'github');
insert into dojo.repositories_in_workspaces (workspace_id, repository_id, tenant_id) values
  ('c0000000-0000-4000-8000-000000000901', '90000000-0000-4000-8000-000000000901', 'b0000000-0000-4000-8000-00000000090a');
insert into dojo.repository_roles (repository_id, principal_id, forge_permission, forge_role) values
  ('90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000901', 'write', 'contributor');

-- Team A: cat, p2, p3, p4, gone — and vic, a viewer put on it (not a member, so
-- still nothing). Team B: cat, q1. Team C: q1, p5, p6 (cat is not on it).
insert into dojo.teams (id, tenant_id, name) values
  ('d0000000-0000-4000-8000-000000000901', 'b0000000-0000-4000-8000-00000000090a', 'ztest-r8t-a'),
  ('d0000000-0000-4000-8000-000000000902', 'b0000000-0000-4000-8000-00000000090a', 'ztest-r8t-b'),
  ('d0000000-0000-4000-8000-000000000903', 'b0000000-0000-4000-8000-00000000090a', 'ztest-r8t-c');
insert into dojo.team_members (team_id, principal_id)
select v.team::uuid, v.who::uuid from (values
  ('d0000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000901'),
  ('d0000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000902'),
  ('d0000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000903'),
  ('d0000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000904'),
  ('d0000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000906'),
  ('d0000000-0000-4000-8000-000000000902', 'a0000000-0000-4000-8000-000000000901'),
  ('d0000000-0000-4000-8000-000000000902', 'a0000000-0000-4000-8000-000000000905'),
  ('d0000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000908'),
  ('d0000000-0000-4000-8000-000000000903', 'a0000000-0000-4000-8000-000000000905'),
  ('d0000000-0000-4000-8000-000000000903', 'a0000000-0000-4000-8000-000000000909'),
  ('d0000000-0000-4000-8000-000000000903', 'a0000000-0000-4000-8000-000000000910')
) as v(team, who);

-- ftr (a rate; props.denominator = sessions) and throughput (a count), today.
-- On R1: cat 1.0 of 1, p2 0.5 of 4, p3 0.0 of 5, q1 0.5 of 2, gone 1.0 of 9.
-- On R2: p4 1.0 of 10.
-- rework_ratio on R1 for cat, p2, p3 — p3's row carries no denominator.
insert into dojo.repository_metrics (tenant_id, repository_id, metric_id, scope, principal_id, computed_on, value, props)
select 'b0000000-0000-4000-8000-00000000090a', v.repo::uuid, (select id from sensei.metrics where key = v.k),
       'user', v.who::uuid, current_date, v.value, v.props::jsonb
  from (values
    ('ftr',          '90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000901', 1.0, '{"numerator":1,"denominator":1}'),
    ('ftr',          '90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000902', 0.5, '{"numerator":2,"denominator":4}'),
    ('ftr',          '90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000903', 0.0, '{"numerator":0,"denominator":5}'),
    ('ftr',          '90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000905', 0.5, '{"numerator":1,"denominator":2}'),
    ('ftr',          '90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000906', 1.0, '{"numerator":9,"denominator":9}'),
    ('ftr',          '90000000-0000-4000-8000-000000000902', 'a0000000-0000-4000-8000-000000000904', 1.0, '{"numerator":10,"denominator":10}'),
    ('throughput',   '90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000901', 1,   '{}'),
    ('throughput',   '90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000902', 4,   '{}'),
    ('throughput',   '90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000903', 5,   '{}'),
    ('throughput',   '90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000905', 2,   '{}'),
    ('throughput',   '90000000-0000-4000-8000-000000000902', 'a0000000-0000-4000-8000-000000000904', 10,  '{}'),
    ('rework_ratio', '90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000901', 0.1, '{"numerator":1,"denominator":10}'),
    ('rework_ratio', '90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000902', 0.2, '{"numerator":2,"denominator":10}'),
    ('rework_ratio', '90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000903', 0.3, '{}'),
    ('ftr',          '90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000909', 0.5, '{"numerator":1,"denominator":2}'),
    ('ftr',          '90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000910', 0.5, '{"numerator":1,"denominator":2}'),
    ('throughput',   '90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000909', 2,   '{}'),
    ('throughput',   '90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000910', 2,   '{}')
  ) as v(k, repo, who, value, props);

-- The repository reader needs a role: give ada none (admins read the tenant).
set local role authenticated;

create temp table expect (who text, sub text, want text) on commit drop;
insert into expect values
  -- cat: team A on R1 only (p4's R2 row and gone's row excluded); team B has 2 people: hidden; not on C.
  ('cat', 'a1000000-0000-4000-8000-000000000901',
   'ztest-r8t-a:ftr=0.30/3,ztest-r8t-a:rework_ratio=null/3,ztest-r8t-a:throughput=10/3'),
  -- ada (admin): every team; team A includes p4 on R2 (4 people); B under 3; C (q1, p5, p6) answered.
  ('ada', 'a1000000-0000-4000-8000-000000000907',
   'ztest-r8t-a:ftr=0.65/4,ztest-r8t-a:rework_ratio=null/3,ztest-r8t-a:throughput=20/4,ztest-r8t-c:ftr=0.50/3,ztest-r8t-c:throughput=6/3'),
  -- the viewer is no member, even on a team: nothing.
  ('vic', 'a1000000-0000-4000-8000-000000000908', '');

do $$
declare
    r record;
    got text;
begin
    for r in select * from expect loop
        perform set_config('request.jwt.claims', json_build_object('sub', r.sub)::text, true);
        select coalesce(string_agg(
                 format('%s:%s=%s/%s', t.name, x.metric_key,
                        case when x.metric_key = 'throughput' then x.total::int::text
                             else coalesce(round(x.weighted, 2)::text, 'null') end,
                        x.contributors),
                 ',' order by t.name, x.metric_key), '')
          into got
          from dojo.team_metric_totals('b0000000-0000-4000-8000-00000000090a',
                                       array['ftr', 'throughput', 'rework_ratio'], current_date - 7) x
          join dojo.teams t on t.id = x.team_id;
        if got is distinct from r.want then
            raise exception 'team_metric_totals: % should get [%], got [%]', r.who, r.want, got;
        end if;
    end loop;
end $$;

-- The per-person read pools the same way: cat's own ftr is 1/1.
select set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000901"}', true);
do $$
begin
    if (select weighted from dojo.team_member_metrics('b0000000-0000-4000-8000-00000000090a',
            array['a0000000-0000-4000-8000-000000000901']::uuid[], array['ftr'], current_date - 7)) is distinct from 1 then
        raise exception 'team_member_metrics does not pool the rate by its denominator';
    end if;
end $$;

reset role;

-- The floor is the dōjō's k-anonymity floor.
do $$
begin
    if dojo.k_anonymity_min() is distinct from 3 then
        raise exception 'k_anonymity_min is %, the dōjō floor is 3', dojo.k_anonymity_min();
    end if;
end $$;

rollback;
