set search_path to dojo, extensions;

-- Take away someone's read-only view of a workspace (#307 D8). Admins only.
create or replace function dojo.revoke_workspace_viewer(p_workspace uuid, p_principal uuid)
    returns void
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
begin
    if not dojo.administers_workspace(p_workspace) then
        raise exception 'only an admin of this workspace''s dōjō revokes its viewers'
            using errcode = 'insufficient_privilege';
    end if;
    delete from dojo.workspace_viewers where workspace_id = p_workspace and principal_id = p_principal;
    if found then
        perform dojo.append_audit_event(
            (select tenant_id from dojo.workspaces where id = p_workspace), dojo.current_principal_id(),
            'workspace_view_revoked', jsonb_build_object('workspace_id', p_workspace, 'principal_id', p_principal));
    end if;
end;
$$;

revoke all on function dojo.revoke_workspace_viewer(uuid, uuid) from public;
grant execute on function dojo.revoke_workspace_viewer(uuid, uuid) to authenticated;

comment on function dojo.revoke_workspace_viewer(uuid, uuid) is
'Revoke the read-only workspace view (#307 D8). Admins of the workspace''s dōjō only.';
