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
          join dojo.team_members tm on tm.team_id = tp.team_id
         where tp.project_id = p_project
           and tm.principal_id = dojo.current_principal_id()
    );
$$;

revoke all on function dojo.on_team_of(uuid) from public;
grant execute on function dojo.on_team_of(uuid) to authenticated;

comment on function dojo.on_team_of(uuid) is
'Whether the signed-in principal is on a team that owns the project — the
"team" scope of the access matrix.';
