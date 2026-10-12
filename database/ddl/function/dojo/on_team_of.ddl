set search_path to dojo, extensions;

create or replace function dojo.on_team_of(p_project uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select exists (
        select 1
          from dojo.team_projects tp
          join dojo.teams t         on t.id = tp.team_id
          join dojo.projects p      on p.id = tp.project_id and p.tenant_id = t.tenant_id
          join dojo.team_members tm on tm.team_id = tp.team_id
         where tp.project_id = p_project
           and tm.principal_id = dojo.current_principal_id()
    );
$$;

revoke all on function dojo.on_team_of(uuid) from public;
grant execute on function dojo.on_team_of(uuid) to authenticated;

comment on function dojo.on_team_of(uuid) is
'Whether the signed-in principal is on a team that owns the project — the
"team" scope of the access matrix. Only while the project is homed in the
team''s tenant: a project follows its binding, and a link left from a tenant it
has left (or from before it went personal) grants nothing.';
