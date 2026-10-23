set search_path to dojo, extensions;

-- Per-person metric totals for a set of people in one dōjō, since a day (#307 R8,
-- D38) — the lead's team view reads this.
--
-- SECURITY INVOKER, deliberately: it reads dojo.repository_metrics AS THE CALLER,
-- so the table's policy (can_read_repository_metric) decides which rows count.
-- Asked about anyone, it answers only with what the caller could read row by row
-- — a lead their teammates on repositories the lead can read, a contributor only
-- themselves. It aggregates so the console never pages raw rows (a page cap would
-- drop rows without saying so).
--
-- Over the daily user-scoped rows in the window, across the repositories readable:
-- `total` sums (a count per day); `weighted` pools a rate by its own denominator,
-- Σ(value × props.denominator) / Σ props.denominator — Σ numerator / Σ
-- denominator, the daemon's own pooling — and is null when any row lacks a
-- positive denominator (never an unweighted mean). Unreleased when its return
-- type changed, hence the drop.
drop function if exists dojo.team_member_metrics(uuid, uuid[], text[], date);
create or replace function dojo.team_member_metrics(
    p_tenant     uuid,
    p_principals uuid[],
    p_keys       text[],
    p_since      date
) returns table (
    principal_id uuid,
    metric_key   text,
    total        numeric,
    weighted     numeric,
    days         integer,
    repositories integer,
    last_on      date
)
language sql
stable
security invoker
set search_path = dojo, pg_temp
as $$
    select rm.principal_id
         , c.key
         , sum(rm.value)
         , case when bool_and(coalesce((rm.props->>'denominator')::numeric, 0) > 0)
                then sum(rm.value * (rm.props->>'denominator')::numeric)
                   / sum((rm.props->>'denominator')::numeric)
           end
         , count(distinct rm.computed_on)::integer
         , count(distinct rm.repository_id)::integer
         , max(rm.computed_on)
      from dojo.repository_metrics rm
      join dojo.metric_catalogue c on c.id = rm.metric_id
     where rm.tenant_id = p_tenant
       and rm.scope = 'user'
       and rm.grain = 'daily'
       and rm.principal_id = any (p_principals)
       and c.key = any (p_keys)
       and rm.computed_on >= p_since
     group by rm.principal_id, c.key;
$$;

revoke all on function dojo.team_member_metrics(uuid, uuid[], text[], date) from public;
grant execute on function dojo.team_member_metrics(uuid, uuid[], text[], date) to authenticated;

comment on function dojo.team_member_metrics(uuid, uuid[], text[], date) is
'Per-person totals and denominator-pooled rates of daily user-scoped metric rows in
one dōjō since a day (#307 R8, D38). SECURITY INVOKER: dojo.repository_metrics''
policy decides which rows count, so a caller learns only what they could read row by row.';
