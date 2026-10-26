set search_path to dojo, extensions;

-- The caller's own metrics, one row per metric and day, from a day on — the
-- developer console's chart reads this.
--
-- Pools a day's rows across the caller's repositories the way
-- dojo.team_member_metrics pools a period: `total` sums (a count per day);
-- `weighted` is Σ(value × props.denominator) / Σ props.denominator, null when any
-- row lacks a positive denominator (never an unweighted mean); `denominator` is
-- the day's Σ denominator, null with `weighted`, so a reader pools a rate across
-- days by the same rule.
--
-- SECURITY INVOKER: reads dojo.repository_metrics AS THE CALLER, so the table's
-- policy still decides which rows count; the principal filter makes the series
-- the caller's own and nobody else's, whatever else they may read.
-- Unreleased when its return type changed, hence the drop.
drop function if exists dojo.my_metric_series(date);
create or replace function dojo.my_metric_series(p_since date)
returns table (
    metric_key   text,
    computed_on  date,
    total        numeric,
    weighted     numeric,
    denominator  numeric,
    repositories integer
)
language sql
stable
security invoker
set search_path = dojo, pg_temp
as $$
    select c.key
         , rm.computed_on
         , sum(rm.value)
         , case when bool_and(coalesce((rm.props->>'denominator')::numeric, 0) > 0)
                then sum(rm.value * (rm.props->>'denominator')::numeric)
                   / sum((rm.props->>'denominator')::numeric)
           end
         , case when bool_and(coalesce((rm.props->>'denominator')::numeric, 0) > 0)
                then sum((rm.props->>'denominator')::numeric)
           end
         , count(distinct rm.repository_id)::integer
      from dojo.repository_metrics rm
      join dojo.metric_catalogue c on c.id = rm.metric_id
     where rm.scope = 'user'
       and rm.grain = 'daily'
       and rm.principal_id = dojo.current_principal_id()
       and rm.computed_on >= p_since
     group by c.key, rm.computed_on;
$$;

revoke all on function dojo.my_metric_series(date) from public;
grant execute on function dojo.my_metric_series(date) to authenticated;

comment on function dojo.my_metric_series(date) is
'The caller''s own daily user-scoped metric rows from a day on, one row per metric and
day, totals summed and rates pooled by their denominators. SECURITY INVOKER: the table''s
policy decides which rows count, and the series is bound to dojo.current_principal_id().';
