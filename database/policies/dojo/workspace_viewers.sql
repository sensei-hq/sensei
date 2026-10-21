-- RLS for dojo.workspace_viewers (#307 D8).
--
-- A viewer sees their own grants; the dōjō's admins see every grant on its
-- workspaces. Writes go through dojo.grant_workspace_viewer / revoke_workspace_viewer
-- (SECURITY DEFINER, admins only). Covered by database/tests/dojo/workspace_viewers.sql.
alter table dojo.workspace_viewers enable row level security;

grant select on dojo.workspace_viewers to authenticated;
revoke insert, update, delete on dojo.workspace_viewers from authenticated;

drop policy if exists workspace_viewers_select on dojo.workspace_viewers;
create policy workspace_viewers_select
    on dojo.workspace_viewers
    for select
    to authenticated
    using (
        principal_id = dojo.current_principal_id()
        or dojo.administers_workspace(workspace_id)
    );
