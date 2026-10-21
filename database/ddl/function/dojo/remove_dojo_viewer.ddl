set search_path to dojo, extensions;

-- Remove a viewer from a dōjō (#307 D8), and with it every workspace view they hold
-- there. Admins of the dōjō only.
create or replace function dojo.remove_dojo_viewer(p_tenant uuid, p_principal uuid)
    returns void
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
begin
    if not dojo.tenant_role_at_least(p_tenant, 'admin') then
        raise exception 'only an admin of this dōjō removes its viewers'
            using errcode = 'insufficient_privilege';
    end if;
    delete from dojo.workspace_viewers v
     using dojo.workspaces p
     where v.workspace_id = p.id and p.tenant_id = p_tenant and v.principal_id = p_principal
       and not exists (select 1 from dojo.memberships m
                        where m.tenant_id = p_tenant and m.user_id = p_principal and m.disabled_at is null);
    delete from dojo.dojo_viewers where tenant_id = p_tenant and principal_id = p_principal;
end;
$$;

revoke all on function dojo.remove_dojo_viewer(uuid, uuid) from public;
grant execute on function dojo.remove_dojo_viewer(uuid, uuid) to authenticated;

comment on function dojo.remove_dojo_viewer(uuid, uuid) is
'Remove a dōjō viewer and their workspace views there (#307 D8). Admins only.';
