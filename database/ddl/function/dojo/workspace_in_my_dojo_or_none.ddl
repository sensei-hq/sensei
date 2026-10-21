set search_path to dojo, extensions;

-- Whether a relay session may name this workspace (#252, #299): none at all, or a
-- workspace in a dōjō the signed-in principal is a member of. Workspaces are shared
-- items with no owner, so the runner's membership is what matters — a run is never
-- filed under a workspace in somebody else's dōjō.
create or replace function dojo.workspace_in_my_dojo_or_none(p_workspace uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select p_workspace is null
        or exists (select 1 from dojo.workspaces p
                    where p.id = p_workspace and dojo.is_member(p.tenant_id));
$$;

revoke all on function dojo.workspace_in_my_dojo_or_none(uuid) from public;
grant execute on function dojo.workspace_in_my_dojo_or_none(uuid) to authenticated;

comment on function dojo.workspace_in_my_dojo_or_none(uuid) is
'True for null or a workspace in a dōjō the signed-in principal is a member of —
the check on a relay session''s workspace_id (#252, #299).';
