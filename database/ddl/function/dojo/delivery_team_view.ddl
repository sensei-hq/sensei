set search_path to dojo, extensions;

-- What a workspace reader is shown of one item's merged effort (spec
-- tracker-delivery-metrics B16; #353): nothing when fewer than two people
-- contributed (one contributor's "merged" value is that person's own), and never
-- a cost at the contributors' own rates (cost over time is their rate). F4 opens
-- both to the readers allowed per-person rows.
create or replace function dojo.delivery_team_view(p_merged jsonb, p_contributors integer)
    returns jsonb
    language sql
    immutable
    set search_path = dojo, pg_temp
as $$
    select case when p_contributors >= 2 then
        jsonb_build_object(
            'values',  (p_merged -> 'values')  - array['effort_cost', 'rework_cost'],
            'reasons', (p_merged -> 'reasons') - array['effort_cost', 'rework_cost'],
            'partial', (p_merged -> 'partial') - array['effort_cost', 'rework_cost'])
    end;
$$;

revoke all on function dojo.delivery_team_view(jsonb, integer) from public;

comment on function dojo.delivery_team_view(jsonb, integer) is
'B16: a merged item as a workspace reader sees it — NULL from fewer than two contributors, and without effort_cost and rework_cost. Pure (#353).';
