set search_path to dojo, extensions;

create or replace function dojo.leads_team_of(p_project uuid)
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
           and tm.role >= 'lead'::dojo.member_role
    );
$$;

revoke all on function dojo.leads_team_of(uuid) from public;
grant execute on function dojo.leads_team_of(uuid) to authenticated;

comment on function dojo.leads_team_of(uuid) is
'Whether the signed-in principal LEADS a team that owns the project — the
"team-lead" scope that unlocks cross-user project data (decision 2026-10-12,
docs/spec/dojo/access-matrix.md). The TEAM role decides it, not the tenant role:
a tenant lead on no team is not a team lead.';
