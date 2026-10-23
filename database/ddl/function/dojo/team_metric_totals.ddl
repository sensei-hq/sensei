set search_path to dojo, extensions;

-- Team totals for any member (#307 R8 follow-up, 2026-10-22): for each team the
-- caller is ON — every team of the dōjō for its admins — each metric's total over
-- the window, never a person's number.
--
-- What counts: daily user-scoped rows of the team's ACTIVE members, in this dōjō,
-- on repositories the CALLER can read (can_read_repository — access lives on
-- repositories, D5), since p_since. `total` sums; `weighted` pools a rate by its
-- own denominator, Σ(value × props.denominator) / Σ props.denominator (null when a
-- row lacks one) — the same pooling as team_member_metrics.
--
-- k-anonymity: a (team, metric) is answered only when at least
-- dojo.k_anonymity_min() distinct people sent it, so no member can recover a
-- colleague's number from a total over one or two people.
--
-- SECURITY DEFINER: it must read teammates' rows the caller may not read one by
-- one, and returns only the aggregate. The caller comes from the session, never
-- an argument; a non-member (a viewer included) gets nothing.
create or replace function dojo.team_metric_totals(
    p_tenant uuid,
    p_keys   text[],
    p_since  date
) returns table (
    team_id      uuid,
    metric_key   text,
    total        numeric,
    weighted     numeric,
    contributors integer
)
language sql
stable
security definer
set search_path = dojo, pg_temp
as $$
    with my_teams as (
        select t.id
          from dojo.teams t
         where t.tenant_id = p_tenant
           and dojo.is_member(p_tenant)
           and (dojo.tenant_role_at_least(p_tenant, 'admin')
                or exists (select 1 from dojo.team_members me
                            where me.team_id = t.id
                              and me.principal_id = dojo.current_principal_id()))
    )
    select tm.team_id
         , c.key
         , sum(rm.value)
         , case when bool_and(coalesce((rm.props->>'denominator')::numeric, 0) > 0)
                then sum(rm.value * (rm.props->>'denominator')::numeric)
                   / sum((rm.props->>'denominator')::numeric)
           end
         , count(distinct rm.principal_id)::integer
      from my_teams mt
      join dojo.team_members tm on tm.team_id = mt.id
      join dojo.memberships m
        on m.tenant_id = p_tenant
       and m.user_id = tm.principal_id
       and m.disabled_at is null
      join dojo.repository_metrics rm
        on rm.tenant_id = p_tenant
       and rm.principal_id = tm.principal_id
       and rm.scope = 'user'
       and rm.grain = 'daily'
       and rm.computed_on >= p_since
      join dojo.metric_catalogue c on c.id = rm.metric_id and c.key = any (p_keys)
     where dojo.can_read_repository(rm.repository_id, p_tenant)
     group by tm.team_id, c.key
    having count(distinct rm.principal_id) >= dojo.k_anonymity_min();
$$;

revoke all on function dojo.team_metric_totals(uuid, text[], date) from public;
grant execute on function dojo.team_metric_totals(uuid, text[], date) to authenticated;

comment on function dojo.team_metric_totals(uuid, text[], date) is
'Per-team totals and denominator-pooled rates for the caller''s teams (every team for
an admin), over active members'' rows on repositories the caller can read; a
(team, metric) only when at least k_anonymity_min() people sent it (#307 R8).';
