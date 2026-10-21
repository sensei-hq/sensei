set search_path to dojo, extensions;

-- Delete a team (#307 A3). Admins of its dōjō only. Viewer grants made through it
-- stay: a team grant is a grant to each person, not to the team.
create or replace function dojo.delete_team(p_team uuid)
    returns void
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
begin
    if not dojo.administers_team(p_team) then
        raise exception 'only an admin of this team''s dōjō deletes it'
            using errcode = 'insufficient_privilege';
    end if;
    delete from dojo.teams where id = p_team;
end;
$$;

revoke all on function dojo.delete_team(uuid) from public;
grant execute on function dojo.delete_team(uuid) to authenticated;

comment on function dojo.delete_team(uuid) is
'Delete a team. Admins of its dōjō only; viewer grants made through it stay.';
