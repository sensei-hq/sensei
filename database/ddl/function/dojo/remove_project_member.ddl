set search_path to dojo, extensions;

-- Take someone off a project (#307 A1); they keep their dōjō role. The caller must
-- be a maintainer or above on this project.
create or replace function dojo.remove_project_member(p_project uuid, p_principal uuid)
    returns void
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
begin
    if not dojo.project_role_at_least(p_project, 'maintainer') then
        raise exception 'only a maintainer or above on this project changes its members'
            using errcode = 'insufficient_privilege';
    end if;
    delete from dojo.project_members where project_id = p_project and principal_id = p_principal;
end;
$$;

revoke all on function dojo.remove_project_member(uuid, uuid) from public;
grant execute on function dojo.remove_project_member(uuid, uuid) to authenticated;

comment on function dojo.remove_project_member(uuid, uuid) is
'Take a person off a project; their dōjō role holds. Maintainer and above on the project.';
