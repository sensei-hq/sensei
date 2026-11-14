set search_path to dojo, extensions;

-- Per-workspace latest activity for the ids passed in that the caller may
-- read: the newest of any reading landed (`repository_metrics.pushed_at`) or
-- graph sync filed (`graph_heads.pushed_at`) across the workspace's
-- repositories. Backs the workspaces sort (most recently active first, quiet
-- last, never invented).
--
-- SECURITY DEFINER, gated per id by `dojo.can_read_workspace` — called, never
-- copied: the links admit no direct reads, so an invoker body fails every
-- caller with `permission denied`. The caller learns activity only for
-- workspaces they may read, which is the same posture as the boolean.
--
-- Workspace level: the latest landing of ANY of the workspace's repositories
-- counts, even ones the caller may not read row by row. A viewer reads the
-- sort key of a workspace they were granted; a timestamp exposes no metric
-- value.
--
-- Each max is taken in its own subquery: readings and graph heads never share
-- a FROM, so no join multiplies readings by branch count (MAX would still
-- answer right; the rows would still cost).
--
-- The reader is `listOrgWorkspaces` / `listUserWorkspaces`, calling it as the
-- signed-in person via rpc with the listed ids. Unknown ids read no row;
-- quiet workspaces read one NULL row.
create or replace function dojo.workspace_activity(p_ids uuid[])
returns table (
    workspace_id uuid
  , tenant_id uuid
  , last_activity_at timestamptz
)
language sql
stable
security definer
set search_path = dojo, pg_temp
as $$
  select w.id
       , w.tenant_id
       , greatest(
             (select max(rm.pushed_at)
                from dojo.repository_metrics rm
                join dojo.repositories_in_workspaces rip on rip.repository_id = rm.repository_id
               where rip.workspace_id = w.id),
             (select max(gh.pushed_at)
                from dojo.graph_heads gh
                join dojo.repositories_in_workspaces rip on rip.repository_id = gh.repository_id
               where rip.workspace_id = w.id))
    from unnest(p_ids) u(id)
    join dojo.workspaces w on w.id = u.id
   where dojo.can_read_workspace(w.id);
$$;

revoke all on function dojo.workspace_activity(uuid[]) from public;
grant execute on function dojo.workspace_activity(uuid[]) to authenticated;

comment on function dojo.workspace_activity(uuid[]) is
'Per-workspace latest activity for the given ids the caller may read: the newest reading landing or graph sync across each workspace''s repositories (pushed_at, never computed_on). SECURITY DEFINER, gated per id by dojo.can_read_workspace — the links admit no direct reads, so an invoker body fails every caller. Workspace level: any repository''s landing counts, even ones the caller may not read; a timestamp exposes no metric value. Read by listOrgWorkspaces / listUserWorkspaces as the signed-in person via rpc. Unknown ids read no row; quiet workspaces read one NULL row.';
