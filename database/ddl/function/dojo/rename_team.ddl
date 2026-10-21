set search_path to dojo, extensions;

-- Rename a team (#307 A3). Admins of its dōjō only.
create or replace function dojo.rename_team(p_team uuid, p_name text)
    returns void
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
begin
    if not dojo.administers_team(p_team) then
        raise exception 'only an admin of this team''s dōjō renames it'
            using errcode = 'insufficient_privilege';
    end if;
    if nullif(btrim(p_name), '') is null then
        raise exception 'a team needs a name' using errcode = 'invalid_parameter_value';
    end if;
    update dojo.teams set name = btrim(p_name) where id = p_team;
end;
$$;

revoke all on function dojo.rename_team(uuid, text) from public;
grant execute on function dojo.rename_team(uuid, text) to authenticated;

comment on function dojo.rename_team(uuid, text) is 'Rename a team. Admins of its dōjō only.';
