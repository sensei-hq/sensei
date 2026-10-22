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
-- `total` and `mean` are over the daily user-scoped rows in the window, across the
-- repositories readable; the caller picks the one that fits the metric (a count
-- per day sums, a rate averages).
create or replace function dojo.team_member_metrics(
    p_tenant     uuid,
    p_principals uuid[],
    p_keys       text[],
    p_since      date
) returns table (
    principal_id uuid,
    metric_key   text,
    total        numeric,
    mean         numeric,
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
         , avg(rm.value)
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
'Per-person totals and means of daily user-scoped metric rows in one dōjō since a
day (#307 R8, D38). SECURITY INVOKER: dojo.repository_metrics'' policy decides which
rows count, so a caller learns only what they could read row by row.';
