set search_path to dojo, extensions;

-- Take someone off a team (#307 A3). Admins of its dōjō only.
create or replace function dojo.remove_team_member(p_team uuid, p_principal uuid)
    returns void
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
begin
    if not dojo.administers_team(p_team) then
        raise exception 'only an admin of this team''s dōjō changes its members'
            using errcode = 'insufficient_privilege';
    end if;
    delete from dojo.team_members where team_id = p_team and principal_id = p_principal;
end;
$$;

revoke all on function dojo.remove_team_member(uuid, uuid) from public;
grant execute on function dojo.remove_team_member(uuid, uuid) to authenticated;

comment on function dojo.remove_team_member(uuid, uuid) is
'Take someone off a team. Admins of its dōjō only.';
