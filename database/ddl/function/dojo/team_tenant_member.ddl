set search_path to dojo, extensions;

-- Whether the signed-in principal is an active member of the team's dōjō. SECURITY
-- DEFINER so team RLS does not depend on dojo.teams' own RLS.
create or replace function dojo.team_tenant_member(p_team uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select exists (select 1 from dojo.teams t where t.id = p_team and dojo.is_member(t.tenant_id));
$$;

revoke all on function dojo.team_tenant_member(uuid) from public;
grant execute on function dojo.team_tenant_member(uuid) to authenticated;

comment on function dojo.team_tenant_member(uuid) is
'Whether the signed-in principal is an active member of the team''s dōjō.';
