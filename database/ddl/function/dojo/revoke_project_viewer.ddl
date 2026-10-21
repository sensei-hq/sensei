set search_path to dojo, extensions;

-- Take away someone's read-only view of a project (#307 D8). Admins only.
create or replace function dojo.revoke_project_viewer(p_project uuid, p_principal uuid)
    returns void
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
begin
    if not dojo.administers_project(p_project) then
        raise exception 'only an admin of this project''s dōjō revokes its viewers'
            using errcode = 'insufficient_privilege';
    end if;
    delete from dojo.project_viewers where project_id = p_project and principal_id = p_principal;
end;
$$;

revoke all on function dojo.revoke_project_viewer(uuid, uuid) from public;
grant execute on function dojo.revoke_project_viewer(uuid, uuid) to authenticated;

comment on function dojo.revoke_project_viewer(uuid, uuid) is
'Revoke the read-only project view (#307 D8). Admins of the project''s dōjō only.';
