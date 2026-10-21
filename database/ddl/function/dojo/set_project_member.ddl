set search_path to dojo, extensions;

-- Put someone on a project with a role, or change it (#307 A1). The caller must be
-- a maintainer or above ON THIS PROJECT; the person must be an active member of
-- the project's dōjō; admin is never a project role.
create or replace function dojo.set_project_member(
    p_project   uuid,
    p_principal uuid,
    p_role      dojo.member_role
)
    returns void
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
begin
    if not dojo.project_role_at_least(p_project, 'maintainer') then
        raise exception 'only a maintainer or above on this project sets its roles'
            using errcode = 'insufficient_privilege';
    end if;
    if p_role = 'admin' then
        raise exception 'admin is dōjō-wide, not a project role'
            using errcode = 'invalid_parameter_value';
    end if;
    if not exists (
        select 1
          from dojo.projects p
          join dojo.memberships m on m.tenant_id = p.tenant_id
         where p.id = p_project
           and m.user_id = p_principal
           and m.disabled_at is null
    ) then
        raise exception 'that person is not a member of this project''s dōjō'
            using errcode = 'foreign_key_violation';
    end if;

    insert into dojo.project_members (project_id, principal_id, role, added_by)
    values (p_project, p_principal, p_role, dojo.current_principal_id())
    on conflict (project_id, principal_id)
    do update set role = excluded.role, updated_at = now();
end;
$$;

revoke all on function dojo.set_project_member(uuid, uuid, dojo.member_role) from public;
grant execute on function dojo.set_project_member(uuid, uuid, dojo.member_role) to authenticated;

comment on function dojo.set_project_member(uuid, uuid, dojo.member_role) is
'Put a member of the project''s dōjō on the project with a role, or change it.
Maintainer and above on the project; never admin.';
