set search_path to dojo, extensions;

-- Grant someone the read-only view of a project (#307 D8). Only an admin of the
-- project's dōjō; the viewer may be anyone — a member, or an outsider in no dōjō.
create or replace function dojo.grant_project_viewer(p_project uuid, p_principal uuid)
    returns void
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
begin
    if not dojo.administers_project(p_project) then
        raise exception 'only an admin of this project''s dōjō grants its viewers'
            using errcode = 'insufficient_privilege';
    end if;
    insert into dojo.project_viewers (project_id, principal_id, granted_by)
    values (p_project, p_principal, dojo.current_principal_id())
    on conflict (project_id, principal_id) do nothing;
end;
$$;

revoke all on function dojo.grant_project_viewer(uuid, uuid) from public;
grant execute on function dojo.grant_project_viewer(uuid, uuid) to authenticated;

comment on function dojo.grant_project_viewer(uuid, uuid) is
'Grant the read-only project view (#307 D8). Admins of the project''s dōjō only;
the viewer may be an outsider.';
