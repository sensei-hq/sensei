set search_path to dojo, extensions;

-- Whether the signed-in principal leads `p_principal` in the dōjō `p_tenant`
-- (#307 R8, D38): the caller's ACTIVE membership there is `lead` or above, and the
-- two share a team of that dōjō on which `p_principal` is an active member.
--
-- A team is still a grouping (D7): this answers "whose numbers does a team lead
-- see", never "which repository may they read" — can_read_repository_metric pairs
-- it with the caller's own repository access (D5). The team and the lead role are
-- both the row's dōjō's, so a lead in one dōjō leads no one in another.
--
-- SECURITY DEFINER for the reason can_read_repository_metric gives: the caller
-- learns only the boolean, not the memberships it traverses.
create or replace function dojo.leads_team_member(p_principal uuid, p_tenant uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select dojo.tenant_role_at_least(p_tenant, 'lead')
       and exists (
           select 1
             from dojo.team_members mine
             join dojo.teams t
               on t.id = mine.team_id
              and t.tenant_id = p_tenant
             join dojo.team_members theirs
               on theirs.team_id = mine.team_id
              and theirs.principal_id = p_principal
             join dojo.memberships m
               on m.tenant_id = p_tenant
              and m.user_id = p_principal
              and m.disabled_at is null
            where mine.principal_id = dojo.current_principal_id()
       );
$$;

revoke all on function dojo.leads_team_member(uuid, uuid) from public;
grant execute on function dojo.leads_team_member(uuid, uuid) to authenticated;

comment on function dojo.leads_team_member(uuid, uuid) is
'Whether the signed-in principal is a lead (or above) of the dōjō and shares one of
its teams with p_principal, an active member there (#307 R8, D38). Grants nothing
alone: can_read_repository_metric pairs it with the caller''s repository access (D5).';
