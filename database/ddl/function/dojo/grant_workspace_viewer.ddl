set search_path to dojo, extensions;

-- Grant someone the read-only view of a workspace (#307 D8). Only an admin of the
-- workspace's dōjō, and only to someone in it — a member, or a dōjō viewer invited in.
create or replace function dojo.grant_workspace_viewer(p_workspace uuid, p_principal uuid)
    returns void
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
begin
    if not dojo.administers_workspace(p_workspace) then
        raise exception 'only an admin of this workspace''s dōjō grants its viewers'
            using errcode = 'insufficient_privilege';
    end if;
    -- In the dōjō first (D8, revised): a member, or someone invited as a dōjō viewer.
    if not exists (
        select 1 from dojo.workspaces p
         where p.id = p_workspace
           and (exists (select 1 from dojo.memberships m
                         where m.tenant_id = p.tenant_id and m.user_id = p_principal and m.disabled_at is null)
                or exists (select 1 from dojo.dojo_viewers v
                            where v.tenant_id = p.tenant_id and v.principal_id = p_principal))
    ) then
        raise exception 'invite them to this dōjō as a viewer first'
            using errcode = 'foreign_key_violation';
    end if;
    insert into dojo.workspace_viewers (workspace_id, principal_id, granted_by)
    values (p_workspace, p_principal, dojo.current_principal_id())
    on conflict (workspace_id, principal_id) do nothing;
end;
$$;

revoke all on function dojo.grant_workspace_viewer(uuid, uuid) from public;
grant execute on function dojo.grant_workspace_viewer(uuid, uuid) to authenticated;

comment on function dojo.grant_workspace_viewer(uuid, uuid) is
'Grant the read-only workspace view (#307 D8). Admins of the workspace''s dōjō only, to
a member or a dōjō viewer of it.';
