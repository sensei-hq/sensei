set search_path to dojo, extensions;

-- Who may read a workspace (#299): a workspace is ONE SHARED item with no owner, so
-- whoever may read one of its repositories may read it — the repository's
-- access (dojo.can_read_repository) is the workspace's — and so may its read-only
-- viewers (#307 D8), who may be outsiders, and its dōjō's admins, who see it
-- before it holds any repository.
--
-- SECURITY DEFINER because the repository links (dojo.repositories_in_workspaces)
-- admit no direct reads; the caller learns only the boolean. One definition, for
-- the workspaces read policy and anything else that asks.
create or replace function dojo.can_read_workspace(p_workspace uuid)
    returns boolean
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
    select dojo.is_workspace_viewer(p_workspace)
        or dojo.administers_workspace(p_workspace)
        or exists (select 1
                     from dojo.repositories_in_workspaces rip
                    where rip.workspace_id = p_workspace
                      and dojo.can_read_repository(rip.repository_id, rip.tenant_id));
$$;

revoke all on function dojo.can_read_workspace(uuid) from public;
grant execute on function dojo.can_read_workspace(uuid) to authenticated;

comment on function dojo.can_read_workspace(uuid) is
'Read authorization for a shared workspace (#299): true when the caller may read at
least one of its repositories (dojo.can_read_repository), is its viewer (#307 D8),
or administers its dōjō — so an admin sees a workspace before it holds a repository,
to fill it and grant viewers on it. SECURITY DEFINER so the caller learns only the
boolean.';
