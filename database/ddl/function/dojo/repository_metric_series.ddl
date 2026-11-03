set search_path to dojo, extensions;

-- One repository's daily metric rows, from a day on — the per-repository read
-- behind the workspace charts (WP4).
--
-- SECURITY INVOKER, deliberately: it reads dojo.repository_metrics AS THE
-- CALLER, so the table's own policy (can_read_repository_metric) decides row by
-- row — a role holder its repo rows, nobody another's user rows below
-- maintainer — and no second statement of the rule can drift from the first.
-- The metric key is resolved, not passed through as a uuid the screen cannot
-- read: `sensei.metrics.id` differs between the two planes.
--
-- No repo_key: the caller names the repository it asked about. The workspace
-- rollup carries the key, because it answers for many.
create or replace function dojo.repository_metric_series(
    p_repository_id uuid,
    p_since         date
)
returns table (
    repository_id uuid,
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
security invoker
set search_path = dojo, pg_temp
as $$
    select rm.repository_id
         , c.key
         , rm.scope
         , rm.principal_id
         , rm.computed_on
         , rm.grain
         , rm.value
         , rm.props
      from dojo.repository_metrics rm
      join dojo.metric_catalogue c on c.id = rm.metric_id
     where rm.repository_id = p_repository_id
       and rm.grain = 'daily'
       and rm.computed_on >= p_since;
$$;

revoke all on function dojo.repository_metric_series(uuid, date) from public;
grant execute on function dojo.repository_metric_series(uuid, date) to authenticated;

comment on function dojo.repository_metric_series(uuid, date) is
'One repository''s daily metric rows from a day on, with the metric key resolved.
SECURITY INVOKER: the table''s own policy decides row by row (a role holder its
repo rows, another''s user rows for a maintainer or above).';
