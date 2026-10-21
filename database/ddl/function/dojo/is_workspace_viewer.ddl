set search_path to dojo, extensions;

-- Whether the signed-in principal holds a viewer grant on the workspace (#307 D8).
create or replace function dojo.is_workspace_viewer(p_workspace uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select exists (
        select 1 from dojo.workspace_viewers v
         where v.workspace_id = p_workspace
           and v.principal_id = dojo.current_principal_id()
    );
$$;

revoke all on function dojo.is_workspace_viewer(uuid) from public;
grant execute on function dojo.is_workspace_viewer(uuid) to authenticated;

comment on function dojo.is_workspace_viewer(uuid) is
'Whether the signed-in principal is a read-only viewer of the workspace (#307 D8).';
