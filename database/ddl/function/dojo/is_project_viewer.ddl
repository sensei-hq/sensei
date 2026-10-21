set search_path to dojo, extensions;

-- Whether the signed-in principal holds a viewer grant on the project (#307 D8).
create or replace function dojo.is_project_viewer(p_project uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select exists (
        select 1 from dojo.project_viewers v
         where v.project_id = p_project
           and v.principal_id = dojo.current_principal_id()
    );
$$;

revoke all on function dojo.is_project_viewer(uuid) from public;
grant execute on function dojo.is_project_viewer(uuid) to authenticated;

comment on function dojo.is_project_viewer(uuid) is
'Whether the signed-in principal is a read-only viewer of the project (#307 D8).';
