-- A developer's own metrics, day by day (the developer console's chart).
--
-- dojo.my_metric_series(since) answers for the CALLER only: their daily
-- user-scoped rows from `since` on, one row per metric and day, pooled across
-- the repositories they work in the way dojo.team_member_metrics pools a
-- period — `total` sums, `weighted` is Σ(value × denominator) / Σ denominator,
-- and null when a row lacks a positive denominator (never an unweighted mean);
-- `denominator` is the day's Σ denominator, null with it, so a reader can pool a
-- rate across days the same way.
--
-- | row                                       | in the caller's series? |
-- |-------------------------------------------|-------------------------|
-- | own, R1, yesterday, 0.5 over 4            | yes, pooled with R2     |
-- | own, R2, yesterday, 1.0 over 2            | yes, pooled with R1     |
-- | own, R1, today, 3, no denominator         | yes, weighted null      |
-- | a colleague's (the admin), R1, today      | no — not the caller's   |
-- | R1's repository-wide row, today, filed    | no — not a person's     |
-- |   under the caller                        |                         |
-- | own, R1, thirty days ago                  | no — before `since`     |
--
-- SECURITY INVOKER: the table's read policy still decides what the caller sees.
--
-- WHAT BREAKS THIS TEST: a series not bound to the caller, repository rows
-- counted as a person's, `since` ignored, or a rate averaged without its
-- denominators.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000901', 'a1000000-0000-4000-8000-000000000901', 'pia'),
  ('a0000000-0000-4000-8000-000000000902', 'a1000000-0000-4000-8000-000000000902', 'quinn');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000090a', 'organization/ztest-series', 'organization', 'ztest-series', 'Series', 'x');

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000090a', 'a0000000-0000-4000-8000-000000000901', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000090a', 'a0000000-0000-4000-8000-000000000902', 'admin',       'employer', 'github_oauth');

insert into dojo.repositories (id, tenant_id, repo_key, name, provider) values
  ('90000000-0000-4000-8000-000000000901', 'b0000000-0000-4000-8000-00000000090a', 'github.com/ztest-series/r1', 'r1', 'github'),
  ('90000000-0000-4000-8000-000000000902', 'b0000000-0000-4000-8000-00000000090a', 'github.com/ztest-series/r2', 'r2', 'github');
insert into dojo.repository_roles (repository_id, principal_id, forge_permission, forge_role) values
  ('90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000901', 'write', 'contributor'),
  ('90000000-0000-4000-8000-000000000902', 'a0000000-0000-4000-8000-000000000901', 'write', 'contributor'),
  ('90000000-0000-4000-8000-000000000901', 'a0000000-0000-4000-8000-000000000902', 'write', 'contributor');

insert into dojo.repository_metrics (tenant_id, repository_id, metric_id, scope, principal_id, computed_on, value, props)
select 'b0000000-0000-4000-8000-00000000090a'::uuid, v.repo::uuid, m.id, v.scope, v.who::uuid,
       current_date - v.ago, v.value, v.props::jsonb
  from (values
    ('90000000-0000-4000-8000-000000000901', 'user', 'a0000000-0000-4000-8000-000000000901',  1, 0.5, '{"denominator": 4}'),
    ('90000000-0000-4000-8000-000000000902', 'user', 'a0000000-0000-4000-8000-000000000901',  1, 1.0, '{"denominator": 2}'),
    ('90000000-0000-4000-8000-000000000901', 'user', 'a0000000-0000-4000-8000-000000000901',  0, 3,   '{}'),
    ('90000000-0000-4000-8000-000000000901', 'user', 'a0000000-0000-4000-8000-000000000902',  0, 7,   '{}'),
    ('90000000-0000-4000-8000-000000000901', 'repo', 'a0000000-0000-4000-8000-000000000901',  0, 19,  '{}'),
    ('90000000-0000-4000-8000-000000000901', 'user', 'a0000000-0000-4000-8000-000000000901', 30, 23,  '{}')
  ) as v(repo, scope, who, ago, value, props)
 cross join (select id from dojo.metric_catalogue order by key limit 1) m;

set local role authenticated;
select set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000901"}', true);

do $$
declare
    got text;
begin
    select string_agg(
             (current_date - computed_on) || 'd=' || total || '/'
             || coalesce(round(weighted, 4)::text, 'null') || '/'
             || coalesce(denominator::text, 'null') || '/' || repositories,
             ',' order by computed_on)
      into got
      from dojo.my_metric_series(current_date - 7)
     where metric_key = (select key from dojo.metric_catalogue order by key limit 1);
    if got is distinct from '1d=1.5/0.6667/6/2,0d=3/null/null/1' then
        raise exception 'my_metric_series returned [%], expected [1d=1.5/0.6667/6/2,0d=3/null/null/1]', got;
    end if;
end $$;

-- The colleague is the dōjō's admin and may read every row, yet their series holds only their own.
select set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000902"}', true);
do $$
declare
    got text;
begin
    select string_agg(total::text, ',') into got from dojo.my_metric_series(current_date - 7);
    if got is distinct from '7' then
        raise exception 'my_metric_series for the colleague returned [%], expected [7]', got;
    end if;
end $$;

reset role;
rollback;
