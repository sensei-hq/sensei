set search_path to dojo, extensions;

-- A workspace's metric series, from a day on — the read behind its charts
-- (WP4): every repository it holds, each row the caller may read, plus the
-- aggregate view a workspace viewer grant gives (#307 D8).
--
-- THREE branches over one shared CTE, one row shape:
--   detail:         the workspace's daily rows WHERE the caller may read them —
--                   the same can_read_repository_metric the table's policy
--                   applies, called not copied, so the rule still has one
--                   definition.
--   viewer repo:    repo-scoped rows the grant reveals that detail did NOT
--                   already give (a viewer reads per-repository splits); the
--                   NOT keeps a member's grant from doubling their detail rows.
--   viewer pooled:  user-scoped readings pooled to one row per repository,
--                   metric and day, principal NULL, scope 'pooled' — never
--                   per-person, and never under scope 'user', so a screen
--                   summing that scope cannot count the same people twice.
--                   One reading per person per day: a person's commit rows
--                   collapse to the latest by authored_at desc nulls last,
--                   pushed_at desc, commit_sha desc (the L0 total order, same
--                   on both planes — a keyless row never beats a keyed one,
--                   nothing is dropped); commit-less rows add as now. A pool
--                   with fewer than 2 contributors is withheld (F4, which
--                   supersedes D10 for viewer pools). `value` sums (a count);
--                   rates pool by the same rule as team_member_metrics, with
--                   the pooling carried in props (pooled_rows over the
--                   readings, contributors, numerator, denominator — the rate
--                   pair null unless every reading has a positive
--                   denominator), so a reader pools across days the same way
--                   instead of summing rates. A screen shows `contributors`
--                   (people), never `pooled_rows` (readings).
--
-- SECURITY DEFINER because the viewer branches read rows the caller cannot read
-- row by row; what leaves is repo-scoped rows and sums, never another person's
-- row — and only for a grantee of THIS workspace (is_workspace_viewer, called
-- not copied). A member with no grant reads detail alone; a non-member reads
-- nothing.
create or replace function dojo.workspace_metric_series(
    p_workspace_id uuid,
    p_since        date
)
returns table (
    repository_id uuid,
    repo_key      text,
    metric_key    text,
    scope         text,
    principal_id  uuid,
    computed_on   date,
    grain         text,
    value         numeric,
    props         jsonb
)
language sql
stable
security definer
set search_path = dojo, pg_temp
as $$
    with ws as (
        select rm.repository_id
             , rm.scope
             , rm.principal_id
             , rm.computed_on
             , rm.grain
             , rm.value
             , rm.props
             , rm.tenant_id
             , rm.commit_sha
             , rm.pushed_at
             , r.repo_key
             , c.key as metric_key
          from dojo.repository_metrics rm
          join dojo.repositories_in_workspaces riw
            on riw.repository_id = rm.repository_id
           and riw.workspace_id = p_workspace_id
          join dojo.repositories r on r.id = rm.repository_id
          join dojo.metric_catalogue c on c.id = rm.metric_id
         where rm.grain = 'daily'
           and rm.computed_on >= p_since
    )
    , latest_commit as (
        select distinct on (ws.repository_id, ws.metric_key, ws.computed_on, ws.principal_id)
               ws.repository_id
             , ws.repo_key
             , ws.metric_key
             , ws.computed_on
             , ws.grain
             , ws.value
             , ws.props
             , ws.principal_id
          from ws
         where ws.scope = 'user'
           and ws.commit_sha is not null
         order by ws.repository_id, ws.metric_key, ws.computed_on, ws.principal_id,
                  (ws.props->>'authored_at')::timestamptz desc nulls last,
                  ws.pushed_at desc, ws.commit_sha desc
    )
    , pooled_readings as (
        select * from latest_commit
        union all
        select ws.repository_id
             , ws.repo_key
             , ws.metric_key
             , ws.computed_on
             , ws.grain
             , ws.value
             , ws.props
             , ws.principal_id
          from ws
         where ws.scope = 'user'
           and ws.commit_sha is null
    )
    select ws.repository_id
         , ws.repo_key
         , ws.metric_key
         , ws.scope
         , ws.principal_id
         , ws.computed_on
         , ws.grain
         , ws.value
         , ws.props
      from ws
     where dojo.can_read_repository_metric(ws.repository_id, ws.scope, ws.principal_id, ws.tenant_id)
    union all
    select ws.repository_id
         , ws.repo_key
         , ws.metric_key
         , ws.scope
         , ws.principal_id
         , ws.computed_on
         , ws.grain
         , ws.value
         , ws.props
      from ws
     where ws.scope = 'repo'
       and dojo.is_workspace_viewer(p_workspace_id)
       and not dojo.can_read_repository_metric(ws.repository_id, ws.scope, ws.principal_id, ws.tenant_id)
    union all
    select pr.repository_id
         , pr.repo_key
         , pr.metric_key
         , 'pooled'
         , null::uuid
         , pr.computed_on
         , pr.grain
         , sum(pr.value)
         , jsonb_build_object(
               'pooled_rows', count(*),
               'contributors', count(distinct pr.principal_id),
               'numerator', case when bool_and(coalesce((pr.props->>'denominator')::numeric, 0) > 0)
                                 then sum(pr.value * (pr.props->>'denominator')::numeric)
                            end,
               'denominator', case when bool_and(coalesce((pr.props->>'denominator')::numeric, 0) > 0)
                                   then sum((pr.props->>'denominator')::numeric)
                              end)
      from pooled_readings pr
     where dojo.is_workspace_viewer(p_workspace_id)
     group by pr.repository_id, pr.repo_key, pr.metric_key, pr.computed_on, pr.grain
    having count(distinct pr.principal_id) >= 2;
$$;

revoke all on function dojo.workspace_metric_series(uuid, date) from public;
grant execute on function dojo.workspace_metric_series(uuid, date) to authenticated;

comment on function dojo.workspace_metric_series(uuid, date) is
'A workspace''s daily metric series from a day on, across the repositories it
holds: the detail rows the caller may read (can_read_repository_metric, as the
table''s policy), plus for a workspace viewer the D8 aggregate view — repo rows
as-is, user readings pooled per repository, metric and day under scope
''pooled'' with no principal (one reading per person per day, latest commit
first; pools under 2 contributors withheld, F4), rates by the same rule as
team_member_metrics.
SECURITY DEFINER: the viewer branches read rows the caller cannot read row by
row, and return only repo rows and sums.';
