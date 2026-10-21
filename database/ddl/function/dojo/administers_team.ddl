set search_path to dojo, extensions;

-- Whether the signed-in principal is an admin of the team's dōjō.
create or replace function dojo.administers_team(p_team uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select exists (select 1 from dojo.teams t
                    where t.id = p_team and dojo.tenant_role_at_least(t.tenant_id, 'admin'));
$$;

revoke all on function dojo.administers_team(uuid) from public;
grant execute on function dojo.administers_team(uuid) to authenticated;

comment on function dojo.administers_team(uuid) is
'Whether the signed-in principal is an admin of the team''s dōjō.';
