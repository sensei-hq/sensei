set search_path to dojo, extensions;

-- Grant the read-only project view (#307 D8) to everyone on a team at once (D7).
-- Admins of the project's dōjō only, and only with a team of that same dōjō. It is
-- a grant to each person now on the team — later members are not added, and
-- deleting the team leaves the grants.
create or replace function dojo.grant_project_viewer_to_team(p_project uuid, p_team uuid)
    returns integer
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_count integer;
begin
    if not dojo.administers_project(p_project)
       or not exists (select 1 from dojo.teams t join dojo.projects p on p.tenant_id = t.tenant_id
                       where t.id = p_team and p.id = p_project) then
        raise exception 'only an admin of this project''s dōjō grants viewers, with one of its own teams'
            using errcode = 'insufficient_privilege';
    end if;
    insert into dojo.project_viewers (project_id, principal_id, granted_by)
    select p_project, tm.principal_id, dojo.current_principal_id()
      from dojo.team_members tm
     where tm.team_id = p_team
    on conflict (project_id, principal_id) do nothing;
    get diagnostics v_count = row_count;
    return v_count;
end;
$$;

revoke all on function dojo.grant_project_viewer_to_team(uuid, uuid) from public;
grant execute on function dojo.grant_project_viewer_to_team(uuid, uuid) to authenticated;

comment on function dojo.grant_project_viewer_to_team(uuid, uuid) is
'Grant the project viewer to each person now on a team (#307 D7, D8). Admins of the
project''s dōjō, with a team of that dōjō. Returns how many grants were added.';
