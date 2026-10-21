set search_path to dojo, extensions;

-- Put a member of the dōjō on one of its teams (#307 A3). Admins only; idempotent.
create or replace function dojo.add_team_member(p_team uuid, p_principal uuid)
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
    if not exists (
        select 1 from dojo.teams t
          join dojo.memberships m on m.tenant_id = t.tenant_id
         where t.id = p_team and m.user_id = p_principal and m.disabled_at is null
    ) then
        raise exception 'that person is not a member of this team''s dōjō'
            using errcode = 'foreign_key_violation';
    end if;
    insert into dojo.team_members (team_id, principal_id) values (p_team, p_principal)
    on conflict (team_id, principal_id) do nothing;
end;
$$;

revoke all on function dojo.add_team_member(uuid, uuid) from public;
grant execute on function dojo.add_team_member(uuid, uuid) to authenticated;

comment on function dojo.add_team_member(uuid, uuid) is
'Put a member of the dōjō on one of its teams. Admins only; idempotent.';
